// nodes/clipboard_container.rs — 剪贴板容器执行器
//
// 根据 config.action 路由到 clipboard_read 或 clipboard_write

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipboardContainerConfig {
    #[serde(default = "default_action")]
    pub action: String,
    #[serde(default)]
    pub text: String,
}

fn default_action() -> String {
    "read".to_string()
}

#[derive(Default)]
pub struct ClipboardContainerNode;

#[async_trait]
impl NodeExecutor for ClipboardContainerNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config: ClipboardContainerConfig = serde_json::from_value(step.config.clone())
            .map_err(|e| anyhow!("剪贴板容器配置解析失败: {}", e))?;

        match config.action.as_str() {
            "read" => {
                let text = tokio::task::spawn_blocking(|| {
                    let mut clipboard = arboard::Clipboard::new()
                        .map_err(|e| anyhow!("无法打开剪贴板: {}", e))?;
                    clipboard
                        .get_text()
                        .map_err(|e| anyhow!("读取剪贴板失败: {}", e))
                })
                .await
                .map_err(|e| anyhow!("剪贴板读取任务失败: {}", e))??;

                Ok(serde_json::json!({ "text": text, "length": text.len() }))
            }
            "write" => {
                let text_owned = config.text.clone();
                let len = text_owned.len();

                tokio::task::spawn_blocking(move || {
                    let mut clipboard = arboard::Clipboard::new()
                        .map_err(|e| anyhow!("无法打开剪贴板: {}", e))?;
                    clipboard
                        .set_text(&text_owned)
                        .map_err(|e| anyhow!("写入剪贴板失败: {}", e))
                })
                .await
                .map_err(|e| anyhow!("剪贴板写入任务失败: {}", e))??;

                Ok(serde_json::json!({
                    "text": config.text,
                    "length": len,
                    "written": true
                }))
            }
            other => Err(anyhow!("剪贴板容器未知操作: {}", other)),
        }
    }
}
