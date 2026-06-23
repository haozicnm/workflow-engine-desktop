// nodes/redis_node.rs — Redis 操作节点（通过 HTTP API 或本地 redis-cli）
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
pub struct RedisNode;

#[async_trait]
impl NodeExecutor for RedisNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "redis".into(),
            version: "1.0".into(),
            display_name: "Redis".into(),
            description: "Redis 键值操作（get/set/del/keys/lpush/rpush）".into(),
            category: "data".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "value".into(), data_type: "text".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "json".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["get", "set", "del", "keys", "lpush", "rpush", "lrange", "incr"], "default": "get" },
                    "key": { "type": "string", "description": "Redis key" },
                    "value": { "type": "string", "description": "set/lpush/rpush 的值" },
                    "ttl": { "type": "number", "description": "set 的过期时间(秒)" },
                    "pattern": { "type": "string", "default": "*", "description": "keys 的匹配模式" },
                    "start": { "type": "number", "default": 0, "description": "lrange 起始" },
                    "stop": { "type": "number", "default": -1, "description": "lrange 结束" },
                    "api_url": { "type": "string", "description": "Redis HTTP API 地址（如 http://localhost:8001）" }
                },
                "required": ["action"]
            }),
            params: vec![],
        }
    }

    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<Value> {
        let config = &step.config;
        let action = config.get("action").and_then(|v| v.as_str()).unwrap_or("get");
        let key = config.get("key").and_then(|v| v.as_str()).unwrap_or("");
        let api_url = config.get("api_url").and_then(|v| v.as_str())
            .unwrap_or("http://localhost:8001");

        let client = reqwest::Client::new();
        let url = format!("{}/{}", api_url, action);

        let body = match action {
            "get" | "del" | "incr" => json!({"key": key}),
            "set" => {
                let val = ctx.input_ports.get("value").and_then(|v| v.as_str()).map(String::from)
                    .or_else(|| config.get("value").and_then(|v| v.as_str()).map(String::from))
                    .unwrap_or_default();
                let ttl = config.get("ttl").and_then(|v| v.as_u64());
                let mut b = json!({"key": key, "value": val});
                if let Some(t) = ttl { b["ttl"] = json!(t); }
                b
            }
            "keys" => json!({"pattern": config.get("pattern").and_then(|v| v.as_str()).unwrap_or("*")}),
            "lpush" | "rpush" => {
                let val = ctx.input_ports.get("value").and_then(|v| v.as_str()).map(String::from)
                    .or_else(|| config.get("value").and_then(|v| v.as_str()).map(String::from))
                    .unwrap_or_default();
                json!({"key": key, "value": val})
            }
            "lrange" => json!({
                "key": key,
                "start": config.get("start").and_then(|v| v.as_i64()).unwrap_or(0),
                "stop": config.get("stop").and_then(|v| v.as_i64()).unwrap_or(-1)
            }),
            _ => return Err(anyhow!("redis: 不支持的操作 '{}'", action)),
        };

        info!("Redis {}: key={}", action, key);
        let resp = client.post(&url).json(&body).send().await
            .map_err(|e| anyhow!("Redis API 请求失败: {}", e))?;
        let status = resp.status();
        let result: Value = resp.json().await.unwrap_or(json!({}));
        if status.is_success() {
            Ok(json!({"result": result}))
        } else {
            Err(anyhow!("Redis API 返回 {}: {}", status, result))
        }
    }
}
