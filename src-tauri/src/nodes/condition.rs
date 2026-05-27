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

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::info;
use std::sync::Arc;

#[derive(Default)]
pub struct ConditionNode;

#[async_trait]
impl NodeExecutor for ConditionNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;

        // 上游注入的原始值（将透传到输出）
        let original_value = config
            .get("value")
            .cloned()
            .unwrap_or(serde_json::Value::Null);

        // ── 新格式：conditionGroup（可视化条件构建器） ──
        // 优先从 step.condition_group 读，回退到 config.condition_group
        let condition_group_owned: Option<crate::engine::workflow::LogicConditionGroup> =
            step.condition_group.clone().or_else(|| {
                config
                    .get("condition_group")
                    .and_then(|cg| serde_json::from_value(cg.clone()).ok())
            });
        if let Some(ref group) = condition_group_owned {
            if !group.conditions.is_empty() {
                let results: Vec<bool> = group
                    .conditions
                    .iter()
                    .map(|cond| {
                        let left = ctx.resolve_config(&serde_json::json!(cond.left));
                        let right = ctx.resolve_config(&serde_json::json!(cond.right));
                        let result = eval_condition(&left, &cond.op, &right);
                        info!(
                        "[逻辑判断] left_template={} left={} op={} right_template={} right={} → {}",
                        cond.left, left, cond.op, cond.right, right, if result { "✓" } else { "✗" }
                    );
                        result
                    })
                    .collect();

                let pass = if group.combinator == "or" {
                    results.iter().any(|&r| r)
                } else {
                    results.iter().all(|&r| r)
                };

                info!(
                    "[逻辑判断] conditionGroup combinator={} conditions={} → {}",
                    group.combinator,
                    results.len(),
                    if pass { "✓ true" } else { "✗ false" }
                );

                let branch_str = if pass { "true" } else { "false" };
                let output_value =
                    resolve_output_template(&step.config, &original_value, branch_str, pass, ctx);
                return Ok(serde_json::json!({
                    "branch": branch_str,
                    "value": output_value,
                    "result": pass,
                }));
            }
        }

        // ── actions 数组（从 step.actions 读取） ──
        let step_actions = step.actions.as_deref().unwrap_or(&[]);
        if !step_actions.is_empty() {
            let resolved_left = ctx.resolve_config(&original_value);
            let mut all_pass = true;
            for (i, action) in step_actions.iter().enumerate() {
                let op = action
                    .get("type")
                    .or_else(|| action.get("op"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let action_config = action.get("config");
                let right = action_config
                    .and_then(|c| c.get("right"))
                    .unwrap_or(&serde_json::Value::Null);

                let pass = eval_condition(&resolved_left, op, right);
                info!(
                    "[逻辑判断] action[{}] op={} left={} right={} → {}",
                    i,
                    op,
                    resolved_left,
                    right,
                    if pass { "✓" } else { "✗" }
                );

                if !pass {
                    all_pass = false;
                    break;
                }
            }

            let branch_str = if all_pass { "true" } else { "false" };
            if !all_pass {
                info!("[逻辑判断] 条件不通过，走 false 出口");
            }
            let output_value =
                resolve_output_template(&step.config, &original_value, branch_str, all_pass, ctx);
            return Ok(serde_json::json!({
                "branch": branch_str,
                "value": output_value,
                "result": all_pass,
            }));
        }

        // ── 旧格式兼容：left/op/right（声明式） ──
        if let Some(op) = config.get("op").and_then(|v| v.as_str()) {
            let left = ctx.resolve_config(config.get("left").unwrap_or(&serde_json::Value::Null));
            let right = ctx.resolve_config(config.get("right").unwrap_or(&serde_json::Value::Null));
            let result = eval_condition(&left, op, &right);
            if result {
                return Ok(left);
            } else {
                return Ok(serde_json::Value::Null);
            }
        }

        // ── 旧格式兼容：表达式 ──
        let condition = config
            .get("condition")
            .and_then(|v| v.as_str())
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

/// 解析 output_template：如果有 output_template 配置则渲染模板，否则返回原始值
fn resolve_output_template(
    config: &serde_json::Value,
    original_value: &serde_json::Value,
    branch: &str,
    result: bool,
    ctx: &ExecutionContext,
) -> serde_json::Value {
    if let Some(template) = config.get("output_template").and_then(|v| v.as_str()) {
        if !template.is_empty() {
            // 构建模板变量上下文
            let mut template_ctx = serde_json::json!({
                "left": original_value,
                "branch": branch,
                "result": result,
            });
            // 注入 right 值（如果存在）
            if let Some(right) = config.get("right") {
                if let Some(obj) = template_ctx.as_object_mut() {
                    obj.insert("right".to_string(), right.clone());
                }
            }
            // 尝试用 ctx 的变量解析模板
            let resolved = ctx.resolve_config(&serde_json::json!(template));
            return resolved;
        }
    }
    original_value.clone()
}

// ─── 条件求值（pub(crate) 让 approval 等节点可复用） ───

pub(crate) fn eval_condition(
    left: &serde_json::Value,
    op: &str,
    right: &serde_json::Value,
) -> bool {
    match op {
        "==" | "equals" => left == right,
        "!=" | "not_equals" => left != right,
        ">" | "gt" | "greater_than" => compare_values(left, right) > 0,
        "<" | "lt" | "less_than" => compare_values(left, right) < 0,
        ">=" | "gte" | "greater_equal" => compare_values(left, right) >= 0,
        "<=" | "lte" | "less_equal" => compare_values(left, right) <= 0,
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
        "starts_with" | "start_with" => match (left, right) {
            (serde_json::Value::String(l), serde_json::Value::String(r)) => {
                l.starts_with(r.as_str())
            }
            _ => false,
        },
        "ends_with" | "end_with" => match (left, right) {
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
            (serde_json::Value::String(l), serde_json::Value::String(r)) => regex::Regex::new(r)
                .map(|re| re.is_match(l))
                .unwrap_or(false),
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
