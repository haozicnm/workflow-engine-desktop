// nodes/condition.rs — 条件分支节点
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use anyhow::{Result, anyhow};

pub struct ConditionNode;

#[async_trait]
impl NodeExecutor for ConditionNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext) -> Result<serde_json::Value> {
        let condition = step.config.get("condition").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("条件节点缺少 condition 参数"))?;

        let result = ctx.eval_expr(condition).map_err(|e| anyhow!(e))?;
        let is_true = result.as_bool().unwrap_or(false);

        Ok(serde_json::json!({
            "result": is_true,
            "branch": if is_true { "true" } else { "false" },
        }))
    }
}
