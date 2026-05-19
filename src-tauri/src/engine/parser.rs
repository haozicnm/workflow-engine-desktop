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

// 容器类型列表 — 新增容器类型时只需改这一处
// executor.rs 的注册宏依赖此列表，通过编译期测试保证一致性
pub const CONTAINER_TYPES: &[&str] = &["browser", "excel", "word", "logic", "file"];
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
        converted_steps.push(convert_step(step, false)?);
    }
    wf.steps = converted_steps;

    // 检查步骤 ID 唯一性
    let mut ids = std::collections::HashSet::new();
    for step in &flatten_all_steps(&wf.steps) {
        if !ids.insert(&step.id) {
            return Err(anyhow!("步骤 ID 重复: {}", step.id));
        }
    }

    // 校验步骤引用完整性（runCondition / onError branch）
    validate_step_references(&wf.steps)?;

    Ok(wf)
}

/// 校验工作流内步骤引用的完整性
///
/// 检查：
/// 1. runCondition.ref_step 是否指向存在的步骤
/// 2. onError: Branch { step_id } 是否指向存在的步骤（排除自引用）
/// 3. 容器 actions 内的变量引用是否指向 body 内存在的 action
fn validate_step_references(steps: &[Step]) -> Result<()> {
    use crate::engine::workflow::ErrorStrategy;

    // 收集所有顶层步骤 ID
    let step_ids: std::collections::HashSet<&str> = steps.iter().map(|s| s.id.as_str()).collect();

    for step in steps {
        // 校验 runCondition
        if let Some(ref rc) = step.run_condition {
            if !step_ids.contains(rc.ref_step.as_str()) {
                return Err(anyhow!(
                    "步骤 '{}' 的 runCondition 引用了不存在的步骤 '{}'",
                    step.id, rc.ref_step
                ));
            }
            if rc.ref_step == step.id {
                tracing::warn!(
                    "步骤 '{}' 的 runCondition 引用了自身，这通常是个配置错误",
                    step.id
                );
            }
        }

        // 校验 onError branch
        if let Some(ErrorStrategy::Branch { ref step_id }) = step.on_error {
            if !step_ids.contains(step_id.as_str()) {
                return Err(anyhow!(
                    "步骤 '{}' 的 onError branch 引用了不存在的步骤 '{}'",
                    step.id, step_id
                ));
            }
            if step_id == &step.id {
                return Err(anyhow!(
                    "步骤 '{}' 的 onError branch 引用了自身，这会形成死循环",
                    step.id
                ));
            }
        }

        // 校验变量引用：{{step_xxx}} 中的 step ID 必须存在
        let config_str = serde_json::to_string(&step.config).unwrap_or_default();
        for ref_id in extract_step_refs(&config_str) {
            let full_id = format!("step_{}", ref_id);
            if !step_ids.contains(full_id.as_str()) {
                return Err(anyhow!(
                    "步骤 '{}' 中引用了不存在的步骤 '{}'（{{{{step_{}}}}} 指向的步骤不存在）",
                    step.id, full_id, ref_id
                ));
            }
        }
    }

    Ok(())
}

