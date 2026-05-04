// nodes/condition.rs — 逻辑判断容器（v4.0: 容器+动作模式）
//
// config 格式：
//   config.value: 上游注入的原始值（作为 left 操作数）
//   config.actions: [
//     { "id": "a1", "type": "contains", "label": "包含异常", "config": { "right": "异常" } },
//     { "id": "a2", "type": "gt", "label": "大于100", "config": { "right": "100" } },
//   ]
//
// 所有 action 全部通过 → branch: "true"，否则 → branch: "false"
// 输出同时包含原始 value 以便下游使用

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use log::info;

#[derive(Default)]
pub struct ConditionNode;

#[async_trait]
impl NodeExecutor for ConditionNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;

        // 上游注入的原始值（将透传到输出）
        let original_value = config.get("value").cloned().unwrap_or(serde_json::Value::Null);

        // ── 新格式：actions 数组 ──
        if let Some(actions) = config.get("actions").and_then(|v| v.as_array()) {
            if actions.is_empty() {
                // 没有判断动作 → 默认走 true
                return Ok(serde_json::json!({
                    "branch": "true",
                    "value": original_value,
                }));
            }

            let mut all_pass = true;
            for (i, action) in actions.iter().enumerate() {
                let op = action.get("type")
                    .or_else(|| action.get("op"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let action_config = action.get("config");
                let right = action_config
                    .and_then(|c| c.get("right"))
                    .unwrap_or(&serde_json::Value::Null);

                let pass = eval_condition(&original_value, op, right);
                info!(
                    "[逻辑判断] action[{}] op={} left={} right={} → {}",
                    i, op, original_value, right, if pass { "✓" } else { "✗" }
                );

                if !pass {
                    all_pass = false;
                    break;
                }
            }

            if all_pass {
                return Ok(serde_json::json!({
                    "branch": "true",
                    "value": original_value,
                }));
            } else {
                info!("[逻辑判断] 条件不通过，走 false 出口");
                return Ok(serde_json::json!({
                    "branch": "false",
                    "value": original_value,
                }));
            }
        }

        // ── 旧格式兼容：left/op/right（声明式） ──
        if let Some(op) = config.get("op").and_then(|v| v.as_str()) {
            let left = ctx.resolve_config(
                config.get("left").unwrap_or(&serde_json::Value::Null)
            );
            let right = ctx.resolve_config(
                config.get("right").unwrap_or(&serde_json::Value::Null)
            );
            let result = eval_condition(&left, op, &right);
            if result {
                return Ok(left);
            } else {
                return Ok(serde_json::Value::Null);
            }
        }

        // ── 旧格式兼容：表达式 ──
        let condition = config.get("condition").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("逻辑判断节点需要 actions 数组、left/op 或 condition"))?;
        let result = ctx.eval_expr(condition).map_err(|e| anyhow!(e))?;
        let is_true = result.as_bool().unwrap_or(false);
        if is_true {
            Ok(original_value)
        } else {
            Ok(serde_json::Value::Null)
        }
    }
}

// ─── 条件求值 ───

fn eval_condition(left: &serde_json::Value, op: &str, right: &serde_json::Value) -> bool {
    match op {
        "==" | "equals" => left == right,
        "!=" | "not_equals" => left != right,
        ">" | "gt" => compare_values(left, right) > 0,
        "<" | "lt" => compare_values(left, right) < 0,
        ">=" | "gte" => compare_values(left, right) >= 0,
        "<=" | "lte" => compare_values(left, right) <= 0,
        "contains" => match (left, right) {
            (serde_json::Value::String(l), serde_json::Value::String(r)) => l.contains(r.as_str()),
            (serde_json::Value::Array(arr), _) => arr.iter().any(|v| v == right),
            _ => false,
        },
        "not_contains" => !match (left, right) {
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
        "empty" | "is_empty" => is_empty(left),
        "not_empty" => !is_empty(left),
        "in" => match right {
            serde_json::Value::Array(arr) => arr.iter().any(|v| v == left),
            _ => false,
        },
        "not_in" => match right {
            serde_json::Value::Array(arr) => !arr.iter().any(|v| v == left),
            _ => true,
        },
        "regex" => match (left, right) {
            (serde_json::Value::String(l), serde_json::Value::String(r)) => {
                regex::Regex::new(r).map(|re| re.is_match(l)).unwrap_or(false)
            }
            _ => false,
        },
        _ => false,
    }
}

/// 比较两个 JSON 值：先尝试数字比较，再退化为字符串比较
fn compare_values(left: &serde_json::Value, right: &serde_json::Value) -> i32 {
    if let (Some(l), Some(r)) = (left.as_f64(), right.as_f64()) {
        return l.partial_cmp(&r).map(|o| o as i32).unwrap_or(0);
    }
    if let (Some(l), Some(r)) = (left.as_bool(), right.as_bool()) {
        return (l as i32).cmp(&(r as i32)) as i32;
    }
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
