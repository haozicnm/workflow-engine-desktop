// nodes/window.rs — 窗口管理节点（v2：跨平台）
//
// 通过 platform::window() trait 统一接口调用，不再硬编码 PowerShell。
// Windows → platform/windows_window.rs（PowerShell + user32.dll）
// Linux   → platform/linux_window.rs（xdotool + wmctrl CLI）

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Default)]
pub struct WindowNode;

#[async_trait]
impl NodeExecutor for WindowNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "window".into(),
            version: "1.0".into(),
            display_name: "窗口操作".into(),
            description: "管理桌面窗口：查找、激活、最大化、最小化、关闭、调整大小等".into(),
            category: "系统".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "action".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "windows".into(), data_type: "array".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": {"type": "string", "enum": ["find", "activate", "maximize", "minimize", "restore", "close", "resize", "wait", "list"], "description": "操作类型"},
                    "title": {"type": "string", "description": "窗口标题"},
                    "width": {"type": "number", "description": "调整宽度", "default": 800},
                    "height": {"type": "number", "description": "调整高度", "default": 600},
                    "timeout_s": {"type": "number", "description": "等待超时秒数", "default": 30}
                }
            }),
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("find");

        let backend = crate::platform::window();

        match action {
            "find" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let windows = backend.find(title)?;
                Ok(serde_json::json!({"action": "find", "windows": windows}))
            }

            "activate" => {
                let title = config.get("title").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("缺少 title 参数"))?;
                backend.activate(title)?;
                Ok(serde_json::json!({"action": "activate", "title": title}))
            }

            "maximize" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                backend.maximize(title)?;
                Ok(serde_json::json!({"action": "maximize", "title": title}))
            }

            "minimize" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                backend.minimize(title)?;
                Ok(serde_json::json!({"action": "minimize", "title": title}))
            }

            "restore" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                backend.restore(title)?;
                Ok(serde_json::json!({"action": "restore", "title": title}))
            }

            "close" => {
                let title = config.get("title").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("缺少 title 参数"))?;
                backend.close(title)?;
                Ok(serde_json::json!({"action": "close", "title": title}))
            }

            "resize" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let width = config.get("width").and_then(|v| v.as_i64()).unwrap_or(800) as i32;
                let height = config.get("height").and_then(|v| v.as_i64()).unwrap_or(600) as i32;
                backend.resize(title, width, height)?;
                Ok(serde_json::json!({
                    "action": "resize", "title": title,
                    "width": width, "height": height,
                }))
            }

            "wait" => {
                let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
                let timeout = config.get("timeout_s").and_then(|v| v.as_u64()).unwrap_or(30);
                backend.wait(title, timeout)?;
                Ok(serde_json::json!({"action": "wait", "title": title, "found": true}))
            }

            "list" => {
                let windows = backend.list()?;
                Ok(serde_json::json!({"action": "list", "windows": windows}))
            }

            _ => Err(anyhow!(
                "未知窗口操作: {}（支持: find/activate/maximize/minimize/restore/close/resize/wait/list）",
                action
            )),
        }
    }
}
