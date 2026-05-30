// tests/context_tests.rs — ExecutionContext 单元测试
//
// 测试范围：
//   - resolve_var: basic lookup, nested field access, step_ prefix stripping, not found
//   - resolve_config: string interpolation, nested object, array, non-string passthrough
//   - eval_expr: arithmetic, comparisons, boolean logic, variable substitution
//   - open_session: create, idempotent, status transitions

use serde_json::json;
use workflow_engine::engine::context::{ExecutionContext, SessionStatus};
use workflow_engine::engine::workflow::Workflow;

// ─── 辅助 ───

fn new_ctx() -> ExecutionContext {
    let wf: Workflow = Default::default();
    ExecutionContext::new("test-run", &wf)
}

fn new_ctx_with_vars(vars: Vec<(&str, serde_json::Value)>) -> ExecutionContext {
    let mut ctx = new_ctx();
    for (k, v) in vars {
        ctx.set_var(k.to_string(), v);
    }
    ctx
}

// ═══════════════════════════════════════════════════
// resolve_var
// ═══════════════════════════════════════════════════

#[test]
fn resolve_var_basic_lookup() {
    let ctx = new_ctx_with_vars(vec![("name", json!("小夏"))]);
    let result = ctx.resolve_var("name");
    assert_eq!(result, Some(&json!("小夏")));
}

#[test]
fn resolve_var_nested_field_access() {
    let ctx = new_ctx_with_vars(vec![(
        "user",
        json!({"name": "张三", "age": 30, "address": {"city": "北京"}}),
    )]);

    // 一级嵌套
    assert_eq!(ctx.resolve_var("user.name"), Some(&json!("张三")));
    assert_eq!(ctx.resolve_var("user.age"), Some(&json!(30)));

    // 二级嵌套
    assert_eq!(ctx.resolve_var("user.address.city"), Some(&json!("北京")));
}

#[test]
fn resolve_var_step_output_lookup() {
    let mut ctx = new_ctx();
    ctx.set_output("step1", json!({"result": "ok", "count": 42}));

    assert_eq!(ctx.resolve_var("step1.result"), Some(&json!("ok")));
    assert_eq!(ctx.resolve_var("step1.count"), Some(&json!(42)));
}

#[test]
fn resolve_var_step_prefix_stripping() {
    let mut ctx = new_ctx();
    // step_outputs 存储 key="abc123"
    ctx.set_output("abc123", json!({"data": [1, 2, 3]}));

    // 通过 step_abc123 前缀查找 → strip step_ → 在 step_outputs 中找 "abc123"
    let result = ctx.resolve_var("step_abc123.data");
    assert!(result.is_some(), "step_ prefix stripping should work");
    assert_eq!(result.unwrap(), &json!([1, 2, 3]));
}

#[test]
fn resolve_var_not_found_returns_none() {
    let ctx = new_ctx();
    assert_eq!(ctx.resolve_var("nonexistent"), None);
    assert_eq!(ctx.resolve_var("missing.field"), None);
    assert_eq!(ctx.resolve_var("user.nonexistent.deep"), None);
}

#[test]
fn resolve_var_priority_step_outputs_over_variables() {
    let mut ctx = new_ctx_with_vars(vec![("foo", json!("from_variables"))]);
    ctx.set_output("foo", json!("from_step_outputs"));

    // step_outputs 优先于 variables
    assert_eq!(ctx.resolve_var("foo"), Some(&json!("from_step_outputs")));
}

#[test]
fn resolve_var_nested_field_missing_returns_none() {
    let ctx = new_ctx_with_vars(vec![("user", json!({"name": "张三"}))]);
    // user.name 存在，但 user.name.nonexistent 不存在
    assert_eq!(ctx.resolve_var("user.name.nonexistent"), None);
}

