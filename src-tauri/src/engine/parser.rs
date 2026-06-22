// engine/parser.rs — 工作流解析（v8: 1:1 透传，砍掉中间转换层）
//
// 前端 JSON 与后端 Step 严格一一对应：
//   { type: "browser", actions: [{ id, type: "navigate", params: { url } }] }
//
// parser 只做验证和 body step 递归处理，不再做类型名/字段名转换。

use crate::engine::workflow::{Step, Workflow};
use anyhow::{anyhow, Result};
use serde_json::Value;
use tracing::warn;

// v8: 容器类型从 registry（node-schema.json）读取，不再硬编码
// 迭代类型列表（body 步骤存在 actions 里，需转为 body_steps）
const ITERATION_TYPES: &[&str] = &["cursor", "loop"];

/// 解析工作流 JSON（支持新旧两种格式）
pub fn parse_workflow(json_str: &str) -> Result<Workflow> {
    // 先尝试 JSON，再尝试 YAML
    let raw: Value = serde_json::from_str(json_str)
        .or_else(|_| serde_yaml::from_str(json_str))
        .map_err(|e| anyhow!("工作流解析失败: {}", e))?;

    let mut wf: Workflow =
        serde_json::from_value(raw.clone()).map_err(|e| anyhow!("工作流结构解析失败: {}", e))?;

    // 基本校验
    if wf.name.trim().is_empty() {
        return Err(anyhow!("工作流名称不能为空"));
    }
    if wf.steps.is_empty() {
        return Err(anyhow!("工作流至少需要一个步骤"));
    }

    // 版本兼容性检查
    crate::engine::yaml_format::check_version_compatibility(&wf)?;

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

    // DAG 模式：edges 非空时检测循环依赖
    if !wf.edges.is_empty() {
        detect_cycles(&wf.steps, &wf.edges)?;

        // 警告：edges + runCondition 共存时提醒用户
        let has_run_condition = wf.steps.iter().any(|s| s.run_condition.is_some());
        if has_run_condition {
            warn!("工作流同时使用了 edges 和 runCondition。DAG 模式下 edges 优先，runCondition 会被忽略。建议迁移到纯 edges 模式。");
        }
    }

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
                    step.id,
                    rc.ref_step
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
                    step.id,
                    step_id
                ));
            }
            if step_id == &step.id {
                return Err(anyhow!(
                    "步骤 '{}' 的 onError branch 引用了自身，这会形成死循环",
                    step.id
                ));
            }
        }

        // 校验变量引用：{{step_xxx}} / {{step_xxx.y}} 中的 step ID 必须存在
        let config_str = serde_json::to_string(&step.config).unwrap_or_default();
        for ref_id in extract_step_refs(&config_str) {
            let full_id = format!("step_{}", ref_id);
            // 同时检查带 "step_" 前缀和原始 ID（兼容 step.id == "1" 等非 step_ 前缀命名）
            let stripped = full_id.strip_prefix("step_").unwrap_or(&full_id);
            if !step_ids.contains(full_id.as_str()) && !step_ids.contains(stripped) {
                return Err(anyhow!(
                    "步骤 '{}' 中引用了不存在的步骤 '{}'（{{{{step_{}}}}} 指向的步骤不存在）",
                    step.id,
                    full_id,
                    ref_id
                ));
            }
        }
    }

    Ok(())
}

/// 转换单个步骤：容器类型加 _container，actions 移入 config
/// StepParser trait — 责任链模式
/// 每种节点类型由独立的 parser 处理，替代大段 if/else
trait StepParser {
    /// 是否能处理此步骤类型
    fn can_parse(step_type: &str) -> bool;
    /// 解析步骤：返回 (new_type, new_config, body_steps)
    fn parse(step: &Step, is_recursive: bool) -> Result<(String, Value, Option<Vec<Step>>)>;
}

/// 容器 Parser: browser, excel, word, logic, file
/// v8: 不再加 _container 后缀，不再搬运 actions——JSON 与 Step 1:1 对应
struct ContainerParser;
impl StepParser for ContainerParser {
    fn can_parse(step_type: &str) -> bool {
        crate::nodes::registry::is_container(step_type)
    }

