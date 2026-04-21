// nodes/data.rs — 数据处理节点
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use anyhow::{Result, anyhow};

pub struct DataNode;

#[async_trait]
impl NodeExecutor for DataNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("数据节点缺少 action 参数"))?;

        match action {
            "set" => {
                let key = config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("set 需要 key 参数"))?;
                let value = config.get("value").cloned().unwrap_or(serde_json::Value::Null);
                ctx.variables.insert(key.to_string(), value.clone());
                Ok(value)
            }
            "get" => {
                let key = config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("get 需要 key 参数"))?;
                Ok(ctx.variables.get(key).cloned().unwrap_or(serde_json::Value::Null))
            }
            _ => Err(anyhow!("未知的数据操作: {}", action)),
        }
    }
}
