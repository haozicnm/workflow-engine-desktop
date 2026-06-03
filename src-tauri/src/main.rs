// main.rs — 独立 HTTP 服务器入口
// 启动 axum HTTP 服务器，提供 API + Vue 前端静态文件服务

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;
use tracing::info;
use tracing_subscriber::prelude::*;
use workflow_engine::App;

#[tokio::main]
async fn main() {
    setup_logging();

    let app = Arc::new(App::new().unwrap_or_else(|e| {
        eprintln!("❌ 应用初始化失败: {}", e);
        std::process::exit(1);
    }));

    let bind_addr = std::env::var("BIND").unwrap_or_else(|_| "127.0.0.1:19529".to_string());
    let static_dir = std::env::var("STATIC_DIR").unwrap_or_else(|_| "dist".to_string());

    let router = workflow_engine::server::build_router(app)
        .fallback_service(tower_http::services::ServeDir::new(&static_dir));

    let bind_addr: std::net::SocketAddr = bind_addr.parse().unwrap_or_else(|e| {
        eprintln!("❌ 无效的绑定地址 '{}': {}", bind_addr, e);
        std::process::exit(1);
    });

    // 固定端口，重试 3 次（处理 Windows 僵尸 socket 延迟释放）
    let listener = {
        let mut last_err = None;
        let mut listener = None;
        for attempt in 1..=3 {
            match tokio::net::TcpListener::bind(bind_addr).await {
                Ok(l) => { listener = Some(l); break; }
                Err(e) => {
                    if attempt < 3 {
                        tracing::warn!("端口 {} 绑定失败 (尝试 {}/3)，{}秒后重试...", bind_addr.port(), attempt, attempt * 2);
                        tokio::time::sleep(std::time::Duration::from_secs(attempt * 2)).await;
                    }
                    last_err = Some(e);
                }
            }
        }
        match listener {
            Some(l) => l,
            None => {
                eprintln!("❌ 无法绑定端口 {}: {}", bind_addr.port(), last_err.unwrap());
                eprintln!("   请检查是否有其他 workflow-engine 实例在运行");
                std::process::exit(1);
            }
        }
    };

    info!("服务器启动: http://{}  (静态文件: {})", listener.local_addr().unwrap_or(bind_addr), static_dir);

    if let Err(e) = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    {
        eprintln!("❌ 服务器错误: {}", e);
        std::process::exit(1);
    }
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
                            tracing::debug!("清理过期日志: {:?}", path.file_name());
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
