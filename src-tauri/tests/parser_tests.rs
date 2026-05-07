// tests/parser_tests.rs — Parser 转换测试
//
// 测试范围：
//   - Container type gets _container suffix
//   - Actions moved into config
//   - action.params → action.config
//   - logic container gets condition_group in config
//   - Non-container types pass through unchanged

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
// 容器类型加 _container 后缀
// ═══════════════════════════════════════════════════

#[test]
fn container_type_browser_gets_suffix() {
    let steps = parse_steps(json!([
        {"id": "b1", "name": "Browser", "type": "browser", "config": {}, "actions": []}
    ]));
    assert_eq!(steps[0].step_type, "browser_container");
}

#[test]
fn container_type_excel_gets_suffix() {
    let steps = parse_steps(json!([
        {"id": "e1", "name": "Excel", "type": "excel", "config": {}, "actions": []}
    ]));
    assert_eq!(steps[0].step_type, "excel_container");
}

#[test]
fn container_type_word_gets_suffix() {
    let steps = parse_steps(json!([
        {"id": "w1", "name": "Word", "type": "word", "config": {}, "actions": []}
    ]));
    assert_eq!(steps[0].step_type, "word_container");
}

#[test]
fn container_type_logic_gets_suffix() {
    let steps = parse_steps(json!([
        {"id": "l1", "name": "Logic", "type": "logic", "config": {}, "actions": []}
    ]));
    assert_eq!(steps[0].step_type, "logic_container");
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
// Actions moved into config
// ═══════════════════════════════════════════════════

#[test]
fn container_actions_moved_into_config() {
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

    // step.actions 应为 None（已移入 config）
    assert!(steps[0].actions.is_none());

    // config.actions 应存在且有 2 个
    let config_actions = steps[0].config["actions"].as_array().unwrap();
    assert_eq!(config_actions.len(), 2);

    // 原始 config 字段保留
    assert_eq!(steps[0].config["headless"], json!(true));
}

#[test]
fn container_with_no_actions() {
    let steps = parse_steps(json!([
        {"id": "b1", "name": "Browser", "type": "browser", "config": {}, "actions": []}
    ]));

    // 空 actions 数组
    let config_actions = steps[0].config["actions"].as_array().unwrap();
    assert_eq!(config_actions.len(), 0);
}

#[test]
fn container_no_actions_field() {
    let steps = parse_steps(json!([
        {"id": "b1", "name": "Browser", "type": "browser", "config": {"key": "val"}}
    ]));

    // 没有 actions 字段 → config.actions 为空数组
    let config_actions = steps[0].config["actions"].as_array().unwrap();
    assert_eq!(config_actions.len(), 0);
    // 原始 config 保留
    assert_eq!(steps[0].config["key"], json!("val"));
}

// ═══════════════════════════════════════════════════
// action.params → action.config
// ═══════════════════════════════════════════════════

#[test]
fn action_params_converted_to_config() {
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

    let action = &steps[0].config["actions"][0];
    // params 应被转换为 config
    assert!(action.get("params").is_none(), "params should be removed");
    assert_eq!(action["config"]["url"], json!("https://example.com"));
}

#[test]
fn action_no_params_stays_unchanged() {
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

    let action = &steps[0].config["actions"][0];
    // 没有 params → 不应出现 config（除非原有）
    assert_eq!(action["type"], json!("screenshot"));
    assert_eq!(action["label"], json!("截图"));
}

#[test]
fn action_label_auto_filled() {
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

    let action = &steps[0].config["actions"][0];
    // label 应自动填充为 type
    assert_eq!(action["label"], json!("navigate"));
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

    let action = &steps[0].config["actions"][0];
    assert_eq!(action["label"], json!("打开网页"));
}

// ═══════════════════════════════════════════════════
// logic container gets condition_group in config
// ═══════════════════════════════════════════════════

#[test]
fn logic_container_condition_group_in_config() {
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

    // condition_group 应在 config 中
    assert_eq!(steps[0].config["condition_group"]["combinator"], json!("or"));
    let conds = steps[0].config["condition_group"]["conditions"].as_array().unwrap();
    assert_eq!(conds.len(), 2);
    assert_eq!(conds[0]["op"], json!("equals"));
    assert_eq!(conds[1]["op"], json!("gt"));

    // condition 也应在 config 中
    assert_eq!(steps[0].config["condition"], json!("some_expr"));
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

    // 没有 condition_group 字段 → config 中不应有
    assert!(steps[0].config.get("condition_group").is_none());
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

    // browser 容器不应在 config 中注入 condition_group
    assert!(steps[0].config.get("condition_group").is_none());
    assert!(steps[0].config.get("condition").is_none());
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

    assert_eq!(steps[0].step_type, "browser_container");
    assert_eq!(steps[1].step_type, "logic_container");
    assert_eq!(steps[2].step_type, "http");
    assert_eq!(steps[3].step_type, "excel_container");

    // browser action params → config
    assert_eq!(steps[0].config["actions"][0]["config"]["url"], json!("https://example.com"));

    // excel action params → config
    assert_eq!(steps[3].config["actions"][0]["config"]["path"], json!("data.xlsx"));
}

// ═══════════════════════════════════════════════════
// Parser 校验
// ═══════════════════════════════════════════════════

#[test]
fn parse_workflow_empty_name_fails() {
    let result = parser::parse_workflow(r#"{"name": "  ", "steps": [{"id": "s1", "name": "A", "type": "http", "config": {}}]}"#);
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

// ═══════════════════════════════════════════════════
// then_steps / else_steps 递归转换
// ═══════════════════════════════════════════════════

#[test]
fn then_else_steps_recursively_converted() {
    let steps = parse_steps(json!([
        {
            "id": "cond1",
            "name": "Condition",
            "type": "condition",
            "config": {},
            "then_steps": [
                {"id": "b1", "name": "Browse", "type": "browser", "config": {}, "actions": []}
            ],
            "else_steps": [
                {"id": "e1", "name": "Excel", "type": "excel", "config": {}, "actions": []}
            ]
        }
    ]));

    let then = steps[0].then_steps.as_ref().unwrap();
    let els = steps[0].else_steps.as_ref().unwrap();

    assert_eq!(then.len(), 1);
    assert_eq!(then[0].step_type, "browser_container");

    assert_eq!(els.len(), 1);
    assert_eq!(els[0].step_type, "excel_container");
}
