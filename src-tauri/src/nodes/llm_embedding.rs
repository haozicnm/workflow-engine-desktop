// nodes/llm_embedding.rs — 文本嵌入向量生成节点
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
pub struct LlmEmbeddingNode;

#[async_trait]
impl NodeExecutor for LlmEmbeddingNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "llm_embedding".into(),
            version: "1.0".into(),
            display_name: "文本嵌入".into(),
            description: "生成文本嵌入向量（OpenAI-compatible embedding API）".into(),
            category: "ai".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "text".into(), data_type: "text".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "embedding".into(), data_type: "json".into(), required: false },
                crate::nodes::traits::PortDef { label: "dimensions".into(), data_type: "number".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "api_key": { "type": "string", "description": "API Key 或从环境变量 EMBEDDING_API_KEY 读取" },
                    "api_url": { "type": "string", "default": "https://api.openai.com/v1/embeddings" },
                    "model": { "type": "string", "default": "text-embedding-3-small" },
                    "text": { "type": "string", "description": "要嵌入的文本" }
                },
                "required": ["api_key"]
            }),
            params: vec![],
        }
    }

    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<Value> {
        let config = &step.config;
        let api_key_owned;
        let api_key = match config.get("api_key").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => {
                api_key_owned = std::env::var("EMBEDDING_API_KEY").map_err(|_| anyhow!("llm_embedding: 缺少 api_key"))?;
                &api_key_owned
            }
        };
        let api_url = config.get("api_url").and_then(|v| v.as_str())
            .unwrap_or("https://api.openai.com/v1/embeddings");
        let model = config.get("model").and_then(|v| v.as_str())
            .unwrap_or("text-embedding-3-small");

        let text = ctx.input_ports.get("text").and_then(|v| v.as_str()).map(String::from)
            .or_else(|| config.get("text").and_then(|v| v.as_str()).map(String::from))
            .ok_or_else(|| anyhow!("llm_embedding: 缺少 text"))?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        let resp = client.post(api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&json!({"input": text, "model": model}))
            .send().await.map_err(|e| anyhow!("embedding API 请求失败: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(anyhow!("embedding API 返回 {}: {}", status, err));
        }
        let result: Value = resp.json().await.map_err(|e| anyhow!("解析失败: {}", e))?;
        let embedding = result["data"][0]["embedding"].clone();
        let dims = embedding.as_array().map(|a| a.len()).unwrap_or(0);
        info!("文本嵌入完成: model={} dims={}", model, dims);
        Ok(json!({"embedding": embedding, "dimensions": dims}))
    }
}
