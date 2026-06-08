// server/routes.rs — 路由定义（无状态版本，handlers 通过 state::get() 获取 App）
use crate::server::handlers;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::{
    extract::Path,
    response::IntoResponse,
    routing::{delete, get, post, put},
    Router,
};

/// 静态测试路由：无参数
async fn test_static() -> impl IntoResponse {
    "STATIC_ROUTE_OK".to_string()
}

/// 测试路由：验证路由是否匹配 (无 extractor，但 route 有 {id})
async fn test_param() -> impl IntoResponse {
    "test_param_no_extractor".to_string()
}

/// 测试路由2：验证路径参数是否工作
async fn test_param2(Path(id): Path<String>) -> impl IntoResponse {
    format!("test_param2: id={id}")
}

pub fn build() -> Router {
    Router::new()
        .route("/api/__static", get(test_static))
        .route("/api/__test/{id}", get(test_param))
        .route("/api/__test2/{id}", get(test_param2))
        .route("/api/events", get(handlers::events_sse))
        .route("/api/health", get(handlers::system_health))
        .route("/api/sidecar/health", get(handlers::sidecar_health))
        .route("/api/webbridge/health", get(handlers::webbridge_health))
        .route("/api/settings", get(handlers::settings_get))
        .route("/api/settings", put(handlers::settings_update))
        .route(
            "/api/system/check-browser",
            get(handlers::system_check_browser),
        )
        .route("/api/system/log-path", get(handlers::get_log_path))
        .route("/api/system/open-log-dir", post(handlers::open_log_dir))
        .route("/api/system/clear-logs", post(handlers::clear_logs))
        .route("/api/system/check-ipc", get(handlers::check_ipc))
        .route("/api/nodes/types", get(handlers::node_list_types))
        .route("/api/nodes/schema", get(handlers::node_schema))
        .route("/api/blocks", get(handlers::blocks_list))
        .route("/api/blocks/categories", get(handlers::blocks_categories))
        .route("/api/blocks/{type}", get(handlers::blocks_get))
        .route("/api/workflows", get(handlers::workflow_list))
        .route("/api/workflows", post(handlers::workflow_create))
        .route("/api/workflows/{id}", get(handlers::workflow_get))
        .route("/api/workflows/{id}", put(handlers::workflow_update))
        .route("/api/workflows/{id}", delete(handlers::workflow_delete))
        .route("/api/workflows/{id}/lock", post(handlers::workflow_lock))
        .route(
            "/api/workflows/{id}/yaml",
            post(handlers::workflow_save_yaml),
        )
        .route(
            "/api/workflows/{id}/export-yaml",
            get(handlers::workflow_export_yaml),
        )
        .route("/api/workflows/validate", post(handlers::workflow_validate))
        .route("/api/workflows/assemble", post(handlers::workflow_assemble))
        .route(
            "/api/workflows/auto-order",
            post(handlers::workflow_auto_order),
        )
        .route("/api/workflows/export", post(handlers::workflow_export))
        .route("/api/workflows/import", post(handlers::workflow_import))
        .route("/api/runs", post(handlers::run_start))
        .route("/api/runs", get(handlers::run_list))
        .route("/api/runs/{run_id}/cancel", post(handlers::run_cancel))
        .route("/api/runs/{run_id}/pause", post(handlers::run_pause))
        .route("/api/runs/{run_id}/resume", post(handlers::run_resume))
        .route("/api/runs/{run_id}/status", get(handlers::run_status))
        .route("/api/runs/{run_id}/detail", get(handlers::run_detail))
        .route("/api/runs/{run_id}/logs", get(handlers::run_logs))
        .route("/api/runs/{run_id}/step-logs", get(handlers::run_step_logs))
        .route(
            "/api/approvals/pending",
            get(handlers::approval_list_pending),
        )
        .route("/api/approvals/respond", post(handlers::approval_respond))
        .route("/api/schedules", get(handlers::schedule_list))
        .route("/api/schedules", post(handlers::schedule_create))
        .route("/api/schedules/{id}", put(handlers::schedule_update))
        .route("/api/schedules/{id}", delete(handlers::schedule_delete))
        .route("/api/preview/excel", post(handlers::preview_excel))
        .route("/api/preview/word", post(handlers::preview_word))
        .route(
            "/api/preview/trajectory/{run_id}",
            get(handlers::get_trajectory),
        )
        .route(
            "/api/preview/bundle-files/{run_id}/{step_id}",
            get(handlers::get_bundle_files),
        )
        .route(
            "/api/preview/bundle-file/{run_id}/{step_id}/{filename}",
            get(handlers::read_bundle_file),
        )
        .route(
            "/api/preview/web-scrape",
            post(handlers::web_scrape_preview),
        )
        .route("/api/debug/step/{run_id}", post(handlers::debug_step))
        .route(
            "/api/debug/continue/{run_id}",
            post(handlers::debug_continue),
        )
        .route(
            "/api/debug/breakpoints",
            post(handlers::debug_set_breakpoint),
        )
        .route(
            "/api/debug/breakpoints/remove",
            post(handlers::debug_remove_breakpoint),
        )
        .route(
            "/api/debug/breakpoints/{workflow_id}",
            get(handlers::debug_get_breakpoints),
        )
        .route("/api/debug/vars/{run_id}", get(handlers::debug_vars))
        .route("/api/step-test", post(handlers::step_test))
        .route("/api/plugins", get(handlers::plugin_list))
        .route("/api/plugins/install", post(handlers::plugin_install))
        .route("/api/plugins/upload", post(handlers::plugin_upload))
        .route("/api/plugins/uninstall", post(handlers::plugin_uninstall))
        .route("/api/pipeline/run", post(handlers::run_pipeline))
        .route(
            "/api/browser/pick-start",
            post(handlers::browser_pick_start),
        )
        .route("/api/browser/pick-next", get(handlers::browser_pick_next))
        .route("/api/browser/pick-stop", post(handlers::browser_pick_stop))
        .route(
            "/api/browser/snapshot",
            post(handlers::browser_snapshot),
        )
        // 模板库
        .route("/api/templates", get(handlers::template_list))
        .route("/api/templates/categories", get(handlers::template_categories))
        .route("/api/templates/import", post(handlers::template_import))
        .route("/api/templates/{name}", get(handlers::template_get))
        .route(
            "/api/templates/{name}/instantiate",
            post(handlers::template_instantiate),
        )
        // 保存为模板
        .route(
            "/api/workflows/{id}/save-as-template",
            post(handlers::workflow_save_as_template),
        )
        // 组合助手
        .route("/api/compose/chain", post(handlers::compose_chain))
        // WebBridge WebSocket endpoint
        .route("/ws/browser", get(ws_browser_handler))
}

/// WebSocket handler for Workflow WebBridge extension
async fn ws_browser_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_ws_browser)
}

async fn handle_ws_browser(socket: WebSocket) {
    use crate::nodes::webbridge;
    use futures_util::{SinkExt, StreamExt};

    let state = webbridge::get_state();
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();

    // 设置连接状态
    state.set_connected(tx).await;

    // 发送任务：从内部通道 → WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(Message::Text(msg.into())).await.is_err() {
                break;
            }
        }
    });

    // 接收任务：从 WebSocket → 内部处理
    let recv_state = state.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(text) => {
                    recv_state.handle_message(&text).await;
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    // 等待任一任务结束
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    // 清理
    state.set_disconnected().await;
    tracing::info!("WebBridge 扩展已断开");
}
