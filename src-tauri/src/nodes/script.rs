// nodes/script.rs — rhai 脚本节点
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use anyhow::{Result, anyhow};

pub struct ScriptNode;

#[async_trait]
impl NodeExecutor for ScriptNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext) -> Result<serde_json::Value> {
        let script = step.config.get("script").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("脚本节点缺少 script 参数"))?;

        ctx.eval_expr(script).map_err(|e| anyhow!(e))
    }
}
