// main.rs — Tauri 入口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use workflow_engine::App;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let app = App::new();

    tauri::Builder::default()
        .manage(app)
        .invoke_handler(tauri::generate_handler![
            // Workflow CRUD
            workflow_engine::commands::workflow::workflow_list,
            workflow_engine::commands::workflow::workflow_create,
            workflow_engine::commands::workflow::workflow_get,
            workflow_engine::commands::workflow::workflow_update,
            workflow_engine::commands::workflow::workflow_delete,
            workflow_engine::commands::workflow::workflow_validate,
            // Run control
            workflow_engine::commands::run::run_start,
            workflow_engine::commands::run::run_pause,
            workflow_engine::commands::run::run_resume,
            workflow_engine::commands::run::run_cancel,
            workflow_engine::commands::run::run_status,
            workflow_engine::commands::run::run_logs,
            // System
            workflow_engine::commands::system::system_check_browser,
            workflow_engine::commands::system::settings_get,
            workflow_engine::commands::system::settings_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
