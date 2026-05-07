// tests/scheduler_tests.rs — determine_next_step 单元测试
//
// 测试范围：
//   - Condition node: true branch, false branch
//   - Cursor node: done=false returns None, done=true continues
//   - Approval node: awaiting_approval returns None
//   - Loop/parallel/while nodes return None
//   - Default: uses step.next, falls back to list order

use serde_json::json;
use workflow_engine::engine::context::ExecutionContext;
use workflow_engine::engine::scheduler::determine_next_step;
use workflow_engine::engine::workflow::{Step, Workflow};

// ─── 辅助 ───

fn make_step(id: &str, step_type: &str, config: serde_json::Value) -> Step {
    Step {
        id: id.to_string(),
        name: format!("Step {}", id),
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

fn make_step_with_next(id: &str, step_type: &str, next: &str) -> Step {
    let mut s = make_step(id, step_type, json!({}));
    s.next = Some(next.to_string());
    s
}

fn make_workflow(steps: Vec<Step>) -> Workflow {
    Workflow {
        name: "test".to_string(),
        description: None,
        steps,
        variables: None,
    }
}

fn new_ctx() -> ExecutionContext {
    ExecutionContext::new("test-scheduler", &Default::default())
}

// ═══════════════════════════════════════════════════
// Condition node
// ═══════════════════════════════════════════════════

#[test]
fn condition_true_branch() {
    let step = Step {
        config: json!({"true_next": "step_yes", "false_next": "step_no"}),
        ..make_step("cond1", "condition", json!({}))
    };
    let wf = make_workflow(vec![
        step.clone(),
        make_step("step_yes", "http", json!({})),
        make_step("step_no", "http", json!({})),
    ]);

    let mut ctx = new_ctx();
    ctx.set_output("cond1", json!({"result": true, "branch": "true"}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("step_yes".to_string()));
}

#[test]
fn condition_false_branch() {
    let step = Step {
        config: json!({"true_next": "step_yes", "false_next": "step_no"}),
        ..make_step("cond1", "condition", json!({}))
    };
    let wf = make_workflow(vec![
        step.clone(),
        make_step("step_yes", "http", json!({})),
        make_step("step_no", "http", json!({})),
    ]);

    let mut ctx = new_ctx();
    ctx.set_output("cond1", json!({"result": false, "branch": "false"}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("step_no".to_string()));
}

#[test]
fn condition_no_output_returns_none() {
    let step = Step {
        config: json!({"true_next": "step_yes", "false_next": "step_no"}),
        ..make_step("cond1", "condition", json!({}))
    };
    let wf = make_workflow(vec![step.clone()]);

    let ctx = new_ctx();
    // 没有输出 → None
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None);
}

#[test]
fn condition_true_no_true_next_returns_none() {
    let step = Step {
        config: json!({"false_next": "step_no"}),
        ..make_step("cond1", "condition", json!({}))
    };
    let wf = make_workflow(vec![step.clone()]);

    let mut ctx = new_ctx();
    ctx.set_output("cond1", json!({"result": true, "branch": "true"}));

    // true 但没有 true_next → None
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None);
}

// ═══════════════════════════════════════════════════
// Cursor node
// ═══════════════════════════════════════════════════

#[test]
fn cursor_done_false_returns_none() {
    let step = make_step("cur1", "cursor", json!({}));
    let wf = make_workflow(vec![step.clone()]);

    let mut ctx = new_ctx();
    ctx.set_output("cur1", json!({"done": false}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None, "cursor done=false should return None");
}

#[test]
fn cursor_done_true_continues() {
    let step = make_step_with_next("cur1", "cursor", "notify1");
    let wf = make_workflow(vec![
        step.clone(),
        make_step("notify1", "notify", json!({})),
    ]);

    let mut ctx = new_ctx();
    ctx.set_output("cur1", json!({"done": true}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("notify1".to_string()));
}

#[test]
fn cursor_done_true_no_next_falls_back_to_list() {
    let step = make_step("cur1", "cursor", json!({}));
    let wf = make_workflow(vec![
        step.clone(),
        make_step("next_in_list", "http", json!({})),
    ]);

    let mut ctx = new_ctx();
    ctx.set_output("cur1", json!({"done": true}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("next_in_list".to_string()));
}

#[test]
fn cursor_no_output_returns_none() {
    let step = make_step("cur1", "cursor", json!({}));
    let wf = make_workflow(vec![step.clone()]);

    let ctx = new_ctx();
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None);
}

// ═══════════════════════════════════════════════════
// Approval node
// ═══════════════════════════════════════════════════

#[test]
fn approval_awaiting_returns_none() {
    let step = make_step("app1", "approval", json!({}));
    let wf = make_workflow(vec![step.clone()]);

    let mut ctx = new_ctx();
    ctx.set_output("app1", json!({"status": "awaiting_approval"}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None, "awaiting_approval should return None");
}

#[test]
fn approval_approved_continues() {
    let step = make_step_with_next("app1", "approval", "next1");
    let wf = make_workflow(vec![
        step.clone(),
        make_step("next1", "http", json!({})),
    ]);

    let mut ctx = new_ctx();
    ctx.set_output("app1", json!({"status": "approved"}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("next1".to_string()));
}

#[test]
fn approval_rejected_continues() {
    let step = make_step_with_next("app1", "approval", "next1");
    let wf = make_workflow(vec![
        step.clone(),
        make_step("next1", "http", json!({})),
    ]);

    let mut ctx = new_ctx();
    ctx.set_output("app1", json!({"status": "rejected"}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("next1".to_string()));
}

#[test]
fn approval_no_output_continues() {
    let step = make_step_with_next("app1", "approval", "next1");
    let wf = make_workflow(vec![
        step.clone(),
        make_step("next1", "http", json!({})),
    ]);

    let ctx = new_ctx();
    // 没有 output → 不满足 awaiting_approval 条件 → 继续
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("next1".to_string()));
}

// ═══════════════════════════════════════════════════
// Loop / Parallel / While nodes return None
// ═══════════════════════════════════════════════════

#[test]
fn loop_node_returns_none() {
    let step = make_step_with_next("loop1", "loop", "next1");
    let wf = make_workflow(vec![
        step.clone(),
        make_step("next1", "http", json!({})),
    ]);

    let ctx = new_ctx();
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None, "loop node should always return None");
}

#[test]
fn parallel_node_returns_none() {
    let step = make_step_with_next("par1", "parallel", "next1");
    let wf = make_workflow(vec![
        step.clone(),
        make_step("next1", "http", json!({})),
    ]);

    let ctx = new_ctx();
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None, "parallel node should always return None");
}

#[test]
fn while_node_returns_none() {
    let step = make_step_with_next("while1", "while", "next1");
    let wf = make_workflow(vec![
        step.clone(),
        make_step("next1", "http", json!({})),
    ]);

    let ctx = new_ctx();
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None, "while node should always return None");
}

// ═══════════════════════════════════════════════════
// Default: uses step.next, falls back to list order
// ═══════════════════════════════════════════════════

#[test]
fn default_uses_step_next() {
    let step = make_step_with_next("s1", "http", "s3");
    let wf = make_workflow(vec![
        step.clone(),
        make_step("s2", "http", json!({})),
        make_step("s3", "http", json!({})),
    ]);

    let ctx = new_ctx();
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("s3".to_string()), "should use step.next");
}

#[test]
fn default_falls_back_to_list_order() {
    let step = make_step("s1", "http", json!({}));
    let wf = make_workflow(vec![
        step.clone(),
        make_step("s2", "http", json!({})),
        make_step("s3", "http", json!({})),
    ]);

    let ctx = new_ctx();
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("s2".to_string()), "should fall back to next in list");
}

#[test]
fn default_last_step_returns_none() {
    let step = make_step("s3", "http", json!({}));
    let wf = make_workflow(vec![
        make_step("s1", "http", json!({})),
        make_step("s2", "http", json!({})),
        step.clone(),
    ]);

    let ctx = new_ctx();
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None, "last step with no next should return None");
}

