// nodes/loop_node.rs — 循环节点（for-each 全遍历）
use crate::engine::collect;
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Default)]
pub struct LoopNode;

/// 解析 YAML body 为 Vec<Step>：优先 step.body_steps（编辑器 UI），回退 config.body（手写 JSON）
fn parse_body_steps(step: &Step) -> Result<Vec<Step>> {
    if let Some(ref body) = step.body_steps {
        if !body.is_empty() {
            return Ok(body.clone());
        }
    }
    let steps: Vec<Step> =
        serde_json::from_value(step.config.get("body").cloned().unwrap_or(json!([])))
            .map_err(|e| anyhow!("循环 body 解析失败: {}", e))?;
    if steps.is_empty() {
        return Err(anyhow!("循环 body 不能为空"));
    }
    Ok(steps)
}

#[async_trait]
impl NodeExecutor for LoopNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "loop".into(),
            version: "1.0".into(),
            display_name: "循环".into(),
            description: "遍历数组或重复执行子步骤".into(),
            category: "流程控制".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object" }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let items_value = step
            .config
            .get("items")
            .ok_or_else(|| anyhow!("循环节点缺少 items 参数"))?;
        let items = crate::engine::common::resolve_iteration_items(items_value, ctx, "循环")?;
        let body_steps = parse_body_steps(step)?;

        // 安全上限：最大迭代次数
        let max_iter = step
            .config
            .get("max_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000) as usize;
        let total = items.len().min(max_iter);

        // 逐轮执行
        let mut results = Vec::new();
        for (i, item) in items.iter().enumerate().take(total) {
            ctx.set_var("__item".to_string(), item.clone());
            ctx.set_var("__index".to_string(), json!(i));
            ctx.set_var("__index1".to_string(), json!(i + 1));
            // 友好别名：{{loop.current}} / {{loop.index}}
            ctx.set_var(
                "loop".to_string(),
                json!({
                    "current": item,
                    "index": i,
                    "index1": i + 1,
                }),
            );

            let mut item_outputs = serde_json::Map::new();
            // 迭代变量已设入 ctx，容器/节点各自解析模板
            for body_step in &body_steps {
                let output = executor.execute(body_step, ctx).await?;
                item_outputs.insert(body_step.id.clone(), output.clone());
                ctx.set_output(&body_step.id, output);
            }
            results.push(Value::Object(item_outputs));
        }

        // 清理循环变量，防止后续步骤误用
        ctx.variables.remove("__item");
        ctx.variables.remove("__index");
        ctx.variables.remove("__index1");
        ctx.variables.remove("loop");

        let mut output = json!({
            "count": results.len(),
            "results": results,
        });

        // collect 后处理
        if let Some(collect_cfg) = step.config.get("collect") {
            collect::apply_collect(&mut output, collect_cfg, &items, &results);
        }

        // table 后处理
        if let Some(table_cfg) = step.config.get("table") {
            collect::apply_table(&mut output, table_cfg, &items, &results);
        }

        Ok(output)
    }
}
