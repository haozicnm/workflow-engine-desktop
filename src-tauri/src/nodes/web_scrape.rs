// nodes/web_scrape.rs — 声明式网页抓取节点
//
// 用法示例：
//   type: web_scrape
//   config:
//     url: "https://example.com/products"
//     wait_for: ".product-list"
//     extract:
//       - selector: ".product-card"
//         fields:
//           name: ".title"
//           price: ".price"
//           link: "a[href]"
//           image: "img[src]"
//     pagination:
//       next: ".next-page"
//       max_pages: 5
//     scroll: true
//     scroll_times: 3
//     delay_ms: 1000
//     headless: true
//     proxy: "http://proxy:8080"
//     cookies: [...]
//     user_agent: "..."
//
// 输出：
//   {
//     "pages_scraped": 3,
//     "total_items": 45,
//     "items": [ { "name": "...", "price": "...", ... }, ... ]
//   }

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

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

        let url = config
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("web_scrape 节点缺少 url 参数"))?;

        // 离线模式：支持 file:// 读取本地 HTML
        if url.starts_with("file://") {
            return scrape_local_file(url, config);
        }

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

        let headless = config
            .get("headless")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        let user_agent = config.get("user_agent").and_then(|v| v.as_str());

        let proxy = config.get("proxy").and_then(|v| v.as_str());

        let max_pages = config
            .get("pagination")
            .and_then(|p| p.get("max_pages"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as usize;

        let next_selector = config
            .get("pagination")
            .and_then(|p| p.get("next"))
            .and_then(|v| v.as_str());

        // 提取规则
        let extract_rules = config
            .get("extract")
            .and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("web_scrape 节点缺少 extract 参数"))?;

        // ── 构建 sidecar launch 参数 ──
        let mut launch_params = serde_json::json!({
            "headless": headless,
            "channel": "auto",
        });
        if let Some(ua) = user_agent {
            launch_params["user_agent"] = serde_json::json!(ua);
            launch_params["random_ua"] = serde_json::json!(false);
        }
        if let Some(p) = proxy {
            launch_params["proxy"] = serde_json::json!(p);
        }

        // 启动浏览器
        let _launch_result =
            crate::nodes::browser::send_sidecar_action("launch", &launch_params).await?;

        // ── 翻页抓取循环 ──
        let mut all_items: Vec<serde_json::Value> = Vec::new();
        let mut pages_scraped = 0usize;
        let mut current_url = url.to_string();

        for page_num in 0..max_pages {
            // 导航到页面
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

            // 无限滚动加载
            if scroll && scroll_times > 0 {
                let scroll_params = serde_json::json!({
                    "to": "bottom",
                    "times": scroll_times,
                    "delay_ms": delay_ms,
                });
                let _ =
                    crate::nodes::browser::send_sidecar_action("scroll_to", &scroll_params).await;
            }

            // ── 按 extract 规则提取数据 ──
            for rule in extract_rules {
                let item_selector = rule
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or("body");

                let fields = rule.get("fields").and_then(|v| v.as_object());

                if let Some(fields) = fields {
                    // 字段模式：每个 item_selector 匹配的元素，按 fields 提取子字段
                    // 用 evaluate 计算元素数量
                    let count_script = format!(
                        "document.querySelectorAll({}).length",
                        serde_json::json!(item_selector)
                    );
                    let count_result = crate::nodes::browser::send_sidecar_action(
                        "evaluate",
                        &serde_json::json!({
                            "script": count_script,
                        }),
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
                            // 判断是否提取属性（如 a[href]、img[src]）
                            if let Some(attr_name) = extract_attr_from_selector(sel) {
                                let base_sel = sel.split('[').next().unwrap_or(sel);
                                let script = format!(
                                    "(() => {{ const items = document.querySelectorAll({}); if ({} < items.length) {{ const el = items[{}].querySelector({}); return el ? el.getAttribute({}) : null; }} return null; }})()",
                                    serde_json::json!(item_selector),
                                    i,
                                    i,
                                    serde_json::json!(base_sel),
                                    serde_json::json!(attr_name),
                                );
                                let result = crate::nodes::browser::send_sidecar_action(
                                    "evaluate",
                                    &serde_json::json!({
                                        "script": script,
                                    }),
                                )
                                .await?;
                                let value = result
                                    .get("data")
                                    .and_then(|d| d.get("result"))
                                    .cloned()
                                    .unwrap_or(serde_json::Value::Null);
                                item.insert(field_name.clone(), value);
                            } else {
                                // 提取文本
                                let script = format!(
                                    "(() => {{ const items = document.querySelectorAll({}); if ({} < items.length) {{ const el = items[{}].querySelector({}); return el ? el.textContent.trim() : null; }} return null; }})()",
                                    serde_json::json!(item_selector),
                                    i,
                                    i,
                                    serde_json::json!(sel),
                                );
                                let result = crate::nodes::browser::send_sidecar_action(
                                    "evaluate",
                                    &serde_json::json!({
                                        "script": script,
                                    }),
                                )
                                .await?;
                                let value = result
                                    .get("data")
                                    .and_then(|d| d.get("result"))
                                    .cloned()
                                    .unwrap_or(serde_json::Value::Null);
                                item.insert(field_name.clone(), value);
                            }
                        }
                        all_items.push(serde_json::Value::Object(item));
                    }
                } else {
                    // 简单模式：只提取文本
                    let result = crate::nodes::browser::send_sidecar_action(
                        "extract_text",
                        &serde_json::json!({
                            "selector": item_selector,
                        }),
                    )
                    .await?;
                    if let Some(texts) = result
                        .get("data")
                        .and_then(|d| d.get("texts"))
                        .and_then(|v| v.as_array())
                    {
                        for text in texts {
                            all_items.push(serde_json::json!({
                                "text": text,
                            }));
                        }
                    }
                }
            }

            pages_scraped += 1;

            // ── 翻页 ──
            if let Some(next_sel) = next_selector {
                if page_num < max_pages - 1 {
                    // 尝试点击下一页
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
                    // 更新当前 URL
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
                break; // 没有翻页选择器，只抓一页
            }
        }

        Ok(serde_json::json!({
            "pages_scraped": pages_scraped,
            "total_items": all_items.len(),
            "items": all_items,
        }))
    }
}

/// 从 CSS 选择器中提取属性名
/// 例: "a[href]" → Some("href"), "img[src]" → Some("src"), ".title" → None
fn extract_attr_from_selector(sel: &str) -> Option<&str> {
    if let Some(start) = sel.find('[') {
        if let Some(end) = sel[start..].find(']') {
            let attr = &sel[start + 1..start + end];
            // 排除 CSS 伪选择器如 :not(...)、[attr=value] 等
            if !attr.contains('=') && !attr.contains(':') {
                return Some(attr);
            }
        }
    }
    None
}

/// 离线模式：读取本地 HTML 文件并用 scrapper crate 提取内容
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
                tracing::warn!("无效选择器 '{}': {:?}", selector_str, e);
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
