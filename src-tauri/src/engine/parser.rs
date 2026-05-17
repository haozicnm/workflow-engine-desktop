// engine/parser.rs — 工作流解析（v5 容器/动作格式 → 执行格式）
//
// 前端发送的新格式：
//   { type: "browser", actions: [{ id, type: "navigate", params: { url } }] }
//
// 后端执行器期望的格式：
//   { type: "browser_container", config: { actions: [{ id, type, label, config }] } }
//
// 本解析器负责：
//   1. 容器类型加 _container 后缀
//   2. actions 从顶层移入 config.actions
//   3. action.params → action.config


use crate::engine::workflow::{Step, Workflow};
use anyhow::{Result, anyhow};
use serde_json::Value;

// 容器类型列表
const CONTAINER_TYPES: &[&str] = &["browser", "excel", "word", "logic", "file"];
// 迭代类型列表（body 步骤存在 actions 里，需转为 body_steps）
const ITERATION_TYPES: &[&str] = &["cursor", "loop"];

/// 解析工作流 JSON（支持新旧两种格式）
pub fn parse_workflow(json_str: &str) -> Result<Workflow> {
    // 先尝试 JSON，再尝试 YAML
    let raw: Value = serde_json::from_str(json_str)
        .or_else(|_| serde_yaml::from_str(json_str))
        .map_err(|e| anyhow!("工作流解析失败: {}", e))?;

    let mut wf: Workflow = serde_json::from_value(raw.clone())
        .map_err(|e| anyhow!("工作流结构解析失败: {}", e))?;

    // 基本校验
    if wf.name.trim().is_empty() {
        return Err(anyhow!("工作流名称不能为空"));
    }
    if wf.steps.is_empty() {
        return Err(anyhow!("工作流至少需要一个步骤"));
    }

    // 转换每个步骤
    let mut converted_steps = Vec::new();
    for step in &wf.steps {
        converted_steps.push(convert_step(step)?);
    }
    wf.steps = converted_steps;

    // 检查步骤 ID 唯一性
    let mut ids = std::collections::HashSet::new();
    for step in &flatten_all_steps(&wf.steps) {
        if !ids.insert(&step.id) {
            return Err(anyhow!("步骤 ID 重复: {}", step.id));
        }
    }

    Ok(wf)
}

/// 转换单个步骤：容器类型加 _container，actions 移入 config
/// 迭代类型（cursor/loop）：actions 转为 body_steps
fn convert_step(step: &Step) -> Result<Step> {
    let is_container = CONTAINER_TYPES.contains(&step.step_type.as_str());
    let is_iteration = ITERATION_TYPES.contains(&step.step_type.as_str());

    let (new_type, new_config) = if is_container {
        // 容器类型：加 _container 后缀，把 actions 移入 config
        let container_type = format!("{}_container", step.step_type);

        let actions = step.actions.clone().unwrap_or_default();
        let converted_actions: Vec<Value> = actions.iter().map(convert_action).collect();

        let mut config = if step.config.is_object() {
            step.config.clone()
        } else {
            serde_json::json!({})
        };

        // 把 actions 放入 config
        if let Value::Object(ref mut map) = config {
            map.insert("actions".to_string(), Value::Array(converted_actions));
        }

        // logic 容器：把 condition 和 conditionGroup 放入 config
        if step.step_type == "logic" {
            // 如果 conditionGroup 在 config 里（前端格式），用 snake_case 复制一份
            if let Some(cg_json) = step.config.get("conditionGroup") {
                if let Value::Object(ref mut map) = config {
                    map.entry("condition_group".to_string())
                        .or_insert_with(|| cg_json.clone());
                }
            }
            if let Some(ref cond) = step.condition {
                if let Value::Object(ref mut map) = config {
                    map.insert("condition".to_string(), Value::String(cond.clone()));
                }
            }
            if let Some(ref cg) = step.condition_group {
                if let Value::Object(ref mut map) = config {
                    map.insert("condition_group".to_string(), serde_json::to_value(cg).unwrap_or_default());
                }
            }
        }

        (container_type, config)
    } else {
        // 非容器步骤：保持原样
        (step.step_type.clone(), step.config.clone())
    };

    // 迭代类型：把 actions 转为 body_steps
    let body_steps = if is_iteration {
        let actions = step.actions.clone().unwrap_or_default();
        if actions.is_empty() {
            // 没有 actions，保留原有 body_steps（可能来自手动 JSON 编辑的 config.body）
            step.body_steps.clone()
        } else {
            // 把前端的 Action 对象转为 Step 对象
            let converted: Vec<Step> = actions
                .iter()
                .map(|a| convert_action_to_step(a))
                .collect::<Result<Vec<_>>>()?;
            Some(converted)
        }
    } else {
        step.body_steps.clone()
    };


    Ok(Step {
        id: step.id.clone(),
        name: step.name.clone(),
        step_type: new_type,
        config: new_config,
        next: step.next.clone(),
        retry: step.retry.clone(),
        timeout: step.timeout,
        body_steps,
        breakpoint: step.breakpoint,
        delay: step.delay,
        on_error: step.on_error.clone(),
        actions: None, // 已移入 config 或 body_steps
        expanded: None,
        condition: step.condition.clone(),
        condition_group: step.condition_group.clone(),
        run_condition: None,
    })
}

