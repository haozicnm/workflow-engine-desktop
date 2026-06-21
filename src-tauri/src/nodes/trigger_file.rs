// nodes/trigger_file.rs — 文件变化触发器节点
// 监测文件/目录的创建、修改、删除事件
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tracing::info;

#[derive(Default)]
pub struct TriggerFileNode;

#[async_trait]
impl NodeExecutor for TriggerFileNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "trigger_file".into(),
            version: "1.0".into(),
            display_name: "文件变化触发器".into(),
            description: "监测文件/目录变化（创建、修改、删除）时触发工作流".into(),
            category: "触发器".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "path".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "event".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "triggered_at".into(), data_type: "string".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "监测的文件/目录路径" },
                    "events": {
                        "type": "array",
                        "items": { "type": "string", "enum": ["create", "modify", "delete"] },
                        "description": "监听的事件类型",
                        "default": ["create", "modify"]
                    }
                },
                "required": ["path"]
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        _step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        // 文件触发时，触发信息已通过 ctx.variables 注入
        let path = ctx.variables
            .get("_file_trigger_path")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let event = ctx.variables
            .get("_file_trigger_event")
            .and_then(|v| v.as_str())
            .unwrap_or("modify");

        info!("文件变化触发: {} ({})", path, event);

        Ok(json!({
            "path": path,
            "event": event,
            "triggered_at": chrono::Utc::now().to_rfc3339(),
        }))
    }
}
