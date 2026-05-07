// tests/integration_test.rs — 端到端集成测试
// 使用真实的 StepExecutor + NodeExecutor trait 接口
use serde_json::json;
use workflow_engine::engine::workflow::Step;
use workflow_engine::engine::context::ExecutionContext;
use workflow_engine::engine::executor::StepExecutor;

/// 辅助：检查测试数据文件是否存在（相对于 src-tauri 的 examples/ 在 ../examples/）
fn test_data(path: &str) -> String { format!("../examples/{}", path) }

/// 辅助函数：构建最小 Step
fn make_step(id: &str, name: &str, step_type: &str, config: serde_json::Value) -> Step {
    Step {
        id: id.to_string(),
        name: name.to_string(),
        step_type: step_type.to_string(),
        config,
        next: None,
        retry: None,
        timeout: None,
        body_steps: None,
        breakpoint: false,
        delay: None,
        on_error: None,
        actions: None,
        expanded: None,
        condition: None,
        condition_group: None,
        then_steps: None,
        else_steps: None,
        run_condition: None,
    }
}

// ═══════════════════════════════════════════════════
// Excel 节点测试
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_excel_read() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-excel-read", &Default::default());
    let step = make_step("excel1", "读取Excel", "excel", json!({
        "action": "read",
        "path": test_data("test_data.xlsx"),
        "sheet": "数据"
    }));

    let result = executor.execute(&step, &mut ctx).await;
    assert!(result.is_ok(), "Excel read failed: {:?}", result.err());
    let val = result.unwrap();
    let data = val["data"].as_array().expect("data should be array");
    assert!(data.len() >= 3, "Should have at least 3 rows, got {}", data.len());
    println!("✅ Excel read OK: {} rows", data.len());
}

#[tokio::test]
async fn test_excel_sheets() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-excel-sheets", &Default::default());
    let step = make_step("excel2", "列出工作表", "excel", json!({
        "action": "sheets",
        "path": test_data("test_data.xlsx")
    }));

    let result = executor.execute(&step, &mut ctx).await;
    assert!(result.is_ok(), "Excel sheets failed: {:?}", result.err());
    let val = result.unwrap();
    let sheets = val["sheets"].as_array().expect("sheets should be array");
    assert!(!sheets.is_empty(), "Should have at least 1 sheet");
    println!("✅ Excel sheets OK: {:?}", sheets);
}

// ═══════════════════════════════════════════════════
// Word 节点测试
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_word_replace() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-word-replace", &Default::default());

    let placeholders = json!({
        "{{DATE}}": "2026-04-22",
        "{{TOTAL}}": "5",
        "{{ROW1_NUM}}": "1",
        "{{ROW1_KEYWORD}}": "Python编程",
        "{{ROW1_RESULT}}": "编程语言 | 8字符",
    });
    ctx.set_var("placeholders".to_string(), placeholders);

    let step = make_step("word1", "替换Word", "word", json!({
        "action": "replace",
        "path": test_data("report_template.docx"),
        "output": test_data("test_report_output.docx"),
        "replacements": "{{placeholders}}"
    }));

    let result = executor.execute(&step, &mut ctx).await;
    assert!(result.is_ok(), "Word replace failed: {:?}", result.err());
    println!("✅ Word replace OK: {:?}", result.unwrap()["output"]);
}

// ═══════════════════════════════════════════════════
// 数据节点测试
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_variable_set_get() {
    let mut ctx = ExecutionContext::new("test-data", &Default::default());

    // v4.1: data 节点已移除，用 context 直接操作变量
    ctx.set_var("greeting".to_string(), json!("Hello, 小夏!"));
    assert_eq!(ctx.variables.get("greeting").and_then(|v| v.as_str()), Some("Hello, 小夏!"));
    println!("✅ Variable set/get OK");
}

// ═══════════════════════════════════════════════════
// 条件节点测试
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_logic_equals() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-logic-eq", &Default::default());

    ctx.set_var("a".to_string(), json!(42));

    // v4.1: logic_container 用 actions 格式
    let step = make_step("lc1", "判断等于", "logic_container", json!({
        "value": "{{a}}",
        "actions": [
            {"id": "l1", "type": "equals", "label": "等于42", "config": {"right": 42}}
        ]
    }));

    let result = executor.execute(&step, &mut ctx).await.unwrap();
    assert_eq!(result["branch"].as_str(), Some("true"));
    println!("✅ Logic equals OK: {:?}", result);
}

