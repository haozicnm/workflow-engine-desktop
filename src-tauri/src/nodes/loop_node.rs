// nodes/loop_node.rs — 循环节点
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use anyhow::{Result, anyhow};

pub struct LoopNode;

#[async_trait]
impl NodeExecutor for LoopNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext) -> Result<serde_json::Value> {
        let items = step.config.get("items").and_then(|v| v.as_array())
            .ok_or_else(|| anyhow!("循环节点缺少 items 参数"))?;

        let mut results = Vec::new();
        for item in items {
            // TODO: 执行循环体
            results.push(item.clone());
        }

        Ok(serde_json::json!({
            "count": results.len(),
            "items": results,
        }))
    }
}
