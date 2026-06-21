// bin/gui.rs — Tauri 桌面应用入口
// 双击打开桌面窗口，同时提供 HTTP API 供外部调用
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tracing::info;
use tracing_subscriber::prelude::*;
use workflow_engine::App;

fn setup_logging() {
    use workflow_engine::data::paths::resolve_log_dir;
    let log_dir = resolve_log_dir();
    std::fs::create_dir_all(&log_dir).ok();

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_writer(std::io::stdout)
        .with_target(false);

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

    std::mem::forget(_guard);
    info!("日志系统已初始化，日志目录: {}", log_dir.display());
}

fn main() {
    setup_logging();

    let app = App::new().unwrap_or_else(|e| {
        eprintln!("❌ 应用初始化失败: {}", e);
        std::process::exit(1);
    });

    // Clone for HTTP server (runs alongside Tauri)
    let app_for_http = app.clone();
    let http_bind = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:19529".to_string());
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| {
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                let beside = dir.join("dist");
                if beside.is_dir() {
                    return beside.to_string_lossy().to_string();
                }
                if let Some(grandparent) = dir.parent().and_then(|p| p.parent()) {
                    let relative = grandparent.join("dist");
                    if relative.is_dir() {
                        return relative.to_string_lossy().to_string();
                    }
                }
            }
        }
        "dist".to_string()
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(app)
        .invoke_handler(tauri::generate_handler![
            workflow_engine::commands::workflow::workflow_list,
            workflow_engine::commands::workflow::workflow_create,
            workflow_engine::commands::workflow::workflow_get,
            workflow_engine::commands::workflow::workflow_update,
            workflow_engine::commands::workflow::workflow_delete,
            workflow_engine::commands::workflow::workflow_lock,
            workflow_engine::commands::workflow::workflow_validate,
            workflow_engine::commands::workflow::workflow_save_yaml,
            workflow_engine::commands::workflow::workflow_auto_order,
            workflow_engine::commands::workflow::step_test,
            workflow_engine::commands::workflow::export_workflow,
            workflow_engine::commands::workflow::import_workflow,
            workflow_engine::commands::run::run_start,
            workflow_engine::commands::run::run_cancel,
            workflow_engine::commands::run::run_pause,
            workflow_engine::commands::run::run_resume,
            workflow_engine::commands::run::run_status,
            workflow_engine::commands::run::run_logs,
            workflow_engine::commands::run::approval_response,
            workflow_engine::commands::run::approval_list_pending,
            workflow_engine::commands::run::run_list,
            workflow_engine::commands::run::run_detail,
            workflow_engine::commands::run::run_step_logs,
            workflow_engine::commands::run::debug_step,
            workflow_engine::commands::run::debug_continue,
            workflow_engine::commands::run::debug_set_breakpoint,
            workflow_engine::commands::run::debug_remove_breakpoint,
            workflow_engine::commands::run::debug_get_breakpoints,
            workflow_engine::commands::run::debug_vars,
            workflow_engine::commands::run::web_scrape_preview,
            workflow_engine::commands::system::system_check_browser,
            workflow_engine::commands::system::settings_get,
            workflow_engine::commands::system::settings_update,
            workflow_engine::commands::system::check_ipc,
            workflow_engine::commands::system::node_list_types,
            workflow_engine::commands::pipeline::run_pipeline,
            workflow_engine::commands::plugin::plugin_pick_file,
            workflow_engine::commands::plugin::plugin_install,
            workflow_engine::commands::plugin::plugin_uninstall,
            workflow_engine::commands::plugin::plugin_list,
            workflow_engine::commands::preview::preview_excel,
            workflow_engine::commands::preview::preview_word,
            workflow_engine::commands::schedule::schedule_list,
            workflow_engine::commands::schedule::schedule_create,
            workflow_engine::commands::schedule::schedule_update,
            workflow_engine::commands::schedule::schedule_delete,
        ])
        .setup(move |app| {
            // Start HTTP server in background for backward compatibility
            let bind_addr: std::net::SocketAddr = http_bind.parse().unwrap_or_else(|e| {
                eprintln!("❌ 无效的绑定地址: {}", e);
                std::process::exit(1);
            });
            let http_app = Arc::new(app_for_http.clone());
            let static_dir_clone = static_dir.clone();

            std::thread::spawn(move || {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async move {
                    let router = workflow_engine::server::build_router(http_app)
                        .fallback_service(tower_http::services::ServeDir::new(&static_dir_clone));

                    match tokio::net::TcpListener::bind(bind_addr).await {
                        Ok(listener) => {
                            info!("HTTP API 服务已启动: http://{}", bind_addr);
                            let _ = axum::serve(
                                listener,
                                router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
                            )
                            .await;
                        }
                        Err(e) => {
                            tracing::warn!("HTTP 端口 {} 绑定失败 (可能被占用): {}", bind_addr.port(), e);
                        }
                    }
                });
            });

            // Setup system tray (minimize to tray on close)
            workflow_engine::system::tray::setup(app)?;

            // IPC server for wf-cli: use tauri async runtime (not raw tokio::spawn
            // which panics when no Tokio runtime is active in the Tauri setup closure)

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
