// nodes/im_message.rs — IM 消息发送节点（通用多平台）
// 支持：Slack、飞书、钉钉、企业微信、Telegram
// 统一通过 HTTP Webhook/API 发送，不引入专用 crate
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;

#[derive(Default)]
pub struct ImMessageNode;

/// 平台特定的请求体构造
fn build_request_body(platform: &str, text: &str, title: &str, webhook_url: &str) -> Result<(String, Value)> {
    match platform {
        "slack" => Ok((
            webhook_url.to_string(),
            json!({
                "text": text,
                "blocks": [{
                    "type": "section",
                    "text": { "type": "mrkdwn", "text": text }
                }]
            }),
        )),
        "feishu" | "lark" => Ok((
            webhook_url.to_string(),
            json!({
                "msg_type": "interactive",
                "card": {
                    "header": {
                        "title": { "tag": "plain_text", "content": title },
                        "template": "blue"
                    },
                    "elements": [{
                        "tag": "markdown",
                        "content": text
                    }]
                }
            }),
        )),
        "dingtalk" | "dingding" => Ok((
            webhook_url.to_string(),
            json!({
                "msgtype": "markdown",
                "markdown": {
                    "title": title,
                    "text": text
                }
            }),
        )),
        "wecom" | "wechat_work" => Ok((
            webhook_url.to_string(),
            json!({
                "msgtype": "markdown",
                "markdown": {
                    "content": format!("### {}\n{}", title, text)
                }
            }),
        )),
        "telegram" => {
            // Telegram Bot API: POST /bot{token}/sendMessage
            let bot_token = webhook_url.split("/bot").nth(1)
                .and_then(|s| s.split('/').next())
                .unwrap_or("");
            let chat_id = webhook_url.split("chat_id=").nth(1)
                .unwrap_or("")
                .split('&')
                .next()
                .unwrap_or("");
            let api_url = format!("https://api.telegram.org/bot{}/sendMessage", bot_token);
            Ok((
                api_url,
                json!({
                    "chat_id": chat_id,
                    "text": format!("*{}*\n{}", title, text),
                    "parse_mode": "Markdown"
                }),
            ))
        }
        _ => Err(anyhow!("im_message: 不支持的平台 '{}'，支持: slack/feishu/dingtalk/wecom/telegram", platform)),
    }
}

#[async_trait]
impl NodeExecutor for ImMessageNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "im_message".into(),
            version: "1.0".into(),
            display_name: "IM 消息".into(),
            description: "发送消息到 Slack/飞书/钉钉/企业微信/Telegram（通过 Webhook）".into(),
            category: "集成".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "text".into(), data_type: "string".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "sent".into(), data_type: "boolean".into(), required: false },
                crate::nodes::traits::PortDef { label: "response".into(), data_type: "object".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "platform": {
                        "type": "string",
                        "enum": ["slack", "feishu", "dingtalk", "wecom", "telegram"],
                        "description": "IM 平台"
                    },
                    "webhook_url": { "type": "string", "description": "Webhook URL（从 Settings 集成服务配置读取，或直接填写）" },
                    "title": { "type": "string", "description": "消息标题", "default": "Workflow Engine" },
                    "text": { "type": "string", "description": "消息内容（支持 Markdown）" }
                },
                "required": ["platform", "webhook_url"]
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config = &step.config;

        let platform = config.get("platform").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("im_message: 缺少 platform"))?;
        let webhook_url = config.get("webhook_url").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("im_message: 缺少 webhook_url"))?;
        let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("Workflow Engine");

        // 优先从输入端口获取文本
        let text = ctx.input_ports.values().next().and_then(|v| v.as_str()).map(String::from)
            .or_else(|| config.get("text").and_then(|v| v.as_str()).map(String::from))
            .unwrap_or_default();

        if text.is_empty() {
            return Err(anyhow!("im_message: 消息内容为空"));
        }

        let (url, body) = build_request_body(platform, &text, title, webhook_url)?;

        info!("IM 消息发送: platform={}, title={}", platform, title);

        let client = reqwest::Client::new();
        let resp = client.post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow!("IM 消息发送失败 [{}]: {}", platform, e))?;

        let status = resp.status();
        let resp_body: Value = resp.json().await.unwrap_or(json!({}));

        if status.is_success() {
            info!("IM 消息发送成功: platform={}", platform);
            Ok(json!({
                "sent": true,
                "response": resp_body,
            }))
        } else {
            Err(anyhow!("IM 消息发送失败 [{}]: HTTP {} - {}", platform, status, resp_body))
        }
    }
}
