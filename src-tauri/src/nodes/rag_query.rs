// nodes/rag_query.rs — RAG 检索增强生成节点
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
pub struct RagQueryNode;

#[async_trait]
impl NodeExecutor for RagQueryNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "rag_query".into(),
            version: "1.0".into(),
            display_name: "RAG 查询".into(),
            description: "RAG 检索增强生成 — 从向量存储检索相关文档，拼接为上下文发送给 LLM".into(),
            category: "ai".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "query".into(), data_type: "text".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "answer".into(), data_type: "text".into(), required: false },
                crate::nodes::traits::PortDef { label: "context".into(), data_type: "text".into(), required: false },
                crate::nodes::traits::PortDef { label: "sources".into(), data_type: "json".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "查询问题" },
                    "collection": { "type": "string", "default": "default" },
                    "db_path": { "type": "string", "default": "vectors.db" },
                    "top_k": { "type": "number", "default": 3 },
                    "api_key": { "type": "string", "required": true },
                    "api_url": { "type": "string", "default": "https://api.openai.com/v1/chat/completions" },
                    "model": { "type": "string", "default": "gpt-4o" }
                },
                "required": ["api_key"]
            }),
            params: vec![],
        }
    }

    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<Value> {
        let config = &step.config;
        let query = ctx.input_ports.get("query").and_then(|v| v.as_str()).map(String::from)
            .or_else(|| config.get("query").and_then(|v| v.as_str()).map(String::from))
            .ok_or_else(|| anyhow!("rag_query: 缺少 query"))?;
        let collection = config.get("collection").and_then(|v| v.as_str()).unwrap_or("default");
        let db_path = config.get("db_path").and_then(|v| v.as_str()).unwrap_or("vectors.db");
        let top_k = config.get("top_k").and_then(|v| v.as_u64()).unwrap_or(3) as usize;

        // 1. 从向量存储检索相关文档
        let docs = {
            let conn = rusqlite::Connection::open(db_path)
                .map_err(|e| anyhow!("向量数据库连接失败: {}", e))?;
            let mut stmt = conn.prepare(&format!(
                "SELECT text FROM vectors_{} ORDER BY id DESC LIMIT ?1", collection
            )).map_err(|e| anyhow!("查询失败: {}", e))?;
            let mut rows = stmt.query(rusqlite::params![top_k])
                .map_err(|e| anyhow!("查询失败: {}", e))?;
            let mut result = Vec::new();
            while let Some(row) = rows.next().map_err(|e| anyhow!("读取行失败: {}", e))? {
                if let Ok(text) = row.get::<_, String>(0) {
                    result.push(text);
                }
            }
            result
        };

        if docs.is_empty() {
            return Ok(json!({"answer": "没有找到相关文档", "context": "", "sources": []}));
        }

        // 2. 拼接检索上下文
        let context = docs.iter().enumerate()
            .map(|(i, doc)| format!("[文档 {}] {}", i + 1, doc))
            .collect::<Vec<_>>()
            .join("\n\n");

        // 3. 调用 LLM 生成回答
        let api_key_owned;
        let api_key = match config.get("api_key").and_then(|v| v.as_str()) {
            Some(k) => k,
            None => {
                api_key_owned = std::env::var("LLM_API_KEY").map_err(|_| anyhow!("rag_query: 缺少 api_key"))?;
                &api_key_owned
            }
        };
        let api_url = config.get("api_url").and_then(|v| v.as_str())
            .unwrap_or("https://api.openai.com/v1/chat/completions");
        let model = config.get("model").and_then(|v| v.as_str()).unwrap_or("gpt-4o");

        let prompt = format!(
            "根据以下参考资料回答用户问题。如果参考资料中没有相关信息，请说明。\n\n参考资料：\n{}\n\n用户问题：{}",
            context, query
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;
        let resp = client.post(api_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&json!({
                "model": model,
                "messages": [
                    {"role": "system", "content": "你是一个知识库问答助手，根据提供的参考资料回答问题。"},
                    {"role": "user", "content": prompt}
                ],
                "temperature": 0.3,
                "max_tokens": 1024
            }))
            .send().await.map_err(|e| anyhow!("RAG LLM 请求失败: {}", e))?;

        let status = resp.status();
        if !status.is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(anyhow!("RAG LLM 返回 {}: {}", status, err));
        }
        let result: Value = resp.json().await.map_err(|e| anyhow!("解析失败: {}", e))?;
        let answer = result["choices"][0]["message"]["content"].as_str().unwrap_or("").to_string();

        info!("RAG 查询完成: {} 个文档, query={}", docs.len(), &query[..query.len().min(50)]);
        Ok(json!({"answer": answer, "context": context, "sources": docs}))
    }
}
