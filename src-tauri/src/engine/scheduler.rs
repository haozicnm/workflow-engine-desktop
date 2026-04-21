// engine/scheduler.rs — 步骤调度器
use crate::engine::workflow::Workflow;
use crate::engine::context::ExecutionContext;
use crate::engine::state::RunState;
use anyhow::Result;
use tauri::Emitter;

/// 执行工作流
pub async fn run_workflow(
    workflow: &Workflow,
    run_id: &str,
    app_handle: &tauri::AppHandle,
) -> Result<RunState> {
    let mut ctx = ExecutionContext::new(run_id, workflow);
    let mut state = RunState::new(run_id);

    // 获取第一个步骤
    let mut current_id = if let Some(first) = workflow.steps.first() {
        first.id.clone()
    } else {
        return Err(anyhow::anyhow!("工作流没有步骤"));
    };

    loop {
        let step = workflow.steps.iter().find(|s| s.id == current_id);
        let step = match step {
            Some(s) => s,
            None => break,
        };

        // 发送步骤状态事件
        let _ = app_handle.emit("step-update", serde_json::json!({
            "run_id": run_id,
            "step_id": step.id,
            "status": "running",
        }));

        // TODO: 实际执行步骤
        state.mark_step_done(&step.id);

        let _ = app_handle.emit("step-update", serde_json::json!({
            "run_id": run_id,
            "step_id": step.id,
            "status": "completed",
        }));

        // 移动到下一步
        match &step.next {
            Some(next) => current_id = next.clone(),
            None => break,
        }
    }

    state.finish();
    Ok(state)
}
