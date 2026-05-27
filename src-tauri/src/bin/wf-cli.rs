// bin/wf-cli.rs — 独立 CLI 入口（无 GUI 依赖）
// 其他 agent 安装后可直接调用: wf-cli list --json
use clap::Parser;
use std::sync::Arc;
use workflow_engine::cli::{run_cli, Cli};
use workflow_engine::App;

fn main() {
    // 初始化日志（只输出到 stderr，不污染 stdout JSON）
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "warn".into()),
        )
        .init();

    let app = match App::new() {
        Ok(a) => Arc::new(a),
        Err(e) => {
            eprintln!("初始化失败: {}", e);
            std::process::exit(1);
        }
    };

    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}", e);
            std::process::exit(2);
        }
    };

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    if let Err(e) = rt.block_on(run_cli(cli, app)) {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }
}
