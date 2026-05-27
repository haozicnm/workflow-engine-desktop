// nodes/notify.rs — 通知节点
// 支持：系统通知、Webhook
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::{info, warn};

/// Windows: 禁止子进程弹出 cmd 窗口
#[cfg(target_os = "windows")]
fn hide_console(cmd: &mut tokio::process::Command) {
    #[allow(unused_imports)]
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x08000000);
}
#[cfg(not(target_os = "windows"))]
#[allow(dead_code)]
fn hide_console(_cmd: &mut tokio::process::Command) {}

// nodes/notify.rs
pub struct NotifyNode;

#[async_trait]
impl NodeExecutor for NotifyNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let notify_type = config
            .get("notify_type")
            .and_then(|v| v.as_str())
            .unwrap_or("system");

        match notify_type {
            "system" => send_system_notification(config).await,
            "webhook" => send_webhook(config).await,
            _ => Err(anyhow!(
                "不支持的通知类型: {}（支持 system / webhook）",
                notify_type
            )),
        }
    }
}

/// 发送系统桌面通知
async fn send_system_notification(config: &serde_json::Value) -> Result<serde_json::Value> {
    let title = config
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("Workflow Engine");
    let body = config
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("系统通知缺少 body 参数"))?;

    // Windows: 使用 powershell 发送 toast 通知
    #[cfg(target_os = "windows")]
    {
        let script = format!(
            r#"[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] | Out-Null
[Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom, ContentType = WindowsRuntime] | Out-Null
$template = @"
<toast>
  <visual>
    <binding template="ToastGeneric">
      <text>{}</text>
      <text>{}</text>
    </binding>
  </visual>
</toast>
"@
$xml = New-Object Windows.Data.Xml.Dom.XmlDocument
$xml.LoadXml($template)
$toast = [Windows.UI.Notifications.ToastNotification]::new($xml)
[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier("Workflow Engine").Show($toast)
"#,
            title.replace('"', "'"),
            body.replace('"', "'"),
        );

        let mut cmd = tokio::process::Command::new("powershell");
        hide_console(&mut cmd);
        match cmd.args(["-Command", &script]).output().await {
            Ok(_) => info!("系统通知已发送: {}", title),
            Err(e) => warn!("系统通知发送失败: {}（非关键错误）", e),
        }
    }

    // 非 Windows: 暂时只记录日志
    #[cfg(not(target_os = "windows"))]
    {
        info!("[通知] {}: {}", title, body);
        warn!("桌面通知暂仅支持 Windows（非关键功能）");
    }

    Ok(serde_json::json!({
        "type": "system",
        "title": title,
        "body": body,
        "sent": true,
    }))
}

/// 发送 Webhook 请求
async fn send_webhook(config: &serde_json::Value) -> Result<serde_json::Value> {
    let url = config
        .get("url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("Webhook 缺少 url 参数"))?;

    let method = config
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("POST");

    let body = config
        .get("data")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));

    let client = reqwest::Client::new();
    let mut req = match method.to_uppercase().as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        _ => return Err(anyhow!("不支持的 Webhook 方法: {}", method)),
    };

    // 添加 headers
    if let Some(headers) = config.get("headers").and_then(|v| v.as_object()) {
        for (k, v) in headers {
            if let Some(val) = v.as_str() {
                req = req.header(k, val);
            }
        }
    }

    // 添加 body
    if !matches!(method.to_uppercase().as_str(), "GET") {
        req = req.json(&body);
    }

    let resp = req
        .send()
        .await
        .map_err(|e| anyhow!("Webhook 请求失败: {}", e))?;
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap_or_default();

    let resp_body =
        serde_json::from_str::<serde_json::Value>(&text).unwrap_or(serde_json::Value::String(text));

    Ok(serde_json::json!({
        "type": "webhook",
        "url": url,
        "method": method,
        "status": status,
        "response": resp_body,
    }))
}
