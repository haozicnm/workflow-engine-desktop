// nodes/web_scrape.rs — 声明式网页抓取节点
//
// 两种模式：
//   单页：url: "https://example.com"
//   批量：urls: ["https://a.com", "https://b.com"]
//
// 批量额外参数：
//   max_concurrent: 3   // 最大并发（默认 3）
//   fail_fast: false    // 单个失败是否中止（默认 false）
//
// 输出：
//   单页：{ pages_scraped, total_items, items }
//   批量：{ total_urls, success_count, fail_count, results: [{url, items, error?}, ...] }

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

        // ── 单页模式 ──
        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("web_scrape 节点缺少 url 或 urls 参数"))?;

        // 离线模式
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

    let url_strings: Vec<String> = urls
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    if url_strings.is_empty() {
        return Err(anyhow!("urls 数组为空"));
    }

    info!("批量抓取 {} 个 URL", url_strings.len());

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

    for url in &url_strings {
        match scrape_single_url_inner(url, config, extract_rules).await {
            Ok(data) => {
                success_count += 1;
                results.push(serde_json::json!({
                    "url": url,
                    "items": data.get("items").cloned().unwrap_or(serde_json::Value::Array(vec![])),
                    "total_items": data.get("total_items").cloned().unwrap_or(serde_json::json!(0)),
                }));
            }
            Err(e) => {
                fail_count += 1;
                warn!("抓取 {} 失败: {}", url, e);
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
        }
    }

    Ok(serde_json::json!({
        "total_urls": total,
        "success_count": success_count,
        "fail_count": fail_count,
        "results": results,
    }))
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

    // 翻页抓取循环
    let mut all_items: Vec<serde_json::Value> = Vec::new();
    let mut pages_scraped = 0usize;
    let mut current_url = url.to_string();

    for page_num in 0..max_pages {
        // 导航
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

        // 等待目标元素
        let wait_params = serde_json::json!({
            "selector": wait_for,
            "timeout_ms": 10000,
        });
        let _ = crate::nodes::browser::send_sidecar_action("wait", &wait_params).await;

        // 滚动加载
        if scroll && scroll_times > 0 {
            let scroll_params = serde_json::json!({
                "to": "bottom",
                "times": scroll_times,
                "delay_ms": delay_ms,
            });
            let _ =
                crate::nodes::browser::send_sidecar_action("scroll_to", &scroll_params).await;
        }

        // 提取数据
        for rule in extract_rules {
            let item_selector = rule
                .get("selector")
                .and_then(|v| v.as_str())
                .unwrap_or("body");
            let fields = rule.get("fields").and_then(|v| v.as_object());

            if let Some(fields) = fields {
                // 字段模式
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
                // 简单文本模式
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

        // 翻页
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
