// nodes/print.rs — 控制台打印节点
//
// 支持操作：
//   {message: "要打印的文本", level: "info"|"warn"|"error"}
//
// 通过 tracing 输出日志到后端控制台。
// 同时通过全局 AppHandle 发射 "console-output" Tauri 事件到前端控制台。
//
// 注意：前端事件发射依赖 main.rs 中调用 PrintNode::init_app_handle() 初始化。

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use std::sync::OnceLock;
use tauri::Emitter;
use tracing::{error, info, warn};

/// 全局 AppHandle，用于发射 "console-output" 事件
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

#[derive(Default)]
pub struct PrintNode;

impl PrintNode {
    /// 在应用启动时初始化全局 AppHandle（在 main.rs setup 中调用）
    pub fn init_app_handle(handle: tauri::AppHandle) {
        let _ = APP_HANDLE.set(handle);
    }
}

#[async_trait]
impl NodeExecutor for PrintNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "print".into(),
            version: "1.0".into(),
            display_name: "打印日志".into(),
            description: "打印消息到控制台，支持 info/warn/error 级别".into(),
            category: "数据".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "printed".into(), data_type: "bool".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {"type": "string", "description": "要打印的消息"},
                    "level": {"type": "string", "enum": ["info", "warn", "error"], "description": "日志级别", "default": "info"}
                }
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;

        let message = config.get("message").and_then(|v| v.as_str()).unwrap_or("");

        let level = config
            .get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("info");

        let step_name = &step.name;

        // ── 后端日志输出 ──
        let log_message = format!("[{}] {}", step_name, message);
        match level {
            "error" => error!("{}", log_message),
            "warn" => warn!("{}", log_message),
            _ => info!("{}", log_message),
        }

        // ── 前端控制台事件 ──
        if let Some(handle) = APP_HANDLE.get() {
            let _ = handle.emit(
                "console-output",
                serde_json::json!({
                    "step": step.id,
                    "step_name": step_name,
                    "level": level,
                    "message": message,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                }),
            );
        }

        Ok(serde_json::json!({
            "action": "print",
            "level": level,
            "message": message,
            "printed": true,
        }))
    }
}
