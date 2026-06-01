// commands/system.rs — 系统操作命令
use tauri::State;
use serde::{Deserialize, Serialize};
use crate::App;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub language: String,
    pub auto_start: bool,
    pub log_level: String,
    pub python_path: Option<String>,
    /// 浏览器通道: auto / msedge / chrome / chromium
    pub browser_channel: String,
    /// 浏览器可执行文件路径: 留空=自动检测
    pub browser_executable_path: String,
    /// 软件工作目录
    pub working_dir: String,
    // ── P1 新增 ──
    #[serde(default)]
    pub timeouts: Option<crate::data::config::TimeoutConfig>,
    #[serde(default)]
    pub logging: Option<crate::data::config::LogConfig>,
    #[serde(default)]
    pub execution: Option<crate::data::config::ExecutionConfig>,
}

#[tauri::command]
pub async fn system_check_browser() -> Result<serde_json::Value, String> {
    // WebBridge 作为唯一浏览器后端，检查连接状态
    let state = crate::nodes::webbridge::get_state();
    let webbridge_connected = state.is_connected().await;
    let webbridge_info = state.get_info().await;

    Ok(serde_json::json!({
        "webbridge_connected": webbridge_connected,
        "webbridge_info": webbridge_info,
        "ready": webbridge_connected,
    }))
}

#[tauri::command]
pub async fn settings_get(
    app: State<'_, App>,
) -> Result<AppSettings, String> {
    let config = app.config.read().await;
    Ok(AppSettings {
        theme: config.theme.clone(),
        language: config.language.clone(),
        auto_start: config.auto_start,
        log_level: config.log_level.clone(),
        python_path: config.python_path.clone(),
        browser_channel: config.browser_channel.clone(),
        browser_executable_path: config.browser_executable_path.clone(),
        working_dir: config.working_dir.clone(),
        timeouts: Some(config.timeouts.clone()),
        logging: Some(config.logging.clone()),
        execution: Some(config.execution.clone()),
    })
}

#[tauri::command]
pub async fn settings_update(
    app: State<'_, App>,
    settings: AppSettings,
) -> Result<(), String> {
    let mut config = app.config.write().await;
    config.theme = settings.theme;
    config.language = settings.language;
    config.auto_start = settings.auto_start;
    config.log_level = settings.log_level;
    config.python_path = settings.python_path;
    config.browser_channel = settings.browser_channel;
    config.browser_executable_path = settings.browser_executable_path;
    config.working_dir = settings.working_dir;
    // P1: 可选子配置
    if let Some(t) = settings.timeouts { config.timeouts = t; }
    if let Some(l) = settings.logging { config.logging = l; }
    if let Some(e) = settings.execution { config.execution = e; }
    info!("设置已更新");
    config.save().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_log_path() -> Result<String, String> {
    let log_dir = crate::data::paths::resolve_log_dir();
    Ok(log_dir.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_log_dir() -> Result<(), String> {
    let log_dir = crate::data::paths::resolve_log_dir();
    std::fs::create_dir_all(&log_dir).map_err(|e| e.to_string())?;
    opener::open(&log_dir).map_err(|e| format!("打开日志目录失败: {}", e))
}

#[tauri::command]
pub fn clear_logs() -> Result<(), String> {
    let log_dir = crate::data::paths::resolve_log_dir();
    if log_dir.exists() {
        for entry in std::fs::read_dir(&log_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("log") {
                std::fs::remove_file(&path).map_err(|e| e.to_string())?;
            }
        }
    }
    info!("日志已清空");
    Ok(())
}

/// 检查 IPC WebSocket 守护进程是否在监听
#[tauri::command]
pub async fn check_ipc() -> Result<bool, String> {
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};
    use std::net::SocketAddr;
    let addr: SocketAddr = "127.0.0.1:19527".parse().map_err(|e: std::net::AddrParseError| e.to_string())?;
    match timeout(Duration::from_secs(2), TcpStream::connect(addr)).await {
        Ok(Ok(_)) => Ok(true),
        _ => Ok(false),
    }
}

/// 返回所有已知节点类型（内置 + MCP 内置 + 插件外挂），供前端动态合并到节点面板
#[tauri::command]
pub async fn node_list_types() -> Result<Vec<String>, String> {
    let mut types: Vec<String> = crate::nodes::registry::all_nodes()
        .into_iter().map(|n| n.node_type).collect();
    // 追加 MCP 外挂类型（插件安装的动态类型）
    for t in crate::nodes::mcp_node::get_all_mcp_types() {
        if !types.contains(&t) {
            types.push(t);
        }
    }
    Ok(types)
}
