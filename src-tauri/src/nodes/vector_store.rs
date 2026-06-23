// nodes/vector_store.rs — 向量存储节点（SQLite 本地实现）
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
pub struct VectorStoreNode;

#[async_trait]
impl NodeExecutor for VectorStoreNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "vector_store".into(),
            version: "1.0".into(),
            display_name: "向量存储".into(),
            description: "向量存储操作（add/search/delete）— 基于 SQLite 本地存储".into(),
            category: "ai".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "embedding".into(), data_type: "json".into(), required: false },
                crate::nodes::traits::PortDef { label: "text".into(), data_type: "text".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "results".into(), data_type: "json".into(), required: false },
                crate::nodes::traits::PortDef { label: "count".into(), data_type: "number".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["add", "search", "delete", "list"], "default": "add" },
                    "collection": { "type": "string", "default": "default", "description": "集合名称" },
                    "db_path": { "type": "string", "default": "vectors.db", "description": "SQLite 数据库路径" },
                    "text": { "type": "string", "description": "add: 要存储的文本" },
                    "embedding": { "type": "array", "description": "add/search: 向量" },
                    "top_k": { "type": "number", "default": 5, "description": "search: 返回前 K 个结果" }
                },
                "required": ["action"]
            }),
            params: vec![],
        }
    }

    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<Value> {
        let config = &step.config;
        let action = config.get("action").and_then(|v| v.as_str()).unwrap_or("add");
        let collection = config.get("collection").and_then(|v| v.as_str()).unwrap_or("default");
        let db_path = config.get("db_path").and_then(|v| v.as_str()).unwrap_or("vectors.db");

        let conn = rusqlite::Connection::open(db_path)
            .map_err(|e| anyhow!("向量数据库连接失败: {}", e))?;

        // 自动创建表
        conn.execute_batch(&format!(
            "CREATE TABLE IF NOT EXISTS vectors_{} (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                embedding TEXT NOT NULL,
                metadata TEXT,
                created_at TEXT DEFAULT CURRENT_TIMESTAMP
            );", collection
        )).map_err(|e| anyhow!("创建表失败: {}", e))?;

        match action {
            "add" => {
                let text = ctx.input_ports.get("text").and_then(|v| v.as_str()).map(String::from)
                    .or_else(|| config.get("text").and_then(|v| v.as_str()).map(String::from))
                    .ok_or_else(|| anyhow!("vector_store add: 缺少 text"))?;
                let embedding = ctx.input_ports.get("embedding").cloned()
                    .or_else(|| config.get("embedding").cloned())
                    .ok_or_else(|| anyhow!("vector_store add: 缺少 embedding"))?;
                let embedding_str = serde_json::to_string(&embedding)?;
                conn.execute(
                    &format!("INSERT INTO vectors_{} (text, embedding) VALUES (?1, ?2)", collection),
                    rusqlite::params![text, embedding_str],
                ).map_err(|e| anyhow!("插入失败: {}", e))?;
                info!("向量存储添加: collection={}", collection);
                Ok(json!({"added": true, "collection": collection}))
            }
            "search" => {
                let _top_k = config.get("top_k").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
                // 简单实现：返回最近添加的 K 条（无真正的向量相似度计算）
                let mut stmt = conn.prepare(&format!(
                    "SELECT text, embedding, created_at FROM vectors_{} ORDER BY id DESC LIMIT ?1", collection
                )).map_err(|e| anyhow!("查询失败: {}", e))?;
                let rows: Vec<Value> = stmt.query_map(rusqlite::params![_top_k], |row| {
                    let text: String = row.get(0)?;
                    let embedding_str: String = row.get(1)?;
                    let created_at: String = row.get(2)?;
                    Ok(json!({"text": text, "embedding": embedding_str, "created_at": created_at}))
                }).map_err(|e| anyhow!("查询失败: {}", e))?
                .filter_map(|r| r.ok())
                .collect();
                let count = rows.len();
                Ok(json!({"results": rows, "count": count}))
            }
            "list" => {
                let mut stmt = conn.prepare(&format!(
                    "SELECT COUNT(*) FROM vectors_{}", collection
                )).map_err(|e| anyhow!("查询失败: {}", e))?;
                let count: i64 = stmt.query_row([], |row| row.get(0)).unwrap_or(0);
                Ok(json!({"count": count, "collection": collection}))
            }
            "delete" => {
                conn.execute(&format!("DELETE FROM vectors_{}", collection), [])
                    .map_err(|e| anyhow!("删除失败: {}", e))?;
                Ok(json!({"deleted": true, "collection": collection}))
            }
            _ => Err(anyhow!("vector_store: 未知操作 '{}'", action)),
        }
    }
}
