// nodes/clipboard.rs — 剪贴板节点（v3: 每个操作独立 executor）
//
// clipboard_read  — 读取剪贴板文本
// clipboard_write — 写入剪贴板文本

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

#[derive(Default)]
pub struct ClipboardReadNode;

#[async_trait]
impl NodeExecutor for ClipboardReadNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "clipboard_read".into(),
            version: "1.0".into(),
            display_name: "读取剪贴板".into(),
            description: "读取系统剪贴板中的文本内容".into(),
            category: "系统".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "text".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "length".into(), data_type: "number".into(), required: false },
            ],
            config_schema: serde_json::json!({"type": "object"}),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        _step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let text = tokio::task::spawn_blocking(|| {
            let mut clipboard =
                arboard::Clipboard::new().map_err(|e| anyhow!("无法打开剪贴板: {}", e))?;
            clipboard
                .get_text()
                .map_err(|e| anyhow!("读取剪贴板失败: {}", e))
        })
        .await
        .map_err(|e| anyhow!("剪贴板读取任务失败: {}", e))??;

        info!("读取剪贴板: {} 字符", text.len());
        Ok(serde_json::json!({ "text": text, "length": text.len() }))
    }
}

#[derive(Default)]
pub struct ClipboardWriteNode;

#[async_trait]
impl NodeExecutor for ClipboardWriteNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "clipboard_write".into(),
            version: "1.0".into(),
            display_name: "写入剪贴板".into(),
            description: "写入文本内容到系统剪贴板".into(),
            category: "系统".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "text".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "length".into(), data_type: "number".into(), required: false },
                crate::nodes::traits::PortDef { label: "written".into(), data_type: "boolean".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "required": ["text"],
                "properties": {
                    "text": {"type": "string", "description": "要写入的文本"}
                }
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let text = config
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("write 缺少 text 参数"))?;

        let text_owned = text.to_string();
        let len = text_owned.len();

        tokio::task::spawn_blocking(move || {
            let mut clipboard =
                arboard::Clipboard::new().map_err(|e| anyhow!("无法打开剪贴板: {}", e))?;
            clipboard
                .set_text(&text_owned)
                .map_err(|e| anyhow!("写入剪贴板失败: {}", e))
        })
        .await
        .map_err(|e| anyhow!("剪贴板写入任务失败: {}", e))??;

        info!("写入剪贴板: {} 字符", len);
        Ok(serde_json::json!({ "text": text, "length": len, "written": true }))
    }
}