#[test]
fn resolve_var_array_indexing_via_dot() {
    let ctx = new_ctx_with_vars(vec![("items", json!(["a", "b", "c"]))]);
    // Array indexing via dot notation: items.0, items.1, etc.
    assert_eq!(ctx.resolve_var("items.0"), Some(&json!("a")));
    assert_eq!(ctx.resolve_var("items.1"), Some(&json!("b")));
    assert_eq!(ctx.resolve_var("items.2"), Some(&json!("c")));
    // Out of bounds returns None
    assert_eq!(ctx.resolve_var("items.99"), None);
    // Negative index returns None
    assert_eq!(ctx.resolve_var("items.-1"), None);
    // Object fields inside array items
    let ctx2 = new_ctx_with_vars(vec![("data", json!({"items": ["a", "b", "c"]}))]);
    assert_eq!(
        ctx2.resolve_var("data.items"),
        Some(&json!(["a", "b", "c"]))
    );
    // Nested array indexing
    let ctx3 = new_ctx_with_vars(vec![("matrix", json!([[1, 2], [3, 4]]))]);
    assert_eq!(ctx3.resolve_var("matrix.0"), Some(&json!([1, 2])));
    assert_eq!(ctx3.resolve_var("matrix.1.0"), Some(&json!(3)));
}

#[test]
fn resolve_var_boolean_and_null() {
    let ctx = new_ctx_with_vars(vec![("flag", json!(true)), ("nothing", json!(null))]);
    assert_eq!(ctx.resolve_var("flag"), Some(&json!(true)));
    assert_eq!(ctx.resolve_var("nothing"), Some(&json!(null)));
}

// ═══════════════════════════════════════════════════
// resolve_config
// ═══════════════════════════════════════════════════

#[test]
fn resolve_config_string_interpolation() {
    let ctx = new_ctx_with_vars(vec![("name", json!("Alice")), ("host", json!("localhost"))]);

    // 纯变量替换 — 整个字符串是 {{var}} 时保留类型
    assert_eq!(ctx.resolve_config(&json!("{{name}}")), json!("Alice"));

    // 内插 — 变量嵌入文本
    let result = ctx.resolve_config(&json!("Hello, {{name}}!"));
    assert_eq!(result, json!("Hello, Alice!"));

    // 多变量
    let result = ctx.resolve_config(&json!("http://{{host}}/api"));
    assert_eq!(result, json!("http://localhost/api"));
}

#[test]
fn resolve_config_type_preservation() {
    let ctx = new_ctx_with_vars(vec![
        ("count", json!(42)),
        ("price", json!(3.14)),
        ("active", json!(true)),
        ("data", json!({"key": "value"})),
        ("list", json!([1, 2, 3])),
    ]);

    // 整个字符串是 {{var}} 时保留原始类型
    assert_eq!(ctx.resolve_config(&json!("{{count}}")).as_i64(), Some(42));
    assert_eq!(ctx.resolve_config(&json!("{{price}}")).as_f64(), Some(3.14));
    assert_eq!(
        ctx.resolve_config(&json!("{{active}}")).as_bool(),
        Some(true)
    );
    assert_eq!(
        ctx.resolve_config(&json!("{{data}}")),
        json!({"key": "value"})
    );
    assert_eq!(ctx.resolve_config(&json!("{{list}}")), json!([1, 2, 3]));
}

#[test]
fn resolve_config_nested_object() {
    let ctx = new_ctx_with_vars(vec![("name", json!("小夏")), ("port", json!(8080))]);

    let config = json!({
        "url": "http://{{name}}:{{port}}/api",
        "method": "GET",
        "headers": {
            "Authorization": "Bearer {{name}}",
            "X-Port": "{{port}}"
        }
    });

    let result = ctx.resolve_config(&config);
    assert_eq!(result["url"], json!("http://小夏:8080/api"));
    assert_eq!(result["method"], json!("GET"));
    assert_eq!(result["headers"]["Authorization"], json!("Bearer 小夏"));
    assert_eq!(result["headers"]["X-Port"].as_i64(), Some(8080));
}

#[test]
fn resolve_config_array_resolution() {
    let ctx = new_ctx_with_vars(vec![("a", json!("hello")), ("b", json!("world"))]);

    let config = json!(["{{a}}", "{{b}}", "literal", 42]);
    let result = ctx.resolve_config(&config);

    assert_eq!(result.as_array().unwrap().len(), 4);
    assert_eq!(result[0], json!("hello"));
    assert_eq!(result[1], json!("world"));
    assert_eq!(result[2], json!("literal"));
    assert_eq!(result[3], json!(42));
}

#[test]
fn resolve_config_non_string_passthrough() {
    let ctx = new_ctx();

    // 数字直接传递
    assert_eq!(ctx.resolve_config(&json!(42)), json!(42));
    assert_eq!(ctx.resolve_config(&json!(3.14)), json!(3.14));

    // 布尔直接传递
    assert_eq!(ctx.resolve_config(&json!(true)), json!(true));
    assert_eq!(ctx.resolve_config(&json!(false)), json!(false));

    // null 直接传递
    assert_eq!(ctx.resolve_config(&json!(null)), json!(null));
}

