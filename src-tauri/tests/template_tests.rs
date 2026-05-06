// tests/template_tests.rs — 4 个内置模板结构验证 + parser 转换测试
use serde_json::Value;
use workflow_engine::engine::parser;

/// 模板目录（相对于 src-tauri/）
const TEMPLATE_DIR: &str = "../templates";

/// 辅助：加载模板 JSON
fn load_template(filename: &str) -> Value {
    let path = format!("{}/{}", TEMPLATE_DIR, filename);
    let content = std::fs::read_to_string(&path)
        .expect(&format!("模板文件不存在: {}", path));
    serde_json::from_str(&content)
        .expect(&format!("模板 JSON 解析失败: {}", filename))
}

// ═══════════════════════════════════════════════════
// 模板 1: monitor-excel-alert (浏览器→逻辑判断)
// ═══════════════════════════════════════════════════

#[test]
fn test_template1_monitor_excel_alert() {
    let tmpl = load_template("monitor-excel-alert.json");
    let steps = tmpl["steps"].as_array().expect("steps 应为数组");

    assert_eq!(steps.len(), 2, "模板1应有2个步骤");
    assert_eq!(steps[0]["type"].as_str().unwrap(), "browser");
    assert_eq!(steps[1]["type"].as_str().unwrap(), "logic");

    // 验证 actions
    let actions0 = steps[0]["actions"].as_array().unwrap();
    assert!(actions0.len() >= 2, "浏览器步骤应有至少2个动作");
    assert_eq!(actions0[0]["type"].as_str().unwrap(), "navigate");
    assert_eq!(actions0[1]["type"].as_str().unwrap(), "evaluate");

    // 验证 parser 可以转换
    let yaml = serde_json::to_string(&tmpl).unwrap();
    let wf = parser::parse_workflow(&yaml);
    assert!(wf.is_ok(), "Parser 转换失败: {:?}", wf.err());
    let wf = wf.unwrap();
    assert_eq!(wf.steps.len(), 2);
    assert_eq!(wf.steps[0].step_type, "browser_container");
    assert_eq!(wf.steps[1].step_type, "logic_container");

    println!("✅ 模板1 结构验证通过: browser_container → logic_container");
}

// ═══════════════════════════════════════════════════
// 模板 2: excel-to-word-batch (Excel→逻辑判断)
// ═══════════════════════════════════════════════════

#[test]
fn test_template2_excel_to_word_batch() {
    let tmpl = load_template("excel-to-word-batch.json");
    let steps = tmpl["steps"].as_array().expect("steps 应为数组");

    assert_eq!(steps.len(), 2, "模板2应有2个步骤");
    assert_eq!(steps[0]["type"].as_str().unwrap(), "excel");
    assert_eq!(steps[1]["type"].as_str().unwrap(), "logic");

    let actions0 = steps[0]["actions"].as_array().unwrap();
    assert_eq!(actions0[0]["type"].as_str().unwrap(), "read_cell");

    // Parser 转换
    let yaml = serde_json::to_string(&tmpl).unwrap();
    let wf = parser::parse_workflow(&yaml).unwrap();
    assert_eq!(wf.steps[0].step_type, "excel_container");
    assert_eq!(wf.steps[1].step_type, "logic_container");

    println!("✅ 模板2 结构验证通过: excel_container → logic_container");
}

// ═══════════════════════════════════════════════════
// 模板 3: api-excel-word-branch (Excel→逻辑判断)
// ═══════════════════════════════════════════════════

#[test]
fn test_template3_api_excel_word_branch() {
    let tmpl = load_template("api-excel-word-branch.json");
    let steps = tmpl["steps"].as_array().expect("steps 应为数组");

    assert_eq!(steps.len(), 2, "模板3应有2个步骤");
    assert_eq!(steps[0]["type"].as_str().unwrap(), "excel");
    assert_eq!(steps[1]["type"].as_str().unwrap(), "logic");

    let actions1 = steps[1]["actions"].as_array().unwrap();
    assert_eq!(actions1[0]["type"].as_str().unwrap(), "greater_than");

    // Parser 转换
    let yaml = serde_json::to_string(&tmpl).unwrap();
    let wf = parser::parse_workflow(&yaml).unwrap();
    assert_eq!(wf.steps[0].step_type, "excel_container");
    assert_eq!(wf.steps[1].step_type, "logic_container");

    println!("✅ 模板3 结构验证通过: excel_container → logic_container");
}

// ═══════════════════════════════════════════════════
// 模板 4: word-extract-excel (Word→逻辑判断)
// ═══════════════════════════════════════════════════

#[test]
fn test_template4_word_extract_excel() {
    let tmpl = load_template("word-extract-excel.json");
    let steps = tmpl["steps"].as_array().expect("steps 应为数组");

    assert_eq!(steps.len(), 2, "模板4应有2个步骤");
    assert_eq!(steps[0]["type"].as_str().unwrap(), "word");
    assert_eq!(steps[1]["type"].as_str().unwrap(), "logic");

    let actions0 = steps[0]["actions"].as_array().unwrap();
    assert_eq!(actions0[0]["type"].as_str().unwrap(), "read");

    // Parser 转换
    let yaml = serde_json::to_string(&tmpl).unwrap();
    let wf = parser::parse_workflow(&yaml).unwrap();
    assert_eq!(wf.steps[0].step_type, "word_container");
    assert_eq!(wf.steps[1].step_type, "logic_container");

    println!("✅ 模板4 结构验证通过: word_container → logic_container");
}
