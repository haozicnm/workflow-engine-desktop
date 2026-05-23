// server/routes.rs — 路由定义
//
// 将所有 HTTP API 路由挂载到 axum Router。

use axum::{Router, routing::{get, post, put, delete}};
use std::sync::Arc;
use crate::App;
use crate::server::handlers;

/// 构建所有路由
pub fn build(app: Arc<App>) -> Router {
    let state = app.clone();

    Router::new()
        // ═══ SSE ═══
        .route("/api/events", get(handlers::events_sse))

        // ═══ 系统 ═══
        .route("/api/health", get(handlers::system_health))
        .route("/api/settings", get(handlers::settings_get))
        .route("/api/settings", put(handlers::settings_update))
        .route("/api/system/check-browser", get(handlers::system_check_browser))
        .route("/api/system/log-path", get(handlers::get_log_path))
        .route("/api/system/open-log-dir", post(handlers::open_log_dir))
        .route("/api/system/clear-logs", post(handlers::clear_logs))
        .route("/api/system/check-ipc", get(handlers::check_ipc))
        .route("/api/nodes/types", get(handlers::node_list_types))

        // ═══ 工作流 CRUD ═══
        .route("/api/workflows", get(handlers::workflow_list))
        .route("/api/workflows", post(handlers::workflow_create))
        .route("/api/workflows/{id}", get(handlers::workflow_get))
        .route("/api/workflows/{id}", put(handlers::workflow_update))
        .route("/api/workflows/{id}", delete(handlers::workflow_delete))
        .route("/api/workflows/{id}/lock", post(handlers::workflow_lock))
        .route("/api/workflows/{id}/yaml", post(handlers::workflow_save_yaml))
        .route("/api/workflows/validate", post(handlers::workflow_validate))
        .route("/api/workflows/auto-order", post(handlers::workflow_auto_order))
        .route("/api/workflows/export", post(handlers::workflow_export))
        .route("/api/workflows/import", post(handlers::workflow_import))
        .route("/api/workflows/create-from-recording", post(handlers::workflow_create_from_recording))

        // ═══ 执行控制 ═══
        .route("/api/runs", post(handlers::run_start))
        .route("/api/runs", get(handlers::run_list))
        .route("/api/runs/{run_id}/cancel", post(handlers::run_cancel))
        .route("/api/runs/{run_id}/pause", post(handlers::run_pause))
        .route("/api/runs/{run_id}/resume", post(handlers::run_resume))
        .route("/api/runs/{run_id}/status", get(handlers::run_status))
        .route("/api/runs/{run_id}/detail", get(handlers::run_detail))
        .route("/api/runs/{run_id}/logs", get(handlers::run_logs))
        .route("/api/runs/{run_id}/step-logs", get(handlers::run_step_logs))

        // ═══ 审批 ═══
        .route("/api/approvals/pending", get(handlers::approval_list_pending))
        .route("/api/approvals/respond", post(handlers::approval_respond))

        // ═══ 定时任务 ═══
        .route("/api/schedules", get(handlers::schedule_list))
        .route("/api/schedules", post(handlers::schedule_create))
        .route("/api/schedules/{id}", put(handlers::schedule_update))
        .route("/api/schedules/{id}", delete(handlers::schedule_delete))

        // ═══ 预览 ═══
        .route("/api/preview/excel", post(handlers::preview_excel))
        .route("/api/preview/word", post(handlers::preview_word))
        .route("/api/preview/trajectory/{run_id}", get(handlers::get_trajectory))
        .route("/api/preview/bundle-files/{run_id}/{step_id}", get(handlers::get_bundle_files))
        .route("/api/preview/bundle-file/{run_id}/{step_id}/{filename}", get(handlers::read_bundle_file))
        .route("/api/preview/web-scrape", post(handlers::web_scrape_preview))

        // ═══ 调试 ═══
        .route("/api/debug/step/{run_id}", post(handlers::debug_step))
        .route("/api/debug/continue/{run_id}", post(handlers::debug_continue))
        .route("/api/debug/breakpoints", post(handlers::debug_set_breakpoint))
        .route("/api/debug/breakpoints/remove", post(handlers::debug_remove_breakpoint))
        .route("/api/debug/breakpoints/{workflow_id}", get(handlers::debug_get_breakpoints))
        .route("/api/debug/vars/{run_id}", get(handlers::debug_vars))

        // ═══ Step test ═══
        .route("/api/step-test", post(handlers::step_test))
        .route("/api/recording/status", get(handlers::recording_status))

        // ═══ 插件 ═══
        .route("/api/plugins", get(handlers::plugin_list))
        .route("/api/plugins/install", post(handlers::plugin_install))
        .route("/api/plugins/uninstall", post(handlers::plugin_uninstall))

        // ═══ Pipeline ═══
        .route("/api/pipeline/run", post(handlers::run_pipeline))

        .with_state(state)
}
