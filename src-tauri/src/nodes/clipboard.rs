// nodes/clipboard.rs — 剪贴板节点
//
// 支持操作：
//   read   读取剪贴板文本:  {action: "read"}
//   write  写入剪贴板文本:  {action: "write", text: "hello"}
//
// 使用 arboard crate 进行跨平台剪贴板操作。

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tracing::info;

#[derive(Default)]
pub struct ClipboardNode;

#[async_trait]
impl NodeExecutor for ClipboardNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("剪贴板节点缺少 action 参数"))?;

        match action {
            "read" => clipboard_read().await,
            "write" => clipboard_write(config).await,
            _ => Err(anyhow!(
                "未知的剪贴板操作: {}（支持 read/write）",
                action
            )),
        }
    }
}

/// 读取剪贴板文本内容
async fn clipboard_read() -> Result<serde_json::Value> {
    // arboard 的 get_text 是同步的，在 spawn_blocking 中执行以避免阻塞
    let text = tokio::task::spawn_blocking(|| {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| anyhow!("无法打开剪贴板: {}", e))?;
        clipboard.get_text()
            .map_err(|e| anyhow!("读取剪贴板失败: {}", e))
    }).await
        .map_err(|e| anyhow!("剪贴板读取任务失败: {}", e))??;

    info!("读取剪贴板: {} 字符", text.len());

    Ok(serde_json::json!({
        "action": "read",
        "text": text,
        "length": text.len(),
    }))
}

/// 写入文本到剪贴板
async fn clipboard_write(config: &serde_json::Value) -> Result<serde_json::Value> {
    let text = config.get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("write 操作缺少 text 参数"))?;

    let text_owned = text.to_string();
    let len = text_owned.len();

    // arboard 的 set_text 是同步的，在 spawn_blocking 中执行
    tokio::task::spawn_blocking(move || {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|e| anyhow!("无法打开剪贴板: {}", e))?;
        clipboard.set_text(&text_owned)
            .map_err(|e| anyhow!("写入剪贴板失败: {}", e))
    }).await
        .map_err(|e| anyhow!("剪贴板写入任务失败: {}", e))??;

    info!("写入剪贴板: {} 字符", len);

    Ok(serde_json::json!({
        "action": "write",
        "text": text,
        "length": len,
        "written": true,
    }))
}
