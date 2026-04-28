// engine/dag_scheduler.rs — DAG 工作流调度执行器
//
// 与 scheduler.rs 不同：这里按 DAG 拓扑顺序执行节点，
// 通过 Tauri events 实时推送每个节点的执行状态到前端。
use crate::engine::dag::{DAGWorkflow, DAGNode};
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::data::db::Database;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use anyhow::Result;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn, error};

/// DAG 执行返回
#[derive(Debug)]
pub struct DAGRunResult {
    pub completed: bool,
    pub node_outputs: HashMap<String, serde_json::Value>,
}

/// 执行 DAG 工作流
pub async fn run_dag_workflow(
    dag: &DAGWorkflow,
    run_id: &str,
    app_handle: &tauri::AppHandle,
    db: &Arc<Database>,
    browser_channel: &str,
    cancel_flag: Arc<AtomicBool>,
    cancel_token: CancellationToken,
    pause_flag: Arc<AtomicBool>,
) -> Result<DAGRunResult> {
    if dag.nodes.is_empty() {
        let _ = db.update_run_status(run_id, "completed", None);
        return Ok(DAGRunResult {
            completed: true,
            node_outputs: HashMap::new(),
        });
    }

    // 构建执行计划
    let plan = dag.build_execution_plan()
        .map_err(|e| anyhow::anyhow!(e))?;
    let executor = StepExecutor::new();
    let mut node_outputs: HashMap<String, serde_json::Value> = HashMap::new();
    let mut ctx = ExecutionContext::new(run_id, &crate::engine::workflow::Workflow {
        name: dag.name.clone(),
        description: dag.description.clone(),
        steps: vec![],
        variables: dag.variables.clone(),
    });
    ctx.browser_channel = browser_channel.to_string();

    let workflow_name = dag.name.clone();
    info!("[DAG] 工作流启动: {} (run_id: {})", workflow_name, run_id);
    info!("[DAG] 执行顺序: {:?}", plan.order);

    for node_id in &plan.order {
        // ─── 检查取消 ───
        if cancel_flag.load(Ordering::Relaxed) || cancel_token.is_cancelled() {
            warn!("[DAG] 工作流取消: {}", workflow_name);
            emit_dag_complete(app_handle, run_id, &workflow_name, "cancelled", None);
            return Err(anyhow::anyhow!("cancelled"));
        }

        // ─── 检查暂停 ───
        while pause_flag.load(Ordering::Relaxed) {
            tokio::select! {
                _ = cancel_token.cancelled() => {
                    emit_dag_complete(app_handle, run_id, &workflow_name, "cancelled", None);
                    return Err(anyhow::anyhow!("cancelled"));
                }
                _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {}
            }
        }

        let node = dag.find_node(node_id).unwrap();

        // ─── 发射状态: running ───
        emit_node_status(app_handle, run_id, node_id, "running", node, None, None);

        // 构建 Step（复用现有 executor）
        let step = dag_node_to_step(node, &dag, &node_outputs);
        let step_id = step.id.clone();

        // 记录到数据库
        let _ = db.create_step_run(run_id, &step_id);

        // ─── 执行节点 ───
        let start = std::time::Instant::now();
        match executor.execute(&step, &mut ctx).await {
            Ok(output) => {
                let duration = start.elapsed().as_millis() as u64;
                node_outputs.insert(node_id.clone(), output.clone());

                emit_node_status(
                    app_handle, run_id, node_id, "success",
                    node, Some(&output), Some(duration),
                );

                let _ = db.complete_step_run(run_id, &step_id, Some(&output), None);

                info!("[DAG] 节点完成: {} ({}ms)", node.label, duration);
            }
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                let err_msg = e.to_string();

                emit_node_status(
                    app_handle, run_id, node_id, "error",
                    node, None, Some(duration),
                );

                let _ = db.complete_step_run(run_id, &step_id, None, Some(&err_msg));

                error!("[DAG] 节点失败: {} — {}", node.label, err_msg);
                emit_dag_complete(app_handle, run_id, &workflow_name, "failed", Some(&err_msg));
                return Err(e);
            }
        }
    }

    // 全部完成
    let _ = db.update_run_status(run_id, "completed", None);
    emit_dag_complete(app_handle, run_id, &workflow_name, "completed", None);
    info!("[DAG] 工作流完成: {}", workflow_name);

    Ok(DAGRunResult {
        completed: true,
        node_outputs,
    })
}

/// DAG 节点 → 线性 Step（复用现有执行器）
fn dag_node_to_step(
    node: &DAGNode,
    dag: &DAGWorkflow,
    node_outputs: &HashMap<String, serde_json::Value>,
) -> Step {
    // 如果有连线输入，注入到 config 中
    let mut config = node.config.clone();
    let inputs = dag.collect_inputs(&node.id, node_outputs);
    if !inputs.is_empty() {
        if let Some(obj) = config.as_object_mut() {
            obj.insert("_inputs".to_string(), serde_json::to_value(inputs).unwrap_or_default());
        }
    }

    Step {
        id: node.id.clone(),
        name: node.label.clone(),
        step_type: node.node_type.clone(),
        config,
        next: None,
        retry: None,
        timeout: None,
        body_steps: None,
        breakpoint: false,
        delay: None,
        on_error: None,
    }
}

// ─── 事件发射 ───

fn emit_node_status(
    app_handle: &tauri::AppHandle,
    run_id: &str,
    node_id: &str,
    status: &str,
    node: &DAGNode,
    output: Option<&serde_json::Value>,
    duration: Option<u64>,
) {
    let mut payload = serde_json::json!({
        "run_id": run_id,
        "node_id": node_id,
        "status": status,
        "label": node.label,
        "type": node.node_type,
    });

    if let Some(out) = output {
        payload["output"] = out.clone();
    }
    if let Some(d) = duration {
        payload["duration"] = serde_json::json!(d);
    }

    let _ = app_handle.emit("node-status", payload);
}

fn emit_dag_complete(
    app_handle: &tauri::AppHandle,
    run_id: &str,
    workflow_name: &str,
    status: &str,
    error: Option<&str>,
) {
    let mut payload = serde_json::json!({
        "run_id": run_id,
        "workflow_name": workflow_name,
        "status": status,
    });
    if let Some(e) = error {
        payload["error"] = serde_json::json!(e);
    }
    let _ = app_handle.emit("dag-run-complete", payload);
}
