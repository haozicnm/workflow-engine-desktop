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
                let x = action.config.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let y = action.config.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);

                let params = serde_json::json!({
                    "x": x,
                    "y": y,
                });
                crate::nodes::browser::send_sidecar_action("scroll", &params).await
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
                    .get(&format!("{}_in", &action.id))
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

                let params = serde_json::json!({
                    "selector": selector,
                    "mode": mode,
                });
                let resp = crate::nodes::browser::send_sidecar_action("extract", &params).await
                    .map_err(|e| anyhow!("提取失败: {}", e))?;

                let data = resp
                    .get("data")
                    .or_else(|| resp.get("result"))
                    .cloned()
                    .unwrap_or(Value::Null);
                output_ports.insert(format!("{}_out", &action.id), data);
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
                output_ports.insert(format!("{}_out", &action.id), data);
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
                output_ports.insert(format!("{}_out", &action.id), data);
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
                output_ports.insert(format!("{}_out", &action.id), title);
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
