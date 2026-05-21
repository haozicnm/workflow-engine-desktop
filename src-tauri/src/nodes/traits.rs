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

    /// 是否由节点自行解析 config 中的模板变量。
    /// 返回 true 时，executor 跳过全局 `ctx.resolve_config(&step.config)`，
    /// 由节点在迭代期间自行处理（如 map 节点的 `{{__item}}` 模板）。
    fn resolve_config_self(&self) -> bool {
        false
    }
}