/// 把前端 Action 对象转为后端 Step 对象
/// Action: { id, type, label, params }
/// Step:   { id, name(=label), type, config(=params) }
fn convert_action_to_step(action: &Value) -> Result<Step> {
    let map = action.as_object()
        .ok_or_else(|| anyhow!("action 不是对象"))?;

    let id = map.get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let step_type = map.get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let name = map.get("label")
        .and_then(|v| v.as_str())
        .unwrap_or(&step_type)
        .to_string();

    // params → config（与 convert_action 一致）
    let config = map.get("params")
        .cloned()
        .or_else(|| map.get("config").cloned())
        .unwrap_or(serde_json::json!({}));

    Ok(Step {
        id,
        name,
        step_type,
        config,
        ..Default::default()
    })
}

/// 转换单个 action：params → config，补 label
fn convert_action(action: &Value) -> Value {
    let mut a = action.clone();

    // params → config
    if let Value::Object(ref mut map) = a {
        if let Some(params) = map.remove("params") {
            map.insert("config".to_string(), params);
        }
        // 补 label（如果没有）
        if !map.contains_key("label") {
            let action_type = map.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            map.insert("label".to_string(), Value::String(action_type.to_string()));
        }
    }

    a
}

/// 递归收集所有步骤（包括子步骤）
fn flatten_all_steps(steps: &[Step]) -> Vec<&Step> {
    let mut result = Vec::new();
    for step in steps {
        result.push(step);
    }
    result
}

/// 根据 {{step_xxx}} / output.step_xxx 引用自动推断步骤依赖顺序
pub fn auto_order_steps(steps: &[Step]) -> Vec<usize> {
    use std::collections::HashMap;

    let n = steps.len();
    if n == 0 {
        return Vec::new();
    }

    let id_to_idx: HashMap<&str, usize> = steps.iter().enumerate()
        .map(|(i, s)| (s.id.as_str(), i)).collect();

    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, step) in steps.iter().enumerate() {
        let config_str = serde_json::to_string(&step.config).unwrap_or_default();
        for dep_id in extract_step_refs(&config_str) {
            if let Some(&j) = id_to_idx.get(dep_id.as_str()) {
                if j != i && !deps[i].contains(&j) {
                    deps[i].push(j);
                }
            }
        }
    }

    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];
    let mut in_deg = vec![0usize; n];
    for (i, dep_list) in deps.iter().enumerate() {
        in_deg[i] = dep_list.len();
        for &j in dep_list {
            adj[j].push(i);
        }
    }

    let mut queue: std::collections::VecDeque<usize> = (0..n)
        .filter(|&i| in_deg[i] == 0).collect();
    let mut order = Vec::with_capacity(n);

    while let Some(j) = queue.pop_front() {
        order.push(j);
        for &i in &adj[j] {
            in_deg[i] -= 1;
            if in_deg[i] == 0 {
                queue.push_back(i);
            }
        }
    }

    if order.len() < n {
        for i in 0..n {
            if !order.contains(&i) {
                order.push(i);
            }
        }
    }

    order
}

fn extract_step_refs(s: &str) -> Vec<String> {
    let mut refs = Vec::new();

    let prefix1 = "output.step_";
    for (pos, _) in s.match_indices(prefix1) {
        let rest = &s[pos + prefix1.len()..];
        let id: String = rest.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if !id.is_empty() {
            refs.push(id);
        }
    }

    let prefix2 = "{{step_";
    for (pos, _) in s.match_indices(prefix2) {
        let rest = &s[pos + prefix2.len()..];
        let id: String = rest.chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if !id.is_empty() {
            refs.push(id);
        }
    }

    refs.sort();
    refs.dedup();
    refs
}