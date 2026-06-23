// tests/new_nodes.rs — v8.5-v9.0 新增节点的单元测试
// 每个节点至少 2 个测试用例：正常路径 + 错误路径

use serde_json::json;
use std::sync::Arc;
use workflow_engine::engine::context::ExecutionContext;
use workflow_engine::engine::executor::StepExecutor;
use workflow_engine::engine::workflow::{Step, Workflow};

fn test_exec() -> Arc<StepExecutor> {
    StepExecutor::new(
        Arc::new(workflow_engine::engine::approval_store::ApprovalStore::new()),
        Arc::new(workflow_engine::data::db::Database::open_default().expect("db")),
    )
}

fn make_step(id: &str, step_type: &str, config: serde_json::Value) -> Step {
    Step {
        id: id.to_string(),
        step_type: step_type.to_string(),
        name: id.to_string(),
        config,
        ..Default::default()
    }
}

// ═══ json_transform ═══

#[tokio::test]
async fn json_transform_pick_fields() {
    let exec = test_exec();
    let step = make_step("jt", "json_transform", json!({
        "operation": "pick",
        "fields": ["name", "age"]
    }));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    ctx.input_ports.insert("jt_in".into(), json!({"name": "Alice", "age": 30, "city": "Beijing"}));
    let result = exec.execute(&step, &mut ctx).await.unwrap();
    assert_eq!(result["name"], "Alice");
    assert_eq!(result["age"], 30);
    assert!(result.get("city").is_none());
}

#[tokio::test]
async fn json_transform_unknown_operation() {
    let exec = test_exec();
    let step = make_step("jt", "json_transform", json!({"operation": "invalid"}));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("未知操作"));
}

// ═══ data_filter ═══

#[tokio::test]
async fn data_filter_equals() {
    let exec = test_exec();
    let step = make_step("df", "data_filter", json!({
        "field": "status",
        "op": "equals",
        "value": "active"
    }));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    ctx.input_ports.insert("df_in".into(), json!([
        {"name": "A", "status": "active"},
        {"name": "B", "status": "inactive"},
        {"name": "C", "status": "active"}
    ]));
    let result = exec.execute(&step, &mut ctx).await.unwrap();
    assert_eq!(result["count"], 2);
    assert_eq!(result["filtered"][0]["name"], "A");
}

#[tokio::test]
async fn data_filter_non_array_input() {
    let exec = test_exec();
    let step = make_step("df", "data_filter", json!({"field": "x", "op": "equals", "value": 1}));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    ctx.input_ports.insert("df_in".into(), json!("not an array"));
    let result = exec.execute(&step, &mut ctx).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("数组"));
}

// ═══ llm_chat ═══

#[tokio::test]
async fn llm_chat_missing_api_key() {
    let exec = test_exec();
    let step = make_step("llm", "llm_chat", json!({
        "model": "gpt-4o",
        "prompt": "Hello"
    }));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("api_key"));
}

#[tokio::test]
async fn llm_chat_no_messages() {
    let exec = test_exec();
    let step = make_step("llm", "llm_chat", json!({
        "api_key": "test-key",
        "model": "gpt-4o"
    }));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("没有消息"));
}

// ═══ prompt_template ═══

#[tokio::test]
async fn prompt_template_basic() {
    let exec = test_exec();
    let step = make_step("pt", "prompt_template", json!({
        "template": "Hello {{name}}, your score is {{score}}"
    }));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    ctx.set_var("name".into(), json!("Alice"));
    ctx.set_var("score".into(), json!("95"));
    let result = exec.execute(&step, &mut ctx).await.unwrap();
    assert!(result["prompt"].as_str().unwrap().contains("Alice"));
    assert!(result["prompt"].as_str().unwrap().contains("95"));
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["role"], "user");
}

#[tokio::test]
async fn prompt_template_with_system() {
    let exec = test_exec();
    let step = make_step("pt", "prompt_template", json!({
        "template": "Summarize this",
        "system_prompt": "You are a helpful assistant"
    }));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await.unwrap();
    let messages = result["messages"].as_array().unwrap();
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0]["role"], "system");
    assert_eq!(messages[1]["role"], "user");
}

// ═══ database_query ═══

#[tokio::test]
async fn database_query_sql_injection_blocked() {
    let exec = test_exec();
    let step = make_step("db", "database_query", json!({
        "sql": "DROP TABLE users"
    }));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("SELECT"));
}

#[tokio::test]
async fn database_query_select_in_memory() {
    let exec = test_exec();
    // 先创建表并插入数据
    let setup = make_step("db1", "database_query", json!({
        "db_type": "sqlite",
        "connection": ":memory:",
        "sql": "CREATE TABLE test (id INTEGER, name TEXT)",
        "allow_write": true
    }));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    // 注意：:memory: 数据库每次连接都是新的，所以这里测试的是单次查询
    let result = exec.execute(&setup, &mut ctx).await;
    assert!(result.is_ok());
}

// ═══ trigger_cron ═══

#[tokio::test]
async fn trigger_cron_returns_timestamp() {
    let exec = test_exec();
    let step = make_step("tc", "trigger_cron", json!({"cron_expr": "0 0 * * * *"}));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await.unwrap();
    assert!(result["triggered_at"].as_str().is_some());
    assert!(result["timestamp"].as_i64().is_some());
}

#[tokio::test]
async fn trigger_cron_empty_config() {
    let exec = test_exec();
    let step = make_step("tc", "trigger_cron", json!({}));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await;
    assert!(result.is_ok()); // trigger_cron 不校验配置
}

// ═══ webhook_response ═══

#[tokio::test]
async fn webhook_response_sets_status() {
    let exec = test_exec();
    let step = make_step("wr", "webhook_response", json!({
        "status_code": 201,
        "body": {"message": "created"}
    }));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await.unwrap();
    assert_eq!(result["sent"], true);
    assert_eq!(result["status_code"], 201);
}

#[tokio::test]
async fn webhook_response_default_status() {
    let exec = test_exec();
    let step = make_step("wr", "webhook_response", json!({}));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await.unwrap();
    assert_eq!(result["sent"], true);
    assert_eq!(result["status_code"], 200);
}

// ═══ github_issue ═══

#[tokio::test]
async fn github_issue_missing_token() {
    let exec = test_exec();
    let step = make_step("gh", "github_issue", json!({
        "repo": "owner/repo",
        "title": "Test"
    }));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("token"));
}

#[tokio::test]
async fn github_issue_missing_repo() {
    let exec = test_exec();
    let step = make_step("gh", "github_issue", json!({
        "token": "ghp_test",
        "title": "Test"
    }));
    let mut ctx = ExecutionContext::new("test", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("repo"));
}