#[tokio::test]
async fn test_logic_not_empty() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-logic-ne", &Default::default());

    ctx.set_var("x".to_string(), json!("hello"));

    let step = make_step("lc2", "判断不为空", "logic_container", json!({
        "value": "{{x}}",
        "actions": [
            {"id": "l1", "type": "not_empty", "label": "不为空", "config": {}}
        ]
    }));

    let result = executor.execute(&step, &mut ctx).await.unwrap();
    assert_eq!(result["branch"].as_str(), Some("true"));
    println!("✅ Logic not_empty OK: {:?}", result);
}

// ═══════════════════════════════════════════════════
// 脚本节点测试
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_script_arithmetic() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-script", &Default::default());

    ctx.set_var("x".to_string(), json!(10));
    ctx.set_var("y".to_string(), json!(3));

    let step = make_step("script1", "计算", "script", json!({
        "script": "x + y * 2"
    }));

    let result = executor.execute(&step, &mut ctx).await.unwrap();
    // 10 + 3*2 = 16
    assert_eq!(result.as_i64(), Some(16));
    println!("✅ Script arithmetic OK: {}", result);
}

// ═══════════════════════════════════════════════════
// 循环节点测试 (for-each)
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_loop_simple() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-loop", &Default::default());

    let step = make_step("loop1", "遍历数组", "loop", json!({
        "items": [1, 2, 3, 4, 5],
        "body": [{
            "id": "body_step",
            "name": "处理每个元素",
            "type": "script",
            "config": { "script": "__item * 2" }
        }]
    }));

    let result = executor.execute(&step, &mut ctx).await.unwrap();
    assert_eq!(result["count"].as_u64(), Some(5));
    // 每个结果应该是 __item * 2: [2, 4, 6, 8, 10]
    let results = result["results"].as_array().unwrap();
    assert_eq!(results.len(), 5);
    println!("✅ Loop simple OK: {} iterations", results.len());
}

#[tokio::test]
async fn test_loop_collect() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-loop-collect", &Default::default());

    let step = make_step("loop2", "遍历+收集", "loop", json!({
        "items": [
            {"name": "张三", "age": 25},
            {"name": "李四", "age": 30},
            {"name": "王五", "age": 35},
        ],
        "body": [{
            "id": "body_step",
            "name": "年龄翻倍",
            "type": "script",
            "config": { "script": "__item.age * 2" }
        }],
        "collect": {
            "ages": "__item.age",
            "doubled": "body_step"
        }
    }));

    let result = executor.execute(&step, &mut ctx).await.unwrap();
    assert_eq!(result["count"].as_u64(), Some(3));
    let collected = result["collected"].as_object().unwrap();
    let ages = collected["ages"].as_array().unwrap();
    let doubled = collected["doubled"].as_array().unwrap();
    assert_eq!(ages.len(), 3);
    assert_eq!(doubled[0].as_i64(), Some(50)); // body_step = __item.age * 2 = 25*2=50
    assert_eq!(doubled[1].as_i64(), Some(60));
    assert_eq!(doubled[2].as_i64(), Some(70));
    println!("✅ Loop collect OK: ages={:?}, doubled={:?}", ages, doubled);
}

// ═══════════════════════════════════════════════════
// While 循环节点测试
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_while_stops_on_empty() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-while", &Default::default());

    let step = make_step("while1", "While遍历", "while", json!({
        "items": ["hello", "world", "", "should_not_reach", ""],
        "condition": { "op": "not_empty" },
        "body": [{
            "id": "upper",
            "name": "转大写",
            "type": "script",
            "config": { "script": "__item.to_upper()" }
        }]
    }));

    let result = executor.execute(&step, &mut ctx).await.unwrap();
    assert_eq!(result["count"].as_u64(), Some(2));
    assert_eq!(result["stopped_at"].as_u64(), Some(2));
    println!("✅ While stops on empty OK: {} iterations", result["count"]);
}

// ═══════════════════════════════════════════════════
// 并行节点测试
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_parallel_branches() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-parallel", &Default::default());

    let step = make_step("par1", "并行执行", "parallel", json!({
        "branches": [
            [{
                "id": "branch_a_step",
                "name": "分支A",
                "type": "script",
                "config": { "script": "100 + 1" }
            }],
            [{
                "id": "branch_b_step",
                "name": "分支B",
                "type": "script",
                "config": { "script": "200 + 2" }
            }],
            [{
                "id": "branch_c_step",
                "name": "分支C",
                "type": "script",
                "config": { "script": "300 + 3" }
            }]
        ]
    }));

    let result = executor.execute(&step, &mut ctx).await.unwrap();
    assert_eq!(result["branch_count"].as_u64(), Some(3));
    let results = result["results"].as_array().unwrap();
    assert_eq!(results.len(), 3);

    // 所有分支都应成功
    for r in results {
        assert_eq!(r["success"].as_bool(), Some(true),
            "Branch failed: {:?}", r.get("error"));
    }
    println!("✅ Parallel branches OK: {} branches completed", results.len());
}

