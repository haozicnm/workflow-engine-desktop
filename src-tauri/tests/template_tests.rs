// tests/template_tests.rs — 内置模板结构验证 + parser 转换测试
use serde_json::Value;
use workflow_engine::engine::parser;

const TEMPLATE_DIR: &str = "../templates";

fn load_template(filename: &str) -> Value {
    let path = format!("{}/{}", TEMPLATE_DIR, filename);
    let content = std::fs::read_to_string(&path)
        .expect(&format!("模板文件不存在: {}", path));
    serde_json::from_str(&content)
        .expect(&format!("模板 JSON 解析失败: {}", filename))
}

// ═══════════════════════════════════════════════════
// 模板 1: order-to-contracts (Excel → cursor → Word → notify)
// ═══════════════════════════════════════════════════

#[test]
fn test_template1_order_to_contracts() {
    let tmpl = load_template("order-to-contracts.json");
    let steps = tmpl["steps"].as_array().expect("steps 应为数组");

    assert_eq!(steps.len(), 3, "模板1应有3个步骤");
    assert_eq!(steps[0]["type"].as_str().unwrap(), "excel");
    assert_eq!(steps[1]["type"].as_str().unwrap(), "cursor");
    assert_eq!(steps[2]["type"].as_str().unwrap(), "notify");

    // 验证 Excel actions
    let actions0 = steps[0]["actions"].as_array().unwrap();
    assert_eq!(actions0.len(), 1);
    assert_eq!(actions0[0]["type"].as_str().unwrap(), "read");

    // 验证 cursor items 引用
    let cursor_items = steps[1]["config"]["items"].as_str().unwrap();
    assert!(cursor_items.contains("step_excel"));
    assert!(cursor_items.contains("data"));

    // 验证 cursor body
    let body = steps[1]["config"]["body"].as_array().unwrap();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["type"].as_str().unwrap(), "word");

    // Parser 转换
    let yaml = serde_json::to_string(&tmpl).unwrap();
    let wf = parser::parse_workflow(&yaml);
    assert!(wf.is_ok(), "Parser 转换失败: {:?}", wf.err());
    let wf = wf.unwrap();
    assert_eq!(wf.steps.len(), 3);
    assert_eq!(wf.steps[0].step_type, "excel_container");
    assert_eq!(wf.steps[1].step_type, "cursor");  // 不做容器转换
    assert_eq!(wf.steps[2].step_type, "notify");

    println!("✅ 模板1: excel_container → cursor → notify");
}

// ═══════════════════════════════════════════════════
// 模板 2: monitor-to-report (Browser → logic → Excel/Word 分支)
// ═══════════════════════════════════════════════════

#[test]
fn test_template2_monitor_to_report() {
    let tmpl = load_template("monitor-to-report.json");
    let steps = tmpl["steps"].as_array().expect("steps 应为数组");

    assert_eq!(steps.len(), 5, "模板2应有5个步骤");
    assert_eq!(steps[0]["type"].as_str().unwrap(), "browser");
    assert_eq!(steps[1]["type"].as_str().unwrap(), "logic");
    assert_eq!(steps[2]["type"].as_str().unwrap(), "excel");
    assert_eq!(steps[3]["type"].as_str().unwrap(), "notify");
    assert_eq!(steps[4]["type"].as_str().unwrap(), "word");

    // 验证 browser actions
    let browser_actions = steps[0]["actions"].as_array().unwrap();
    assert_eq!(browser_actions.len(), 2);
    assert_eq!(browser_actions[0]["type"].as_str().unwrap(), "navigate");
    assert_eq!(browser_actions[1]["type"].as_str().unwrap(), "evaluate");

    // 验证 conditionGroup
    let cg = &steps[1]["conditionGroup"];
    assert_eq!(cg["combinator"].as_str().unwrap(), "or");
    let conds = cg["conditions"].as_array().unwrap();
    assert_eq!(conds.len(), 2);
    assert_eq!(conds[0]["op"].as_str().unwrap(), "contains");

    // 验证条件路由
    assert_eq!(steps[1]["config"]["true_next"].as_str().unwrap(), "step_error");
    assert_eq!(steps[1]["config"]["false_next"].as_str().unwrap(), "step_normal");

    // 验证 runCondition
    assert_eq!(steps[2]["runCondition"]["ref"].as_str().unwrap(), "step_logic");
    assert_eq!(steps[2]["runCondition"]["when"].as_str().unwrap(), "true");
    assert_eq!(steps[4]["runCondition"]["ref"].as_str().unwrap(), "step_logic");
    assert_eq!(steps[4]["runCondition"]["when"].as_str().unwrap(), "false");

    // Parser 转换
    let yaml = serde_json::to_string(&tmpl).unwrap();
    let wf = parser::parse_workflow(&yaml);
    assert!(wf.is_ok(), "Parser 转换失败: {:?}", wf.err());
    let wf = wf.unwrap();
    assert_eq!(wf.steps.len(), 5);
    assert_eq!(wf.steps[0].step_type, "browser_container");
    assert_eq!(wf.steps[1].step_type, "logic_container");

    println!("✅ 模板2: browser_container → logic_container → excel/word 分支");
}