#[test]
fn resolve_config_unresolved_variable_preserved() {
    let ctx = new_ctx();

    // 未定义变量替换为空字符串（避免 {{...}} 字面量泄漏到 shell 命令等场景）
    let result = ctx.resolve_config(&json!("Hello, {{unknown}}!"));
    assert_eq!(result, json!("Hello, !"));
}

#[test]
fn resolve_config_nested_array_in_object() {
    let ctx = new_ctx_with_vars(vec![("item", json!("task"))]);

    let config = json!({
        "items": ["{{item}}_1", "{{item}}_2"],
        "count": 2
    });

    let result = ctx.resolve_config(&config);
    assert_eq!(result["items"][0], json!("task_1"));
    assert_eq!(result["items"][1], json!("task_2"));
    assert_eq!(result["count"], json!(2));
}

// ═══════════════════════════════════════════════════
// eval_expr
// ═══════════════════════════════════════════════════

#[test]
fn eval_expr_arithmetic_addition() {
    let ctx = new_ctx_with_vars(vec![("x", json!(10)), ("y", json!(3))]);
    let result = ctx.eval_expr("__vars__.x + __vars__.y");
    assert_eq!(result.unwrap(), json!(13));
}

#[test]
fn eval_expr_arithmetic_complex() {
    let ctx = new_ctx_with_vars(vec![("a", json!(10)), ("b", json!(3)), ("c", json!(2))]);

    // 乘除优先
    assert_eq!(
        ctx.eval_expr("__vars__.a + __vars__.b * __vars__.c")
            .unwrap(),
        json!(16)
    );

    // 括号
    assert_eq!(
        ctx.eval_expr("(__vars__.a + __vars__.b) * __vars__.c")
            .unwrap(),
        json!(26)
    );

    // 减法
    assert_eq!(ctx.eval_expr("__vars__.a - __vars__.b").unwrap(), json!(7));

    // 除法
    assert_eq!(ctx.eval_expr("__vars__.a / __vars__.b").unwrap(), json!(3)); // 整数除法
}

#[test]
fn eval_expr_comparison() {
    let ctx = new_ctx_with_vars(vec![("x", json!(10)), ("y", json!(20))]);

    assert_eq!(
        ctx.eval_expr("__vars__.x < __vars__.y").unwrap(),
        json!(true)
    );
    assert_eq!(
        ctx.eval_expr("__vars__.x > __vars__.y").unwrap(),
        json!(false)
    );
    assert_eq!(ctx.eval_expr("__vars__.x == 10").unwrap(), json!(true));
    assert_eq!(
        ctx.eval_expr("__vars__.x != __vars__.y").unwrap(),
        json!(true)
    );
    assert_eq!(ctx.eval_expr("__vars__.x >= 10").unwrap(), json!(true));
    assert_eq!(ctx.eval_expr("__vars__.x <= 5").unwrap(), json!(false));
}

#[test]
fn eval_expr_boolean_logic() {
    let ctx = new_ctx_with_vars(vec![("a", json!(true)), ("b", json!(false))]);

    assert_eq!(
        ctx.eval_expr("__vars__.a && __vars__.b").unwrap(),
        json!(false)
    );
    assert_eq!(
        ctx.eval_expr("__vars__.a || __vars__.b").unwrap(),
        json!(true)
    );
    assert_eq!(ctx.eval_expr("!__vars__.b").unwrap(), json!(true));
    assert_eq!(
        ctx.eval_expr("__vars__.a && !__vars__.b").unwrap(),
        json!(true)
    );
}

#[test]
fn eval_expr_variable_substitution() {
    let ctx = new_ctx_with_vars(vec![("name", json!("test")), ("count", json!(5))]);

    // 字符串拼接
    assert_eq!(
        ctx.eval_expr("`hello ${__vars__.name}`").unwrap(),
        json!("hello test")
    );

    // 数值运算
    assert_eq!(ctx.eval_expr("__vars__.count * 2").unwrap(), json!(10));
}

#[test]
fn eval_expr_step_output_access() {
    let mut ctx = new_ctx();
    ctx.set_output("step1", json!({"count": 10, "label": "done"}));
    ctx.set_output("step2", json!(20));

    // 通过 step_ 前缀访问
    assert_eq!(
        ctx.eval_expr("step_step1.count + step_step2").unwrap(),
        json!(30)
    );
}

