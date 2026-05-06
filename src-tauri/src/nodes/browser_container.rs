// nodes/browser_container.rs — 浏览器容器执行器
//
// v4.0: 在一个浏览器窗口内按顺序执行多个 action，
// 支持通过 DAG 连线从上游容器端口接收数据并产出输出端口数据。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing;

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};

// ─── 数据结构 ───

/// 容器内的单个 action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerAction {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: String,
    pub label: String,
    #[serde(default)]
    pub config: HashMap<String, Value>,
}

/// 浏览器容器配置（来自前端节点的 config）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserContainerConfig {
    #[serde(default = "default_browser")]
    pub browser: String,
    #[serde(default)]
    pub headless: bool,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    pub actions: Vec<ContainerAction>,
}

fn default_browser() -> String {
    "chromium".to_string()
}

fn default_timeout() -> u64 {
    30000
}

/// 浏览器容器执行结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerResult {
    pub output_ports: HashMap<String, Value>,
    pub error: Option<String>,
}

// ─── 核心执行逻辑 ───

/// 执行浏览器容器
///
/// 使用现有的 browser.rs sidecar 机制，按顺序执行所有 action：
/// - input/fill 类型优先从 input_ports（连线传入）获取值
/// - extract/screenshot/evaluate/get_title 等产出数据存入 output_ports
pub async fn execute_browser_container(
    config: &BrowserContainerConfig,
    input_ports: &HashMap<String, Value>,
) -> Result<ContainerResult> {
    let mut output_ports: HashMap<String, Value> = HashMap::new();

    for action in &config.actions {
        tracing::info!(
            "BrowserContainer — 执行 action: {} ({})",
            action.label,
            action.action_type
        );

        match action.action_type.as_str() {
            "navigate" => {
                let url = action
                    .config
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("about:blank");
                let wait_until = action
                    .config
                    .get("wait_until")
                    .and_then(|v| v.as_str())
                    .unwrap_or("load");

                let params = serde_json::json!({
                    "url": url,
                    "wait_until": wait_until,
                });
                crate::nodes::browser::send_sidecar_action("navigate", &params).await
                    .map_err(|e| anyhow!("导航失败: {}", e))?;
            }

            "wait" => {
                let selector = action
                    .config
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let timeout = action
                    .config
                    .get("timeout")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5000);

                let params = serde_json::json!({
                    "selector": selector,
                    "timeout": timeout,
                });
                crate::nodes::browser::send_sidecar_action("wait", &params).await
                    .map_err(|e| anyhow!("等待失败: {}", e))?;
            }

            "click" => {
                let selector = action
                    .config
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let params = serde_json::json!({
                    "selector": selector,
                });
                crate::nodes::browser::send_sidecar_action("click", &params).await
                    .map_err(|e| anyhow!("点击失败: {}", e))?;
            }

            "scroll" => {
                let y = action.config.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                // Py sidecar: scroll_to(to: "bottom"|"top"|pixels)
                let to: serde_json::Value = if y > 0.0 {
                    serde_json::json!(y)
                } else {
                    serde_json::json!("bottom")
                };
                let params = serde_json::json!({
                    "to": to,
                });
                crate::nodes::browser::send_sidecar_action("scroll_to", &params).await
                    .map_err(|e| anyhow!("滚动失败: {}", e))?;
            }

            "input" | "fill" => {
                let selector = action
                    .config
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                // 优先从 input_ports（连线传入）获取值，其次从 config.value
                let value = input_ports
                    .get(&format!("{}_in", &action.label))
                    .and_then(|v| v.as_str())
                    .or_else(|| action.config.get("value").and_then(|v| v.as_str()))
                    .unwrap_or("");
                let clear = action
                    .config
                    .get("clear")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let params = serde_json::json!({
                    "selector": selector,
                    "value": value,
                    "clear": clear,
                });
                crate::nodes::browser::send_sidecar_action("fill", &params).await
                    .map_err(|e| anyhow!("填写失败: {}", e))?;
            }

            "extract" => {
                let selector = action
                    .config
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .unwrap_or("body");
                let mode = action
                    .config
                    .get("mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("text");

                // Py sidecar has extract_text / extract_html / extract_attribute (no "extract")
                let (py_action, py_params) = match mode {
                    "html" => ("extract_html", serde_json::json!({ "selector": selector })),
                    "attr" => {
                        let attribute = action.config.get("attribute")
                            .and_then(|v| v.as_str())
                            .unwrap_or("href");
                        ("extract_attribute", serde_json::json!({ "selector": selector, "attribute": attribute }))
                    }
                    _ => ("extract_text", serde_json::json!({ "selector": selector })),
                };
                let resp = crate::nodes::browser::send_sidecar_action(py_action, &py_params).await
                    .map_err(|e| anyhow!("提取失败: {}", e))?;

                let data = resp
                    .get("data")
                    .or_else(|| resp.get("result"))
                    .cloned()
                    .unwrap_or(Value::Null);
                output_ports.insert(action.label.clone(), data);
            }

            "screenshot" => {
                let full_page = action
                    .config
                    .get("full_page")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let params = serde_json::json!({
                    "fullPage": full_page,
                });
                let resp = crate::nodes::browser::send_sidecar_action("screenshot", &params).await
                    .map_err(|e| anyhow!("截图失败: {}", e))?;

                let data = resp
                    .get("data")
                    .or_else(|| resp.get("screenshot"))
                    .cloned()
                    .unwrap_or(Value::Null);
                output_ports.insert(action.label.clone(), data);
            }

            "evaluate" => {
                let script = action
                    .config
                    .get("script")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let params = serde_json::json!({
                    "script": script,
                });
                let resp = crate::nodes::browser::send_sidecar_action("evaluate", &params).await
                    .map_err(|e| anyhow!("执行JS失败: {}", e))?;

                let data = resp
                    .get("data")
                    .or_else(|| resp.get("result"))
                    .cloned()
                    .unwrap_or(Value::Null);
                output_ports.insert(action.label.clone(), data);
            }

            "get_title" => {
                let params = serde_json::json!({
                    "script": "document.title",
                });
                let resp = crate::nodes::browser::send_sidecar_action("evaluate", &params).await
                    .map_err(|e| anyhow!("获取标题失败: {}", e))?;

                let title = resp
                    .get("data")
                    .or_else(|| resp.get("result"))
                    .cloned()
                    .unwrap_or(Value::String("".to_string()));
                output_ports.insert(action.label.clone(), title);
            }

            // ─── v1.1+ 扩展动作 ───

            "extract_table" => {
                let selector = action.config.get("selector")
                    .and_then(|v| v.as_str()).unwrap_or("table");
                let params = serde_json::json!({ "selector": selector });
                let resp = crate::nodes::browser::send_sidecar_action("extract_table", &params).await
                    .map_err(|e| anyhow!("表格提取失败: {}", e))?;
                let data = resp.get("data").cloned().unwrap_or(Value::Null);
                output_ports.insert(action.label.clone(), data);
            }

            "extract_links" => {
                let selector = action.config.get("selector")
                    .and_then(|v| v.as_str()).unwrap_or("body");
                let params = serde_json::json!({ "selector": selector });
                let resp = crate::nodes::browser::send_sidecar_action("extract_links", &params).await
                    .map_err(|e| anyhow!("链接提取失败: {}", e))?;
                let data = resp.get("data").cloned().unwrap_or(Value::Null);
                output_ports.insert(action.label.clone(), data);
            }

            "select" => {
                let selector = action.config.get("selector")
                    .and_then(|v| v.as_str()).unwrap_or("");
                let value = action.config.get("value")
                    .and_then(|v| v.as_str())
                    .or_else(|| input_ports.get(&format!("{}_in", &action.label))
                        .and_then(|v| v.as_str()))
                    .unwrap_or("");
                let params = serde_json::json!({ "selector": selector, "value": value });
                crate::nodes::browser::send_sidecar_action("select", &params).await
                    .map_err(|e| anyhow!("下拉选择失败: {}", e))?;
            }

            "check" => {
                let selector = action.config.get("selector")
                    .and_then(|v| v.as_str()).unwrap_or("");
                let checked = action.config.get("checked")
                    .and_then(|v| v.as_bool()).unwrap_or(true);
                let params = serde_json::json!({ "selector": selector, "checked": checked });
                crate::nodes::browser::send_sidecar_action("check", &params).await
                    .map_err(|e| anyhow!("勾选失败: {}", e))?;
            }

            "hover" => {
                let selector = action.config.get("selector")
                    .and_then(|v| v.as_str()).unwrap_or("");
                // Playwright hover via evaluate (no native hover action in sidecar)
                let script = format!(
                    "document.querySelector({})?.dispatchEvent(new MouseEvent('mouseover', {{bubbles:true}}))",
                    serde_json::to_string(selector).unwrap_or_default()
                );
                let params = serde_json::json!({ "script": script });
                crate::nodes::browser::send_sidecar_action("evaluate", &params).await
                    .map_err(|e| anyhow!("悬停失败: {}", e))?;
            }

            "new_page" => {
                let url = action.config.get("url").and_then(|v| v.as_str());
                let params = serde_json::json!({ "url": url });
                crate::nodes::browser::send_sidecar_action("new_page", &params).await
                    .map_err(|e| anyhow!("新建标签页失败: {}", e))?;
            }

            "close_page" => {
                let index = action.config.get("index").and_then(|v| v.as_u64());
                let params = serde_json::json!({ "index": index });
                crate::nodes::browser::send_sidecar_action("close_page", &params).await
                    .map_err(|e| anyhow!("关闭标签页失败: {}", e))?;
            }

            "switch_page" => {
                let index = action.config.get("index")
                    .and_then(|v| v.as_u64()).unwrap_or(0);
                let params = serde_json::json!({ "index": index });
                crate::nodes::browser::send_sidecar_action("switch_page", &params).await
                    .map_err(|e| anyhow!("切换标签页失败: {}", e))?;
            }

            "pages" => {
                let resp = crate::nodes::browser::send_sidecar_action("pages", &serde_json::json!({})).await
                    .map_err(|e| anyhow!("获取标签页列表失败: {}", e))?;
                let data = resp.get("data").cloned().unwrap_or(Value::Null);
                output_ports.insert(action.label.clone(), data);
            }

            "cookies" => {
                let cookie_action = action.config.get("action")
                    .and_then(|v| v.as_str()).unwrap_or("get");
                let mut params = serde_json::json!({ "action": cookie_action });
                if cookie_action == "set" {
                    if let Some(cookies) = action.config.get("cookies") {
                        params["cookies"] = cookies.clone();
                    }
                }
                let resp = crate::nodes::browser::send_sidecar_action("cookies", &params).await
                    .map_err(|e| anyhow!("Cookie 操作失败: {}", e))?;
                let data = resp.get("data").cloned().unwrap_or(Value::Null);
                output_ports.insert(action.label.clone(), data);
            }

            "set_headers" => {
                let headers = action.config.get("headers")
                    .cloned().unwrap_or(serde_json::json!({}));
                let params = serde_json::json!({ "headers": headers });
                crate::nodes::browser::send_sidecar_action("set_headers", &params).await
                    .map_err(|e| anyhow!("设置请求头失败: {}", e))?;
            }

            "back" => {
                crate::nodes::browser::send_sidecar_action("back", &serde_json::json!({})).await
                    .map_err(|e| anyhow!("后退失败: {}", e))?;
            }

            "forward" => {
                crate::nodes::browser::send_sidecar_action("forward", &serde_json::json!({})).await
                    .map_err(|e| anyhow!("前进失败: {}", e))?;
            }

            "reload" => {
                crate::nodes::browser::send_sidecar_action("reload", &serde_json::json!({})).await
                    .map_err(|e| anyhow!("刷新失败: {}", e))?;
            }

            "current_url" => {
                let resp = crate::nodes::browser::send_sidecar_action("current_url", &serde_json::json!({})).await
                    .map_err(|e| anyhow!("获取URL失败: {}", e))?;
                let data = resp.get("data").cloned().unwrap_or(Value::Null);
                output_ports.insert(action.label.clone(), data);
            }

            "pdf" => {
                let path = action.config.get("path")
                    .and_then(|v| v.as_str()).unwrap_or("output.pdf");
                let params = serde_json::json!({ "path": path });
                crate::nodes::browser::send_sidecar_action("pdf", &params).await
                    .map_err(|e| anyhow!("生成PDF失败: {}", e))?;
            }

            _ => {
                tracing::warn!(
                    "BrowserContainer — 未知 action 类型: {}",
                    action.action_type
                );
            }
        }
    }

    Ok(ContainerResult {
        output_ports,
        error: None,
    })
}

// ─── NodeExecutor trait 实现 ───

#[derive(Default)]
pub struct BrowserContainerNode;

#[async_trait]
impl NodeExecutor for BrowserContainerNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        // 解析配置
        let config: BrowserContainerConfig = serde_json::from_value(step.config.clone())
            .map_err(|e| anyhow!("浏览器容器配置解析失败: {}", e))?;

        // 从 ctx.input_ports 获取连线传入的数据
        let input_ports = ctx.input_ports.clone();

        // 执行
        let result = execute_browser_container(&config, &input_ports).await?;

        if let Some(ref err) = result.error {
            return Err(anyhow!("浏览器容器执行失败: {}", err));
        }

        // 返回 output_ports 供下游节点使用
        Ok(serde_json::to_value(&result.output_ports).unwrap_or(Value::Null))
    }
}
