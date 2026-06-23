// nodes/text_splitter.rs — 文本分块节点（用于 RAG 文档处理）
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::warn;

#[derive(Default)]
pub struct TextSplitterNode;

#[async_trait]
impl NodeExecutor for TextSplitterNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "text_splitter".into(),
            version: "1.0".into(),
            display_name: "文本分块".into(),
            description: "将长文本按 chunk_size 分割为多个块（用于 RAG 文档处理）".into(),
            category: "ai".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "text".into(), data_type: "text".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "chunks".into(), data_type: "json".into(), required: false },
                crate::nodes::traits::PortDef { label: "count".into(), data_type: "number".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "要分块的文本" },
                    "chunk_size": { "type": "number", "default": 1000, "description": "每块最大字符数" },
                    "chunk_overlap": { "type": "number", "default": 200, "description": "块之间重叠字符数" },
                    "separator": { "type": "string", "default": "\n", "description": "优先分割符" }
                }
            }),
            params: vec![],
        }
    }

    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<Value> {
        let config = &step.config;
        let text = ctx.input_ports.get("text").and_then(|v| v.as_str()).map(String::from)
            .or_else(|| config.get("text").and_then(|v| v.as_str()).map(String::from))
            .ok_or_else(|| anyhow!("text_splitter: 缺少 text"))?;

        let chunk_size = config.get("chunk_size").and_then(|v| v.as_u64()).unwrap_or(1000) as usize;
        let chunk_overlap = config.get("chunk_overlap").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
        let separator = config.get("separator").and_then(|v| v.as_str()).unwrap_or("\n");

        let mut chunks: Vec<String> = Vec::new();
        let chars: Vec<char> = text.chars().collect();
        let total = chars.len();

        if total <= chunk_size {
            chunks.push(text.clone());
        } else {
            let mut start = 0;
            while start < total {
                let end = (start + chunk_size).min(total);
                let chunk: String = chars[start..end].iter().collect();

                // 尝试在 separator 处断开
                if end < total {
                    if let Some(last_sep) = chunk.rfind(separator) {
                        if last_sep > chunk_size / 4 {
                            let actual_end = start + last_sep + separator.len();
                            let actual_chunk: String = chars[start..actual_end].iter().collect();
                            chunks.push(actual_chunk);
                            start = actual_end - chunk_overlap;
                            continue;
                        }
                    }
                }
                chunks.push(chunk);
                if end >= total { break; }
                start = end - chunk_overlap;
            }
        }

        let count = chunks.len();
        Ok(json!({"chunks": chunks, "count": count}))
    }
}