#[test]
fn eval_expr_string_literal() {
    let ctx = new_ctx();
    assert_eq!(
        ctx.eval_expr("`hello world`").unwrap(),
        json!("hello world")
    );
}

#[test]
fn eval_expr_ternary_like() {
    let ctx = new_ctx_with_vars(vec![("score", json!(85))]);

    // Rhai 支持 if 表达式
    let result = ctx.eval_expr("if __vars__.score >= 60 { \"pass\" } else { \"fail\" }");
    assert_eq!(result.unwrap(), json!("pass"));
}

#[test]
fn eval_expr_invalid_returns_err() {
    let ctx = new_ctx();
    assert!(ctx.eval_expr("invalid!!!syntax").is_err());
}

// ═══════════════════════════════════════════════════
// open_session / close_session
// ═══════════════════════════════════════════════════

#[test]
fn open_session_creates_new_session() {
    let mut ctx = new_ctx();
    let session = ctx.open_session("node_1", "browser");

    assert_eq!(session.node_id, "node_1");
    assert_eq!(session.node_type, "browser");
    // open_session 将 Created → Running
    assert_eq!(session.status, SessionStatus::Running);
    assert!(!session.session_id.is_empty());
    assert!(session.session_id.contains("node_1"));
}

#[test]
fn open_session_idempotent() {
    let mut ctx = new_ctx();

    // 第一次打开
    let s1 = ctx.open_session("node_1", "browser").session_id.clone();

    // 第二次打开 — 应返回同一个 session
    let s2 = ctx.open_session("node_1", "browser").session_id.clone();

    assert_eq!(
        s1, s2,
        "Calling open_session twice should return the same session"
    );
}

#[test]
fn open_session_different_nodes() {
    let mut ctx = new_ctx();

    let s1_id = ctx.open_session("node_a", "browser").session_id.clone();
    let s2_id = ctx.open_session("node_b", "excel").session_id.clone();

    assert_ne!(
        s1_id, s2_id,
        "Different nodes should get different sessions"
    );
    assert_eq!(ctx.sessions.len(), 2);
}

#[test]
fn open_session_status_transitions() {
    let mut ctx = new_ctx();

    // 新建 → Running
    let session = ctx.open_session("node_1", "browser");
    assert_eq!(session.status, SessionStatus::Running);

    // 关闭 → Closed
    ctx.close_session("node_1");
    let session = ctx.sessions.get("node_1").unwrap();
    assert_eq!(session.status, SessionStatus::Closed);
}

#[test]
fn open_session_already_running_stays_running() {
    let mut ctx = new_ctx();

    // 第一次打开 — Created → Running
    ctx.open_session("node_1", "browser");

    // 第二次打开 — 已是 Running，不应改变
    let session = ctx.open_session("node_1", "browser");
    assert_eq!(session.status, SessionStatus::Running);
}

#[test]
fn close_session_nonexistent_is_noop() {
    let mut ctx = new_ctx();
    // 关闭不存在的 session 不应 panic
    ctx.close_session("nonexistent");
    assert!(ctx.sessions.is_empty());
}

#[test]
fn open_session_preserves_node_type() {
    let mut ctx = new_ctx();

    ctx.open_session("s1", "browser");
    ctx.open_session("s2", "excel");
    ctx.open_session("s3", "logic");

    assert_eq!(ctx.sessions["s1"].node_type, "browser");
    assert_eq!(ctx.sessions["s2"].node_type, "excel");
    assert_eq!(ctx.sessions["s3"].node_type, "logic");
}

// ═══════════════════════════════════════════════════
// set_var / set_output / get_output
// ═══════════════════════════════════════════════════

#[test]
fn set_var_and_resolve() {
    let mut ctx = new_ctx();
    ctx.set_var("key".to_string(), json!("value"));
    assert_eq!(ctx.resolve_var("key"), Some(&json!("value")));
}

#[test]
fn set_output_and_get_output() {
    let mut ctx = new_ctx();
    ctx.set_output("step_1", json!({"status": "ok"}));

    let output = ctx.get_output("step_1");
    assert!(output.is_some());
    assert_eq!(output.unwrap()["status"], json!("ok"));
}

#[test]
fn get_output_missing_returns_none() {
    let ctx = new_ctx();
    assert!(ctx.get_output("nonexistent").is_none());
}
