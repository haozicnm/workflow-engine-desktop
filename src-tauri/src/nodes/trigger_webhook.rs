// nodes/trigger_webhook.rs — Webhook 触发器节点
// 工作流入口：通过 HTTP POST 触发工作流，支持返回响应
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
pub struct TriggerWebhookNode;

#[async_trait]
impl NodeExecutor for TriggerWebhookNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "trigger_webhook".into(),
            version: "1.0".into(),
            display_name: "Webhook 触发器".into(),
            description: "通过 HTTP POST 请求触发工作流执行".into(),
            category: "触发器".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "body".into(), data_type: "any".into(), required: false },
                crate::nodes::traits::PortDef { label: "headers".into(), data_type: "object".into(), required: false },
                crate::nodes::traits::PortDef { label: "method".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "query".into(), data_type: "object".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Webhook 路径 (如 /my-hook)" },
                    "method": { "type": "string", "description": "HTTP 方法", "default": "POST" },
                    "auth_header": { "type": "string", "description": "认证头 (可选，格式: Bearer xxx)" }
                },
                "required": ["path"]
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        // Webhook 触发时，请求数据已通过 ctx.variables 注入
        let body = ctx.variables.get("_webhook_body").cloned().unwrap_or(json!({}));
        let headers = ctx.variables.get("_webhook_headers").cloned().unwrap_or(json!({}));
        let method = step.config.get("method").and_then(|v| v.as_str()).unwrap_or("POST");
        let query = ctx.variables.get("_webhook_query").cloned().unwrap_or(json!({}));

        info!("Webhook 触发: {} {}", method, step.config.get("path").and_then(|v| v.as_str()).unwrap_or(""));

        Ok(json!({
            "body": body,
            "headers": headers,
            "method": method,
            "query": query,
            "triggered_at": chrono::Utc::now().to_rfc3339(),
        }))
    }
}
