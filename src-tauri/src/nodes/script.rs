// nodes/script.rs — rhai 脚本节点
//
// 支持：
//   - script: rhai 脚本内容（支持 {{变量}} 引用）
//   - timeout_secs: 超时秒数（默认 30，最大 300）
//
// 输出：脚本的返回值（自动转换为 JSON）
//
// 安全限制（在 context.rs 中配置）：
//   - 最大操作数：100,000
//   - 最大字符串大小：1MB
//   - 最大数组/Map 大小：10,000

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[derive(Default)]
pub struct ScriptNode;

#[async_trait]
impl NodeExecutor for ScriptNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "script".into(),
            version: "1.0".into(),
            display_name: "脚本".into(),
            description: "执行 Rhai 脚本，支持变量读写和逻辑处理".into(),
            category: "逻辑".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({"type": "object", "properties": {"script": {"type": "string"}}}),
        }
    }

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

        let timeout_secs = step
            .config
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(30)
            .min(300); // 最大 5 分钟

        // 克隆 ctx 以便在 spawn_blocking 中使用
        let ctx_clone = ctx.clone();
        let script_owned = script.to_string();

        // 使用 tokio::time::timeout 包装执行，防止脚本卡死
        let result = timeout(
            Duration::from_secs(timeout_secs),
            tokio::task::spawn_blocking(move || {
                ctx_clone.eval_expr(&script_owned).map_err(|e| anyhow!(e))
            }),
        )
        .await;

        match result {
            Ok(Ok(Ok(value))) => Ok(value),
            Ok(Ok(Err(e))) => Err(e),
            Ok(Err(e)) => Err(anyhow!("脚本执行失败: {}", e)),
            Err(_) => Err(anyhow!(
                "脚本执行超时（{}秒）",
                timeout_secs
            )),
        }
    }
}
