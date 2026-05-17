// nodes/loop_node.rs — 循环节点（for-each 全遍历）
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use crate::engine::collect;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};

#[derive(Default)]
pub struct LoopNode;

/// 解析循环 items：支持直接数组、JSON 编码字符串、字符串引用（`output.xxx` 或变量名）
fn resolve_items(items_value: &Value, ctx: &ExecutionContext) -> Result<Vec<Value>> {
    if let Some(arr) = items_value.as_array() {
        return Ok(arr.clone());
    }
    if let Some(s) = items_value.as_str() {
        // 尝试将 JSON 字符串解析为数组（如 "[\"alpha\",\"beta\"]"）
        if let Ok(parsed) = serde_json::from_str::<Value>(s) {
            if let Some(arr) = parsed.as_array() {
                return Ok(arr.clone());
            }
        }
        if let Some(key) = s.strip_prefix("output.") {
            return ctx.get_output(key)
                .and_then(|v| v.as_array())
                .cloned()
                .ok_or_else(|| anyhow!("循环 items 引用 '{}' 无法解析为数组", s));
        }
        return ctx.get_output(s)
            .and_then(|v| v.as_array())
            .cloned()
            .or_else(|| ctx.variables.get(s).and_then(|v| v.as_array()).cloned())
            .ok_or_else(|| anyhow!("循环 items '{}' 不是数组", s));
    }
    Err(anyhow!("循环 items 必须是数组或引用"))
}

/// 解析 YAML body 为 Vec<Step>：优先 step.body_steps（编辑器 UI），回退 config.body（手写 JSON）
fn parse_body_steps(step: &Step) -> Result<Vec<Step>> {
    if let Some(ref body) = step.body_steps {
        if !body.is_empty() {
            return Ok(body.clone());
        }
    }
    let steps: Vec<Step> = serde_json::from_value(
        step.config.get("body").cloned().unwrap_or(json!([]))
    ).map_err(|e| anyhow!("循环 body 解析失败: {}", e))?;
    if steps.is_empty() {
        return Err(anyhow!("循环 body 不能为空"));
    }
    Ok(steps)
}

#[async_trait]
impl NodeExecutor for LoopNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let items_value = step.config.get("items")
            .ok_or_else(|| anyhow!("循环节点缺少 items 参数"))?;
        let items = resolve_items(items_value, ctx)?;
        let body_steps = parse_body_steps(step)?;

        // 逐轮执行
        let mut results = Vec::new();
        for (i, item) in items.iter().enumerate() {
            ctx.set_var("__item".to_string(), item.clone());
            ctx.set_var("__index".to_string(), json!(i));
            ctx.set_var("__index1".to_string(), json!(i + 1));
            // 友好别名：{{loop.current}} / {{loop.index}}
            ctx.set_var("loop".to_string(), json!({
                "current": item,
                "index": i,
                "index1": i + 1,
            }));

            let mut item_outputs = serde_json::Map::new();
            for body_step in &body_steps {
                let mut resolved = body_step.clone();
                resolved.config = ctx.resolve_config(&body_step.config);
                let output = executor.execute(&resolved, ctx).await?;
                item_outputs.insert(body_step.id.clone(), output.clone());
                ctx.set_output(&body_step.id, output);
            }
            results.push(Value::Object(item_outputs));
        }

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
