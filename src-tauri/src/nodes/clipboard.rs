// nodes/clipboard.rs — 剪贴板节点（v3: 每个操作独立 executor）
//
// clipboard_read  — 读取剪贴板文本
// clipboard_write — 写入剪贴板文本

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tracing::info;

#[derive(Default)]
pub struct ClipboardReadNode;

#[async_trait]
impl NodeExecutor for ClipboardReadNode {
    async fn execute(&self, _step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let text = tokio::task::spawn_blocking(|| {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| anyhow!("无法打开剪贴板: {}", e))?;
            clipboard.get_text()
                .map_err(|e| anyhow!("读取剪贴板失败: {}", e))
        }).await.map_err(|e| anyhow!("剪贴板读取任务失败: {}", e))??;

        info!("读取剪贴板: {} 字符", text.len());
        Ok(serde_json::json!({ "text": text, "length": text.len() }))
    }
}

#[derive(Default)]
pub struct ClipboardWriteNode;

#[async_trait]
impl NodeExecutor for ClipboardWriteNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let text = config.get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("write 缺少 text 参数"))?;

        let text_owned = text.to_string();
        let len = text_owned.len();

        tokio::task::spawn_blocking(move || {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| anyhow!("无法打开剪贴板: {}", e))?;
            clipboard.set_text(&text_owned)
                .map_err(|e| anyhow!("写入剪贴板失败: {}", e))
        }).await.map_err(|e| anyhow!("剪贴板写入任务失败: {}", e))??;

        info!("写入剪贴板: {} 字符", len);
        Ok(serde_json::json!({ "text": text, "length": len, "written": true }))
    }
}
