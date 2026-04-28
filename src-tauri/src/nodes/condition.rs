// nodes/condition.rs — 条件分支节点（声明式 + 表达式双模式）
//
// 声明式格式（推荐）：
//   type: condition
//   config:
//     left: "{{step_xxx.count}}"    # 左操作数（支持 {{变量}}）
//     op: ">"                        # 操作符
//     right: "0"                     # 右操作数（支持 {{变量}}）
//
// 支持的操作符：
//   ==  !=  >  <  >=  <=            比较
//   contains                        字符串包含 / 数组包含
//   starts_with  ends_with          字符串匹配
//   empty  not_empty                空值检查（不需要 right）
//   in  not_in                      数组成员（right 为数组）
//
// 表达式格式（向后兼容）：
//   type: condition
//   config:
//     condition: "step_xxx.count > 0"

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};

#[derive(Default)]
pub struct ConditionNode;

#[async_trait]
impl NodeExecutor for ConditionNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;

        // ── 声明式格式 ──
        if let Some(op) = config.get("op").and_then(|v| v.as_str()) {
            let left = ctx.resolve_config(
                config.get("left").unwrap_or(&serde_json::Value::Null)
            );
            let right = ctx.resolve_config(
                config.get("right").unwrap_or(&serde_json::Value::Null)
            );

            let result = eval_condition(&left, op, &right);
            return Ok(serde_json::json!({
                "result": result,
                "branch": if result { "true" } else { "false" },
            }));
        }

        // ── 表达式格式（向后兼容）──
        let condition = config.get("condition").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("条件节点需要 left/op（声明式）或 condition（表达式）参数"))?;

        let result = ctx.eval_expr(condition).map_err(|e| anyhow!(e))?;
        let is_true = result.as_bool().unwrap_or(false);

        Ok(serde_json::json!({
            "result": is_true,
            "branch": if is_true { "true" } else { "false" },
        }))
    }
}

// ─── 条件求值 ───

fn eval_condition(left: &serde_json::Value, op: &str, right: &serde_json::Value) -> bool {
    match op {
        "==" => left == right,
        "!=" => left != right,
        ">"  => compare_values(left, right) > 0,
        "<"  => compare_values(left, right) < 0,
        ">=" => compare_values(left, right) >= 0,
        "<=" => compare_values(left, right) <= 0,
        "contains" => match (left, right) {
            (serde_json::Value::String(l), serde_json::Value::String(r)) => l.contains(r.as_str()),
            (serde_json::Value::Array(arr), _) => arr.iter().any(|v| v == right),
            _ => false,
        },
        "starts_with" => match (left, right) {
            (serde_json::Value::String(l), serde_json::Value::String(r)) => l.starts_with(r.as_str()),
            _ => false,
        },
        "ends_with" => match (left, right) {
            (serde_json::Value::String(l), serde_json::Value::String(r)) => l.ends_with(r.as_str()),
            _ => false,
        },
        "empty" => is_empty(left),
        "not_empty" => !is_empty(left),
        "in" => match right {
            serde_json::Value::Array(arr) => arr.iter().any(|v| v == left),
            _ => false,
        },
        "not_in" => match right {
            serde_json::Value::Array(arr) => !arr.iter().any(|v| v == left),
            _ => true,
        },
        _ => false,
    }
}

/// 比较两个 JSON 值：先尝试数字比较，再退化为字符串比较
fn compare_values(left: &serde_json::Value, right: &serde_json::Value) -> i32 {
    // 数字比较
    if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
        return l.partial_cmp(&r).map(|o| o as i32).unwrap_or(0);
    }
    // 布尔比较
    if let (Some(l), Some(r)) = (left.as_bool(), right.as_bool()) {
        return (l as i32).cmp(&(r as i32)) as i32;
    }
    // 字符串比较
    let l_str = value_to_comp_string(left);
    let r_str = value_to_comp_string(right);
    l_str.cmp(&r_str) as i32
}

fn is_empty(val: &serde_json::Value) -> bool {
    match val {
        serde_json::Value::Null => true,
        serde_json::Value::String(s) => s.is_empty(),
        serde_json::Value::Array(arr) => arr.is_empty(),
        serde_json::Value::Object(obj) => obj.is_empty(),
        _ => false,
    }
}

fn value_to_comp_string(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}
