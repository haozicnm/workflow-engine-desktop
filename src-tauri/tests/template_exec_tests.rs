// tests/template_exec_tests.rs — 模板端到端执行测试
use serde_json::json;
use std::env;
use workflow_engine::engine::parser;
use workflow_engine::engine::workflow::Step;
use workflow_engine::engine::context::ExecutionContext;
use workflow_engine::engine::executor::StepExecutor;

/// 加载模板并替换相对路径为绝对路径
fn parse_template(template_file: &str) -> Vec<Step> {
    // Test runs from src-tauri/, templates are at ../templates/
    let path = format!("../templates/{}", template_file);
    let content = std::fs::read_to_string(&path)
        .expect(&format!("模板文件不存在: {}", path));

    // 替换相对路径（templates/data/ → 绝对路径）
    let project_root = env::current_dir()
        .unwrap()
        .parent()  // src-tauri/ → project root
        .unwrap()
        .to_path_buf();
    let data_dir = project_root.join("templates").join("data");
    let content = content.replace(
        "templates/data/",
        &format!("{}/", data_dir.to_string_lossy()),
    );

    let wf = parser::parse_workflow(&content)
        .expect(&format!("Parser 转换失败: {}", template_file));
    wf.steps
}

// ═══════════════════════════════════════════════════
// 模板 1 执行: order-to-contracts
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_exec_template1_order_to_contracts() {
    let steps = parse_template("order-to-contracts.json");
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("tmpl1", &Default::default());

    // Step 1: excel_container → read orders
    println!("=== Step 1: Excel 读取订单数据 ===");
    let r1 = executor.execute(&steps[0], &mut ctx).await.unwrap();
    ctx.set_output(&steps[0].id, r1.clone());
    println!("  Keys: {:?}", r1.as_object().map(|o| o.keys().collect::<Vec<_>>()));

    let read_data = r1.get("read").and_then(|v| v.get("data")).and_then(|v| v.as_array());
    assert!(read_data.is_some(), "Excel read 应产出 data 数组");
    let rows = read_data.unwrap();
    println!("  读取 {} 行", rows.len());
    assert!(rows.len() >= 4, "至少 4 行数据");

    // Step 2: cursor → iterate
    println!("\n=== Step 2: Cursor 游标迭代 ===");
    let r2 = executor.execute(&steps[1], &mut ctx).await.unwrap();
    let done = r2.get("done").and_then(|v| v.as_bool()).unwrap_or(true);
    let index = r2.get("index").and_then(|v| v.as_i64()).unwrap_or(-1);
    let total = r2.get("total").and_then(|v| v.as_i64()).unwrap_or(0);
    println!("  done={}, index={}, total={}", done, index, total);
    assert!(!done, "第1次应 done=false");
    assert_eq!(index, 0);
    assert_eq!(total, rows.len() as i64);

    // Cursor 迭代剩余行
    for i in 1..rows.len() {
        let r = executor.execute(&steps[1], &mut ctx).await.unwrap();
        let idx = r.get("index").and_then(|v| v.as_i64()).unwrap_or(-1);
        println!("  迭代{}: index={}", i + 1, idx);
        assert_eq!(idx, i as i64);
    }

    // 最后一行后应返回 done
    let r_done = executor.execute(&steps[1], &mut ctx).await.unwrap();
    assert!(r_done.get("done").and_then(|v| v.as_bool()).unwrap_or(false), "游标耗尽应 done=true");

    println!("\n✅ 模板1 游标迭代验证通过");
}

// ═══════════════════════════════════════════════════
// 模板 2 执行: monitor-to-report
// ═══════════════════════════════════════════════════

#[tokio::test]
async fn test_exec_template2_logic_routing() {
    let steps = parse_template("monitor-to-report.json");
    let executor = StepExecutor::new();
    let mut ctx = ExecutionContext::new("tmpl2", &Default::default());

    // 模拟 browser 输出（含"异常"关键字 → branch=true）
    println!("=== 模拟 Browser 输出 ===");
    ctx.set_output("step_browser", json!({
        "navigate": {"url": "file:///status-page.html"},
        "evaluate": "系统监控 | 系统运行状态 异常 响应时间: 120ms"
    }));
    println!("  page contains '异常' → expected branch=true");

    // Step 1: logic_container
    println!("\n=== Step 1: Logic 条件判断 ===");
    let r1 = executor.execute(&steps[1], &mut ctx).await.unwrap();
    ctx.set_output(&steps[1].id, r1.clone());
    println!("  output: {}", r1);

    let branch = r1.get("branch").and_then(|v| v.as_str()).unwrap_or("?");
    let result = r1.get("result").and_then(|v| v.as_bool()).unwrap_or(false);
    println!("  branch={}, result={}", branch, result);
    assert_eq!(branch, "true");
    assert!(result);

    // Step 2: excel_container (runCondition: when=true → executes)
    println!("\n=== Step 2: Excel 异常报告 ===");
    let r2 = executor.execute(&steps[2], &mut ctx).await;
    assert!(r2.is_ok(), "Excel 异常报告应执行成功（create+append 动作不产 output port）");
    println!("  执行成功");

    // Step 3: notify (runCondition: when=true)
    println!("\n=== Step 3: Notify ===");
    let r3 = executor.execute(&steps[3], &mut ctx).await.unwrap();
    println!("  output: {}", r3);

    // Step 4: word (runCondition: when=false → skipped by scheduler)
    println!("\n=== Step 4: Word (正常报告分支) ===");
    let r4 = executor.execute(&steps[4], &mut ctx).await;
    assert!(r4.is_ok(), "Word 正常报告应可执行（runCondition 由 scheduler 层处理）");
    println!("  执行成功");

    println!("\n✅ 模板2 条件路由验证通过");
}
