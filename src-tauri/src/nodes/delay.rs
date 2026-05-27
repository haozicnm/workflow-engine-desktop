// nodes/delay.rs — 延时节点：暂停执行指定时长
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tracing::info;

#[derive(Default)]
pub struct DelayNode;

#[async_trait]
impl NodeExecutor for DelayNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let duration_ms = step
            .config
            .get("duration_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000);

        let max_duration = step
            .config
            .get("max_duration_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(300_000); // 默认最大 5 分钟

        if duration_ms > max_duration {
            return Err(anyhow!(
                "延时 {}ms 超过最大限制 {}ms",
                duration_ms,
                max_duration
            ));
        }

        info!("延时 {}ms ({}秒)", duration_ms, duration_ms as f64 / 1000.0);
        tokio::time::sleep(std::time::Duration::from_millis(duration_ms)).await;

        Ok(json!({
            "duration_ms": duration_ms,
            "duration_sec": duration_ms as f64 / 1000.0,
        }))
    }
}
