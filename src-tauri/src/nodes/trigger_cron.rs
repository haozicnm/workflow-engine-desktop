// nodes/trigger_cron.rs — Cron 定时触发器节点
// 工作流入口：配置 cron 表达式，由调度器定时触发
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;

#[derive(Default)]
pub struct TriggerCronNode;

#[async_trait]
impl NodeExecutor for TriggerCronNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "trigger_cron".into(),
            version: "1.0".into(),
            display_name: "定时触发器".into(),
            description: "按 Cron 表达式定时触发工作流执行".into(),
            category: "触发器".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef {
                    label: "triggered_at".into(),
                    data_type: "string".into(),
                    required: false,
                },
                crate::nodes::traits::PortDef {
                    label: "trigger_count".into(),
                    data_type: "number".into(),
                    required: false,
                },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "cron_expr": { "type": "string", "description": "Cron 表达式 (分 时 日 月 周)" },
                    "timezone": { "type": "string", "description": "时区 (默认 UTC)", "default": "UTC" }
                },
                "required": ["cron_expr"]
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        _step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        // 触发器节点在被调度器触发时执行，返回触发时间
        let now = chrono::Utc::now();
        Ok(json!({
            "triggered_at": now.to_rfc3339(),
            "timestamp": now.timestamp(),
        }))
    }
}
