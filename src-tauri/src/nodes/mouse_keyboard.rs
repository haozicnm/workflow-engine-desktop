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
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "mouse_keyboard".into(),
            version: "1.0".into(),
            display_name: "鼠标键盘".into(),
            description: "模拟鼠标点击、移动和键盘输入操作".into(),
            category: "系统".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "action".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "x".into(), data_type: "number".into(), required: false },
                crate::nodes::traits::PortDef { label: "y".into(), data_type: "number".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "required": [],
                "properties": {
                    "action": {"type": "string", "enum": ["click", "move", "type", "hotkey", "scroll"], "description": "操作类型"},
                    "x": {"type": "number", "description": "X 坐标"},
                    "y": {"type": "number", "description": "Y 坐标"},
                    "button": {"type": "string", "enum": ["left", "right", "middle"], "description": "鼠标按钮"},
                    "clicks": {"type": "number", "description": "点击次数", "default": 1},
                    "text": {"type": "string", "description": "输入的文本"},
                    "delay_ms": {"type": "number", "description": "按键延迟（毫秒）", "default": 50},
                    "keys": {"type": "string", "description": "快捷键组合"},
                    "amount": {"type": "number", "description": "滚动量", "default": 3}
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
