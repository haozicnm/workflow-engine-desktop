// nodes/sub_workflow.rs — 子流程节点（增强版）
// 加载并执行另一个工作流，实现模块化复用
// P4 增强：支持 DAG 上下文、输出映射、子流内部节点计数
use async_trait::async_trait;
use crate::engine::workflow::{Step, Workflow};
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};

#[derive(Default)]
pub struct SubWorkflowNode;

#[async_trait]
impl NodeExecutor for SubWorkflowNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;

        // 支持三种方式加载子流程：
        // 1. workflow_yaml — 直接嵌入的 YAML 字符串
        // 2. inline_steps — 内联步骤（JSON 数组，兼容 DAG FlowNode 格式）
        // 3. workflow_id — 从数据库加载（预留）

        let steps: Vec<Step> = if let Some(wf_yaml) = config.get("workflow_yaml") {
            let yaml_str = wf_yaml
                .as_str()
                .or_else(|| {
                    wf_yaml
                        .as_object()
                        .map(|_| "")
                        .and_then(|_| serde_json::to_string(wf_yaml).ok().and_then(|_| wf_yaml.as_str()))
                })
                .ok_or_else(|| anyhow!("workflow_yaml 必须是字符串"))?;

            if yaml_str.trim().is_empty() {
                return Err(anyhow!("workflow_yaml 内容为空"));
            }

            let sub_wf = crate::engine::parser::parse_workflow(yaml_str)
                .map_err(|e| anyhow!("子流程 YAML 解析失败: {}", e))?;
            sub_wf.steps
        } else if let Some(inline) = config.get("inline_steps") {
            // 内联步骤：可能是 Vec<Step> 或 DAG 格式的 Vec<FlowNode>
            parse_inline_steps(inline)?
        } else if let Some(wf_id) = config.get("workflow_id") {
            let id_str = wf_id.as_str().unwrap_or("");
            return Err(anyhow!(
                "通过 workflow_id={} 加载子流程需要数据库访问（预留功能）",
                id_str
            ));
        } else {
            return Err(anyhow!(
                "子流程需要 workflow_yaml、inline_steps 或 workflow_id"
            ));
        };

        if steps.is_empty() {
            return Err(anyhow!("子流程没有步骤"));
        }

        let internal_count = steps.len();

        // ── 构建子工作流临时对象 ──
        let sub_wf = Workflow {
            name: step
                .config
                .get("_sub_name")
                .and_then(|v| v.as_str())
                .unwrap_or(&step.name)
                .to_string(),
            description: None,
            steps: steps.clone(),
            variables: None,
        };

        // ── 变量隔离：创建子上下文，继承父上下文变量 ──
        let mut sub_ctx = ExecutionContext::new(&ctx.run_id, &sub_wf);
        // 继承父上下文的所有变量
        for (k, v) in &ctx.variables {
            sub_ctx.set_var(k.clone(), v.clone());
        }
        // 继承父上下文的步骤输出
        for (k, v) in &ctx.step_outputs {
            sub_ctx.set_output(k, v.clone());
        }

        // vars_mapping: 将父上下文的变量按映射规则传递到子流程
        if let Some(mapping) = config.get("vars_mapping") {
            if let Some(map) = mapping.as_object() {
                for (sub_key, src_path) in map {
                    if let Some(src) = src_path.as_str() {
                        if let Some(val) = ctx.resolve_var(src) {
                            sub_ctx.set_var(sub_key.clone(), val.clone());
                        }
                    } else if let Some(val) = src_path.as_str() {
                        // 直接字符串值
                        if !val.starts_with("{{") {
                            sub_ctx.set_var(
                                sub_key.clone(),
                                serde_json::Value::String(val.to_string()),
                            );
                        } else if let Some(resolved) = ctx.resolve_var(val) {
                            sub_ctx.set_var(sub_key.clone(), resolved.clone());
                        }
                    }
                }
            }
        }

        // ── 逐步骤执行子流程 ──
        let mut outputs = serde_json::Map::new();
        let mut last_output = serde_json::Value::Null;

        for sub_step in &steps {
            let mut resolved = sub_step.clone();
            resolved.config = sub_ctx.resolve_config(&sub_step.config);

            match executor.execute(&resolved, &mut sub_ctx).await {
                Ok(output) => {
                    outputs.insert(sub_step.id.clone(), output.clone());
                    sub_ctx.set_output(&sub_step.id, output.clone());
                    last_output = output;
                }
                Err(e) => {
                    // 检查是否配置了 on_error 策略
                    if let Some(crate::engine::workflow::ErrorStrategy::Ignore) =
                        sub_step.on_error
                    {
                        tracing::warn!(
                            "子流程步骤 '{}' 失败但被忽略: {}",
                            sub_step.id,
                            e
                        );
                        let err_val = serde_json::json!({
                            "error": e.to_string(),
                            "ignored": true
                        });
                        outputs.insert(sub_step.id.clone(), err_val);
                        last_output = serde_json::Value::Null;
                    } else {
                        return Err(anyhow!(
                            "子流程步骤 '{}' ({}) 执行失败: {}",
                            sub_step.id,
                            sub_step.name,
                            e
                        ));
                    }
                }
            }
        }

        // ── 输出映射：将子流程结果写回父上下文 ──
        let output_key = config
            .get("output_key")
            .and_then(|v| v.as_str())
            .unwrap_or("result");

        // output_mapping: 将子流程特定输出映射到父上下文变量
        if let Some(out_map) = config.get("output_mapping") {
            if let Some(map) = out_map.as_object() {
                for (parent_var, sub_output_key) in map {
                    if let Some(key_str) = sub_output_key.as_str() {
                        if let Some(val) = outputs.get(key_str) {
                            ctx.set_var(parent_var.clone(), val.clone());
                        } else if let Some(val) = sub_ctx.resolve_var(key_str) {
                            ctx.set_var(parent_var.clone(), val.clone());
                        }
                    }
                }
            }
        }

        Ok(serde_json::json!({
            "steps_executed": internal_count,
            "outputs": outputs,
            "sub_workflow_name": sub_wf.name,
            output_key: last_output,
        }))
    }
}

