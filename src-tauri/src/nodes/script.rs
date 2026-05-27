// nodes/script.rs — rhai 脚本节点
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Default)]
pub struct ScriptNode;

#[async_trait]
impl NodeExecutor for ScriptNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let script = step
            .config
            .get("script")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("脚本节点缺少 script 参数"))?;

        ctx.eval_expr(script).map_err(|e| anyhow!(e))
    }
}
