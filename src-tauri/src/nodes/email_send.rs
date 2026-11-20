// nodes/email_send.rs — 邮件发送节点（SMTP）
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
pub struct EmailSendNode;

#[async_trait]
impl NodeExecutor for EmailSendNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "email_send".into(),
            version: "1.0".into(),
            display_name: "发送邮件".into(),
            description: "通过 SMTP 发送邮件（支持纯文本和 HTML）".into(),
            category: "核心".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "body".into(), data_type: "string".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "sent".into(), data_type: "boolean".into(), required: false },
                crate::nodes::traits::PortDef { label: "message_id".into(), data_type: "string".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "smtp_host": { "type": "string", "description": "SMTP 服务器地址" },
                    "smtp_port": { "type": "number", "description": "SMTP 端口", "default": 587 },
                    "username": { "type": "string", "description": "SMTP 用户名" },
                    "password": { "type": "string", "description": "SMTP 密码" },
                    "from": { "type": "string", "description": "发件人地址" },
                    "to": { "type": "string", "description": "收件人地址（逗号分隔）" },
                    "subject": { "type": "string", "description": "邮件主题" },
                    "body": { "type": "string", "description": "邮件正文" },
                    "html": { "type": "boolean", "description": "是否 HTML 格式", "default": false }
                },
                "required": ["smtp_host", "from", "to", "subject"]
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

        let password = config.get("password").and_then(|v| v.as_str()).unwrap_or("");
        let from = config.get("from").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("email_send: 缺少 from"))?;
        let to = config.get("to").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("email_send: 缺少 to"))?;
        let subject = config.get("subject").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("email_send: 缺少 subject"))?;

        // 优先从输入端口获取正文
        let body = ctx.input_ports.values().next().and_then(|v| v.as_str()).map(String::from)
            .or_else(|| config.get("body").and_then(|v| v.as_str()).map(String::from))
            .unwrap_or_default();

        let is_html = config.get("html").and_then(|v| v.as_bool()).unwrap_or(false);

        // 使用 lettre crate 通过 SMTP 发送邮件
        // 注意：lettre 是 optional 依赖，需要在 Cargo.toml 中启用
        #[cfg(feature = "email")]
        {
            use lettre::message::{header::ContentType, Mailbox};
            use lettre::transport::smtp::authentication::Credentials;
            use lettre::{Message, SmtpTransport, Transport};

            // 这三个参数只在启用 email feature 时用到。旧代码把它们绑定成
            // `_smtp_host/_smtp_port/_username`（带下划线 = 视为未使用），
            // 但本分支引用的是无下划线名字 → `cargo build --features email`
            // 一直都编译不过（E0425），该 feature 实际从未可用。
            let smtp_host = config.get("smtp_host").and_then(|v| v.as_str()).unwrap_or("");
            let smtp_port = config.get("smtp_port").and_then(|v| v.as_u64()).unwrap_or(587) as u16;
            let username = config.get("username").and_then(|v| v.as_str()).unwrap_or("");

            let from_mailbox: Mailbox = from.parse().map_err(|e| anyhow!("发件人地址无效: {}", e))?;
            let to_mailboxes: Vec<Mailbox> = to.split(',')
                .map(|addr| addr.trim().parse())
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| anyhow!("收件人地址无效: {}", e))?;

            // lettre 0.11 的 Response 没有 message_id 字段（旧代码引用了它 → E0609），
            // 因此自行生成 Message-ID 并写入邮件头，输出端口仍返回该字符串。
            let message_id = format!("{}@workflow-engine", uuid::Uuid::new_v4());

            let mut builder = Message::builder()
                .from(from_mailbox)
                .subject(subject)
                .message_id(Some(message_id.clone()));
            for mailbox in &to_mailboxes {
                builder = builder.to(mailbox.clone());
            }

            let message = if is_html {
                builder.header(ContentType::TEXT_HTML).body(body.clone())
            } else {
                builder.header(ContentType::TEXT_PLAIN).body(body.clone())
            }.map_err(|e| anyhow!("邮件构造失败: {}", e))?;

            let mut transport_builder = SmtpTransport::builder_dangerous(smtp_host)
                .port(smtp_port);
            if !username.is_empty() {
                transport_builder = transport_builder.credentials(Credentials::new(username.to_string(), password.to_string()));
            }
            let transport = transport_builder.build();

            // lettre 0.11 的 send() 是同步阻塞调用，直接在 async fn 里调用会卡住
            // tokio worker 线程，因此放到 spawn_blocking 里执行。
            let send_result = tokio::task::spawn_blocking(move || transport.send(&message))
                .await
                .map_err(|e| anyhow!("SMTP 发送任务失败: {}", e))?
                .map_err(|e| anyhow!("SMTP 发送失败: {}", e))?;

            info!(
                "邮件已发送: {} → {} (message_id: {}, smtp: {})",
                from,
                to,
                message_id,
                send_result.first_line().unwrap_or("")
            );
            Ok(json!({
                "sent": true,
                "message_id": message_id,
            }))
        }

        #[cfg(not(feature = "email"))]
        {
            // 无 lettre 依赖时使用外部 SMTP 命令（降级方案）
            info!("邮件发送（降级模式）: {} → {} subject={}", from, to, subject);
            // 通过 HTTP API 发送（如 Resend/SendGrid）
            if let Some(api_url) = config.get("smtp_api_url").and_then(|v| v.as_str()) {
                let client = reqwest::Client::new();
                let mut req = client.post(api_url)
                    .json(&json!({
                        "from": from,
                        "to": to.split(',').map(|s| s.trim()).collect::<Vec<_>>(),
                        "subject": subject,
                        if is_html { "html" } else { "text" }: body,
                    }));
                if !password.is_empty() {
                    req = req.header("Authorization", format!("Bearer {}", password));
                }
                let resp = req.send()
                    .await
                    .map_err(|e| anyhow!("邮件 API 请求失败: {}", e))?;
                let status = resp.status();
                if status.is_success() {
                    Ok(json!({"sent": true, "message_id": "api"}))
                } else {
                    Err(anyhow!("邮件 API 返回错误 {}", status))
                }
            } else {
                Err(anyhow!("email_send: 需要启用 email feature 或配置 smtp_api_url"))
            }
        }
    }
}
