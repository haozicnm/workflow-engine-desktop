// tests/variable_flow_tests.rs — 变量流转集成测试
//
// 目标：验证变量在节点间的正确传递，而非测试节点内部逻辑。
// 每个测试 = 一条变量流转链路 + 断言。

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use workflow_engine::engine::context::ExecutionContext;
use workflow_engine::engine::executor::StepExecutor;
use workflow_engine::engine::workflow::{Step, Workflow};

// ═══════════════════════════════════════════════════════════════
// 测试基础设施
// ═══════════════════════════════════════════════════════════════

struct TestChain {
    steps: Vec<(String, String, Value)>,
    inputs: HashMap<String, HashMap<String, Value>>,
    variables: HashMap<String, Value>,
    workflow: Workflow,
}

impl TestChain {
    fn new() -> Self {
        Self {
            steps: Vec::new(),
            inputs: HashMap::new(),
            variables: HashMap::new(),
            workflow: Default::default(),
        }
    }

    fn step(mut self, id: &str, step_type: &str, config: Value) -> Self {
        self.steps
            .push((id.to_string(), step_type.to_string(), config));
        self
    }

    /// 模拟 DAG 入边：在执行指定步骤前解析并注入输入端口。
    fn input(mut self, step_id: &str, port: &str, value: Value) -> Self {
        self.inputs
            .entry(step_id.to_string())
            .or_default()
            .insert(port.to_string(), value);
        self
    }

    fn var(mut self, key: &str, value: Value) -> Self {
        self.variables.insert(key.to_string(), value);
        self
    }

    async fn run(self) -> TestResult {
        let executor = StepExecutor::new(
            Arc::new(workflow_engine::engine::approval_store::ApprovalStore::new()),
            Arc::new(workflow_engine::data::db::Database::open_temp().unwrap()),
        );
        let mut workflow = self.workflow;
        workflow.variables = Some(self.variables);
        let mut ctx = ExecutionContext::new("test-chain", &workflow);

        let mut outputs = HashMap::new();
        for (id, step_type, config) in &self.steps {
            let step = make_chain_step(id, "test", step_type, config);
            let on_error = step.on_error.clone();
            ctx.input_ports.clear();
            if let Some(inputs) = self.inputs.get(id) {
                for (port, value) in inputs {
                    ctx.input_ports
                        .insert(port.clone(), ctx.resolve_config(value));
                }
            }

            let result = executor.execute(&step, &mut ctx).await;
            let should_stop = result.is_err()
                && !matches!(
                    on_error,
                    Some(workflow_engine::engine::workflow::ErrorStrategy::Ignore)
                );
            match &result {
                Ok(output) => {
                    ctx.set_output(id, output.clone());
                }
                Err(e) => {
                    // on_error: Ignore → 设置 Null 输出，继续执行
                    if matches!(
                        on_error,
                        Some(workflow_engine::engine::workflow::ErrorStrategy::Ignore)
                    ) {
                        eprintln!("Step '{}' failed (ignored): {:?}", id, e);
                        ctx.set_output(id, serde_json::Value::Null);
                        outputs.insert(id.clone(), Ok(serde_json::Value::Null));
                        continue;
                    }
                    eprintln!("Step '{}' failed: {:?}", id, e);
                }
            }
            outputs.insert(id.clone(), result);
            if should_stop {
                break;
            }
        }
        TestResult { outputs }
    }
}

struct TestResult {
    outputs: HashMap<String, Result<Value, anyhow::Error>>,
}

impl TestResult {
    fn output(&self, step_id: &str) -> &Value {
        self.outputs
            .get(step_id)
            .unwrap_or_else(|| panic!("Step '{}' not found", step_id))
            .as_ref()
            .unwrap_or_else(|e| panic!("Step '{}' failed: {:?}", step_id, e))
    }

    fn is_ok(&self, step_id: &str) -> bool {
        self.outputs
            .get(step_id)
            .map(|r| r.is_ok())
            .unwrap_or(false)
    }

    fn has_output(&self, step_id: &str) -> bool {
        self.outputs.contains_key(step_id)
    }
}

fn make_chain_step(id: &str, name: &str, step_type: &str, config: &Value) -> Step {
    let actions = config.get("actions").and_then(|a| a.as_array()).cloned();
    let condition_group = config
        .get("conditionGroup")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let body_steps = config
        .get("body_steps")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    // on_error 从 config 中提取
    let on_error = config
        .get("on_error")
        .and_then(|v| serde_json::from_value(v.clone()).ok());

    Step {
        id: id.to_string(),
        name: name.to_string(),
        step_type: step_type.to_string(),
        config: config.clone(),
        next: None,
        retry: None,
        timeout: None,
        body_steps,
        breakpoint: false,
        delay: None,
        on_error,
        actions,
        expanded: None,
        condition: None,
        condition_group,
        run_condition: None,
    }
}

fn fixture(path: &str) -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/tests/fixtures/{}", manifest_dir, path)
}

// ═══════════════════════════════════════════════════════════════
// P0 链路 1: script → excel
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_script_to_excel() {
    let out_dir = tempfile::tempdir().unwrap();
    let out_path = format!("{}/chain_test.xlsx", out_dir.path().display());

    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({
                "script": r#"
                    let rows = [
                        ["1", "标题A", "https://a.com"],
                        ["2", "标题B", "https://b.com"],
                        ["3", "标题C", "https://c.com"]
                    ];
                    let header = ["序号", "标题", "链接"];
                    let full = [header];
                    for row in rows { full.push(row); }
                    #{rows: full, count: rows.len}
                "#
            }),
        )
        .step(
            "step_2",
            "excel",
            json!({
                "file_path": out_path,
                "actions": [
                    {"id": "a1", "type": "write", "config": {"value": "{{step_1.rows}}"}}
                ]
            }),
        )
        .run()
        .await;

    let s1 = result.output("step_1");
    assert_eq!(s1["count"].as_i64().unwrap(), 3);
    assert_eq!(s1["rows"].as_array().unwrap().len(), 4);
    assert_eq!(s1["rows"][0][0].as_str().unwrap(), "序号");
    assert_eq!(s1["rows"][1][1].as_str().unwrap(), "标题A");

    let s2 = result.output("step_2");
    assert_eq!(s2["a1"]["rows_written"].as_i64().unwrap(), 4);
    assert!(std::path::Path::new(&out_path).exists());
}

