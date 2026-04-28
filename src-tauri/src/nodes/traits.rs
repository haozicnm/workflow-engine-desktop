// nodes/traits.rs — 节点执行器 trait
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use std::sync::Arc;
use crate::engine::executor::StepExecutor;
use anyhow::Result;

#[async_trait]
pub trait NodeExecutor: Send + Sync {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value>;
}