    fn parse(step: &Step, _is_recursive: bool) -> Result<(String, Value, Option<Vec<Step>>)> {
        // v8: 类型名透传，不加 _container
        Ok((step.step_type.clone(), step.config.clone(), None))
    }
}

/// 迭代 Parser: cursor, loop
struct IterationParser;
impl StepParser for IterationParser {
    fn can_parse(step_type: &str) -> bool {
        ITERATION_TYPES.contains(&step_type)
    }

    fn parse(step: &Step, _is_recursive: bool) -> Result<(String, Value, Option<Vec<Step>>)> {
        let actions = step.actions.clone().unwrap_or_default();
        let raw_steps: Vec<Step> = if actions.is_empty() {
            step.body_steps.clone().unwrap_or_default()
        } else {
            actions
                .iter()
                .map(convert_action_to_step)
                .collect::<Result<Vec<_>>>()?
        };

        let body_steps: Vec<Step> = raw_steps
            .into_iter()
            .map(|s| convert_step(&s, true))
            .collect::<Result<Vec<_>>>()?;

        Ok((
            step.step_type.clone(),
            step.config.clone(),
            Some(body_steps),
        ))
    }
}

/// 简单步骤 Parser: http, shell, script, delay, notify 等
struct SimpleStepParser;
impl StepParser for SimpleStepParser {
    fn can_parse(_step_type: &str) -> bool {
        true
    }

    fn parse(step: &Step, _is_recursive: bool) -> Result<(String, Value, Option<Vec<Step>>)> {
        Ok((step.step_type.clone(), step.config.clone(), None))
    }
}

/// 责任链分派：按顺序尝试 parser
fn dispatch_parser(step: &Step, is_recursive: bool) -> Result<(String, Value, Option<Vec<Step>>)> {
    if IterationParser::can_parse(&step.step_type) {
        IterationParser::parse(step, is_recursive)
    } else if ContainerParser::can_parse(&step.step_type) {
        ContainerParser::parse(step, is_recursive)
    } else {
        SimpleStepParser::parse(step, is_recursive)
    }
}