// ═══════════════════════════════════════════════════════════════
// P0 链路 2: web_scrape → script → excel（回归测试）
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_web_scrape_to_script_to_excel() {
    let html_path = fixture("simple.html");
    let out_dir = tempfile::tempdir().unwrap();
    let out_path = format!("{}/scrape_chain.xlsx", out_dir.path().display());

    let result = TestChain::new()
        .step(
            "step_1",
            "web_scrape",
            json!({
                "url": format!("file://{}", html_path),
                "extract": [{
                    "selector": "ul.news-list li a",
                    "fields": {"title": "text()", "link": "[href]"}
                }]
            }),
        )
        .step(
            "step_2",
            "script",
            json!({
                "script": r#"
                    let items = step_1.items;
                    let header = ["序号", "标题", "链接"];
                    let full = [header];
                    for i in 0..items.len {
                        let item = items[i];
                        full.push([to_string(i + 1), item.title, item.link]);
                    }
                    #{rows: full, count: items.len}
                "#
            }),
        )
        .step(
            "step_3",
            "excel",
            json!({
                "file_path": out_path,
                "actions": [
                    {"id": "a1", "type": "write", "config": {"value": "{{step_2.rows}}"}}
                ]
            }),
        )
        .run()
        .await;

    let s1 = result.output("step_1");
    assert_eq!(s1["total_items"].as_i64().unwrap(), 3);
    assert_eq!(s1["items"][0]["title"].as_str().unwrap(), "第一条新闻标题");
    assert_eq!(
        s1["items"][0]["link"].as_str().unwrap(),
        "https://example.com/1"
    );

    let s2 = result.output("step_2");
    assert_eq!(s2["count"].as_i64().unwrap(), 3);
    assert_eq!(s2["rows"][1][1].as_str().unwrap(), "第一条新闻标题");

    let s3 = result.output("step_3");
    assert_eq!(s3["a1"]["rows_written"].as_i64().unwrap(), 4);
}

// ═══════════════════════════════════════════════════════════════
// P0 链路 3: script → loop → collect → script
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_script_to_loop_collect() {
    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({
                "script": r#"
                    let items = [
                        #{name: "Alice", score: 85},
                        #{name: "Bob", score: 92},
                        #{name: "Charlie", score: 55}
                    ];
                    #{items: items}
                "#
            }),
        )
        .step(
            "step_2",
            "loop",
            json!({
                "items": "{{step_1.items}}",
                "body": [
                    {
                        "id": "body_1",
                        "type": "script",
                        "config": {
                            "script": "let item = __item; #{name: item.name, score: item.score}"
                        }
                    }
                ],
                "collect": {"scores": "body_1.score", "names": "body_1.name"}
            }),
        )
        .step(
            "step_3",
            "script",
            json!({
                "script": r#"
                    let collected = step_2.collected;
                    let scores = collected.scores;
                    let total = 0;
                    for s in scores { total += s; }
                    let avg = total / scores.len;
                    #{avg: avg, count: scores.len, names: collected.names}
                "#
            }),
        )
        .run()
        .await;

    let s1 = result.output("step_1");
    assert_eq!(s1["items"].as_array().unwrap().len(), 3);

    let s2 = result.output("step_2");
    assert_eq!(s2["count"].as_i64().unwrap(), 3);

    let s3 = result.output("step_3");
    assert_eq!(s3["count"].as_i64().unwrap(), 3);
    assert_eq!(s3["avg"].as_i64().unwrap(), 77); // (85+92+55)/3 = 77
}

