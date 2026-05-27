// nodes/mouse_keyboard.rs — 鼠标/键盘节点（v2：跨平台）
//
// 通过 platform::input() 统一接口调用，不再直接依赖 PowerShell。
// Windows → platform/windows.rs（保留原有 user32.dll 逻辑）
// Linux   → platform/linux.rs（enigo crate）

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Default)]
pub struct MouseKeyboardNode;

#[async_trait]
impl NodeExecutor for MouseKeyboardNode {
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
            .unwrap_or("click");

        let backend = crate::platform::input();

        match action {
            "click" => {
                let x = config.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = config.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let button = config
                    .get("button")
                    .and_then(|v| v.as_str())
                    .unwrap_or("left");
                let clicks = config.get("clicks").and_then(|v| v.as_i64()).unwrap_or(1);

                for _ in 0..clicks {
                    backend.click(x, y, button)?;
                }
                Ok(
                    serde_json::json!({"action":"click","x":x,"y":y,"button":button,"clicks":clicks}),
                )
            }

            "move" => {
                let x = config.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let y = config.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                backend.move_mouse(x, y)?;
                Ok(serde_json::json!({"action":"move","x":x,"y":y}))
            }

            "type" => {
                let text = config.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let delay = config
                    .get("delay_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(50);
                backend.type_text(text, delay)?;
                Ok(serde_json::json!({"action":"type","text":text}))
            }

            "hotkey" => {
                let keys = config.get("keys").and_then(|v| v.as_str()).unwrap_or("");
                backend.hotkey(keys)?;
                Ok(serde_json::json!({"action":"hotkey","keys":keys}))
            }

            "scroll" => {
                let amount = config.get("amount").and_then(|v| v.as_i64()).unwrap_or(3) as i32;
                backend.scroll(amount)?;
                Ok(serde_json::json!({"action":"scroll","amount":amount}))
            }

            _ => Err(anyhow!(
                "未知鼠标/键盘操作: {}（支持: click/move/type/hotkey/scroll）",
                action
            )),
        }
    }
}
