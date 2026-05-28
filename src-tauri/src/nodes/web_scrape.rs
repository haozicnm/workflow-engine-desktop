// nodes/web_scrape.rs — 声明式网页抓取节点
//
// 两种模式：
//   单页：url: "https://example.com"
//   批量：urls: ["https://a.com", "https://b.com"]
//   模式：url_pattern: "https://site.com/page/{{1..10}}"
//
// 批量/模式额外参数：
//   retry: 2            // 每个 URL 最大重试次数（默认 0）
//   retry_delay_ms: 2000 // 重试间隔（默认 1000）
//   delay_between_ms: 1000 // URL 之间的延迟（默认 0，防封）
//   fail_fast: false     // 单个失败是否中止全部（默认 false）
//   excel_output: "result.xlsx" // 结果输出到 Excel（可选）
//
// 输出：
//   单页：{ pages_scraped, total_items, items }
//   批量：{ total_urls, success_count, fail_count, results: [...] }

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Default)]
pub struct WebScrapeNode;

#[async_trait]
impl NodeExecutor for WebScrapeNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;

        // ── 批量模式：urls 数组 ──
        if let Some(urls_val) = config.get("urls") {
            if let Some(urls) = urls_val.as_array() {
                return scrape_url_list(urls, config).await;
            }
        }

        // ── 模式生成：url_pattern ──
        if let Some(pattern) = config.get("url_pattern").and_then(|v| v.as_str()) {
            let urls = expand_url_pattern(pattern)?;
            if urls.is_empty() {
                return Err(anyhow!("url_pattern 展开后为空: {}", pattern));
            }
            info!("url_pattern 展开 {} 个 URL", urls.len());
            let url_values: Vec<serde_json::Value> =
                urls.into_iter().map(serde_json::Value::String).collect();
            return scrape_url_list(&url_values, config).await;
        }

        // ── 单页模式 ──
        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("web_scrape 节点缺少 url、urls 或 url_pattern 参数"))?;

        if url.starts_with("file://") {
            return scrape_local_file(url, config);
        }

        scrape_single_url(url, config).await
    }
}

// ─── 批量模式 ───────────────────────────────────────────

async fn scrape_url_list(
    urls: &[serde_json::Value],
    config: &serde_json::Value,
) -> Result<serde_json::Value> {
    let fail_fast = config
        .get("fail_fast")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let retry = config
        .get("retry")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as usize;

    let retry_delay_ms = config
        .get("retry_delay_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(1000);

    let delay_between_ms = config
        .get("delay_between_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let excel_output = config
        .get("excel_output")
        .and_then(|v| v.as_str());

    let url_strings: Vec<String> = urls
        .iter()
        .filter_map(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.get("url").and_then(|u| u.as_str()).map(|s| s.to_string()))
        })
        .collect();

    if url_strings.is_empty() {
        return Err(anyhow!("urls 数组为空"));
    }

    info!(
        "批量抓取 {} 个 URL（重试 {} 次，间隔 {}ms）",
        url_strings.len(), retry, delay_between_ms
    );

    // 启动浏览器
    let launch_params = build_launch_params(config);
    let _ = crate::nodes::browser::send_sidecar_action("launch", &launch_params).await?;

    let extract_rules = config
        .get("extract")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("web_scrape 节点缺少 extract 参数"))?;

    let total = url_strings.len();
    let mut results = Vec::with_capacity(total);
    let mut success_count = 0u64;
    let mut fail_count = 0u64;

    for (idx, url) in url_strings.iter().enumerate() {
        // URL 间延迟（第一个不延迟）
        if idx > 0 && delay_between_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(delay_between_ms)).await;
        }

        // 带重试的抓取
        let mut last_err = None;
        let mut attempt = 0;
        let max_attempts = 1 + retry; // 首次 + 重试次数

        loop {
            attempt += 1;
            match scrape_single_url_inner(url, config, extract_rules).await {
                Ok(data) => {
                    success_count += 1;
                    results.push(serde_json::json!({
                        "url": url,
                        "items": data.get("items").cloned().unwrap_or(serde_json::Value::Array(vec![])),
                        "total_items": data.get("total_items").cloned().unwrap_or(serde_json::json!(0)),
                    }));
                    break;
                }
                Err(e) => {
                    if attempt < max_attempts {
                        warn!("抓取 {} 第 {} 次失败: {}，{}ms 后重试", url, attempt, e, retry_delay_ms);
                        tokio::time::sleep(tokio::time::Duration::from_millis(retry_delay_ms)).await;
                    } else {
                        last_err = Some(e);
                        break;
                    }
                }
            }
        }

        if let Some(e) = last_err {
            fail_count += 1;
            warn!("抓取 {} 最终失败: {}", url, e);
            if fail_fast {
                return Err(anyhow!("fail_fast: {} 抓取失败: {}", url, e));
            }
            results.push(serde_json::json!({
                "url": url,
                "error": e.to_string(),
                "items": [],
                "total_items": 0,
            }));
        }

        // 进度日志
        if (idx + 1) % 10 == 0 || idx + 1 == total {
            info!("抓取进度: {}/{}", idx + 1, total);
        }
    }

    let output = serde_json::json!({
        "total_urls": total,
        "success_count": success_count,
        "fail_count": fail_count,
        "results": results,
    });

    // ── 写入 Excel ──
    if let Some(path) = excel_output {
        write_excel_output(path, &results)?;
        info!("结果已写入 Excel: {}", path);
    }

    Ok(output)
}

