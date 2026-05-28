// commands/system.rs — 系统操作命令
use tauri::State;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
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
    // ─── Python 检测 ───
    let system_python = which::which("python3")
        .or_else(|_| which::which("python"))
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    #[cfg(target_os = "windows")]
    let scanned_python: Option<String> = {
        use std::path::PathBuf;
        let candidates: [PathBuf; 3] = [
            PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("Programs").join("Python"),
            PathBuf::from(std::env::var("PROGRAMFILES").unwrap_or_default()).join("Python"),
            PathBuf::from("C:\\Python"),
        ];
        let mut found: Vec<PathBuf> = Vec::new();
        for base in &candidates {
            if !base.exists() { continue }
            if let Ok(entries) = std::fs::read_dir(base) {
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with("Python3") {
                        let py = e.path().join("python.exe");
                        if py.exists() { found.push(py); }
                    }
                }
            }
        }
        found.sort_by(|a, b| b.cmp(a));
        found.into_iter().next().map(|p| p.to_string_lossy().to_string())
    };
    #[cfg(not(target_os = "windows"))]
    let scanned_python: Option<String> = None;

    let best_python = scanned_python.or(system_python);

    // ─── 浏览器检测 ───
    let has_edge = {
        #[cfg(target_os = "windows")]
        {
            let pf_x86 = std::env::var("PROGRAMFILES(X86)").unwrap_or_default();
            let pf = std::env::var("PROGRAMFILES").unwrap_or_default();
            let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
            std::path::PathBuf::from(&pf_x86).join("Microsoft/Edge/Application/msedge.exe").exists()
                || std::path::PathBuf::from(&pf).join("Microsoft/Edge/Application/msedge.exe").exists()
                || std::path::PathBuf::from(&local).join("Microsoft/Edge/Application/msedge.exe").exists()
        }
        #[cfg(not(target_os = "windows"))]
        { which::which("microsoft-edge").is_ok() }
    };

    let has_chrome = {
        #[cfg(target_os = "windows")]
        {
            let pf = std::env::var("PROGRAMFILES").unwrap_or_default();
            let pf_x86 = std::env::var("PROGRAMFILES(X86)").unwrap_or_default();
            let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
            which::which("chrome").is_ok()
                || std::path::PathBuf::from(&pf).join("Google/Chrome/Application/chrome.exe").exists()
                || std::path::PathBuf::from(&pf_x86).join("Google/Chrome/Application/chrome.exe").exists()
                || std::path::PathBuf::from(&local).join("Google/Chrome/Application/chrome.exe").exists()
        }
        #[cfg(not(target_os = "windows"))]
        {
            which::which("google-chrome-stable").is_ok()
                || which::which("google-chrome").is_ok()
                || which::which("chromium-browser").is_ok()
                || which::which("chromium").is_ok()
        }
    };

    let python_available = best_python.is_some();
    let has_system_browser = has_edge || has_chrome;

    // ─── Playwright Python 包检测 ───
    let has_playwright_pkg = if let Some(ref py) = best_python {
        let mut cmd = std::process::Command::new(py);
        cmd.args(["-c", "import playwright; print('ok')"]);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd.output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    } else { false };

    // ─── Playwright Chromium 检测 ───
    let has_playwright_chromium = if let Ok(exe) = std::env::current_exe() {
        exe.parent()
            .map(|d| d.join("playwright-browsers"))
            .map(|p| p.exists() && p.read_dir().ok()
                .map(|mut entries| entries.any(|e| e.ok()
                    .map(|f| f.file_name().to_string_lossy().starts_with("chromium-"))
                    .unwrap_or(false)))
                .unwrap_or(false))
            .unwrap_or(false)
    } else { false };

    // 通过系统 Python 检查 Playwright 缓存
    let has_playwright_cache = if let Some(ref py) = best_python {
        let mut cmd = std::process::Command::new(py);
        cmd.args(["-c", r#"
import os, sys
home = os.environ.get('PLAYWRIGHT_BROWSERS_PATH',
    os.path.join(os.environ.get('LOCALAPPDATA', ''), 'ms-playwright') if sys.platform == 'win32'
    else os.path.join(os.path.expanduser('~'), '.cache', 'ms-playwright'))
dirs = [d for d in os.listdir(home) if d.startswith('chromium-')] if os.path.exists(home) else []
print('ok' if dirs else 'missing')
"#]);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        cmd.output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "ok")
            .unwrap_or(false)
    } else { false };

    // 综合就绪状态
    let has_browser = has_system_browser || has_playwright_chromium || has_playwright_cache;
    let ready = python_available && has_playwright_pkg && has_browser;

    Ok(serde_json::json!({
        "python_available": python_available,
        "system_python": best_python,
        "has_playwright_pkg": has_playwright_pkg,
        "has_playwright_chromium": has_playwright_chromium,
        "has_playwright_cache": has_playwright_cache,
        "has_edge": has_edge,
        "has_chrome": has_chrome,
        "has_system_browser": has_system_browser,
        "has_browser": has_browser,
        "ready": ready,
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
