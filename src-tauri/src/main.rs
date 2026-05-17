// main.rs — Tauri 入口 (支持 --cli 模式)
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use workflow_engine::App;
use tauri::Manager;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing::{info, debug};
use std::env;
use std::sync::Arc;
use clap::Parser;

fn main() {
    setup_logging();

    let app = App::new().expect("failed to initialize application");

    // ── CLI 模式: workflow-engine.exe --cli <command> ──
    if env::args().any(|a| a == "--cli") {
        let mut args: Vec<String> = vec!["workflow-engine".to_string()];
        args.extend(env::args().skip_while(|a| a != "--cli").skip(1));
        let app = std::sync::Arc::new(app);

        let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
        rt.block_on(async {
            let cli = match workflow_engine::cli::Cli::try_parse_from(args) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("参数错误: {}\n使用 --help 查看帮助", e);
                    std::process::exit(1);
                }
            };
            if let Err(e) = workflow_engine::cli::run_cli(cli, app).await {
                eprintln!("错误: {}", e);
                std::process::exit(1);
            }
        });
        return;
    }

    tauri::Builder::default()
        .manage(app)
        .setup(|tauri_app| {
            // 系统托盘
            workflow_engine::system::tray::setup(tauri_app)?;

            // 启动后台定时调度器
            let handle = tauri_app.handle().clone();
            let db = tauri_app.state::<App>().db.clone();
            workflow_engine::system::scheduler::start(handle, db);

            // 启动 IPC WebSocket Server
            let app_state = tauri_app.state::<App>();
            let app = Arc::new(App {
                db: app_state.db.clone(),
                config: app_state.config.clone(),
                cancel_flags: app_state.cancel_flags.clone(),
                cancel_tokens: app_state.cancel_tokens.clone(),
                pause_flags: app_state.pause_flags.clone(),
                breakpoint_flags: app_state.breakpoint_flags.clone(),
                step_mode_flags: app_state.step_mode_flags.clone(),
                debug_snapshots: app_state.debug_snapshots.clone(),
                run_semaphore: app_state.run_semaphore.clone(),
                approval_store: app_state.approval_store.clone(),
            });
            let ipc_server = Arc::new(workflow_engine::ipc::IpcServer::new(
                app,
                tauri_app.handle().clone(),
            ));
            tauri::async_runtime::spawn(async move {
                ipc_server.start().await;
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // 关闭窗口时最小化到托盘（而不是退出）
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            // Workflow CRUD
            workflow_engine::commands::workflow::workflow_list,
            workflow_engine::commands::workflow::workflow_create,
            workflow_engine::commands::workflow::workflow_get,
            workflow_engine::commands::workflow::workflow_update,
            workflow_engine::commands::workflow::workflow_delete,
            workflow_engine::commands::workflow::workflow_validate,
            workflow_engine::commands::workflow::workflow_save_yaml,
            workflow_engine::commands::workflow::workflow_auto_order,
            workflow_engine::commands::workflow::workflow_create_from_recording,
            workflow_engine::commands::workflow::recording_status,
            workflow_engine::commands::workflow::step_test,
            // Web scrape preview
            workflow_engine::commands::run::web_scrape_preview,
            // Run control
            workflow_engine::commands::run::run_start,
            workflow_engine::commands::run::run_pause,
            workflow_engine::commands::run::run_resume,
            workflow_engine::commands::run::run_cancel,
            workflow_engine::commands::run::run_status,
            workflow_engine::commands::run::run_logs,
            workflow_engine::commands::run::run_list,
            workflow_engine::commands::run::run_detail,
            workflow_engine::commands::run::run_step_logs,
            workflow_engine::commands::run::approval_response,
            workflow_engine::commands::run::approval_list_pending,
            // Debug
            workflow_engine::commands::run::debug_step,
            workflow_engine::commands::run::debug_continue,
            workflow_engine::commands::run::debug_set_breakpoint,
            workflow_engine::commands::run::debug_remove_breakpoint,
            workflow_engine::commands::run::debug_get_breakpoints,
            workflow_engine::commands::run::debug_vars,
            // System
            workflow_engine::commands::system::system_check_browser,
            workflow_engine::commands::system::settings_get,
            workflow_engine::commands::system::settings_update,
            workflow_engine::commands::system::get_log_path,
            workflow_engine::commands::system::open_log_dir,
            workflow_engine::commands::system::clear_logs,
            // Pipeline
            workflow_engine::commands::pipeline::run_pipeline,
            // Schedules
            workflow_engine::commands::schedule::schedule_list,
            workflow_engine::commands::schedule::schedule_create,
            workflow_engine::commands::schedule::schedule_update,
            workflow_engine::commands::schedule::schedule_delete,
            // Workflow import/export (P4)
            workflow_engine::commands::workflow::export_workflow,
            workflow_engine::commands::workflow::import_workflow,
            // Browser recording
            workflow_engine::commands::browser_recording::browser_recording_start,
            workflow_engine::commands::browser_recording::browser_recording_stop,
            workflow_engine::commands::browser_recording::browser_pick_element,
            workflow_engine::commands::browser_recording::browser_pick_session_start,
            workflow_engine::commands::browser_recording::browser_pick_next,
            workflow_engine::commands::browser_recording::browser_pick_session_stop,
            // Preview
            workflow_engine::commands::preview::preview_excel,
            workflow_engine::commands::preview::preview_word,
            // Templates
            workflow_engine::commands::template::list_templates,
            workflow_engine::commands::template::load_template,
            // IPC health
            workflow_engine::commands::system::check_ipc,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// 日志持久化：同时输出到 stdout 和每日轮转的文件（保留 7 天）
fn setup_logging() {
    use workflow_engine::data::paths::resolve_log_dir;
    let log_dir = resolve_log_dir();
    std::fs::create_dir_all(&log_dir).ok();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    // stdout layer
    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false);

    // 文件 layer: 每日轮转
    let file_appender = tracing_appender::rolling::daily(&log_dir, "workflow-engine.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    let file_layer = tracing_subscriber::fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stdout_layer)
        .with(file_layer)
        .init();

    // 将 guard 泄漏以保持文件写入器存活
    std::mem::forget(_guard);

    info!("日志系统已初始化，日志目录: {}", log_dir.display());

    // 清理超过 7 天的日志
    let log_dir_clone = log_dir.clone();
    std::thread::spawn(move || {
        let _ = cleanup_old_logs(&log_dir_clone, 7);
    });
}

/// 清理超过 `retention_days` 的日志文件
fn cleanup_old_logs(
    log_dir: &std::path::Path,
    retention_days: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(retention_days * 86400));

    if let Some(cutoff) = cutoff {
        for entry in std::fs::read_dir(log_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("log") {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        if modified < cutoff {
                            let _ = std::fs::remove_file(&path);
                            debug!("清理过期日志: {:?}", path.file_name());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