// ─── URL Pattern 展开 ─────────────────────────────────
// 支持格式：
//   "https://site.com/page/{{1..10}}"         → page/1 ~ page/10
//   "https://site.com/page/{{1..10..2}}"      → page/1, 3, 5, 7, 9
//   "https://site.com/{{a,b,c}}/detail"       → /a/detail, /b/detail, /c/detail

fn expand_url_pattern(pattern: &str) -> Result<Vec<String>> {
    let mut result = vec![pattern.to_string()];

    // 找到所有 {{...}} 占位符
    while let Some(start) = result[0].find("{{") {
        if let Some(end) = result[0][start + 2..].find("}}") {
            let inner = &result[0][start + 2..start + 2 + end].trim();
            let expansions = if inner.contains("..") {
                expand_range(inner)?
            } else {
                // 逗号分隔列表
                inner.split(',').map(|s| s.trim().to_string()).collect::<Vec<_>>()
            };

            let mut new_result = Vec::new();
            for template in &result {
                let prefix = &template[..start];
                let suffix = &template[start + 2 + end + 2..];
                for val in &expansions {
                    new_result.push(format!("{}{}{}", prefix, val, suffix));
                }
            }
            result = new_result;
        } else {
            break;
        }
    }

    Ok(result)
}

/// 展开范围表达式：1..10, 1..10..2, 001..010（补零）
fn expand_range(inner: &str) -> Result<Vec<String>> {
    let parts: Vec<&str> = inner.split("..").collect();
    if parts.len() < 2 || parts.len() > 3 {
        return Err(anyhow!("无效范围格式: {{ {} }}", inner));
    }

    let start: i64 = parts[0]
        .trim()
        .parse()
        .map_err(|_| anyhow!("范围起始值不是数字: {}", parts[0]))?;
    let end: i64 = parts[1]
        .trim()
        .parse()
        .map_err(|_| anyhow!("范围结束值不是数字: {}", parts[1]))?;
    let step: i64 = if parts.len() == 3 {
        parts[2]
            .trim()
            .parse()
            .map_err(|_| anyhow!("范围步长不是数字: {}", parts[2]))?
    } else {
        if end >= start { 1 } else { -1 }
    };

    if step == 0 {
        return Err(anyhow!("范围步长不能为 0"));
    }

    // 检测是否需要补零（如 001..010）
    let pad_width = if parts[0].starts_with('0') && parts[0].len() > 1 {
        Some(parts[0].len())
    } else {
        None
    };

    let mut result = Vec::new();
    let mut current = start;
    loop {
        if (step > 0 && current > end) || (step < 0 && current < end) {
            break;
        }
        let s = if let Some(width) = pad_width {
            format!("{:0width$}", current, width = width)
        } else {
            current.to_string()
        };
        result.push(s);
        current += step;
    }

    Ok(result)
}

// ─── Excel 输出 ────────────────────────────────────────

fn write_excel_output(path: &str, results: &[serde_json::Value]) -> Result<()> {
    use rust_xlsxwriter::Workbook;

    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();

    // 表头
    worksheet.write_string(0, 0, "url")?;
    worksheet.write_string(0, 1, "success")?;
    worksheet.write_string(0, 2, "total_items")?;
    worksheet.write_string(0, 3, "error")?;
    worksheet.write_string(0, 4, "items_json")?;

    // 列宽
    worksheet.set_column_width(0, 40)?;
    worksheet.set_column_width(2, 12)?;
    worksheet.set_column_width(4, 80)?;

    for (row, result) in results.iter().enumerate() {
        let r = row as u32 + 1;
        let url = result.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let error = result.get("error").and_then(|v| v.as_str()).unwrap_or("");
        let total = result.get("total_items").and_then(|v| v.as_u64()).unwrap_or(0);
        let items = result.get("items").and_then(|v| v.as_array());
        let success = error.is_empty();

        worksheet.write_string(r, 0, url)?;
        worksheet.write_string(r, 1, if success { "✓" } else { "✗" })?;
        worksheet.write(r, 2, total)?;
        worksheet.write_string(r, 3, error)?;

        if let Some(items) = items {
            let json_str = serde_json::to_string(items).unwrap_or_default();
            // 截断过长内容（Excel 单元格限制 32767 字符）
            if json_str.len() > 32000 {
                worksheet.write_string(r, 4, &json_str[..32000])?;
            } else {
                worksheet.write_string(r, 4, &json_str)?;
            }
        }
    }

    workbook.save(path)?;
    Ok(())
}