// ═══════════════════════════════════════════════════
// HTTP 节点测试 (需要网络)
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_http_get() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-http", &Default::default());

    let step = make_step("http1", "HTTP请求", "http", json!({
        "action": "GET",
        "url": "https://httpbin.org/get"
    }));

    let result = executor.execute(&step, &mut ctx).await;
    match result {
        Ok(val) => {
            assert!(val["status"].as_u64().unwrap_or(0) == 200,
                "Expected 200, got {:?}", val["status"]);
            println!("✅ HTTP GET OK: status={}", val["status"]);
        }
        Err(e) => {
            println!("⚠ HTTP test skipped (no network?): {}", e);
        }
    }
}

// ═══════════════════════════════════════════════════
// Parser 测试
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_parse_valid_yaml() {
    use workflow_engine::engine::parser::parse_workflow;
    let yaml = r#"
name: 测试工作流
description: 单元测试
steps:
  - id: step_001
    name: 第一步
    type: http
    config:
      action: GET
      url: https://example.com
"#;
    let wf = parse_workflow(yaml).unwrap();
    assert_eq!(wf.name, "测试工作流");
    assert_eq!(wf.steps.len(), 1);
    assert_eq!(wf.steps[0].id, "step_001");
    println!("✅ Parse valid YAML OK");
}

#[test]
fn test_parse_duplicate_ids() {
    use workflow_engine::engine::parser::parse_workflow;
    let yaml = r#"
name: 重复ID测试
steps:
  - id: step_001
    name: A
    type: http
    config: {}
  - id: step_001
    name: B
    type: http
    config: {}
"#;
    let result = parse_workflow(yaml);
    assert!(result.is_err(), "Should reject duplicate IDs");
    println!("✅ Duplicate ID detection OK: {}", result.unwrap_err());
}

#[test]
fn test_parse_empty_steps() {
    use workflow_engine::engine::parser::parse_workflow;
    let yaml = "name: 空工作流\nsteps: []";
    let result = parse_workflow(yaml);
    assert!(result.is_err(), "Should reject empty steps");
    println!("✅ Empty steps detection OK");
}

// ═══════════════════════════════════════════════════
// Context 变量替换测试
// ═══════════════════════════════════════════════════

#[test]
fn test_resolve_simple_variable() {
    let mut ctx = ExecutionContext::new("test-resolve", &Default::default());
    ctx.set_var("name".to_string(), json!("小夏"));

    let result = ctx.resolve_config(&json!("Hello, {{name}}!"));
    assert_eq!(result.as_str(), Some("Hello, 小夏!"));
    println!("✅ Variable resolve OK");
}

#[test]
fn test_resolve_nested_variable() {
    let mut ctx = ExecutionContext::new("test-nested", &Default::default());
    ctx.set_var("user".to_string(), json!({"name": "小夏", "age": 30}));

    let result = ctx.resolve_config(&json!("{{user.name}}"));
    assert_eq!(result.as_str(), Some("小夏"));
    println!("✅ Nested variable resolve OK");
}

#[test]
fn test_resolve_keeps_type() {
    let mut ctx = ExecutionContext::new("test-type", &Default::default());
    ctx.set_var("count".to_string(), json!(42));

    // 整个值是单个 {{count}} → 保留整数类型
    let result = ctx.resolve_config(&json!("{{count}}"));
    assert_eq!(result.as_i64(), Some(42));
    println!("✅ Type preservation OK");
}

// ═══════════════════════════════════════════════════
// 完整管道测试
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_full_pipeline() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("pipeline", &Default::default());

    // Step 1: 读取 Excel
    let step1 = make_step("read_excel", "读取数据", "excel", json!({
        "action": "read",
        "path": test_data("test_data.xlsx"),
        "sheet": "数据"
    }));
    let r1 = executor.execute(&step1, &mut ctx).await.unwrap();
    ctx.set_output("read_excel", r1.clone());
    let data = r1["data"].as_array().unwrap();
    println!("[1/3] Read Excel: {} rows", data.len());

    // Step 2: 循环处理 — 给每行加索引
    let step2 = make_step("loop_data", "处理数据", "loop", json!({
        "items": data,
        "body": [{
            "id": "fmt",
            "name": "格式化",
            "type": "script",
            "config": { "script": "`#${__index1} ` + __item[0]" }
        }],
        "collect": {
            "labels": "fmt"
        }
    }));
    let r2 = executor.execute(&step2, &mut ctx).await.unwrap();
    ctx.set_output("loop_data", r2.clone());
    let labels = r2["collected"]["labels"].as_array().unwrap();
    println!("[2/3] Processed {} items", labels.len());
    assert!(!labels.is_empty(), "Should have processed items");

    // Step 3: 保存结果（v4.1: data 节点已移除）
    ctx.set_var("final_count".to_string(), json!(labels.len() as i64));
    println!("[3/3] Saved final count: {}", ctx.variables.get("final_count").unwrap());

    println!("\n✅ Full pipeline completed!");
}