/// 解析内联步骤：支持两种格式
/// 1. Vec<Step> — 标准步骤数组
/// 2. Vec<FlowNode> — DAG 前端传入的节点格式（含 position、label 等额外字段）
fn parse_inline_steps(value: &serde_json::Value) -> Result<Vec<Step>> {
    let arr = value
        .as_array()
        .ok_or_else(|| anyhow!("inline_steps 必须是数组"))?;

    let mut steps = Vec::with_capacity(arr.len());

    for item in arr {
        // 尝试直接反序列化为 Step
        if let Ok(step) = serde_json::from_value::<Step>(item.clone()) {
            steps.push(step);
            continue;
        }

        // 兼容 DAG FlowNode 格式：
        // FlowNode { id, type, label, position, config } → Step { id, name, step_type, config }
        if item.is_object() {
            let obj = item.as_object()
                .expect("item 应在 is_object 检查后为 Object");
            let id = obj
                .get("id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let name = obj
                .get("label")
                .or_else(|| obj.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or(&id)
                .to_string();
            let step_type = obj
                .get("type")
                .or_else(|| obj.get("step_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();
            let config = obj
                .get("config")
                .cloned()
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

            steps.push(Step {
                id,
                name,
                step_type,
                config,
                next: None,
                retry: None,
                timeout: None,
                body_steps: None,
                breakpoint: false,
                delay: None,
                on_error: None,
            });
            continue;
        }

        return Err(anyhow!("无法解析 inline_steps 中的条目: {:?}", item));
    }

    Ok(steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_inline_steps_flow_node_format() {
        let json = serde_json::json!([
            {
                "id": "n1",
                "type": "http",
                "label": "调用API",
                "position": { "x": 0, "y": 0 },
                "config": { "url": "https://example.com" }
            }
        ]);

        let steps = parse_inline_steps(&json).unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].id, "n1");
        assert_eq!(steps[0].name, "调用API");
        assert_eq!(steps[0].step_type, "http");
    }

    #[test]
    fn test_parse_inline_steps_standard_format() {
        let json = serde_json::json!([
            {
                "id": "s1",
                "name": "测试",
                "type": "http",
                "config": { "url": "https://example.com" }
            }
        ]);

        let steps = parse_inline_steps(&json).unwrap();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].id, "s1");
        assert_eq!(steps[0].name, "测试");
    }
}
