// nodes/mongodb_node.rs — MongoDB 操作节点（通过 HTTP API）
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
pub struct MongodbNode;

#[async_trait]
impl NodeExecutor for MongodbNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "mongodb".into(),
            version: "1.0".into(),
            display_name: "MongoDB".into(),
            description: "MongoDB 文档操作（find/insert/update/delete）".into(),
            category: "data".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "document".into(), data_type: "json".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "results".into(), data_type: "json".into(), required: false },
                crate::nodes::traits::PortDef { label: "count".into(), data_type: "number".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["find", "insert", "update", "delete", "count"], "default": "find" },
                    "collection": { "type": "string", "required": true, "description": "集合名称" },
                    "filter": { "type": "object", "description": "find/update/delete 的查询条件" },
                    "document": { "type": "object", "description": "insert 的文档" },
                    "update": { "type": "object", "description": "update 的更新操作" },
                    "limit": { "type": "number", "default": 100 },
                    "api_url": { "type": "string", "description": "MongoDB HTTP API 地址" }
                },
                "required": ["action", "collection"]
            }),
            params: vec![],
        }
    }

    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<Value> {
        let config = &step.config;
        let action = config.get("action").and_then(|v| v.as_str()).unwrap_or("find");
        let collection = config.get("collection").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("mongodb: 缺少 collection"))?;
        let api_url = config.get("api_url").and_then(|v| v.as_str())
            .unwrap_or("http://localhost:8002");

        let client = reqwest::Client::new();
        let url = format!("{}/{}", api_url, action);

        let body = match action {
            "find" => {
                let filter = config.get("filter").cloned().unwrap_or(json!({}));
                let limit = config.get("limit").and_then(|v| v.as_u64()).unwrap_or(100);
                json!({"collection": collection, "filter": filter, "limit": limit})
            }
            "insert" => {
                let doc = ctx.input_ports.get("document").cloned()
                    .or_else(|| config.get("document").cloned())
                    .ok_or_else(|| anyhow!("mongodb insert: 缺少 document"))?;
                json!({"collection": collection, "document": doc})
            }
            "update" => {
                let filter = config.get("filter").cloned().unwrap_or(json!({}));
                let update = config.get("update").cloned()
                    .or_else(|| ctx.input_ports.get("document").cloned())
                    .ok_or_else(|| anyhow!("mongodb update: 缺少 update"))?;
                json!({"collection": collection, "filter": filter, "update": update})
            }
            "delete" => {
                let filter = config.get("filter").cloned().unwrap_or(json!({}));
                json!({"collection": collection, "filter": filter})
            }
            "count" => {
                let filter = config.get("filter").cloned().unwrap_or(json!({}));
                json!({"collection": collection, "filter": filter})
            }
            _ => return Err(anyhow!("mongodb: 不支持的操作 '{}'", action)),
        };

        info!("MongoDB {}: collection={}", action, collection);
        let resp = client.post(&url).json(&body).send().await
            .map_err(|e| anyhow!("MongoDB API 请求失败: {}", e))?;
        let status = resp.status();
        let result: Value = resp.json().await.unwrap_or(json!({}));
        if status.is_success() {
            let count = result["count"].as_u64().or_else(|| result["results"].as_array().map(|a| a.len() as u64));
            Ok(json!({"results": result, "count": count.unwrap_or(0)}))
        } else {
            Err(anyhow!("MongoDB API 返回 {}: {}", status, result))
        }
    }
}