// ═══════════════════════════════════════════════════
// 简易测试（无需外部文件/网络）
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_map_node() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-map", &Default::default());

    let step = make_step("map1", "声明式映射", "map", json!({
        "source": [1, 2, 3],
        "template": { "value": "{{__item}}", "doubled": "{{__item * 2}}" }
    }));

    let result = executor.execute(&step, &mut ctx).await.unwrap();
    // map 节点直接返回映射结果数组
    let data = result.as_array().unwrap();
    assert_eq!(data.len(), 3);
    assert_eq!(data[0]["doubled"].as_i64(), Some(2));
    assert_eq!(data[2]["doubled"].as_i64(), Some(6));
    println!("✅ Map node OK: {} items", data.len());
}

#[tokio::test]
async fn test_map_node_logic_operators() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-map-logic", &Default::default());

    let step = make_step("map_logic", "逻辑运算测试", "map", json!({
        "source": [1, 2, 3, 5],
        "template": {
            "gt_1":     "{{__item > 1}}",
            "lt_3":     "{{__item < 3}}",
            "eq_2":     "{{__item == 2}}",
            "ne_2":     "{{__item != 2}}",
            "gte_3":    "{{__item >= 3}}",
            "lte_2":    "{{__item <= 2}}",
            "between":  "{{__item > 1 && __item < 5}}",
            "extreme":  "{{__item < 2 || __item > 4}}",
            "not_3":    "{{! (__item == 3)}}",
            "combo":    "{{__item > 1 && __item <= 5 && __item != 3}}"
        }
    }));

    let result = executor.execute(&step, &mut ctx).await.unwrap();
    let data = result.as_array().unwrap();
    assert_eq!(data.len(), 4);

    // [1]: gt_1=false, lt_3=true, extreme=true, not_3=true
    assert_eq!(data[0]["gt_1"].as_bool(), Some(false));
    assert_eq!(data[0]["lt_3"].as_bool(), Some(true));
    assert_eq!(data[0]["extreme"].as_bool(), Some(true));
    assert_eq!(data[0]["not_3"].as_bool(), Some(true));

    // [3]: gte_3=true, between=true, extreme=false, not_3=false
    assert_eq!(data[2]["gte_3"].as_bool(), Some(true));
    assert_eq!(data[2]["between"].as_bool(), Some(true));
    assert_eq!(data[2]["extreme"].as_bool(), Some(false));
    assert_eq!(data[2]["not_3"].as_bool(), Some(false));

    // [5]: gt_1=true, lt_3=false, between=false, extreme=true, combo=true
    assert_eq!(data[3]["gt_1"].as_bool(), Some(true));
    assert_eq!(data[3]["lt_3"].as_bool(), Some(false));
    assert_eq!(data[3]["between"].as_bool(), Some(false));
    assert_eq!(data[3]["extreme"].as_bool(), Some(true));
    assert_eq!(data[3]["combo"].as_bool(), Some(true));

    println!("✅ Map logic operators OK: {} items", data.len());
}

// ═══════════════════════════════════════════════════
// 延时节点测试
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_delay_node() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-delay", &Default::default());

    let step = make_step("delay1", "延时100ms", "delay", json!({
        "duration_ms": 100,
    }));

    let start = std::time::Instant::now();
    let result = executor.execute(&step, &mut ctx).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(result["duration_ms"].as_u64(), Some(100));
    assert!(elapsed.as_millis() >= 80, "延时至少 80ms，实际 {}ms", elapsed.as_millis());
    println!("✅ Delay node OK: {}ms (real: {}ms)", result["duration_ms"], elapsed.as_millis());
}

#[tokio::test]
async fn test_delay_node_max_limit() {
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("test-delay-max", &Default::default());

    let step = make_step("delay2", "延时超限", "delay", json!({
        "duration_ms": 500_000,
        "max_duration_ms": 1000,
    }));

    let result = executor.execute(&step, &mut ctx).await;
    assert!(result.is_err(), "超过 max_duration_ms 应返回错误");
    println!("✅ Delay max limit OK: {}", result.unwrap_err());
}