#[test]
fn default_step_not_in_list_returns_none() {
    let step = make_step("orphan", "http", json!({}));
    let wf = make_workflow(vec![
        make_step("s1", "http", json!({})),
        make_step("s2", "http", json!({})),
    ]);

    let ctx = new_ctx();
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None, "step not in workflow list should return None");
}

// ═══════════════════════════════════════════════════
// Non-container types that are not special (http, script, etc.)
// ═══════════════════════════════════════════════════

#[test]
fn http_step_uses_next_field() {
    let step = make_step_with_next("h1", "http", "h2");
    let wf = make_workflow(vec![
        step.clone(),
        make_step("h2", "http", json!({})),
    ]);

    let ctx = new_ctx();
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("h2".to_string()));
}

#[test]
fn script_step_falls_back_to_list() {
    let step = make_step("sc1", "script", json!({"script": "1+1"}));
    let wf = make_workflow(vec![
        step.clone(),
        make_step("sc2", "notify", json!({})),
    ]);

    let ctx = new_ctx();
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("sc2".to_string()));
}

#[test]
fn notify_step_last_in_list_returns_none() {
    let step = make_step("n1", "notify", json!({}));
    let wf = make_workflow(vec![step.clone()]);

    let ctx = new_ctx();
    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None);
}

// ═══════════════════════════════════════════════════
// Edge cases
// ═══════════════════════════════════════════════════

