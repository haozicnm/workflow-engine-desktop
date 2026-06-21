// nodes/webhook_response.rs — Webhook 响应节点
// 在 trigger_webhook 触发的工作流中使用，返回 HTTP 响应给调用方
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
pub struct WebhookResponseNode;

#[async_trait]
impl NodeExecutor for WebhookResponseNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "webhook_response".into(),
            version: "1.0".into(),
            display_name: "Webhook 响应".into(),
            description: "向 Webhook 调用方返回 HTTP 响应（必须与 trigger_webhook 配合使用）".into(),
            category: "触发器".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "body".into(), data_type: "any".into(), required: true },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "sent".into(), data_type: "boolean".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "status_code": { "type": "number", "description": "HTTP 状态码", "default": 200 },
                    "headers": { "type": "object", "description": "响应头 (JSON)" },
                    "body": { "type": "any", "description": "响应体（优先使用输入端口数据）" }
                }
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
        let status_code = step.config.get("status_code").and_then(|v| v.as_u64()).unwrap_or(200);
        let headers = step.config.get("headers").cloned().unwrap_or(json!({}));

        // 响应体：优先从输入端口获取，其次从 config
        let body = ctx.variables
            .get("_webhook_response_body")
            .cloned()
            .or_else(|| step.config.get("body").cloned())
            .unwrap_or(json!({}));

        // 将响应数据存入上下文，供 webhook 路由返回
        ctx.set_var("_webhook_response".to_string(), json!({
            "status_code": status_code,
            "headers": headers,
            "body": body,
        }));

        info!("Webhook 响应已设置: status={}", status_code);

        Ok(json!({
            "sent": true,
            "status_code": status_code,
        }))
    }
}
