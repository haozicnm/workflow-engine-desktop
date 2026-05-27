// tests/parser_tests.rs — Parser 转换测试
//
// v8 行为：parser 是纯 1:1 透传（JSON → Step 结构体），不做字段搬运。
//   - actions 保持在 step.actions，不移到 config
//   - action.params 保持原样（归一化由 executor 的 normalize_actions 在运行时处理）
//   - condition_group 保持在 step.condition_group，不移到 config
//   - 容器类型名透传，不加 _container 后缀
//
// normalize_actions (params→config, 补 label) 由 executor 运行时调用，
// 不在 parser 阶段执行，相关测试在 parser 内部 mod tests 中覆盖。

use serde_json::json;
use workflow_engine::engine::parser;
use workflow_engine::engine::workflow::{LogicCondition, LogicConditionGroup};

// ─── 辅助 ───

/// 构造最简工作流 JSON 并解析
fn parse_steps(steps_json: serde_json::Value) -> Vec<workflow_engine::engine::workflow::Step> {
    let wf_json = json!({
        "name": "test",
        "steps": steps_json
    });
    let wf = parser::parse_workflow(&wf_json.to_string()).expect("parse_workflow failed");
    wf.steps
}

// ═══════════════════════════════════════════════════
// 容器类型名透传（v8: 不加 _container 后缀）
// ═══════════════════════════════════════════════════

#[test]
fn container_type_browser_passthrough() {
    let steps = parse_steps(json!([
        {"id": "b1", "name": "Browser", "type": "browser", "config": {}, "actions": []}
    ]));
    assert_eq!(steps[0].step_type, "browser");
}

#[test]
fn container_type_excel_passthrough() {
    let steps = parse_steps(json!([
        {"id": "e1", "name": "Excel", "type": "excel", "config": {}, "actions": []}
    ]));
    assert_eq!(steps[0].step_type, "excel");
}

#[test]
fn container_type_word_passthrough() {
    let steps = parse_steps(json!([
        {"id": "w1", "name": "Word", "type": "word", "config": {}, "actions": []}
    ]));
    assert_eq!(steps[0].step_type, "word");
}

#[test]
fn container_type_logic_passthrough() {
    let steps = parse_steps(json!([
        {"id": "l1", "name": "Logic", "type": "logic", "config": {}, "actions": []}
    ]));
    assert_eq!(steps[0].step_type, "logic");
}

// ═══════════════════════════════════════════════════
// Non-container types pass through unchanged
// ═══════════════════════════════════════════════════

#[test]
fn non_container_type_http_unchanged() {
    let steps = parse_steps(json!([
        {"id": "h1", "name": "HTTP", "type": "http", "config": {"url": "example.com"}}
    ]));
    assert_eq!(steps[0].step_type, "http");
}

#[test]
fn non_container_type_script_unchanged() {
    let steps = parse_steps(json!([
        {"id": "s1", "name": "Script", "type": "script", "config": {"script": "1+1"}}
    ]));
    assert_eq!(steps[0].step_type, "script");
}

#[test]
fn non_container_type_notify_unchanged() {
    let steps = parse_steps(json!([
        {"id": "n1", "name": "Notify", "type": "notify", "config": {"message": "hi"}}
    ]));
    assert_eq!(steps[0].step_type, "notify");
}

#[test]
fn non_container_type_cursor_unchanged() {
    let steps = parse_steps(json!([
        {"id": "c1", "name": "Cursor", "type": "cursor", "config": {"items": []}}
    ]));
    assert_eq!(steps[0].step_type, "cursor");
}

#[test]
fn non_container_type_condition_unchanged() {
    let steps = parse_steps(json!([
        {"id": "cond1", "name": "Cond", "type": "condition", "config": {}}
    ]));
    assert_eq!(steps[0].step_type, "condition");
}

// ═══════════════════════════════════════════════════
// v8: actions 保持在 step.actions（不移到 config）
// ═══════════════════════════════════════════════════

