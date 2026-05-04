// commands/dag_run.rs — DAG 工作流执行命令
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{State, AppHandle, Emitter};
use tracing::warn;
use crate::App;
use crate::engine::dag::DAGWorkflow;

/// 执行 DAG 工作流（从前端 JSON 数据直接执行）
#[tauri::command]
pub async fn dag_run_start(
    app: State<'_, App>,
    app_handle: AppHandle,
    workflow_json: serde_json::Value,
    step_mode: Option<bool>,
) -> Result<String, String> {
    // 1. 解析 DAG 工作流
    let dag: DAGWorkflow = serde_json::from_value(workflow_json)
        .map_err(|e| format!("DAG 解析失败: {}", e))?;

    let workflow_name = dag.name.clone();
    let workflow_id = uuid::Uuid::new_v4().to_string();

    // 2. 如果提供了 workflow_id，直接用；否则生成
    let run_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    app.db.create_run(&run_id, &workflow_id, &dag.name, &now)
        .map_err(|e| e.to_string())?;

    // 3. 获取浏览器通道设置
    let browser_channel = app.config.read().await.browser_channel.clone();

    // 4. 创建控制标志
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let pause_flag = Arc::new(AtomicBool::new(false));
    let step_mode_on = step_mode.unwrap_or(false);

    // 5. 并发控制
    let semaphore = app.run_semaphore.clone();
    let permit = match semaphore.clone().try_acquire_owned() {
        Ok(p) => p,
        Err(_) => return Err("已达到最大并发工作流数限制".to_string()),
    };
    app.cancel_flags.write().await.insert(run_id.clone(), cancel_flag.clone());
    app.cancel_tokens.write().await.insert(run_id.clone(), cancel_token.clone());
    app.pause_flags.write().await.insert(run_id.clone(), pause_flag.clone());

    // 6. 发射开始事件
    if let Err(e) = app_handle.emit("dag-run-start", serde_json::json!({
        "run_id": run_id,
        "workflow_name": workflow_name,
        "node_count": dag.nodes.len(),
        "edge_count": dag.edges.len(),
    })) {
        warn!("发送 dag-run-start 事件失败: {}", e);
    }

    // 7. 后台执行
    let db = app.db.clone();
    let run_id_clone = run_id.clone();
    let _wf_name = workflow_name.clone();
    let cancel_flags = app.cancel_flags.clone();
    let cancel_tokens = app.cancel_tokens.clone();
    let pause_flags = app.pause_flags.clone();

    tauri::async_runtime::spawn(async move {
        let _permit = permit;

        let result = crate::engine::dag_scheduler::run_dag_workflow(
            &dag, &run_id_clone, &app_handle, &db, &browser_channel,
            cancel_flag, cancel_token, pause_flag, step_mode_on,
        ).await;

        // 清理
        cancel_flags.write().await.remove(&run_id_clone);
        cancel_tokens.write().await.remove(&run_id_clone);
        pause_flags.write().await.remove(&run_id_clone);

        match result {
            Ok(res) => {
                tracing::info!("[DAG] 执行完成: {} ({} 节点)", run_id_clone, res.node_outputs.len());
            }
            Err(e) => {
                let err_msg = e.to_string();
                if !err_msg.contains("cancelled") {
                    tracing::error!("[DAG] 执行失败: {} — {}", run_id_clone, err_msg);
                }
            }
        }
    });

    Ok(run_id)
}

/// 取消 DAG 运行
#[tauri::command]
pub async fn dag_run_cancel(
    app: State<'_, App>,
    run_id: String,
) -> Result<(), String> {
    let flags = app.cancel_flags.read().await;
    let tokens = app.cancel_tokens.read().await;

    if let Some(flag) = flags.get(&run_id) {
        flag.store(true, Ordering::Relaxed);
        if let Some(token) = tokens.get(&run_id) {
            token.cancel();
        }
        Ok(())
    } else {
        Err("运行不存在或已结束".to_string())
    }
}
