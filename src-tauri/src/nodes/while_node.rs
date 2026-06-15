// nodes/while_node.rs — while 条件循环节点
// 遍历数组，每轮检查条件，条件不满足时停止
// 典型场景：读取 Excel A 列，有数据就继续，无数据就停
use crate::engine::collect;
use crate::engine::context::ExecutionContext;
use tracing::warn;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Default)]
pub struct WhileNode;

/// 解析循环 items（与 loop_node 共用逻辑）
fn resolve_items(items_value: &Value, ctx: &ExecutionContext) -> Result<Vec<Value>> {
    if let Some(arr) = items_value.as_array() {
        return Ok(arr.clone());
    }
    if let Some(s) = items_value.as_str() {
        if let Some(key) = s.strip_prefix("output.") {
            return ctx
                .get_output(key)
                .and_then(|v| v.as_array())
                .cloned()
                .ok_or_else(|| anyhow!("while items 引用 '{}' 无法解析为数组", s));
        }
        return ctx
            .get_output(s)
            .and_then(|v| v.as_array())
            .cloned()
            .or_else(|| ctx.variables.get(s).and_then(|v| v.as_array()).cloned())
            .ok_or_else(|| anyhow!("while items '{}' 不是数组", s));
    }
    Err(anyhow!("while items 必须是数组或引用"))
}

/// 解析 body 步骤：优先 step.body_steps（编辑器 UI），回退 config.body（手写 JSON）
fn parse_body_steps(step: &Step) -> Result<Vec<Step>> {
    if let Some(ref body) = step.body_steps {
        if !body.is_empty() {
            return Ok(body.clone());
        }
    }
    let steps: Vec<Step> =
        serde_json::from_value(step.config.get("body").cloned().unwrap_or(json!([])))
            .map_err(|e| anyhow!("while body 解析失败: {}", e))?;
    if steps.is_empty() {
        return Err(anyhow!("while body 不能为空"));
    }
    Ok(steps)
}

/// 评估 while 条件
fn check_condition(check_val: &Value, cond_op: &str, cond_right: Option<&Value>) -> bool {
    match cond_op {
        "not_empty" => {
            !check_val.is_null()
                && check_val.as_str().map(|s| !s.is_empty()).unwrap_or(true)
                && check_val.as_array().map(|a| !a.is_empty()).unwrap_or(true)
        }
        "empty" => {
            check_val.is_null()
                || check_val.as_str().map(|s| s.is_empty()).unwrap_or(false)
                || check_val.as_array().map(|a| a.is_empty()).unwrap_or(false)
        }
        "eq" => *check_val == *cond_right.unwrap_or(&Value::Null),
        "ne" => *check_val != *cond_right.unwrap_or(&Value::Null),
        "gt" => match (check_val.as_f64(), cond_right.and_then(|r| r.as_f64())) {
            (Some(a), Some(b)) => a > b,
            _ => false,
        },
        "gte" => match (check_val.as_f64(), cond_right.and_then(|r| r.as_f64())) {
            (Some(a), Some(b)) => a >= b,
            _ => false,
        },
        "lt" => match (check_val.as_f64(), cond_right.and_then(|r| r.as_f64())) {
            (Some(a), Some(b)) => a < b,
            _ => false,
        },
        "lte" => match (check_val.as_f64(), cond_right.and_then(|r| r.as_f64())) {
            (Some(a), Some(b)) => a <= b,
            _ => false,
        },
        "in" => cond_right
            .and_then(|r| r.as_array())
            .map(|arr| arr.contains(check_val))
            .unwrap_or(false),
        "contains" => check_val
            .as_str()
            .zip(cond_right.and_then(|r| r.as_str()))
            .map(|(haystack, needle)| haystack.contains(needle))
            .unwrap_or(false),
        "not_contains" => check_val
            .as_str()
            .zip(cond_right.and_then(|r| r.as_str()))
            .map(|(haystack, needle)| !haystack.contains(needle))
            .unwrap_or(false),
        _ => {
            warn!("while 节点: 未知条件操作符 '{}', 默认停止循环（安全回退）", cond_op);
            false
        },
    }
}

#[async_trait]
impl NodeExecutor for WhileNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let items_value = step
            .config
            .get("items")
            .ok_or_else(|| anyhow!("while 节点缺少 items 参数"))?;
        let items = resolve_items(items_value, ctx)?;
        let body_steps = parse_body_steps(step)?;

        // 条件配置
        let cond = step.config.get("condition").and_then(|v| v.as_object());
        let cond_op = cond
            .and_then(|c| c.get("op"))
            .and_then(|v| v.as_str())
            .unwrap_or("not_empty");
        let cond_right = cond.and_then(|c| c.get("right"));
        let cond_check = cond
            .and_then(|c| c.get("check"))
            .and_then(|v| v.as_str())
            .unwrap_or("__current");

        // 安全上限
        let max_iter = step
            .config
            .get("max_iterations")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000) as usize;
        let total = items.len().min(max_iter);

        // 逐轮执行：先检查条件，再执行 body
        let mut results = Vec::new();
        for (i, current) in items.iter().enumerate().take(total) {
            // 构建条件检查上下文
            let mut cond_ctx = serde_json::Map::new();
            cond_ctx.insert("__current".to_string(), current.clone());
            cond_ctx.insert("__item".to_string(), current.clone());
            cond_ctx.insert("__index".to_string(), json!(i));
            cond_ctx.insert("__index1".to_string(), json!(i + 1));
            let cond_ctx_val = Value::Object(cond_ctx);
            let check_val = collect::resolve_path(cond_check, &cond_ctx_val);

            if !check_condition(&check_val, cond_op, cond_right) {
                break;
            }

            // 执行 body
            ctx.set_var("__item".to_string(), current.clone());
            ctx.set_var("__current".to_string(), current.clone());
            ctx.set_var("__index".to_string(), json!(i));
            ctx.set_var("__index1".to_string(), json!(i + 1));

            let mut item_outputs = serde_json::Map::new();
            // 迭代变量已设入 ctx，容器/节点各自解析模板
            for body_step in &body_steps {
                let output = executor.execute(body_step, ctx).await?;
                item_outputs.insert(body_step.id.clone(), output.clone());
                ctx.set_output(&body_step.id, output);
            }
            results.push(Value::Object(item_outputs));
        }

        let processed_items: Vec<Value> = items[..results.len()].to_vec();
        let mut output = json!({
            "count": results.len(),
            "stopped_at": results.len(),
            "results": results,
        });

        // collect 后处理
        if let Some(collect_cfg) = step.config.get("collect") {
            collect::apply_collect(&mut output, collect_cfg, &processed_items, &results);
        }

        // table 后处理
        if let Some(table_cfg) = step.config.get("table") {
            collect::apply_table(&mut output, table_cfg, &processed_items, &results);
        }

        Ok(output)
    }
}