#[test]
fn container_actions_stay_in_step_actions() {
    let steps = parse_steps(json!([
        {
            "id": "b1",
            "name": "Browser",
            "type": "browser",
            "config": {"headless": true},
            "actions": [
                {"id": "a1", "type": "navigate", "params": {"url": "https://example.com"}},
                {"id": "a2", "type": "click", "params": {"selector": "#btn"}}
            ]
        }
    ]));

    // v8: actions 保持在 step.actions，不移入 config
    let actions = steps[0]
        .actions
        .as_ref()
        .expect("actions should be preserved");
    assert_eq!(actions.len(), 2);
    assert_eq!(actions[0]["type"], json!("navigate"));
    assert_eq!(actions[1]["type"], json!("click"));

    // config 保持原样，不注入 actions
    assert_eq!(steps[0].config["headless"], json!(true));
    assert!(steps[0].config.get("actions").is_none());
}

#[test]
fn container_with_empty_actions() {
    let steps = parse_steps(json!([
        {"id": "b1", "name": "Browser", "type": "browser", "config": {}, "actions": []}
    ]));

    // 空 actions → step.actions 为 Some([])
    let actions = steps[0].actions.as_ref().unwrap();
    assert_eq!(actions.len(), 0);
}

#[test]
fn container_no_actions_field_stays_none() {
    let steps = parse_steps(json!([
        {"id": "b1", "name": "Browser", "type": "browser", "config": {"key": "val"}}
    ]));

    // 没有 actions 字段 → step.actions 为 None
    assert!(steps[0].actions.is_none());
    // config 保持原样
    assert_eq!(steps[0].config["key"], json!("val"));
}

// ═══════════════════════════════════════════════════
// v8: action.params 保持原样（归一化由 executor 运行时处理）
// ═══════════════════════════════════════════════════

#[test]
fn action_params_preserved_as_is() {
    let steps = parse_steps(json!([
        {
            "id": "b1",
            "name": "Browser",
            "type": "browser",
            "config": {},
            "actions": [
                {"id": "a1", "type": "navigate", "params": {"url": "https://example.com"}}
            ]
        }
    ]));

    let action = &steps[0].actions.as_ref().unwrap()[0];
    // v8: params 保持原样，不在 parser 阶段转换
    assert_eq!(action["params"]["url"], json!("https://example.com"));
}

#[test]
fn action_without_params_stays_unchanged() {
    let steps = parse_steps(json!([
        {
            "id": "b1",
            "name": "Browser",
            "type": "browser",
            "config": {},
            "actions": [
                {"id": "a1", "type": "screenshot", "label": "截图"}
            ]
        }
    ]));

    let action = &steps[0].actions.as_ref().unwrap()[0];
    assert_eq!(action["type"], json!("screenshot"));
    assert_eq!(action["label"], json!("截图"));
}

#[test]
fn action_label_not_auto_filled_by_parser() {
    let steps = parse_steps(json!([
        {
            "id": "b1",
            "name": "Browser",
            "type": "browser",
            "config": {},
            "actions": [
                {"id": "a1", "type": "navigate", "params": {"url": "https://example.com"}}
            ]
        }
    ]));

    let action = &steps[0].actions.as_ref().unwrap()[0];
    // v8: parser 不补 label（由 executor normalize_actions 运行时处理）
    assert!(action.get("label").is_none());
}

#[test]
fn action_existing_label_preserved() {
    let steps = parse_steps(json!([
        {
            "id": "b1",
            "name": "Browser",
            "type": "browser",
            "config": {},
            "actions": [
                {"id": "a1", "type": "navigate", "label": "打开网页", "params": {"url": "https://example.com"}}
            ]
        }
    ]));

    let action = &steps[0].actions.as_ref().unwrap()[0];
    assert_eq!(action["label"], json!("打开网页"));
}

// ═══════════════════════════════════════════════════
// v8: condition_group 保持在 step.condition_group（不移到 config）
// ═══════════════════════════════════════════════════