/// 转换单个步骤（前端格式 → 执行格式）
///
/// `is_recursive`: true 表示这是 body step 的递归调用，
///   此时容器仅改 type 名，不再重复移动 actions。
fn convert_step(step: &Step, is_recursive: bool) -> Result<Step> {
    let (new_type, new_config, parsed_body) = dispatch_parser(step, is_recursive)?;

    // 非迭代类型也可能有 body_steps（如 sub_workflow），同样递归转换
    let body_steps = parsed_body.or_else(|| {
        step.body_steps
            .as_ref()
            .map(|steps| {
                steps
                    .iter()
                    .map(|s| convert_step(s, true))
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()
            .ok()
            .flatten()
    });

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
        actions: step.actions.clone(), // v8: 保持原位，不再移到 config
        expanded: None,
        condition: step.condition.clone(),
        condition_group: step.condition_group.clone().or_else(|| {
            // 如果 JSON 的 condition_group 在 config 内而非 step 顶层，
            // 自动提升到 step 级别，避免 executor 的 resolve_config 改变类型后反序列化失败
            step.config
                .get("condition_group")
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
    let map = action
        .as_object()
        .ok_or_else(|| anyhow!("action 不是对象"))?;

    let id = map
        .get("id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let step_type = map
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let name = map
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or(&step_type)
        .to_string();

    // params → config
    let mut config = map
        .get("params")
        .cloned()
        .or_else(|| map.get("config").cloned())
        .unwrap_or(serde_json::json!({}));

    // v8: 提取容器类型的 actions — 从 params.actions 中提取并放在 Step.actions
    let actions = if crate::nodes::registry::is_container(step_type.as_str()) {
        config
            .as_object()
            .and_then(|c| c.get("actions"))
            .and_then(|v| v.as_array())
            .cloned()
    } else {
        None
    };
    // 从 config 中移除 actions（避免重复，归一化时 executor 会重新注入）
    if let Some(obj) = config.as_object_mut() {
        obj.remove("actions");
    }

    Ok(Step {
        id,
        name,
        step_type,
        config,
        actions,
        ..Default::default()
    })
}

/// 归一化 action：params → config，补 label
/// v8: 从 parser 私有 fn 提升为 pub，供 executor 在容器节点前调用
pub fn normalize_actions(actions: &[Value]) -> Vec<Value> {
    actions
        .iter()
        .map(|action| {
            let mut a = action.clone();
            if let Value::Object(ref mut map) = a {
                if let Some(params) = map.remove("params") {
                    map.insert("config".to_string(), params);
                }
                // 补 label
                if !map.contains_key("label") {
                    let action_type = map
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown");
                    map.insert("label".to_string(), Value::String(action_type.to_string()));
                }
            }
            a
        })
        .collect()
}

/// 递归收集所有步骤（包括 body_steps 子步骤）
fn flatten_all_steps(steps: &[Step]) -> Vec<&Step> {
    let mut result = Vec::new();
    fn recurse<'a>(steps: &'a [Step], out: &mut Vec<&'a Step>) {
        for step in steps {
            out.push(step);
            if let Some(ref body) = step.body_steps {
                recurse(body, out);
            }
        }
    }
    recurse(steps, &mut result);
    result
}

/// 根据 {{step_xxx}} / output.step_xxx 引用自动推断步骤依赖顺序
pub fn auto_order_steps(steps: &[Step]) -> Vec<usize> {
    use std::collections::HashMap;

    let n = steps.len();
    if n == 0 {
        return Vec::new();
    }

    let id_to_idx: HashMap<&str, usize> = steps
        .iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();

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

    let mut queue: std::collections::VecDeque<usize> = (0..n).filter(|&i| in_deg[i] == 0).collect();
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
        let mut remaining: Vec<usize> = (0..n).filter(|i| !order.contains(i)).collect();
        tracing::warn!(
            "auto_order_steps: 检测到循环依赖，{} 个步骤无法排序 (索引: {:?})，追加到末尾",
            remaining.len(),
            remaining
        );
        order.append(&mut remaining);
    }

    order
}

fn extract_step_refs(s: &str) -> Vec<String> {
    let mut refs = Vec::new();

    let prefix1 = "output.step_";
    for (pos, _) in s.match_indices(prefix1) {
        let rest = &s[pos + prefix1.len()..];
        let id: String = rest
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
            .collect();
        if !id.is_empty() {
            refs.push(id);
        }
    }

    let prefix2 = "{{step_";
    for (pos, _) in s.match_indices(prefix2) {
        let rest = &s[pos + prefix2.len()..];
        let id: String = rest
            .chars()
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

/// DAG 环检测（DFS visiting 栈，参考 ComfyUI validate_inputs）
///
/// 时间复杂度 O(V+E)，空间复杂度 O(V)
/// 发现环时返回精确的环路径
fn detect_cycles(
    steps: &[crate::engine::workflow::Step],
    edges: &[crate::engine::workflow::Edge],
) -> Result<()> {
    use std::collections::{HashMap, HashSet};

    // 构建邻接表
    let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
    for step in steps {
        adj.entry(step.id.as_str()).or_default();
    }
    for edge in edges {
        adj.entry(edge.from.as_str()).or_default().push(&edge.to);
    }

    // DFS 环检测
    let mut visited: HashSet<&str> = HashSet::new();   // 已完成的节点
    let mut visiting: Vec<&str> = Vec::new();           // 当前递归路径

    fn dfs<'a>(
        node: &'a str,
        adj: &HashMap<&str, Vec<&'a str>>,
        visited: &mut HashSet<&'a str>,
        visiting: &mut Vec<&'a str>,
    ) -> Result<()> {
        if visiting.contains(&node) {
            // 发现环
            let cycle_start = visiting.iter().position(|&n| n == node).unwrap();
            let cycle_path: Vec<&str> = visiting[cycle_start..].to_vec();
            let path_str = cycle_path.join(" → ");
            return Err(anyhow::anyhow!(
                "工作流包含循环依赖: {} → {}",
                path_str, node
            ));
        }
        if visited.contains(node) {
            return Ok(()); // 已检查过，无环
        }

        visiting.push(node);
        if let Some(neighbors) = adj.get(node) {
            for &neighbor in neighbors {
                dfs(neighbor, adj, visited, visiting)?;
            }
        }
        visiting.pop();
        visited.insert(node);
        Ok(())
    }

    for step in steps {
        if !visited.contains(step.id.as_str()) {
            dfs(&step.id, &adj, &mut visited, &mut visiting)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::context::ExecutionContext;
    use crate::engine::workflow::ErrorStrategy;
    use crate::engine::workflow::{Step, Workflow};

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
            make_step(
                "step_2",
                None,
                Some(ErrorStrategy::Branch {
                    step_id: "step_1".into(),
                }),
            ),
        ];
        assert!(validate_step_references(&steps).is_ok());
    }

    #[test]
    fn missing_branch_target_fails() {
        let steps = vec![
            make_step("step_1", None, None),
            make_step(
                "step_2",
                None,
                Some(ErrorStrategy::Branch {
                    step_id: "step_missing".into(),
                }),
            ),
        ];
        let err = validate_step_references(&steps).unwrap_err();
        assert!(err.to_string().contains("step_missing"));
    }

    #[test]
    fn self_referencing_branch_fails() {
        let steps = vec![make_step(
            "step_1",
            None,
            Some(ErrorStrategy::Branch {
                step_id: "step_1".into(),
            }),
        )];
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
        assert_eq!(body[0].step_type, "browser");
        // v8: actions 从 params 提取到 step.actions
        // 该测试的 actions: [] 为空数组，但 container 类型会保留空 actions
        let _has_actions = body[0]
            .actions
            .as_ref()
            .map(|a| !a.is_empty())
            .unwrap_or(false);
        // body step 容器类型需要 actions（执行时由 executor 归一化补充）
        assert!(
            body[0].actions.is_some(),
            "container body step should have actions field"
        );
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
        assert!(
            msg.contains("step_99"),
            "expected error about step_99, got: {}",
            msg
        );
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
    fn container_type_passthrough() {
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
        // v8: 类型名透传，不再加 _container 后缀
        assert_eq!(wf.steps[0].step_type, "browser");
        // v8: actions 保持在 step.actions，不再移到 config
        let actions = wf.steps[0].actions.as_ref().unwrap();
        assert!(!actions.is_empty(), "step.actions should be preserved");
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
        // v8: 类型名透传
        assert_eq!(wf.steps[0].step_type, "logic");
        // v8: condition_group 由 convert_step 提升到 step 级别
        assert!(
            wf.steps[0].condition_group.is_some(),
            "condition_group should be on step"
        );
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
        assert_eq!(
            val.and_then(|v| v.as_str()),
            Some("output_val"),
            "step_outputs should take priority over variables for same key"
        );
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
        let steps = vec![make_step("step_1", Some("step_1"), None)];
        // self-referencing runCondition is a warning, not an error
        assert!(validate_step_references(&steps).is_ok());
    }

    #[test]
    fn error_strategy_fail_is_default() {
        use crate::engine::workflow::ErrorStrategy;
        let default = ErrorStrategy::default();
        match default {
            ErrorStrategy::Fail => {} // expected
            _ => panic!("default ErrorStrategy should be Fail"),
        }
    }

    #[test]
    fn error_strategy_branch_with_valid_target() {
        let steps = vec![
            make_step("step_1", None, None),
            make_step(
                "step_2",
                None,
                Some(ErrorStrategy::Branch {
                    step_id: "step_1".into(),
                }),
            ),
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
        assert!(
            pos1 < pos2,
            "step_1 must come before step_2 in dependency order"
        );
    }
}
