// nodes/parallel.rs — 并行节点（join_all 并发执行多分支）
// 支持 fail_fast: 任一分支失败立即取消其他分支
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::future::join_all;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Default)]
pub struct ParallelNode;

#[async_trait]
impl NodeExecutor for ParallelNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "parallel".into(),
            version: "1.0".into(),
            display_name: "并行执行".into(),
            description: "同时执行多个分支，支持 fail_fast 模式".into(),
            category: "流程控制".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "branch_count".into(), data_type: "number".into(), required: false },
                crate::nodes::traits::PortDef { label: "results".into(), data_type: "array".into(), required: true },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "required": ["branches"],
                "properties": {
                    "branches": {"type": "array", "description": "并行分支数组，每个分支是一组步骤"},
                    "fail_fast": {"type": "boolean", "description": "任一分支失败时是否取消其他分支", "default": false}
                }
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let branches: Vec<Vec<Step>> =
            serde_json::from_value(step.config.get("branches").cloned().unwrap_or(json!([])))
                .map_err(|e| anyhow!("并行 branches 解析失败: {}", e))?;

        if branches.is_empty() {
            return Err(anyhow!("并行节点 branches 不能为空"));
        }

        let fail_fast = step
            .config
            .get("fail_fast")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let failed = Arc::new(AtomicBool::new(false));

        let branch_count = branches.len();
        let futures: Vec<_> = branches
            .into_iter()
            .enumerate()
            .map(|(i, branch)| {
                let mut branch_ctx = ExecutionContext {
                    run_id: ctx.run_id.clone(),
                    variables: ctx.variables.clone(),
                    step_outputs: ctx.step_outputs.clone(),
                    input_ports: ctx.input_ports.clone(),
                    sessions: HashMap::new(),
                    default_timeouts: ctx.default_timeouts.clone(),
                    shell_allowed_commands: ctx.shell_allowed_commands.clone(),
                    sub_workflow_depth: ctx.sub_workflow_depth,
                    step_mode_flag: ctx.step_mode_flag.clone(),
                    breakpoint_flag: ctx.breakpoint_flag.clone(),
                    pause_flag: ctx.pause_flag.clone(),
                };
                let executor = Arc::clone(executor);
                let failed = Arc::clone(&failed);
                async move {
                    let mut branch_outputs = serde_json::Map::new();
                    for branch_step in &branch {
                        if fail_fast && failed.load(Ordering::Relaxed) {
                            return json!({
                                "branch_index": i,
                                "success": false,
                                "error": "已取消（其他分支失败）",
                                "outputs": Value::Object(branch_outputs),
                            });
                        }
                        match executor.execute(branch_step, &mut branch_ctx).await {
                            Ok(output) => {
                                branch_ctx.set_output(&branch_step.id, output.clone());
                                branch_outputs.insert(branch_step.id.clone(), output);
                            }
                            Err(e) => {
                                if fail_fast { failed.store(true, Ordering::Relaxed); }
                                return json!({
                                    "branch_index": i,
                                    "success": false,
                                    "error": format!("分支 {} 步骤 '{}' 失败: {}", i, branch_step.id, e),
                                    "outputs": Value::Object(branch_outputs),
                                });
                            }
                        }
                    }
                    json!({
                        "branch_index": i,
                        "success": true,
                        "outputs": Value::Object(branch_outputs),
                    })
                }
            })
            .collect();

        let all_results = join_all(futures).await;

        // 将分支输出合并到主上下文
        for result in &all_results {
            if let Some(outputs) = result.get("outputs").and_then(|v| v.as_object()) {
                for (k, v) in outputs {
                    ctx.set_output(k, v.clone());
                }
            }
        }

        Ok(json!({ "branch_count": branch_count, "results": all_results }))
    }
}