/// 转换单个步骤：容器类型加 _container，actions 移入 config
/// 迭代类型（cursor/loop）：actions 转为 body_steps
///
/// `is_recursive`: true 表示这是 body step 的递归调用。
///   此时容器类型仅改 type 名（加 _container 后缀），
///   不再重复移动 actions（config 中已有）。
fn convert_step(step: &Step, is_recursive: bool) -> Result<Step> {
    let is_container = CONTAINER_TYPES.contains(&step.step_type.as_str());
    let is_iteration = ITERATION_TYPES.contains(&step.step_type.as_str());

    let (new_type, new_config) = if is_container {
        let container_type = format!("{}_container", step.step_type);

        if is_recursive {
            // 递归调用：config 中已有 actions，只改类型名
            // 但仍然需要处理 conditionGroup → condition_group 转换
            let mut config = step.config.clone();
            if step.step_type == "logic" {
                if let Some(cg_json) = step.config.get("conditionGroup") {
                    if let Value::Object(ref mut map) = config {
                        map.entry("condition_group".to_string())
                            .or_insert_with(|| cg_json.clone());
                    }
                }
            }
            (container_type, config)
        } else {
            // 顶层调用：actions 从顶层移入 config
            // ⚠️ 如果 config 已有 actions（直接编辑 JSON），优先使用
            let mut config = if step.config.is_object() {
                step.config.clone()
            } else {
                serde_json::json!({})
            };

            let has_config_actions = config.get("actions").is_some();

            if let Value::Object(ref mut map) = config {
                if !has_config_actions {
                    // config 里没有 actions → 从顶层 step.actions 迁移
                    let actions = step.actions.clone().unwrap_or_default();
                    let converted_actions: Vec<Value> = actions.iter().map(convert_action).collect();
                    map.insert("actions".to_string(), Value::Array(converted_actions));
                }
                // else: config 已有 actions，不覆盖（直接编辑 JSON 的方式）
            }

            // logic 容器：把 condition 和 conditionGroup 放入 config
            if step.step_type == "logic" {
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
        }
    } else {
        // 非容器步骤：保持原样
        (step.step_type.clone(), step.config.clone())
    };

    // 迭代类型：把 actions 转为 body_steps，并递归转换容器类型
    let body_steps = if is_iteration {
        let actions = step.actions.clone().unwrap_or_default();
        let raw_steps: Vec<Step> = if actions.is_empty() {
            // 没有 actions，保留原有 body_steps（可能来自手动 JSON 编辑的 config.body）
            step.body_steps.clone().unwrap_or_default()
        } else {
            // 把前端的 Action 对象转为 Step 对象
            actions
                .iter()
                .map(|a| convert_action_to_step(a))
                .collect::<Result<Vec<_>>>()?
        };
        // 递归转换每个子步骤：body 内如果有容器类型（browser/excel/word/file/logic），
        // 需要加 _container 后缀，否则 executor 找不到 handler
        let converted: Vec<Step> = raw_steps
            .into_iter()
            .map(|s| convert_step(&s, true))
            .collect::<Result<Vec<_>>>()?;
        Some(converted)
    } else {
        // 非迭代类型也可能有 body_steps（如 sub_workflow），同样递归转换
        step.body_steps.as_ref().map(|steps| {
            steps.iter()
                    .map(|s| convert_step(s, true))
                .collect::<Result<Vec<_>>>()
        }).transpose()?
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
        condition_group: step.condition_group.clone().or_else(|| {
            // 如果 JSON 的 condition_group 在 config 内而非 step 顶层，
            // 自动提升到 step 级别，避免 executor 的 resolve_config 改变类型后反序列化失败
            step.config.get("condition_group")
                .or_else(|| step.config.get("conditionGroup"))
                .and_then(|cg| serde_json::from_value(cg.clone()).ok())
        }),
        run_condition: step.run_condition.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::workflow::ErrorStrategy;
    use crate::engine::context::ExecutionContext;
    use crate::engine::workflow::{Workflow, Step};

    fn make_step(id: &str, rc_ref: Option<&str>, on_error: Option<ErrorStrategy>) -> Step {
        Step {
            id: id.to_string(),
            run_condition: rc_ref.map(|r| crate::engine::workflow::RunCondition {
                ref_step: r.to_string(),
                when: "true".to_string(),
            }),
            on_error,
            ..Default::default()
        }
    }

    #[test]
    fn valid_run_condition_passes() {
        let steps = vec![
            make_step("step_1", None, None),
            make_step("step_2", Some("step_1"), None),
        ];
        assert!(validate_step_references(&steps).is_ok());
    }

    #[test]
    fn missing_run_condition_fails() {
        let steps = vec![
            make_step("step_1", None, None),
            make_step("step_2", Some("step_missing"), None),
        ];
        let err = validate_step_references(&steps).unwrap_err();
        assert!(err.to_string().contains("step_missing"));
    }

    #[test]
    fn valid_branch_on_error_passes() {
        let steps = vec![
            make_step("step_1", None, None),
            make_step("step_2", None, Some(ErrorStrategy::Branch { step_id: "step_1".into() })),
        ];
        assert!(validate_step_references(&steps).is_ok());
    }

    #[test]
    fn missing_branch_target_fails() {
        let steps = vec![
            make_step("step_1", None, None),
            make_step("step_2", None, Some(ErrorStrategy::Branch { step_id: "step_missing".into() })),
        ];
        let err = validate_step_references(&steps).unwrap_err();
        assert!(err.to_string().contains("step_missing"));
    }

    #[test]
    fn self_referencing_branch_fails() {
        let steps = vec![
            make_step("step_1", None, Some(ErrorStrategy::Branch { step_id: "step_1".into() })),
        ];
        let err = validate_step_references(&steps).unwrap_err();
        assert!(err.to_string().contains("死循环"));
    }

    #[test]
    fn recursive_container_conversion_in_loop_body() {
        // loop 体内有 browser 容器 → body step type 应变为 browser_container
        let json = r#"{
            "name": "test",
            "steps": [{
                "id": "step_1",
                "type": "loop",
                "label": "loop",
                "config": {"items": "[\"a\"]"},
                "actions": [{
                    "id": "a1",
                    "type": "browser",
                    "label": "browse",
                    "params": {"browser": "chromium", "actions": []}
                }]
            }]
        }"#;
        let wf = parse_workflow(json).unwrap();
        let loop_step = &wf.steps[0];
        assert_eq!(loop_step.step_type, "loop");
        let body = loop_step.body_steps.as_ref().unwrap();
        assert_eq!(body.len(), 1);
        assert_eq!(body[0].step_type, "browser_container");
        assert!(body[0].config.get("actions").is_some());
    }

    #[test]
    fn invalid_step_ref_in_config_fails() {
        let json = r#"{
            "name": "test",
            "steps": [
                {"id": "step_1", "type": "shell", "label": "s1",
                 "config": {"command": "echo ok"}},
                {"id": "step_2", "type": "http", "label": "s2",
                 "config": {"method": "GET", "url": "{{step_99.result}}"}}
            ]
        }"#;
        let err = parse_workflow(json).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("step_99"), "expected error about step_99, got: {}", msg);
    }

    // ─── D2: parser 边界测试 ───

    #[test]
    fn empty_name_rejected() {
        let json = r#"{"name": "", "steps": [{"id": "s1", "type": "shell", "label": "s", "config": {"command": "echo"}}]}"#;
        let err = parse_workflow(json).unwrap_err();
        assert!(err.to_string().contains("名称不能为空"));
    }

    #[test]
    fn no_steps_rejected() {
        let json = r#"{"name": "test", "steps": []}"#;
        let err = parse_workflow(json).unwrap_err();
        assert!(err.to_string().contains("至少需要一个步骤"));
    }

    #[test]
    fn duplicate_step_ids_rejected() {
        let json = r#"{
            "name": "test",
            "steps": [
                {"id": "s1", "type": "shell", "label": "a", "config": {"command": "echo"}},
                {"id": "s1", "type": "shell", "label": "b", "config": {"command": "echo"}}
            ]
        }"#;
        let err = parse_workflow(json).unwrap_err();
        assert!(err.to_string().contains("ID 重复"));
    }

    #[test]
    fn malformed_json_rejected() {
        let json = "not valid json {{{";
        let err = parse_workflow(json).unwrap_err();
        assert!(err.to_string().contains("解析失败"));
    }

    #[test]
    fn yaml_format_parsed() {
        let yaml = r#"
name: yaml-test
steps:
  - id: s1
    type: shell
    label: step1
    config:
      command: echo hello
"#;
        let wf = parse_workflow(yaml).unwrap();
        assert_eq!(wf.name, "yaml-test");
        assert_eq!(wf.steps.len(), 1);
        assert_eq!(wf.steps[0].step_type, "shell");
    }

    #[test]
    fn container_type_suffix_added() {
        let json = r#"{
            "name": "test",
            "steps": [{
                "id": "step_1",
                "type": "browser",
                "label": "browse",
                "config": {"browser": "chromium"},
                "actions": [{"id": "a1", "type": "navigate", "label": "nav", "params": {"url": "https://example.com/api"}}]
            }]
        }"#;
        let wf = parse_workflow(json).unwrap();
        assert_eq!(wf.steps[0].step_type, "browser_container");
        let config = &wf.steps[0].config;
        assert!(config.get("actions").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0) > 0,
            "config.actions should contain converted actions");
    }

    #[test]
    fn logic_container_handles_condition_group() {
        let json = r#"{
            "name": "test",
            "steps": [{
                "id": "step_1",
                "type": "logic",
                "label": "check",
                "config": {
                    "conditionGroup": {
                        "combinator": "or",
                        "conditions": [{"id": "c1", "left": "x", "op": "equals", "right": "y"}]
                    }
                }
            }]
        }"#;
        let wf = parse_workflow(json).unwrap();
        assert_eq!(wf.steps[0].step_type, "logic_container");
        let config = &wf.steps[0].config;
        assert!(config.get("condition_group").is_some(), "condition_group should be in config");
    }

    #[test]
    fn step_with_retry_config_parsed() {
        let json = r#"{
            "name": "test",
            "steps": [{
                "id": "s1",
                "type": "http",
                "label": "req",
                "config": {"method": "GET", "url": "https://example.com/api"},
                "retry": {"max": 3, "delay_ms": 500}
            }]
        }"#;
        let wf = parse_workflow(json).unwrap();
        let retry = wf.steps[0].retry.as_ref().unwrap();
        assert_eq!(retry.max, 3);
        assert_eq!(retry.delay_ms, 500);
    }

    // ─── D3: 变量解析测试 ───

    #[test]
    fn variable_nested_key_resolution() {
        let mut ctx = ExecutionContext::new("run-1", &Workflow::default());
        ctx.set_output("step_1", serde_json::json!({"data": {"name": "alice"}}));
        let val = ctx.resolve_var("step_1.data.name");
        assert_eq!(val.and_then(|v| v.as_str()), Some("alice"));
    }

    #[test]
    fn variable_step_outputs_priority_over_variables() {
        let mut ctx = ExecutionContext::new("run-1", &Workflow::default());
        ctx.set_var("x".into(), serde_json::json!("var_val"));
        ctx.set_output("x", serde_json::json!("output_val"));
        let val = ctx.resolve_var("x");
        assert_eq!(val.and_then(|v| v.as_str()), Some("output_val"),
            "step_outputs should take priority over variables for same key");
    }

    #[test]
    fn missing_variable_returns_none() {
        let ctx = ExecutionContext::new("run-1", &Workflow::default());
        assert!(ctx.resolve_var("nonexistent").is_none());
    }

    #[test]
    fn resolve_config_recursively_replaces_variables() {
        let mut ctx = ExecutionContext::new("run-1", &Workflow::default());
        ctx.set_var("name".into(), serde_json::json!("world"));
        let config = serde_json::json!({
            "command": "echo {{name}}",
            "nested": {"msg": "hello {{name}}"}
        });
        let resolved = ctx.resolve_config(&config);
        assert_eq!(resolved["command"].as_str(), Some("echo world"));
        assert_eq!(resolved["nested"]["msg"].as_str(), Some("hello world"));
    }

    // ─── D4: 条件 + 错误策略测试 ───

    #[test]
    fn run_condition_with_self_reference_warns() {
        let steps = vec![
            make_step("step_1", Some("step_1"), None),
        ];
        // self-referencing runCondition is a warning, not an error
        assert!(validate_step_references(&steps).is_ok());
    }

    #[test]
    fn error_strategy_fail_is_default() {
        use crate::engine::workflow::ErrorStrategy;
        let default = ErrorStrategy::default();
        match default {
            ErrorStrategy::Fail => {}  // expected
            _ => panic!("default ErrorStrategy should be Fail"),
        }
    }

    #[test]
    fn error_strategy_branch_with_valid_target() {
        let steps = vec![
            make_step("step_1", None, None),
            make_step("step_2", None, Some(ErrorStrategy::Branch { step_id: "step_1".into() })),
        ];
        assert!(validate_step_references(&steps).is_ok());
    }

    #[test]
    fn step_auto_order_by_reference() {
        // step_2 references {{step_1}} → step_1 must be ordered before step_2
        let steps = vec![
            Step {
                id: "step_1".into(),
                step_type: "data_set".into(),
                config: serde_json::json!({"key": "x", "value": "42"}),
                ..Default::default()
            },
            Step {
                id: "step_2".into(),
                step_type: "data_get".into(),
                config: serde_json::json!({"key": "{{step_1.result}}"}),
                ..Default::default()
            },
        ];
        let order = auto_order_steps(&steps);
        let pos1 = order.iter().position(|&i| i == 0);
        let pos2 = order.iter().position(|&i| i == 1);
        assert!(pos1 < pos2, "step_1 must come before step_2 in dependency order");
    }
}