// ─── 单页模式（公共入口） ──────────────────────────────

async fn scrape_single_url(url: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let extract_rules = config
        .get("extract")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("web_scrape 节点缺少 extract 参数"))?;

    scrape_single_url_inner(url, config, extract_rules).await
}

// ─── 单页抓取核心逻辑 ─────────────────────────────────

async fn scrape_single_url_inner(
    url: &str,
    config: &serde_json::Value,
    extract_rules: &[serde_json::Value],
) -> Result<serde_json::Value> {
    let wait_for = config
        .get("wait_for")
        .and_then(|v| v.as_str())
        .unwrap_or("body");

    let scroll = config
        .get("scroll")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let scroll_times = config
        .get("scroll_times")
        .and_then(|v| v.as_u64())
        .unwrap_or(3);

    let delay_ms = config
        .get("delay_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(1000);

    let max_pages = config
        .get("pagination")
        .and_then(|p| p.get("max_pages"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as usize;

    let next_selector = config
        .get("pagination")
        .and_then(|p| p.get("next"))
        .and_then(|v| v.as_str());

    // 启动浏览器
    let launch_params = build_launch_params(config);
    let _ = crate::nodes::browser::send_sidecar_action("launch", &launch_params).await?;

    let mut all_items: Vec<serde_json::Value> = Vec::new();
    let mut pages_scraped = 0usize;
    let mut current_url = url.to_string();

    for page_num in 0..max_pages {
        let nav_params = serde_json::json!({
            "url": current_url,
            "wait_until": "domcontentloaded",
        });
        let nav_result =
            crate::nodes::browser::send_sidecar_action("navigate", &nav_params).await?;
        if !nav_result
            .get("success")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let err = nav_result
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or("导航失败");
            return Err(anyhow!("第 {} 页导航失败: {}", page_num + 1, err));
        }

        let wait_params = serde_json::json!({
            "selector": wait_for,
            "timeout_ms": 10000,
        });
        let _ = crate::nodes::browser::send_sidecar_action("wait", &wait_params).await;

        if scroll && scroll_times > 0 {
            let scroll_params = serde_json::json!({
                "to": "bottom",
                "times": scroll_times,
                "delay_ms": delay_ms,
            });
            let _ =
                crate::nodes::browser::send_sidecar_action("scroll_to", &scroll_params).await;
        }

        for rule in extract_rules {
            let item_selector = rule
                .get("selector")
                .and_then(|v| v.as_str())
                .unwrap_or("body");
            let fields = rule.get("fields").and_then(|v| v.as_object());

            if let Some(fields) = fields {
                let count_script = format!(
                    "document.querySelectorAll({}).length",
                    serde_json::json!(item_selector)
                );
                let count_result = crate::nodes::browser::send_sidecar_action(
                    "evaluate",
                    &serde_json::json!({ "script": count_script }),
                )
                .await?;
                let count = count_result
                    .get("data")
                    .and_then(|d| d.get("result"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;

                for i in 0..count {
                    let mut item = serde_json::Map::new();
                    for (field_name, field_selector) in fields {
                        let sel = field_selector.as_str().unwrap_or("");
                        let value = if let Some(attr_name) = extract_attr_from_selector(sel) {
                            let base_sel = sel.split('[').next().unwrap_or(sel);
                            let script = format!(
                                "(() => {{ const items = document.querySelectorAll({}); if ({} < items.length) {{ const el = items[{}].querySelector({}); return el ? el.getAttribute({}) : null; }} return null; }})()",
                                serde_json::json!(item_selector), i, i,
                                serde_json::json!(base_sel), serde_json::json!(attr_name),
                            );
                            let result = crate::nodes::browser::send_sidecar_action(
                                "evaluate",
                                &serde_json::json!({ "script": script }),
                            )
                            .await?;
                            result
                                .get("data")
                                .and_then(|d| d.get("result"))
                                .cloned()
                                .unwrap_or(serde_json::Value::Null)
                        } else {
                            let script = format!(
                                "(() => {{ const items = document.querySelectorAll({}); if ({} < items.length) {{ const el = items[{}].querySelector({}); return el ? el.textContent.trim() : null; }} return null; }})()",
                                serde_json::json!(item_selector), i, i,
                                serde_json::json!(sel),
                            );
                            let result = crate::nodes::browser::send_sidecar_action(
                                "evaluate",
                                &serde_json::json!({ "script": script }),
                            )
                            .await?;
                            result
                                .get("data")
                                .and_then(|d| d.get("result"))
                                .cloned()
                                .unwrap_or(serde_json::Value::Null)
                        };
                        item.insert(field_name.clone(), value);
                    }
                    all_items.push(serde_json::Value::Object(item));
                }
            } else {
                let result = crate::nodes::browser::send_sidecar_action(
                    "extract_text",
                    &serde_json::json!({ "selector": item_selector }),
                )
                .await?;
                if let Some(texts) = result
                    .get("data")
                    .and_then(|d| d.get("texts"))
                    .and_then(|v| v.as_array())
                {
                    for text in texts {
                        all_items.push(serde_json::json!({ "text": text }));
                    }
                }
            }
        }

        pages_scraped += 1;

        if let Some(next_sel) = next_selector {
            if page_num < max_pages - 1 {
                let click_params = serde_json::json!({
                    "selector": next_sel,
                    "wait_ms": delay_ms,
                });
                let click_result =
                    crate::nodes::browser::send_sidecar_action("click", &click_params).await?;
                if !click_result
                    .get("success")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    info!("翻页失败（可能已到最后一页），停止抓取");
                    break;
                }
                let url_result = crate::nodes::browser::send_sidecar_action(
                    "current_url",
                    &serde_json::json!({}),
                )
                .await?;
                if let Some(new_url) = url_result
                    .get("data")
                    .and_then(|d| d.get("url"))
                    .and_then(|v| v.as_str())
                {
                    current_url = new_url.to_string();
                }
            }
        } else {
            break;
        }
    }

    Ok(serde_json::json!({
        "pages_scraped": pages_scraped,
        "total_items": all_items.len(),
        "items": all_items,
    }))
}

// ─── 工具函数 ──────────────────────────────────────────

fn build_launch_params(config: &serde_json::Value) -> serde_json::Value {
    let headless = config
        .get("headless")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let user_agent = config.get("user_agent").and_then(|v| v.as_str());
    let proxy = config.get("proxy").and_then(|v| v.as_str());

    let mut params = serde_json::json!({
        "headless": headless,
        "channel": "auto",
    });
    if let Some(ua) = user_agent {
        params["user_agent"] = serde_json::json!(ua);
        params["random_ua"] = serde_json::json!(false);
    }
    if let Some(p) = proxy {
        params["proxy"] = serde_json::json!(p);
    }
    params
}

fn extract_attr_from_selector(sel: &str) -> Option<&str> {
    if let Some(start) = sel.find('[') {
        if let Some(end) = sel[start..].find(']') {
            let attr = &sel[start + 1..start + end];
            if !attr.contains('=') && !attr.contains(':') {
                return Some(attr);
            }
        }
    }
    None
}

/// 离线模式：读取本地 HTML 文件
fn scrape_local_file(url: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = url.strip_prefix("file://").unwrap_or(url);
    let html =
        std::fs::read_to_string(path).map_err(|e| anyhow!("无法读取本地文件 {}: {}", path, e))?;

    let extract_rules = config
        .get("extract")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("web_scrape 节点缺少 extract 参数"))?;

    let document = scraper::Html::parse_document(&html);
    let mut items: Vec<serde_json::Value> = Vec::new();

    for rule in extract_rules {
        let selector_str = rule
            .get("selector")
            .and_then(|v| v.as_str())
            .unwrap_or("body");
        let fields = rule.get("fields").and_then(|v| v.as_object());

        let selector = match scraper::Selector::parse(selector_str) {
            Ok(s) => s,
            Err(e) => {
                warn!("无效选择器 '{}': {:?}", selector_str, e);
                continue;
            }
        };

        for element in document.select(&selector) {
            if let Some(fields) = fields {
                let mut item = serde_json::Map::new();
                for (name, field_sel) in fields {
                    let field_str = field_sel.as_str().unwrap_or("");
                    let value = if field_str.is_empty() {
                        element
                            .text()
                            .collect::<Vec<_>>()
                            .join(" ")
                            .trim()
                            .to_string()
                    } else if let Ok(sub_sel) = scraper::Selector::parse(field_str) {
                        element
                            .select(&sub_sel)
                            .flat_map(|e| e.text())
                            .collect::<Vec<_>>()
                            .join(" ")
                            .trim()
                            .to_string()
                    } else {
                        String::new()
                    };
                    item.insert(name.clone(), serde_json::Value::String(value));
                }
                items.push(serde_json::Value::Object(item));
            } else {
                let text = element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
                items.push(serde_json::Value::String(text));
            }
        }
    }

    Ok(serde_json::json!({
        "source": "local_file",
        "path": path,
        "total_items": items.len(),
        "items": items,
    }))
}
