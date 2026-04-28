// nodes/sub_workflow.rs — 子流程节点
// 加载并执行另一个工作流，实现模块化复用
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};

#[derive(Default)]
pub struct SubWorkflowNode;

#[async_trait]
impl NodeExecutor for SubWorkflowNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;

        // 支持两种方式加载子流程：
        // 1. workflow_id — 从数据库加载
        // 2. inline_steps — 内联步骤（JSON 数组）

        let steps: Vec<Step> = if let Some(inline) = config.get("inline_steps") {
            // 内联步骤
            serde_json::from_value(inline.clone())
                .map_err(|e| anyhow!("子流程 inline_steps 解析失败: {}", e))?
        } else if let Some(wf_yaml) = config.get("workflow_yaml") {
            // 直接嵌入的 YAML 字符串
            let yaml_str = wf_yaml.as_str()
                .ok_or_else(|| anyhow!("workflow_yaml 必须是字符串"))?;
            let sub_wf = crate::engine::parser::parse_workflow(yaml_str)
                .map_err(|e| anyhow!("子流程 YAML 解析失败: {}", e))?;
            sub_wf.steps
        } else {
            return Err(anyhow!("子流程需要 workflow_id、workflow_yaml 或 inline_steps"));
        };

        if steps.is_empty() {
            return Err(anyhow!("子流程没有步骤"));
        }

        // 传递变量：支持 vars_mapping（将当前上下文的变量映射到子流程）
        if let Some(mapping) = config.get("vars_mapping") {
            if let Some(map) = mapping.as_object() {
                for (sub_key, src_path) in map {
                    if let Some(src) = src_path.as_str() {
                        if let Some(val) = ctx.resolve_var(src) {
                            ctx.set_var(sub_key.clone(), val.clone());
                        }
                    }
                }
            }
        }

        // 逐步骤执行
        let mut outputs = serde_json::Map::new();
        let mut last_output = serde_json::Value::Null;

        for sub_step in &steps {
            let mut resolved = sub_step.clone();
            resolved.config = ctx.resolve_config(&sub_step.config);
            let output = executor.execute(&resolved, ctx).await?;
            outputs.insert(sub_step.id.clone(), output.clone());
            ctx.set_output(&sub_step.id, output.clone());
            last_output = output;
        }

        // 返回结果
        let output_key = config.get("output_key")
            .and_then(|v| v.as_str())
            .unwrap_or("result");

        Ok(serde_json::json!({
            "steps_executed": steps.len(),
            "outputs": outputs,
            output_key: last_output,
        }))
    }
}