// ═══════════════════════════════════════════════════════════════
// P0 链路 4-5: condition 分支
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_condition_true_branch() {
    let result = TestChain::new()
        .step("step_1", "script", json!({"script": r#"#{score: 90}"#}))
        .step(
            "step_2",
            "condition",
            json!({
                "value": "{{step_1.score}}",
                "conditionGroup": {
                    "combinator": "and",
                    "conditions": [
                        {"id": "c1", "left": "{{step_1.score}}", "op": "gte", "right": "60"}
                    ]
                }
            }),
        )
        .step(
            "step_3",
            "script",
            json!({"script": r#"#{branch: step_2.branch}"#}),
        )
        .run()
        .await;

    let s2 = result.output("step_2");
    assert_eq!(s2["branch"].as_str().unwrap(), "true");
    assert_eq!(s2["result"].as_bool().unwrap(), true);

    let s3 = result.output("step_3");
    assert_eq!(s3["branch"].as_str().unwrap(), "true");
}

#[tokio::test]
async fn test_chain_condition_false_branch() {
    let result = TestChain::new()
        .step("step_1", "script", json!({"script": r#"#{score: 40}"#}))
        .step(
            "step_2",
            "condition",
            json!({
                "value": "{{step_1.score}}",
                "conditionGroup": {
                    "combinator": "and",
                    "conditions": [
                        {"id": "c1", "left": "{{step_1.score}}", "op": "gte", "right": "60"}
                    ]
                }
            }),
        )
        .run()
        .await;

    let s2 = result.output("step_2");
    assert_eq!(s2["branch"].as_str().unwrap(), "false");
    assert_eq!(s2["result"].as_bool().unwrap(), false);
}

// ═══════════════════════════════════════════════════════════════
// P0 链路 6: params 注入
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_params_injection() {
    let result = TestChain::new()
        .var("title", json!("测试标题"))
        .var("count", json!("3"))
        .step(
            "step_1",
            "script",
            json!({
                "script": r#"
                    let t = "{{params.title}}";
                    let n = parse_int("{{params.count}}");
                    let rows = [];
                    for i in 0..n { rows.push([to_string(i + 1), t]); }
                    #{rows: rows, count: n}
                "#
            }),
        )
        .run()
        .await;

    let s1 = result.output("step_1");
    assert_eq!(s1["count"].as_i64().unwrap(), 3);
    assert_eq!(s1["rows"][0][1].as_str().unwrap(), "测试标题");
}

// ═══════════════════════════════════════════════════════════════
// P0 节点: data_set → data_get
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_data_set_to_data_get() {
    let result = TestChain::new()
        .step(
            "step_1",
            "data_set",
            json!({"key": "my_data", "value": {"name": "test", "count": 42}}),
        )
        .step("step_2", "data_get", json!({"key": "my_data"}))
        .step(
            "step_3",
            "script",
            json!({
                "script": r#"
                    let data = step_2;
                    #{name: data.name, count: data.count}
                "#
            }),
        )
        .run()
        .await;

    let s3 = result.output("step_3");
    assert_eq!(s3["name"].as_str().unwrap(), "test");
    assert_eq!(s3["count"].as_i64().unwrap(), 42);
}

// ═══════════════════════════════════════════════════════════════
// P0 节点: data_filter → json_transform(sort) → script(分页)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_array_operations() {
    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({
                "script": r#"
                    let items = [
                        #{name: "Alice", age: 30},
                        #{name: "Bob", age: 25},
                        #{name: "Charlie", age: 35},
                        #{name: "David", age: 28},
                        #{name: "Eve", age: 22}
                    ];
                    #{items: items}
                "#
            }),
        )
        .step(
            "step_2",
            "data_filter",
            json!({"field": "age", "op": "gte", "value": 28}),
        )
        .input("step_2", "data", json!("{{step_1.items}}"))
        .step(
            "step_3",
            "json_transform",
            json!({"operation": "sort", "sort_key": "age", "sort_desc": false}),
        )
        .input("step_3", "data", json!("{{step_2.filtered}}"))
        .step(
            "step_4",
            "script",
            json!({
                "script": r#"
                    let sorted = step_3;
                    let page = [];
                    for i in 0..2 { page.push(sorted[i]); }
                    #{result: page, count: page.len}
                "#
            }),
        )
        .run()
        .await;

    let s2 = result.output("step_2");
    assert_eq!(s2["count"].as_i64().unwrap(), 3);

    let s3 = result.output("step_3");
    let sorted = s3.as_array().unwrap();
    assert_eq!(sorted[0]["name"].as_str().unwrap(), "David");
    assert_eq!(sorted[1]["name"].as_str().unwrap(), "Alice");
    assert_eq!(sorted[2]["name"].as_str().unwrap(), "Charlie");

    let s4 = result.output("step_4");
    assert_eq!(s4["count"].as_i64().unwrap(), 2);
    assert_eq!(s4["result"][0]["name"].as_str().unwrap(), "David");
}

// ═══════════════════════════════════════════════════════════════
// P0 节点: file_read → json_parse → script → file_write
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_file_pipeline() {
    let out_dir = tempfile::tempdir().unwrap();
    let input_path = out_dir.path().join("input.json");
    let out_path = out_dir.path().join("output.txt");
    std::fs::write(
        &input_path,
        r#"{"data":{"items":[{"name":"Alice","age":30},{"name":"Bob","age":25},{"name":"Charlie","age":35}]}}"#,
    )
    .unwrap();

    let result = TestChain::new()
        .step("step_1", "file_read", json!({"path": input_path}))
        .step(
            "step_2",
            "json_parse",
            json!({"data": "{{step_1.content}}"}),
        )
        .step(
            "step_3",
            "script",
            json!({
                "script": r#"
                    let items = step_2.data.items;
                    let content = "";
                    for i in 0..items.len {
                        let item = items[i];
                        content += item.name + ": " + to_string(item.age) + "\n";
                    }
                    #{content: content, count: items.len}
                "#
            }),
        )
        .step(
            "step_4",
            "file_write",
            json!({"path": out_path, "content": "{{step_3.content}}"}),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"));

    let s2 = result.output("step_2");
    let items = s2["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 3);

    let s3 = result.output("step_3");
    assert_eq!(s3["count"].as_i64().unwrap(), 3);

    assert!(result.is_ok("step_4"));
    let content = std::fs::read_to_string(&out_path).unwrap();
    assert!(content.contains("Alice: 30"));
    assert!(content.contains("Bob: 25"));
}

// ═══════════════════════════════════════════════════════════════
// P1 节点: 嵌套字段访问
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_nested_field_access() {
    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({
                "script": r#"
                    let data = #{
                        user: #{name: "Alice", address: #{city: "Beijing"}},
                        scores: [85, 92, 78]
                    };
                    #{data: data}
                "#
            }),
        )
        .step(
            "step_2",
            "script",
            json!({
                "script": r#"
                    let city = step_1.data.user.address.city;
                    let first = step_1.data.scores[0];
                    #{city: city, first_score: first}
                "#
            }),
        )
        .run()
        .await;

    let s2 = result.output("step_2");
    assert_eq!(s2["city"].as_str().unwrap(), "Beijing");
    assert_eq!(s2["first_score"].as_i64().unwrap(), 85);
}

// ═══════════════════════════════════════════════════════════════
// P1 节点: prompt_template
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_text_template() {
    let result = TestChain::new()
        .var("user_name", json!("Alice"))
        .var("score", json!("95"))
        .step(
            "step_1",
            "prompt_template",
            json!({
                "template": "用户 {{user_name}} 的分数是 {{score}} 分"
            }),
        )
        .run()
        .await;

    let s1 = result.output("step_1");
    assert_eq!(s1["prompt"].as_str().unwrap(), "用户 Alice 的分数是 95 分");
    assert_eq!(
        s1["messages"][0]["content"].as_str().unwrap(),
        "用户 Alice 的分数是 95 分"
    );
}

// ═══════════════════════════════════════════════════════════════
// P1 节点: regex(extract)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_regex_extract() {
    let result = TestChain::new()
        .step(
            "step_1",
            "regex",
            json!({
                "input": "Order #123 shipped, Order #456 pending",
                "pattern": r"Order #(\d+)",
                "mode": "extract",
                "global": true
            }),
        )
        .run()
        .await;

    let s1 = result.output("step_1");
    let captures = s1["captures"].as_array().unwrap();
    assert_eq!(s1["count"].as_i64().unwrap(), 2);
    assert_eq!(captures.len(), 2);
    assert_eq!(captures[0][1].as_str().unwrap(), "123");
    assert_eq!(captures[1][1].as_str().unwrap(), "456");
}

// ═══════════════════════════════════════════════════════════════
// 场景 2: 数据搬运
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_2_data_pipeline() {
    let out_dir = tempfile::tempdir().unwrap();
    let out_path = format!("{}/output.csv", out_dir.path().display());
    let result = TestChain::new()
        .step(
            "fetch_data",
            "script",
            json!({"script": r#"
            let items = [
                #{id: 1, name: "Alice", age: 30, city: "Beijing"},
                #{id: 2, name: "Bob", age: 25, city: "Shanghai"},
                #{id: 3, name: "Charlie", age: 35, city: "Beijing"},
                #{id: 4, name: "David", age: 28, city: "Shenzhen"},
                #{id: 5, name: "Eve", age: 22, city: "Beijing"}
            ];
            #{items: items}
        "#}),
        )
        .step(
            "filter_beijing",
            "data_filter",
            json!({"field": "city", "op": "equals", "value": "Beijing"}),
        )
        .input("filter_beijing", "data", json!("{{fetch_data.items}}"))
        .step(
            "to_csv",
            "script",
            json!({"script": r#"
                let rows = step_filter_beijing.filtered;
                let csv = "name,age,city\n";
                for i in 0..rows.len {
                    let item = rows[i];
                    csv += item.name + "," + to_string(item.age) + "," + item.city + "\n";
                }
                #{result: csv, row_count: rows.len}
            "#}),
        )
        .step(
            "save_csv",
            "file_write",
            json!({"path": out_path, "content": "{{to_csv.result}}"}),
        )
        .run()
        .await;
    assert!(result.is_ok("fetch_data"));
    let filtered = result.output("filter_beijing");
    assert_eq!(filtered["count"].as_i64().unwrap(), 3);
    assert_eq!(result.output("to_csv")["row_count"].as_i64().unwrap(), 3);
    assert!(result.is_ok("save_csv"));
    let fc = std::fs::read_to_string(&out_path).unwrap();
    assert!(fc.contains("Alice") && fc.contains("Charlie") && fc.contains("Eve"));
    assert!(!fc.contains("Bob"));
}

// ═══════════════════════════════════════════════════════════════
// 场景 3: 条件分支 AND
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_3_condition_and_branch() {
    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({"script": r#"#{amount: 1500, level: "VIP", order_id: "ORD-001"}"#}),
        )
        .step(
            "step_2",
            "condition",
            json!({
                "value": "{{step_1.amount}}",
                "conditionGroup": {
                    "combinator": "and",
                    "conditions": [
                        {"id": "c1", "left": "{{step_1.amount}}", "op": "gt", "right": "1000"},
                        {"id": "c2", "left": "{{step_1.level}}", "op": "==", "right": "VIP"}
                    ]
                }
            }),
        )
        .step(
            "step_3",
            "script",
            json!({"script": r#"
            let b = step_2.branch;
            let msg = if b == "true" { "审批通过 " + step_1.order_id } else { "拒绝" };
            #{branch: b, message: msg}
        "#}),
        )
        .run()
        .await;
    assert_eq!(result.output("step_2")["branch"].as_str().unwrap(), "true");
    let s3 = result.output("step_3");
    assert_eq!(s3["branch"].as_str().unwrap(), "true");
    assert!(s3["message"].as_str().unwrap().contains("ORD-001"));
}

// ═══════════════════════════════════════════════════════════════
// 场景 3: 条件分支 OR — 用 step_ 前缀 ID
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_3_condition_or_branch() {
    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({"script": r#"#{amount: 3000, urgent: 1}"#}),
        )
        .step(
            "step_2",
            "condition",
            json!({
                "value": "{{step_1.amount}}",
                "conditionGroup": {
                    "combinator": "or",
                    "conditions": [
                        {"id": "c1", "left": "{{step_1.amount}}", "op": "gt", "right": "5000"},
                        {"id": "c2", "left": "{{step_1.urgent}}", "op": "==", "right": "1"}
                    ]
                }
            }),
        )
        .step(
            "step_3",
            "script",
            json!({"script": r#"
            let is_vip = step_2.branch == "true";
            let ch = if is_vip { "VIP通道" } else { "普通通道" };
            #{channel: ch, is_vip: is_vip}
        "#}),
        )
        .run()
        .await;
    assert_eq!(result.output("step_2")["branch"].as_str().unwrap(), "true");
    assert_eq!(
        result.output("step_3")["channel"].as_str().unwrap(),
        "VIP通道"
    );
}

// ═══════════════════════════════════════════════════════════════
// 场景 4: 循环处理 — script → loop(body: script) → 汇总
// 验证：__item/__index 注入、collect 收集、嵌套字段访问
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_4_loop_collect_summary() {
    let result = TestChain::new()
        // Step 1: 准备数据
        .step(
            "step_1",
            "script",
            json!({"script": r#"
            let products = [
                #{name: "键盘", price: 299, qty: 3},
                #{name: "鼠标", price: 99, qty: 5},
                #{name: "显示器", price: 1999, qty: 1},
                #{name: "耳机", price: 399, qty: 2}
            ];
            #{products: products}
        "#}),
        )
        // Step 2: 循环计算每个商品的小计
        .step(
            "step_2",
            "loop",
            json!({
                "items": "{{step_1.products}}",
                "body": [{
                    "id": "calc",
                    "type": "script",
                    "config": {"script": r#"
                    let item = __item;
                    let subtotal = item.price * item.qty;
                    #{name: item.name, subtotal: subtotal}
                "#}
                }],
                "collect": {"names": "calc.name", "subtotals": "calc.subtotal"}
            }),
        )
        // Step 3: 汇总
        .step(
            "step_3",
            "script",
            json!({"script": r#"
            let collected = step_2.collected;
            let subtotals = collected.subtotals;
            let names = collected.names;
            let total = 0;
            for s in subtotals { total += s; }
            let count = step_2.count;
            let summary = "";
            for i in 0..count {
                summary += names[i] + ": ¥" + to_string(subtotals[i]);
                if i < count - 1 { summary += ", "; }
            }
            #{total: total, count: count, summary: summary}
        "#}),
        )
        .run()
        .await;

    // 断言
    assert!(result.is_ok("step_1"));
    assert!(result.is_ok("step_2"));

    let s2 = result.output("step_2");
    assert_eq!(
        s2["count"].as_i64().unwrap(),
        4,
        "Should iterate 4 products"
    );

    assert!(result.is_ok("step_3"));
    let s3 = result.output("step_3");
    assert_eq!(s3["count"].as_i64().unwrap(), 4);
    // 299*3 + 99*5 + 1999*1 + 399*2 = 897 + 495 + 1999 + 798 = 4189
    assert_eq!(s3["total"].as_i64().unwrap(), 4189, "Total should be 4189");
    let summary = s3["summary"].as_str().unwrap();
    assert!(summary.contains("键盘"), "Summary should mention 键盘");
    assert!(summary.contains("显示器"), "Summary should mention 显示器");
}

// ═══════════════════════════════════════════════════════════════
// 场景 5: 错误恢复 — on_error: Ignore
// 验证：失败步骤被忽略、后续步骤继续执行
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_5_error_ignore_continue() {
    let result = TestChain::new()
        // Step 1: 正常步骤
        .step("step_1", "script", json!({"script": r#"#{value: 42}"#}))
        // Step 2: 故意失败，on_error: Ignore
        .step(
            "step_2",
            "script",
            json!({
                "script": r#"throw "故意失败";"#,
                "on_error": "ignore"
            }),
        )
        // Step 3: 应该继续执行
        .step(
            "step_3",
            "script",
            json!({"script": r#"
            let v = step_1.value;
            #{result: v * 2, continued: true}
        "#}),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"), "step_1 should succeed");
    // step_2 失败但被忽略，输出为 Null
    let s2 = result.output("step_2");
    assert!(s2.is_null(), "step_2 output should be Null (ignored error)");
    // step_3 继续执行
    assert!(
        result.is_ok("step_3"),
        "step_3 should continue after ignored error"
    );
    let s3 = result.output("step_3");
    assert_eq!(s3["result"].as_i64().unwrap(), 84);
    assert!(s3["continued"].as_bool().unwrap());
}

#[tokio::test]
async fn scenario_5_error_default_stops() {
    // 默认 Fail 策略：失败后停止
    let result = TestChain::new()
        .step("step_1", "script", json!({"script": r#"#{value: 10}"#}))
        .step(
            "step_2",
            "script",
            json!({
                "script": r#"throw "失败";"#
                // 无 on_error → 默认 Fail
            }),
        )
        .step(
            "step_3",
            "script",
            json!({"script": r#"#{should_not: "run"}"#}),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"));
    assert!(!result.is_ok("step_2"), "step_2 should fail");
    assert!(
        !result.has_output("step_3"),
        "default Fail strategy must stop the chain before step_3"
    );
}

// ═══════════════════════════════════════════════════════════════
// U3: eval_condition 操作符别名测试
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn u3_eval_condition_operator_aliases() {
    // 测试 "eq" 和 "ne" 操作符（前端 UI 生成的格式）
    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({"script": r#"#{score: 90, status: "active"}"#}),
        )
        // 测试 "eq" 操作符
        .step(
            "step_2",
            "condition",
            json!({
                "value": "{{step_1.status}}",
                "conditionGroup": {
                    "combinator": "and",
                    "conditions": [
                        {"id": "c1", "left": "{{step_1.status}}", "op": "eq", "right": "active"},
                        {"id": "c2", "left": "{{step_1.score}}", "op": "gt", "right": "80"}
                    ]
                }
            }),
        )
        // 测试 "ne" 操作符
        .step(
            "step_3",
            "condition",
            json!({
                "value": "{{step_1.status}}",
                "conditionGroup": {
                    "combinator": "and",
                    "conditions": [
                        {"id": "c1", "left": "{{step_1.status}}", "op": "ne", "right": "inactive"}
                    ]
                }
            }),
        )
        .run()
        .await;

    assert!(result.is_ok("step_2"));
    let s2 = result.output("step_2");
    assert_eq!(
        s2["branch"].as_str().unwrap(),
        "true",
        "eq 'active' should pass"
    );

    assert!(result.is_ok("step_3"));
    let s3 = result.output("step_3");
    assert_eq!(
        s3["branch"].as_str().unwrap(),
        "true",
        "ne 'inactive' should pass"
    );
}

// ═══════════════════════════════════════════════════════════════
// U2: 数组索引变量引用
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn u2_array_index_variable_reference() {
    let result = TestChain::new()
        .step("step_1", "script", json!({"script": r#"
            let items = [
                #{name: "Alice", scores: [90, 85, 92]},
                #{name: "Bob", scores: [78, 88, 95]},
                #{name: "Charlie", scores: [65, 72, 80]}
            ];
            #{items: items}
        "#}))
        // 测试数组索引访问：step_1.items[0].name
        .step("step_2", "script", json!({"script": r#"
            let first_name = step_1.items[0].name;
            let first_score = step_1.items[0].scores[0];
            let second_name = step_1.items[1].name;
            let third_score = step_1.items[2].scores[2];
            #{first_name: first_name, first_score: first_score, second_name: second_name, third_score: third_score}
        "#}))
        .run().await;

    assert!(result.is_ok("step_1"));
    assert!(result.is_ok("step_2"), "Array indexing should work");

    let s2 = result.output("step_2");
    assert_eq!(s2["first_name"].as_str().unwrap(), "Alice");
    assert_eq!(s2["first_score"].as_i64().unwrap(), 90);
    assert_eq!(s2["second_name"].as_str().unwrap(), "Bob");
    assert_eq!(s2["third_score"].as_i64().unwrap(), 80);
}

// ═══════════════════════════════════════════════════════════════
// U2: 变量解析失败返回 Null
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn u2_missing_variable_returns_null() {
    let result = TestChain::new()
        .step("step_1", "script", json!({"script": r#"#{value: 42}"#}))
        // 引用不存在的字段
        .step(
            "step_2",
            "script",
            json!({"script": r#"
            let missing = step_1.nonexistent;
            let existing = step_1.value;
            #{missing: missing, existing: existing}
        "#}),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"));
    assert!(
        result.is_ok("step_2"),
        "Missing variables should return null, not error"
    );

    let s2 = result.output("step_2");
    assert!(s2["missing"].is_null(), "step_1.nonexistent should be null");
    assert_eq!(
        s2["existing"].as_i64().unwrap(),
        42,
        "step_1.value should work"
    );
}

// ═══════════════════════════════════════════════════════════════
// 场景 6: JSON 解析 + 正则提取链
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_6_json_parse_regex_chain() {
    // script(生成JSON) → json_parse(提取字段) → regex(提取数字)
    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({
                "script": r#"
                    let data = #{
                        order: #{
                            id: "ORD-2024-001",
                            total: 1599.50,
                            items: [
                                #{name: "手机", price: 999},
                                #{name: "耳机", price: 599.50}
                            ]
                        }
                    };
                    to_json(data)
                "#
            }),
        )
        .step("step_2", "json_parse", json!({"data": "{{step_1}}"}))
        .step(
            "step_3",
            "regex",
            json!({
                "input": "{{step_2.order.id}}",
                "pattern": "\\d{4}-(\\d+)",
                "mode": "extract",
                "global": false
            }),
        )
        .run()
        .await;

    assert_eq!(
        result.output("step_2")["order"]["id"].as_str().unwrap(),
        "ORD-2024-001"
    );
    let s3 = result.output("step_3");
    assert_eq!(s3["count"].as_i64().unwrap(), 1);
    assert_eq!(s3["captures"][0][0].as_str().unwrap(), "2024-001");
    assert_eq!(s3["captures"][0][1].as_str().unwrap(), "001");
}

// ═══════════════════════════════════════════════════════════════
// 场景 7: 文本模板 + 变量注入
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_7_text_template() {
    // script(生成数据) → prompt_template(模板拼接)
    let result = TestChain::new()
        .var("project", json!("workflow-engine"))
        .step(
            "step_1",
            "script",
            json!({
                "script": r#"
                    let info = #{
                        version: "7.5.0",
                        author: "haozi",
                        stars: 128
                    };
                    info
                "#
            }),
        )
        .step(
            "step_2",
            "prompt_template",
            json!({
                "template": "项目: {{project}}\n版本: {{step_1.version}}\n作者: {{step_1.author}}\nStars: {{step_1.stars}}"
            }),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"), "场景 7 step_1 失败");
    assert!(result.is_ok("step_2"), "场景 7 step_2 失败");
    assert_eq!(
        result.output("step_2")["prompt"].as_str().unwrap(),
        "项目: workflow-engine\n版本: 7.5.0\n作者: haozi\nStars: 128"
    );
}

// ═══════════════════════════════════════════════════════════════
// 场景 8: HTTP 请求 + 错误重试
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
#[ignore = "依赖外部 httpbin 服务；默认回归套件不访问公网"]
async fn scenario_8_http_retry() {
    // http(GET 公共API) → script(处理响应)
    let result = TestChain::new()
        .step(
            "step_1",
            "http",
            json!({
                "method": "GET",
                "url": "https://httpbin.org/get",
                "timeout": 10,
                "retry": 2,
                "retry_delay_ms": 10
            }),
        )
        .step(
            "step_2",
            "script",
            json!({
                "script": r#"
                    let status = step_1.status;
                    #{status: status, received: status == 200}
                "#
            }),
        )
        .run()
        .await;

    assert_eq!(result.output("step_2")["status"].as_i64().unwrap(), 200);
    assert!(result.output("step_2")["received"].as_bool().unwrap());
}

// ═══════════════════════════════════════════════════════════════
// 场景 9: Shell 命令执行
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_9_shell_command() {
    // shell(执行命令) → script(处理输出)
    let result = TestChain::new()
        .step(
            "step_1",
            "shell",
            json!({
                "command": "echo Hello from Shell",
                "timeout_secs": 5
            }),
        )
        .step(
            "step_2",
            "script",
            json!({
                "script": r#"
                    let output = step_1.stdout;
                    let code = step_1.exit_code;
                    #{output: output, exit_code: code}
                "#
            }),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"), "场景 9 step_1 失败");
    assert!(result.is_ok("step_2"), "场景 9 step_2 失败");

    let s2 = result.output("step_2");
    assert!(s2["output"].as_str().unwrap().contains("Hello from Shell"));
    assert_eq!(s2["exit_code"].as_i64().unwrap(), 0);
}

// ═══════════════════════════════════════════════════════════════
// 场景 10: Map 操作（数组映射）
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_10_map_transform() {
    // script(生成数组) → map(转换) → script(汇总)
    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({
                "script": r#"
                    let products = [
                        #{name: "手机", price: 999, qty: 2},
                        #{name: "耳机", price: 299, qty: 5},
                        #{name: "充电器", price: 99, qty: 3}
                    ];
                    products
                "#
            }),
        )
        .step(
            "step_2",
            "map",
            json!({
                "source": "output.step_1",
                "template": {
                    "product": "{{__item.name}}",
                    "total": "{{__item.price}} x {{__item.qty}}",
                    "index": "{{__index1}}"
                }
            }),
        )
        .step(
            "step_3",
            "script",
            json!({
                "script": r#"
                    let count = step_2.len;
                    let first = step_2[0].product;
                    #{count: count, first_product: first}
                "#
            }),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"), "场景 10 step_1 失败");
    assert!(result.is_ok("step_2"), "场景 10 step_2 失败");
    // 调试：查看 step_2 输出
    let s2 = result.output("step_2");
    println!(
        "step_2 输出: {}",
        serde_json::to_string_pretty(&s2).unwrap()
    );

    assert!(result.is_ok("step_3"), "场景 10 step_3 失败");

    let s3 = result.output("step_3");
    println!(
        "Map 结果: {} 个商品，第一个: {}",
        s3["count"].as_i64().unwrap_or(0),
        s3["first_product"].as_str().unwrap_or("")
    );
}

// ═══════════════════════════════════════════════════════════════
// 场景 11: 文件读写全链路
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_11_file_read_write() {
    use std::fs;

    let test_dir = tempfile::tempdir().unwrap();
    let input_path = test_dir.path().join("input.txt");
    let output_path = test_dir.path().join("output.txt");

    // 创建测试输入文件
    fs::write(&input_path, "Hello, Workflow Engine!\n这是文件读写测试。").unwrap();

    // file_read → script(处理) → file_write
    let result = TestChain::new()
        .step("step_1", "file_read", json!({"path": input_path}))
        .step(
            "step_2",
            "script",
            json!({
                "script": r#"
                    let content = step_1.content;
                    let size = step_1.size;
                    let upper = content.to_upper();
                    #{original: content, upper: upper, size: size}
                "#
            }),
        )
        .step(
            "step_3",
            "file_write",
            json!({
                "path": output_path,
                "content": "{{step_2.upper}}"
            }),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"), "场景 11 step_1 失败");
    assert!(result.is_ok("step_2"), "场景 11 step_2 失败");
    assert!(result.output("step_2")["size"].as_u64().unwrap() > 0);
    assert!(result.output("step_3")["written"].as_bool().unwrap());

    let output_content = fs::read_to_string(&output_path).unwrap();
    assert!(output_content.contains("HELLO, WORKFLOW ENGINE!"));
    assert!(output_content.contains("这是文件读写测试。"));
}

// ═══════════════════════════════════════════════════════════════
// 场景 12: 延迟 + 变量流转
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_12_delay_variable_flow() {
    // script → delay(100ms) → script(验证上游消息与延时输出)
    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({
                "script": r#"
                    let msg = "任务开始处理";
                    #{message: msg, timestamp: 1234567890}
                "#
            }),
        )
        .step("step_2", "delay", json!({"duration_ms": 100}))
        .step(
            "step_3",
            "script",
            json!({
                "script": r#"
                    let msg = step_1.message;
                    let duration = step_2.duration_ms;
                    #{message: msg, duration_ms: duration}
                "#
            }),
        )
        .run()
        .await;

    assert_eq!(
        result.output("step_2")["duration_ms"].as_u64().unwrap(),
        100
    );
    let s3 = result.output("step_3");
    assert_eq!(s3["message"].as_str().unwrap(), "任务开始处理");
    assert_eq!(s3["duration_ms"].as_u64().unwrap(), 100);
}

// ═══════════════════════════════════════════════════════════════
// 场景 13: 并行执行
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_13_parallel_branches() {
    // parallel(3个分支) → script(合并结果)
    let result = TestChain::new()
        .step(
            "step_1",
            "parallel",
            json!({
                "branches": [
                    [
                        {
                            "id": "branch_a",
                            "name": "分支A",
                            "type": "script",
                            "config": {"script": r#"#{branch: "A", value: 100}"#}
                        }
                    ],
                    [
                        {
                            "id": "branch_b",
                            "name": "分支B",
                            "type": "script",
                            "config": {"script": r#"#{branch: "B", value: 200}"#}
                        }
                    ],
                    [
                        {
                            "id": "branch_c",
                            "name": "分支C",
                            "type": "script",
                            "config": {"script": r#"#{branch: "C", value: 300}"#}
                        }
                    ]
                ]
            }),
        )
        .step(
            "step_2",
            "script",
            json!({
                "script": r#"
                    let results = step_1.results;
                    let count = step_1.branch_count;
                    let a = results[0].outputs.branch_a.value;
                    let b = results[1].outputs.branch_b.value;
                    let c = results[2].outputs.branch_c.value;
                    let total = a + b + c;
                    #{count: count, a: a, b: b, c: c, total: total}
                "#
            }),
        )
        .run()
        .await;

    // 调试：查看 step_1 输出
    if result.is_ok("step_1") {
        let s1 = result.output("step_1");
        println!(
            "step_1 输出: {}",
            serde_json::to_string_pretty(&s1).unwrap()
        );
    }

    assert!(result.is_ok("step_1"), "场景 13 step_1 失败");
    assert!(result.is_ok("step_2"), "场景 13 step_2 失败");

    let s2 = result.output("step_2");
    println!(
        "并行结果: A={} B={} C={} 总计={}",
        s2["a"].as_i64().unwrap_or(0),
        s2["b"].as_i64().unwrap_or(0),
        s2["c"].as_i64().unwrap_or(0),
        s2["total"].as_i64().unwrap_or(0)
    );
}

// ═══════════════════════════════════════════════════════════════
// 场景 14: While 循环
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_14_while_loop() {
    // script(生成数据) → while(遍历) → script(汇总)
    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({
                "script": r#"
                    let items = [
                        #{name: "任务1", status: "done"},
                        #{name: "任务2", status: "done"},
                        #{name: "任务3", status: "pending"}
                    ];
                    items
                "#
            }),
        )
        .step(
            "step_2",
            "while",
            json!({
                "items": "output.step_1",
                "condition": {
                    "check": "__item.status",
                    "op": "eq",
                    "right": "done"
                },
                "body": [
                    {
                        "id": "process",
                        "name": "处理",
                        "type": "script",
                        "config": {
                            "script": r#"let name = __item.name; #{processed: name}"#
                        }
                    }
                ],
                "collect": "process"
            }),
        )
        .step(
            "step_3",
            "script",
            json!({
                "script": r#"
                    let count = step_2.count;
                    let results = step_2.collected;
                    #{count: count, results: results}
                "#
            }),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"), "场景 14 step_1 失败");
    // 调试：查看 step_2 输出
    if result.is_ok("step_2") {
        let s2 = result.output("step_2");
        println!(
            "step_2 输出: {}",
            serde_json::to_string_pretty(&s2).unwrap()
        );
    }

    // 调试：查看 step_2 状态
    if !result.is_ok("step_2") {
        println!("step_2 失败");
    }

    assert!(result.is_ok("step_2"), "场景 14 step_2 失败");
    assert!(result.is_ok("step_3"), "场景 14 step_3 失败");

    let s3 = result.output("step_3");
    println!(
        "While 循环: 处理了 {} 个任务",
        s3["count"].as_i64().unwrap_or(0)
    );
}

// ═══════════════════════════════════════════════════════════════
// 场景 15: 子工作流
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_15_sub_workflow() {
    // sub_workflow(内联步骤) → script(使用子流程输出)
    let result = TestChain::new()
        .step(
            "step_1",
            "sub_workflow",
            json!({
                "inline_steps": [
                    {
                        "id": "sub_1",
                        "name": "子步骤1",
                        "type": "script",
                        "config": {"script": r#"#{value: 100}"#}
                    },
                    {
                        "id": "sub_2",
                        "name": "子步骤2",
                        "type": "script",
                        "config": {"script": r#"let v = step_sub_1.value; #{doubled: v * 2}"#}
                    }
                ]
            }),
        )
        .step(
            "step_2",
            "script",
            json!({
                "script": r#"
                    let v = step_1.outputs.sub_1.value;
                    let d = step_1.outputs.sub_2.doubled;
                    let count = step_1.steps_executed;
                    #{original: v, doubled: d, steps: count}
                "#
            }),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"), "场景 15 step_1 失败");
    assert!(result.is_ok("step_2"), "场景 15 step_2 失败");

    let s2 = result.output("step_2");
    println!(
        "子工作流: original={} doubled={}",
        s2["original"].as_i64().unwrap_or(0),
        s2["doubled"].as_i64().unwrap_or(0)
    );
}

// ═══════════════════════════════════════════════════════════════
// 场景 16: 审批节点（自动模式）
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_16_approval_auto() {
    // approval(require_review=false) → script(使用审批结果)
    let result = TestChain::new()
        .step(
            "step_1",
            "approval",
            json!({
                "title": "发布审批",
                "message": "确认发布 v7.5.0？",
                "options": "同意,拒绝",
                "recommended": "同意",
                "require_review": false
            }),
        )
        .step(
            "step_2",
            "script",
            json!({
                "script": r#"
                    let decision = step_1.decision;
                    let auto = step_1.auto;
                    #{decision: decision, auto: auto}
                "#
            }),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"), "场景 16 step_1 失败");
    assert!(result.is_ok("step_2"), "场景 16 step_2 失败");

    let s2 = result.output("step_2");
    println!(
        "审批结果: {} (自动: {})",
        s2["decision"].as_str().unwrap_or(""),
        s2["auto"].as_bool().unwrap_or(false)
    );
}

// ═══════════════════════════════════════════════════════════════
// 场景 17: Excel 读取
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_17_excel_read() {
    let test_dir = tempfile::tempdir().unwrap();
    let xlsx_path = test_dir.path().join("test.xlsx");

    // 先用现行 Excel 容器生成真实工作簿，再读取并传递给脚本。
    let result = TestChain::new()
        .step(
            "step_1",
            "excel",
            json!({
                "file_path": xlsx_path,
                "actions": [{
                    "id": "write",
                    "type": "write",
                    "config": {"value": [
                        ["name", "age", "city"],
                        ["张三", 25, "北京"],
                        ["李四", 30, "上海"],
                        ["王五", 28, "广州"]
                    ]}
                }]
            }),
        )
        .step(
            "step_2",
            "excel",
            json!({
                "file_path": xlsx_path,
                "actions": [{"id": "read", "type": "read", "label": "读取"}]
            }),
        )
        .step(
            "step_3",
            "script",
            json!({
                "script": r#"
                    let rows = step_2.read.data;
                    let count = step_2.read.rows;
                    let first = rows[1][0];
                    #{count: count, first_name: first}
                "#
            }),
        )
        .run()
        .await;

    assert_eq!(
        result.output("step_1")["write"]["rows_written"]
            .as_u64()
            .unwrap(),
        4
    );
    assert_eq!(result.output("step_2")["read"]["rows"].as_u64().unwrap(), 4);
    let s3 = result.output("step_3");
    assert_eq!(s3["count"].as_u64().unwrap(), 4);
    assert_eq!(s3["first_name"].as_str().unwrap(), "张三");
}

// ═══════════════════════════════════════════════════════════════
// 场景 18: Web Scrape 本地文件
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_18_web_scrape_local() {
    use std::fs;

    // 创建独立临时 HTML 文件
    let test_dir = tempfile::tempdir().unwrap();
    let html_path = test_dir.path().join("test.html");

    fs::write(
        &html_path,
        r#"
    <html>
    <head><title>测试页面</title></head>
    <body>
        <h1>产品列表</h1>
        <ul>
            <li class="product">手机 - 999元</li>
            <li class="product">耳机 - 299元</li>
            <li class="product">充电器 - 99元</li>
        </ul>
    </body>
    </html>
    "#,
    )
    .unwrap();

    // web_scrape → script(提取数据)
    let result = TestChain::new()
        .step(
            "step_1",
            "web_scrape",
            json!({
                "url": format!("file://{}", html_path.display()),
                "extract": [
                    {
                        "selector": "li.product",
                        "fields": {
                            "text": "text()"
                        }
                    }
                ]
            }),
        )
        .step(
            "step_2",
            "script",
            json!({
                "script": r#"
                    let items = step_1.items;
                    let count = step_1.total_items;
                    let first = items[0].text;
                    #{count: count, first_item: first}
                "#
            }),
        )
        .run()
        .await;

    assert!(result.is_ok("step_1"), "场景 18 step_1 失败");
    assert!(result.is_ok("step_2"), "场景 18 step_2 失败");

    let s2 = result.output("step_2");
    assert_eq!(s2["count"].as_i64().unwrap(), 3);
    assert_eq!(s2["first_item"].as_str().unwrap(), "手机 - 999元");
}