#[test]
fn condition_with_step_output_boolean_false() {
    let step = Step {
        config: json!({"true_next": "yes", "false_next": "no"}),
        ..make_step("c1", "condition", json!({}))
    };
    let wf = make_workflow(vec![
        step.clone(),
        make_step("yes", "http", json!({})),
        make_step("no", "http", json!({})),
    ]);

    let mut ctx = new_ctx();
    // result 是 false 但 branch 是 true — result 字段决定路由
    ctx.set_output("c1", json!({"result": false, "branch": "true"}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("no".to_string()));
}

#[test]
fn condition_result_missing_defaults_false() {
    let step = Step {
        config: json!({"true_next": "yes", "false_next": "no"}),
        ..make_step("c1", "condition", json!({}))
    };
    let wf = make_workflow(vec![
        step.clone(),
        make_step("yes", "http", json!({})),
        make_step("no", "http", json!({})),
    ]);

    let mut ctx = new_ctx();
    // 输出中没有 result 字段 → 默认 false
    ctx.set_output("c1", json!({"branch": "false"}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("no".to_string()));
}

#[test]
fn cursor_done_missing_defaults_false() {
    let step = make_step("cur1", "cursor", json!({}));
    let wf = make_workflow(vec![step.clone()]);

    let mut ctx = new_ctx();
    // 输出中没有 done 字段 → 默认 false → None
    ctx.set_output("cur1", json!({"data": [1, 2, 3]}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, None);
}

#[test]
fn approval_status_other_than_awaiting_continues() {
    let step = make_step_with_next("app1", "approval", "next1");
    let wf = make_workflow(vec![
        step.clone(),
        make_step("next1", "http", json!({})),
    ]);

    let mut ctx = new_ctx();
    ctx.set_output("app1", json!({"status": "timeout"}));

    let next = determine_next_step(&step, &wf, &ctx);
    assert_eq!(next, Some("next1".to_string()));
}

// ═══════════════════════════════════════════════════
// RunCondition (should_run logic)
// ═══════════════════════════════════════════════════

#[test]
fn run_condition_should_run_true_branch() {
    use workflow_engine::engine::workflow::RunCondition;

    let rc = RunCondition {
        ref_step: "logic1".to_string(),
        when: "true".to_string(),
    };
    assert!(rc.should_run("true"));
    assert!(!rc.should_run("false"));
}

#[test]
fn run_condition_should_run_false_branch() {
    use workflow_engine::engine::workflow::RunCondition;

    let rc = RunCondition {
        ref_step: "logic1".to_string(),
        when: "false".to_string(),
    };
    assert!(!rc.should_run("true"));
    assert!(rc.should_run("false"));
}

#[test]
fn run_condition_should_run_both() {
    use workflow_engine::engine::workflow::RunCondition;

    let rc = RunCondition {
        ref_step: "logic1".to_string(),
        when: "both".to_string(),
    };
    assert!(rc.should_run("true"));
    assert!(rc.should_run("false"));
}

#[test]
fn run_condition_is_merge() {
    use workflow_engine::engine::workflow::RunCondition;

    let rc_merge = RunCondition {
        ref_step: "logic1".to_string(),
        when: "merge".to_string(),
    };
    assert!(rc_merge.is_merge());

    let rc_true = RunCondition {
        ref_step: "logic1".to_string(),
        when: "true".to_string(),
    };
    assert!(!rc_true.is_merge());
}