#[test]
fn logic_condition_group_stays_in_step() {
    let cg = LogicConditionGroup {
        combinator: "or".to_string(),
        conditions: vec![
            LogicCondition {
                id: "c1".to_string(),
                left: "status".to_string(),
                op: "equals".to_string(),
                right: "error".to_string(),
            },
            LogicCondition {
                id: "c2".to_string(),
                left: "count".to_string(),
                op: "gt".to_string(),
                right: "100".to_string(),
            },
        ],
    };

    let cg_json = serde_json::to_value(&cg).unwrap();

    let steps = parse_steps(json!([
        {
            "id": "l1",
            "name": "判断",
            "type": "logic",
            "config": {},
            "actions": [],
            "condition": "some_expr",
            "conditionGroup": cg_json
        }
    ]));

    // v8: condition_group 保持在 step 级别
    let step_cg = steps[0]
        .condition_group
        .as_ref()
        .expect("condition_group should be preserved");
    assert_eq!(step_cg.combinator, "or");
    assert_eq!(step_cg.conditions.len(), 2);
    assert_eq!(step_cg.conditions[0].op, "equals");
    assert_eq!(step_cg.conditions[1].op, "gt");

    // condition 保持在 step 级别
    assert_eq!(steps[0].condition.as_deref(), Some("some_expr"));

    // config 不注入 condition_group
    assert!(steps[0].config.get("condition_group").is_none());
}

#[test]
fn logic_container_without_condition_group() {
    let steps = parse_steps(json!([
        {
            "id": "l1",
            "name": "Logic",
            "type": "logic",
            "config": {},
            "actions": []
        }
    ]));

    assert!(steps[0].condition_group.is_none());
}

#[test]
fn non_logic_container_no_condition_group() {
    let steps = parse_steps(json!([
        {
            "id": "b1",
            "name": "Browser",
            "type": "browser",
            "config": {},
            "actions": []
        }
    ]));

    assert!(steps[0].condition_group.is_none());
    assert!(steps[0].condition.is_none());
}

// ═══════════════════════════════════════════════════
// 综合：多个步骤混合
// ═══════════════════════════════════════════════════

#[test]
fn mixed_container_and_non_container_steps() {
    let steps = parse_steps(json!([
        {"id": "b1", "name": "Browse", "type": "browser", "config": {}, "actions": [
            {"id": "a1", "type": "navigate", "params": {"url": "https://example.com"}}
        ]},
        {"id": "l1", "name": "Check", "type": "logic", "config": {}, "actions": []},
        {"id": "h1", "name": "HTTP", "type": "http", "config": {"url": "https://api.example.com"}},
        {"id": "e1", "name": "Excel", "type": "excel", "config": {}, "actions": [
            {"id": "a2", "type": "read", "params": {"path": "data.xlsx"}}
        ]}
    ]));

    assert_eq!(steps[0].step_type, "browser");
    assert_eq!(steps[1].step_type, "logic");
    assert_eq!(steps[2].step_type, "http");
    assert_eq!(steps[3].step_type, "excel");

    // v8: actions 保持在 step.actions，params 保持原样
    let b_actions = steps[0].actions.as_ref().unwrap();
    assert_eq!(b_actions[0]["params"]["url"], json!("https://example.com"));

    let e_actions = steps[3].actions.as_ref().unwrap();
    assert_eq!(e_actions[0]["params"]["path"], json!("data.xlsx"));
}

// ═══════════════════════════════════════════════════
// Parser 校验
// ═══════════════════════════════════════════════════

#[test]
fn parse_workflow_empty_name_fails() {
    let result = parser::parse_workflow(
        r#"{"name": "  ", "steps": [{"id": "s1", "name": "A", "type": "http", "config": {}}]}"#,
    );
    assert!(result.is_err());
}

#[test]
fn parse_workflow_no_steps_fails() {
    let result = parser::parse_workflow(r#"{"name": "test", "steps": []}"#);
    assert!(result.is_err());
}

#[test]
fn parse_workflow_duplicate_ids_fails() {
    let wf_json = json!({
        "name": "test",
        "steps": [
            {"id": "s1", "name": "A", "type": "http", "config": {}},
            {"id": "s1", "name": "B", "type": "http", "config": {}}
        ]
    });
    let result = parser::parse_workflow(&wf_json.to_string());
    assert!(result.is_err());
}

#[test]
fn parse_workflow_preserves_next_field() {
    let steps = parse_steps(json!([
        {"id": "s1", "name": "A", "type": "http", "config": {}, "next": "s2"},
        {"id": "s2", "name": "B", "type": "http", "config": {}}
    ]));
    assert_eq!(steps[0].next, Some("s2".to_string()));
    assert_eq!(steps[1].next, None);
}

#[test]
fn parse_workflow_step_name_alias_label() {
    let steps = parse_steps(json!([
        {"id": "s1", "label": "AliasedName", "type": "http", "config": {}}
    ]));
    assert_eq!(steps[0].name, "AliasedName");
}
