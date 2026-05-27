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
