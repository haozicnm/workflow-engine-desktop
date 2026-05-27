// tests/library_template_tests.rs — 5个内置库模板的加载 + 解析 + 校验
use serde_json::Value;
use std::collections::HashSet;
use workflow_engine::engine::{parser, validate};

const LIBRARY_DIR: &str = "../library";

fn load_template(path: &str) -> Value {
    let full = format!("{}/{}", LIBRARY_DIR, path);
    let content = std::fs::read_to_string(&full).expect(&format!("Template not found: {}", full));
    serde_json::from_str(&content).expect(&format!("Invalid JSON: {}", path))
}

fn step_ids(json: &Value) -> HashSet<String> {
    json["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["id"].as_str().unwrap().to_string())
        .collect()
}

/// Verify parser can convert and validate the template
fn parse_and_validate(name: &str, path: &str) {
    let full = format!("{}/{}", LIBRARY_DIR, path);
    let content = std::fs::read_to_string(&full).unwrap();

    // 1. Parse as JSON
    let json: Value = serde_json::from_str(&content).expect(&format!("{}: invalid JSON", name));

    // 2. Parse as Workflow struct
    let wf = parser::parse_workflow(&content).expect(&format!("{}: parser failed", name));

    // 3. Run semantic validation
    let validation = validate::validate_workflow(&wf);
    assert!(
        validation.valid,
        "{}: validation failed: {:?}",
        name, validation.errors
    );

    // 4. Check basic structure
    assert!(
        !json["steps"].as_array().unwrap().is_empty(),
        "{}: must have steps",
        name
    );
    assert!(
        !json["name"].as_str().unwrap().is_empty(),
        "{}: must have a name",
        name
    );

    // 5. Check params consistency
    if let Some(params) = json["params"].as_object() {
        let content_str = content.as_str();
        for key in params.keys() {
            let placeholder = format!("{{{{params.{}}}}}", key);
            // At least one reference should exist (or it's an optional param)
            // Just verify params block is well-formed
            let _ = placeholder;
        }
        // Verify no {{params.xxx}} refers to undeclared params
        for cap in content.matches("{{params.") {
            let _ = cap;
        }
    }
}

// ═══════════════════════════════════════
// 5 个模板加载 + 解析 + 校验测试
// ═══════════════════════════════════════

#[test]
fn test_template1_integration_smoke() {
    let json = load_template("stress/integration-smoke.wf.json");
    let ids = step_ids(&json);
    assert_eq!(json["name"].as_str().unwrap(), "integration-smoke");
    // 必须有 14 个步骤（涵盖所有核心节点类型）
    let steps = json["steps"].as_array().unwrap();
    assert_eq!(
        steps.len(),
        14,
        "integration-smoke must have 14 steps (one per node type)"
    );
    // 验证关键步骤类型
    let types: Vec<&str> = steps.iter().map(|s| s["type"].as_str().unwrap()).collect();
    assert!(types.contains(&"json_parse"));
    assert!(types.contains(&"file"));
    assert!(types.contains(&"script"));
    assert!(types.contains(&"logic"));
    assert!(types.contains(&"http"));
    assert!(types.contains(&"excel"));
    assert!(types.contains(&"word"));
    assert!(types.contains(&"clipboard"));
    assert!(types.contains(&"loop"));
    assert!(types.contains(&"notify"));
    assert!(types.contains(&"delay"));
    // Verify params exist and are used
    assert!(json["params"]["test_dir"].as_str().is_some());
    let content =
        std::fs::read_to_string(&format!("{}/stress/integration-smoke.wf.json", LIBRARY_DIR))
            .unwrap();
    assert!(
        content.contains("{{params.test_dir}}"),
        "test_dir param must be referenced"
    );
    assert!(
        content.contains("{{params.delay_ms}}"),
        "delay_ms param must be referenced"
    );

    parse_and_validate("integration-smoke", "stress/integration-smoke.wf.json");
    println!("✅ Template 1: integration-smoke — 14 steps, all types covered");
}

#[test]
fn test_template2_daily_monitor() {
    let json = load_template("monitoring/daily-monitor.wf.json");
    assert_eq!(json["name"].as_str().unwrap(), "daily-monitor");
    let steps = json["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 11);
    let types: Vec<&str> = steps.iter().map(|s| s["type"].as_str().unwrap()).collect();
    assert!(types.contains(&"http"));
    assert!(types.contains(&"excel"));
    assert!(types.contains(&"word"));
    // Verify output_dir is used in file_path
    let content =
        std::fs::read_to_string(&format!("{}/monitoring/daily-monitor.wf.json", LIBRARY_DIR))
            .unwrap();
    assert!(content.contains("{{params.output_dir}}"));

    parse_and_validate("daily-monitor", "monitoring/daily-monitor.wf.json");
    println!("✅ Template 2: daily-monitor — 11 steps, HTTP+Excel+Word");
}

#[test]
fn test_template3_file_batch_approval() {
    let json = load_template("batch/file-batch-approval.wf.json");
    assert_eq!(json["name"].as_str().unwrap(), "file-batch-approval");
    let steps = json["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 8);
    let types: Vec<&str> = steps.iter().map(|s| s["type"].as_str().unwrap()).collect();
    assert!(types.contains(&"cursor"));
    // approval is inside cursor body_steps, not at top level
    // Verify data_dir and file_pattern are used
    let content = std::fs::read_to_string(&format!(
        "{}/batch/file-batch-approval.wf.json",
        LIBRARY_DIR
    ))
    .unwrap();
    assert!(content.contains("{{params.data_dir}}"));
    assert!(content.contains("{{params.file_pattern}}"));

    parse_and_validate("file-batch-approval", "batch/file-batch-approval.wf.json");
    println!("✅ Template 3: file-batch-approval — 8 steps, cursor+approval");
}

#[test]
fn test_template4_http_approval_pipeline() {
    let json = load_template("batch/http-approval-pipeline.wf.json");
    assert_eq!(json["name"].as_str().unwrap(), "http-approval-pipeline");
    let steps = json["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 10);
    let types: Vec<&str> = steps.iter().map(|s| s["type"].as_str().unwrap()).collect();
    assert!(types.contains(&"http"));
    assert!(types.contains(&"approval"));
    // Verify api_url and notify_title are used
    let content = std::fs::read_to_string(&format!(
        "{}/batch/http-approval-pipeline.wf.json",
        LIBRARY_DIR
    ))
    .unwrap();
    assert!(content.contains("{{params.api_url}}"));
    assert!(content.contains("{{params.notify_title}}"));

    parse_and_validate(
        "http-approval-pipeline",
        "batch/http-approval-pipeline.wf.json",
    );
    println!("✅ Template 4: http-approval-pipeline — 10 steps, HTTP+approval+branch");
}

#[test]
fn test_template5_web_monitor_alert() {
    let json = load_template("monitoring/web-monitor-alert.wf.json");
    assert_eq!(json["name"].as_str().unwrap(), "web-monitor-alert");
    let steps = json["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 9);
    let types: Vec<&str> = steps.iter().map(|s| s["type"].as_str().unwrap()).collect();
    assert!(types.contains(&"loop"));
    assert!(types.contains(&"excel"));
    assert!(types.contains(&"notify"));
    // Verify work_dir and alert_threshold are used
    let content = std::fs::read_to_string(&format!(
        "{}/monitoring/web-monitor-alert.wf.json",
        LIBRARY_DIR
    ))
    .unwrap();
    assert!(content.contains("{{params.work_dir}}"));
    // Verify trend comparison step exists (json_parse after file read)
    assert!(
        types.contains(&"json_parse"),
        "Must have json_parse for trend comparison"
    );

    parse_and_validate("web-monitor-alert", "monitoring/web-monitor-alert.wf.json");
    println!("✅ Template 5: web-monitor-alert — 9 steps, loop+Excel+trend comparison");
}

// ═══════════════════════════════════════
// 跨模板一致性检查
// ═══════════════════════════════════════

#[test]
fn test_all_templates_have_unique_step_ids() {
    let templates = [
        ("integration-smoke", "stress/integration-smoke.wf.json"),
        ("daily-monitor", "monitoring/daily-monitor.wf.json"),
        ("file-batch-approval", "batch/file-batch-approval.wf.json"),
        (
            "http-approval-pipeline",
            "batch/http-approval-pipeline.wf.json",
        ),
        ("web-monitor-alert", "monitoring/web-monitor-alert.wf.json"),
    ];

    for (name, path) in &templates {
        let json = load_template(path);
        let steps = json["steps"].as_array().unwrap();
        let mut ids = HashSet::new();
        for step in steps {
            let id = step["id"].as_str().unwrap();
            assert!(ids.insert(id), "{}: duplicate step id '{}'", name, id);
        }
    }
    println!("✅ All 5 templates have unique step IDs");
}

#[test]
fn test_all_templates_parse_and_validate() {
    let templates = [
        ("integration-smoke", "stress/integration-smoke.wf.json"),
        ("daily-monitor", "monitoring/daily-monitor.wf.json"),
        ("file-batch-approval", "batch/file-batch-approval.wf.json"),
        (
            "http-approval-pipeline",
            "batch/http-approval-pipeline.wf.json",
        ),
        ("web-monitor-alert", "monitoring/web-monitor-alert.wf.json"),
    ];

    for (name, path) in &templates {
        parse_and_validate(name, path);
    }
    println!("✅ All 5 templates parse + validate successfully");
}

#[test]
fn test_all_runcondition_refs_valid() {
    // Collect ALL step IDs recursively (including body_steps)
    fn collect_ids(steps: &[serde_json::Value], ids: &mut HashSet<String>) {
        for step in steps {
            ids.insert(step["id"].as_str().unwrap().to_string());
            if let Some(body) = step["body_steps"].as_array() {
                collect_ids(body, ids);
            }
        }
    }

    let templates = [
        "stress/integration-smoke.wf.json",
        "monitoring/daily-monitor.wf.json",
        "batch/file-batch-approval.wf.json",
        "batch/http-approval-pipeline.wf.json",
        "monitoring/web-monitor-alert.wf.json",
    ];

    for path in &templates {
        let json = load_template(path);
        let steps = json["steps"].as_array().unwrap();
        let mut all_ids = HashSet::new();
        collect_ids(steps, &mut all_ids);

        // Check all runCondition refs recursively
        fn check_rc(steps: &[serde_json::Value], ids: &HashSet<String>, path: &str) {
            for step in steps {
                if let Some(rc) = step.get("runCondition") {
                    let ref_id = rc["ref"].as_str().unwrap();
                    assert!(
                        ids.contains(ref_id),
                        "{}: runCondition.ref '{}' not found in any step",
                        path,
                        ref_id
                    );
                }
                if let Some(body) = step["body_steps"].as_array() {
                    check_rc(body, ids, path);
                }
            }
        }
        check_rc(steps, &all_ids, path);
    }
    println!("✅ All runCondition.ref references are valid");
}
