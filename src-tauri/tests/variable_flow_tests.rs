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
    variables: HashMap<String, Value>,
    workflow: Workflow,
}

impl TestChain {
    fn new() -> Self {
        Self {
            steps: Vec::new(),
            variables: HashMap::new(),
            workflow: Default::default(),
        }
    }

    fn step(mut self, id: &str, step_type: &str, config: Value) -> Self {
        self.steps
            .push((id.to_string(), step_type.to_string(), config));
        self
    }

    fn var(mut self, key: &str, value: Value) -> Self {
        self.variables.insert(key.to_string(), value);
        self
    }

    async fn run(self) -> TestResult {
        let executor = StepExecutor::new(
            Arc::new(workflow_engine::engine::approval_store::ApprovalStore::new()),
            Arc::new(workflow_engine::data::db::Database::open_default().unwrap()),
        );
        let mut workflow = self.workflow;
        workflow.variables = Some(self.variables);
        let mut ctx = ExecutionContext::new("test-chain", &workflow);

        let mut outputs = HashMap::new();
        for (id, step_type, config) in &self.steps {
            let step = make_chain_step(id, "test", step_type, config);
            let result = executor.execute(&step, &mut ctx).await;
            match &result {
                Ok(output) => {
                    ctx.set_output(id, output.clone());
                }
                Err(e) => {
                    eprintln!("Step '{}' failed: {:?}", id, e);
                }
            }
            outputs.insert(id.clone(), result);
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
        self.outputs.get(step_id).map(|r| r.is_ok()).unwrap_or(false)
    }
}

fn make_chain_step(id: &str, name: &str, step_type: &str, config: &Value) -> Step {
    let actions = config
        .get("actions")
        .and_then(|a| a.as_array())
        .map(|arr| arr.clone());
    // logic 节点的 conditionGroup 在 step 级别
    let condition_group = config
        .get("conditionGroup")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    // loop 节点的 body_steps 在 step 级别
    let body_steps = config
        .get("body_steps")
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
        on_error: None,
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
    assert_eq!(s1["items"][0]["link"].as_str().unwrap(), "https://example.com/1");

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
            "logic",
            json!({
                "config": {"value": "{{step_1.score}}"},
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

    let s3 = result.output("step_3");
    assert_eq!(s3["branch"].as_str().unwrap(), "true");
}

#[tokio::test]
async fn test_chain_condition_false_branch() {
    let result = TestChain::new()
        .step("step_1", "script", json!({"script": r#"#{score: 40}"#}))
        .step(
            "step_2",
            "logic",
            json!({
                "config": {"value": "{{step_1.score}}"},
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
// P0 节点: array_filter → array_sort → array_paginate
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
            "array_filter",
            json!({
                "source": "{{step_1.items}}",
                "condition": {"field": "age", "op": ">=", "value": 28}
            }),
        )
        .step(
            "step_3",
            "array_sort",
            json!({
                "source": "{{step_2.result}}",
                "field": "age",
                "order": "asc"
            }),
        )
        .step(
            "step_4",
            "array_paginate",
            json!({
                "source": "{{step_3.result}}",
                "page": 1,
                "page_size": 2
            }),
        )
        .run()
        .await;

    let s2 = result.output("step_2");
    assert_eq!(s2["result_count"].as_i64().unwrap(), 3);

    let s3 = result.output("step_3");
    let sorted = s3["result"].as_array().unwrap();
    assert_eq!(sorted[0]["name"].as_str().unwrap(), "David");
    assert_eq!(sorted[1]["name"].as_str().unwrap(), "Alice");

    let s4 = result.output("step_4");
    assert_eq!(s4["count"].as_i64().unwrap(), 2);
    assert_eq!(s4["result"][0]["name"].as_str().unwrap(), "David");
}

// ═══════════════════════════════════════════════════════════════
// P0 节点: convert_to_csv
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_script_to_convert_to_csv() {
    let result = TestChain::new()
        .step(
            "step_1",
            "script",
            json!({
                "script": r#"
                    let items = [
                        #{name: "Alice", age: 30},
                        #{name: "Bob", age: 25}
                    ];
                    #{items: items}
                "#
            }),
        )
        .step(
            "step_2",
            "convert_to_csv",
            json!({
                "input": "{{step_1.items}}"
            }),
        )
        .run()
        .await;

    let s2 = result.output("step_2");
    // convert_to_csv 可能返回 result 字符串或 row_count
    let csv = s2["result"].as_str().unwrap_or("");
    let row_count = s2["row_count"].as_i64().unwrap_or(0);
    assert!(row_count == 2 || csv.contains("Alice"), "Expected 2 rows or CSV with Alice, got: {:?}", s2);
}

// ═══════════════════════════════════════════════════════════════
// P0 节点: file_read → json_parse → script → file_write
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_file_pipeline() {
    let out_dir = tempfile::tempdir().unwrap();
    let out_path = format!("{}/output.txt", out_dir.path().display());

    let result = TestChain::new()
        .step(
            "step_1",
            "file_read",
            json!({"path": fixture("sample.json")}),
        )
        .step(
            "step_2",
            "json_parse",
            json!({"data": "{{step_1.content}}", "expression": "$.data.items"}),
        )
        .step(
            "step_3",
            "script",
            json!({
                "script": r#"
                    let items = step_2.result;
                    let content = "";
                    for item in items {
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
    let items = s2["result"].as_array().unwrap();
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
// P1 节点: text_template
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_text_template() {
    let result = TestChain::new()
        .var("user_name", json!("Alice"))
        .var("score", json!("95"))
        .step(
            "step_1",
            "text_template",
            json!({
                "template": "用户 {{user_name}} 的分数是 {{score}} 分"
            }),
        )
        .run()
        .await;

    let s1 = result.output("step_1");
    let text = s1["result"].as_str().unwrap();
    assert_eq!(text, "用户 Alice 的分数是 95 分");
}

// ═══════════════════════════════════════════════════════════════
// P1 节点: regex_extract
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_chain_regex_extract() {
    let result = TestChain::new()
        .step(
            "step_1",
            "regex_extract",
            json!({
                "input": "Order #123 shipped, Order #456 pending",
                "pattern": r"Order #(\d+)",
                "global": true
            }),
        )
        .run()
        .await;

    let s1 = result.output("step_1");
    let captures = s1["captures"].as_array().unwrap();
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
        .step("fetch_data", "script", json!({"script": r#"
            let items = [
                #{id: 1, name: "Alice", age: 30, city: "Beijing"},
                #{id: 2, name: "Bob", age: 25, city: "Shanghai"},
                #{id: 3, name: "Charlie", age: 35, city: "Beijing"},
                #{id: 4, name: "David", age: 28, city: "Shenzhen"},
                #{id: 5, name: "Eve", age: 22, city: "Beijing"}
            ];
            #{items: items}
        "#}))
        .step("filter_beijing", "array_filter", json!({
            "source": "{{fetch_data.items}}",
            "condition": {"field": "city", "op": "==", "value": "Beijing"}
        }))
        .step("to_csv", "convert_to_csv", json!({"input": "{{filter_beijing.result}}"}))
        .step("save_csv", "file_write", json!({"path": out_path, "content": "{{to_csv.result}}"}))
        .run().await;
    assert!(result.is_ok("fetch_data"));
    let filtered = result.output("filter_beijing");
    assert_eq!(filtered["result_count"].as_i64().unwrap(), 3);
    assert!(result.is_ok("to_csv"));
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
        .step("step_1", "script", json!({"script": r#"#{amount: 1500, level: "VIP", order_id: "ORD-001"}"#}))
        .step("step_2", "logic", json!({
            "config": {"value": "{{step_1.amount}}"},
            "conditionGroup": {
                "combinator": "and",
                "conditions": [
                    {"id": "c1", "left": "{{step_1.amount}}", "op": "gt", "right": "1000"},
                    {"id": "c2", "left": "{{step_1.level}}", "op": "==", "right": "VIP"}
                ]
            }
        }))
        .step("step_3", "script", json!({"script": r#"
            let b = step_2.branch;
            let msg = if b == "true" { "审批通过 " + step_1.order_id } else { "拒绝" };
            #{branch: b, message: msg}
        "#}))
        .run().await;
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
        .step("step_1", "script", json!({"script": r#"#{amount: 3000, urgent: 1}"#}))
        .step("step_2", "logic", json!({
            "config": {"value": "{{step_1.amount}}"},
            "conditionGroup": {
                "combinator": "or",
                "conditions": [
                    {"id": "c1", "left": "{{step_1.amount}}", "op": "gt", "right": "5000"},
                    {"id": "c2", "left": "{{step_1.urgent}}", "op": "==", "right": "1"}
                ]
            }
        }))
        .step("step_3", "script", json!({"script": r#"
            let is_vip = step_2.branch == "true";
            let ch = if is_vip { "VIP通道" } else { "普通通道" };
            #{channel: ch, is_vip: is_vip}
        "#}))
        .run().await;
    assert_eq!(result.output("step_2")["branch"].as_str().unwrap(), "true");
    assert_eq!(result.output("step_3")["channel"].as_str().unwrap(), "VIP通道");
}
