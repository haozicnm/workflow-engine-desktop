use axum::{
    extract::Multipart,
    http::StatusCode,
    response::{Json, Response},
};
use serde::Deserialize;
use tracing::info;

use crate::server::handlers::{err_response, ok_response};

// ═══════════════════════════════════════════════════════════
// Request body types
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct SettingsUpdateBody {
    pub theme: String,
    pub language: String,
    pub auto_start: bool,
    pub log_level: String,
    pub python_path: Option<String>,
    pub browser_channel: String,
    pub browser_executable_path: String,
    pub working_dir: String,
    // ── P1 新增（serde default 兼容旧前端） ──
    #[serde(default)]
    pub timeouts: Option<crate::data::config::TimeoutConfig>,
    #[serde(default)]
    pub logging: Option<crate::data::config::LogConfig>,
    #[serde(default)]
    pub execution: Option<crate::data::config::ExecutionConfig>,
}

#[derive(Debug, Deserialize)]
pub struct PluginInstallBody {
    pub wfplug_path: String,
}

#[derive(Debug, Deserialize)]
pub struct PluginUninstallBody {
    pub name: String,
}

// ═══════════════════════════════════════════════════════════
// System handlers
// ═══════════════════════════════════════════════════════════

pub async fn system_health() -> Response {
    ok_response(serde_json::json!({
        "status": "ok",
        "version": "7.3.0",
    }))
}

pub async fn sidecar_health() -> Response {
    let info = crate::nodes::browser::get_heartbeat_info().await;
    ok_response(serde_json::json!({
        "sidecar": info,
    }))
}

pub async fn webbridge_health() -> Response {
    let state = crate::nodes::webbridge::get_state();
    let connected = state.is_connected().await;
    let info = state.get_info().await;
    ok_response(serde_json::json!({
        "connected": connected,
        "version": info.as_ref().map(|i| i.version.as_str()).unwrap_or(""),
        "client": info.as_ref().map(|i| i.client.as_str()).unwrap_or(""),
        "capabilities": info.as_ref().map(|i| &i.capabilities).unwrap(&vec![]),
    }))
}

pub async fn node_list_types() -> Response {
    let mut types: Vec<String> = crate::nodes::registry::all_nodes()
        .into_iter()
        .map(|n| n.node_type)
        .collect();
    for t in crate::nodes::mcp_node::get_all_mcp_types() {
        if !types.contains(&t) {
            types.push(t);
        }
    }
    ok_response(types)
}

/// 返回完整 node-schema.json（前端动态加载节点定义）
pub async fn node_schema() -> Response {
    let json_str = include_str!("../../../node-schema.json");
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(val) => ok_response(val),
        Err(e) => err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("node-schema.json 解析失败: {e}"),
        ),
    }
}

pub async fn settings_get() -> Response {
    let app = crate::server::state::get();
    let config = app.config.read().await;
    ok_response(serde_json::json!({
        "theme": config.theme,
        "language": config.language,
        "auto_start": config.auto_start,
        "log_level": config.log_level,
        "python_path": config.python_path,
        "browser_channel": config.browser_channel,
        "browser_executable_path": config.browser_executable_path,
        "working_dir": config.working_dir,
        "timeouts": config.timeouts,
        "logging": config.logging,
        "execution": config.execution,
    }))
}

pub async fn settings_update(Json(body): Json<SettingsUpdateBody>) -> Response {
    let app = crate::server::state::get();
    let mut config = app.config.write().await;
    config.theme = body.theme;
    config.language = body.language;
    config.auto_start = body.auto_start;
    config.log_level = body.log_level;
    config.python_path = body.python_path;
    config.browser_channel = body.browser_channel;
    config.browser_executable_path = body.browser_executable_path;
    config.working_dir = body.working_dir;
    // P1: 可选子配置（前端未传则保留旧值）
    if let Some(t) = body.timeouts { config.timeouts = t; }
    if let Some(l) = body.logging { config.logging = l; }
    if let Some(e) = body.execution { config.execution = e; }
    info!("设置已更新");
    match config.save() {
        Ok(()) => ok_response(serde_json::json!({ "success": true })),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn system_check_browser() -> Response {
    let system_python = which::which("python3")
        .or_else(|_| which::which("python"))
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    #[cfg(target_os = "windows")]
    let scanned_python: Option<String> = {
        use std::path::PathBuf;
        let candidates: [PathBuf; 3] = [
            PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default())
                .join("Programs")
                .join("Python"),
            PathBuf::from(std::env::var("PROGRAMFILES").unwrap_or_default()).join("Python"),
            PathBuf::from("C:\\Python"),
        ];
        let mut found: Vec<PathBuf> = Vec::new();
        for base in &candidates {
            if !base.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(base) {
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with("Python3") {
                        let py = e.path().join("python.exe");
                        if py.exists() {
                            found.push(py);
                        }
                    }
                }
            }
        }
        found.sort_by(|a, b| b.cmp(a));
        found
            .into_iter()
            .next()
            .map(|p| p.to_string_lossy().to_string())
    };
    #[cfg(not(target_os = "windows"))]
    let scanned_python: Option<String> = None;

    let best_python = scanned_python.or(system_python);

    let has_edge = {
        #[cfg(target_os = "windows")]
        {
            let pf_x86 = std::env::var("PROGRAMFILES(X86)").unwrap_or_default();
            let pf = std::env::var("PROGRAMFILES").unwrap_or_default();
            let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
            std::path::PathBuf::from(&pf_x86)
                .join("Microsoft/Edge/Application/msedge.exe")
                .exists()
                || std::path::PathBuf::from(&pf)
                    .join("Microsoft/Edge/Application/msedge.exe")
                    .exists()
                || std::path::PathBuf::from(&local)
                    .join("Microsoft/Edge/Application/msedge.exe")
                    .exists()
        }
        #[cfg(not(target_os = "windows"))]
        {
            which::which("microsoft-edge").is_ok()
        }
    };

    let has_chrome = {
        #[cfg(target_os = "windows")]
        {
            let pf = std::env::var("PROGRAMFILES").unwrap_or_default();
            let pf_x86 = std::env::var("PROGRAMFILES(X86)").unwrap_or_default();
            let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
            which::which("chrome").is_ok()
                || std::path::PathBuf::from(&pf)
                    .join("Google/Chrome/Application/chrome.exe")
                    .exists()
                || std::path::PathBuf::from(&pf_x86)
                    .join("Google/Chrome/Application/chrome.exe")
                    .exists()
                || std::path::PathBuf::from(&local)
                    .join("Google/Chrome/Application/chrome.exe")
                    .exists()
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

    let has_playwright_pkg = if let Some(ref py) = best_python {
        let mut cmd = std::process::Command::new(py);
        cmd.args(["-c", "import playwright; print('ok')"]);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        cmd.output().map(|o| o.status.success()).unwrap_or(false)
    } else {
        false
    };

    let has_playwright_chromium = if let Ok(exe) = std::env::current_exe() {
        exe.parent()
            .map(|d| d.join("playwright-browsers"))
            .map(|p| {
                p.exists()
                    && p.read_dir()
                        .ok()
                        .map(|mut entries| {
                            entries.any(|e| {
                                e.ok()
                                    .map(|f| {
                                        f.file_name().to_string_lossy().starts_with("chromium-")
                                    })
                                    .unwrap_or(false)
                            })
                        })
                        .unwrap_or(false)
            })
            .unwrap_or(false)
    } else {
        false
    };

    let has_playwright_cache = if let Some(ref py) = best_python {
        let mut cmd = std::process::Command::new(py);
        cmd.args([
            "-c",
            r#"
import os, sys
home = os.environ.get('PLAYWRIGHT_BROWSERS_PATH',
    os.path.join(os.environ.get('LOCALAPPDATA', ''), 'ms-playwright') if sys.platform == 'win32'
    else os.path.join(os.path.expanduser('~'), '.cache', 'ms-playwright'))
dirs = [d for d in os.listdir(home) if d.startswith('chromium-')] if os.path.exists(home) else []
print('ok' if dirs else 'missing')
"#,
        ]);
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        cmd.output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "ok")
            .unwrap_or(false)
    } else {
        false
    };

    let has_browser = has_system_browser || has_playwright_chromium || has_playwright_cache;
    let ready = python_available && has_playwright_pkg && has_browser;

    ok_response(serde_json::json!({
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

pub async fn get_log_path() -> Response {
    let log_dir = crate::data::paths::resolve_log_dir();
    ok_response(serde_json::json!({ "path": log_dir.to_string_lossy().to_string() }))
}

pub async fn open_log_dir() -> Response {
    let log_dir = crate::data::paths::resolve_log_dir();
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
    }
    // 使用系统命令打开目录
    #[cfg(target_os = "linux")]
    let result = std::process::Command::new("xdg-open").arg(&log_dir).spawn();
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(&log_dir).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("explorer").arg(&log_dir).spawn();
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    let result = std::process::Command::new("xdg-open").arg(&log_dir).spawn();

    match result {
        Ok(_) => ok_response(serde_json::json!({ "success": true })),
        Err(e) => err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("打开日志目录失败: {e}"),
        ),
    }
}

pub async fn clear_logs() -> Response {
    let log_dir = crate::data::paths::resolve_log_dir();
    if log_dir.exists() {
        let entries = match std::fs::read_dir(&log_dir) {
            Ok(e) => e,
            Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("log") {
                if let Err(e) = std::fs::remove_file(&path) {
                    return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string());
                }
            }
        }
    }
    info!("日志已清空");
    ok_response(serde_json::json!({ "success": true }))
}

pub async fn check_ipc() -> Response {
    use std::net::SocketAddr;
    use tokio::net::TcpStream;
    use tokio::time::{timeout, Duration};
    let addr: SocketAddr = match "127.0.0.1:19527".parse() {
        Ok(a) => a,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    match timeout(Duration::from_secs(2), TcpStream::connect(addr)).await {
        Ok(Ok(_)) => ok_response(serde_json::json!({ "alive": true })),
        _ => ok_response(serde_json::json!({ "alive": false })),
    }
}

// ═══════════════════════════════════════════════════════════
// Plugin handlers
// ═══════════════════════════════════════════════════════════

pub async fn plugin_list() -> Response {
    let plugins = match crate::engine::plugin_manager::list_plugins() {
        Ok(p) => p,
        Err(e) => {
            return err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("获取插件列表失败: {e}"),
            )
        }
    };

    let items: Vec<serde_json::Value> = plugins
        .iter()
        .map(|p| {
            serde_json::json!({
                "name": p.name,
                "version": p.version,
                "title": p.title,
                "description": p.description,
                "author": p.author,
                "icon": p.icon,
                "mcp_count": p.mcp_mappings.len(),
                "template_count": p.templates.len(),
            })
        })
        .collect();

    ok_response(serde_json::json!({ "plugins": items }))
}

pub async fn plugin_install(Json(body): Json<PluginInstallBody>) -> Response {
    let path = std::path::Path::new(&body.wfplug_path);
    let meta = match crate::engine::plugin_manager::install_plugin(path) {
        Ok(m) => m,
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("安装失败: {e}")),
    };

    ok_response(serde_json::json!({
        "success": true,
        "plugin": {
            "name": meta.name,
            "version": meta.version,
            "title": meta.title,
            "description": meta.description,
            "mcp_count": meta.mcp_mappings.len(),
            "template_count": meta.templates.len(),
        }
    }))
}

pub async fn plugin_uninstall(Json(body): Json<PluginUninstallBody>) -> Response {
    match crate::engine::plugin_manager::uninstall_plugin(&body.name) {
        Ok(()) => ok_response(serde_json::json!({
            "success": true,
            "message": format!("插件 {} 已卸载", body.name),
        })),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("卸载失败: {e}")),
    }
}

pub async fn plugin_pick_file() -> Response {
    err_response(
        StatusCode::NOT_IMPLEMENTED,
        "plugin_pick_file is not available in standalone server mode — use the REST API directly",
    )
}

/// 上传 .wfplug 文件并安装（multipart/form-data）
pub async fn plugin_upload(mut multipart: Multipart) -> Response {
    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_name = String::new();

    while let Some(field) = match multipart.next_field().await {
        Ok(f) => f,
        Err(e) => {
            return err_response(
                StatusCode::BAD_REQUEST,
                format!("读取上传文件失败: {e}"),
            )
        }
    } {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "wfplug" {
            file_name = field.file_name().unwrap_or("plugin.wfplug").to_string();
            match field.bytes().await {
                Ok(b) => file_bytes = Some(b.to_vec()),
                Err(e) => {
                    return err_response(
                        StatusCode::BAD_REQUEST,
                        format!("读取文件内容失败: {e}"),
                    )
                }
            }
            break;
        }
    }

    let bytes = match file_bytes {
        Some(b) => b,
        None => {
            return err_response(
                StatusCode::BAD_REQUEST,
                "未找到上传文件，请使用 'file' 字段上传 .wfplug 文件",
            )
        }
    };

    // 写入临时文件
    let tmp_dir = std::env::temp_dir().join("wf-plugins");
    if let Err(e) = std::fs::create_dir_all(&tmp_dir) {
        return err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("创建临时目录失败: {e}"),
        );
    }
    let tmp_path = tmp_dir.join(&file_name);
    if let Err(e) = std::fs::write(&tmp_path, &bytes) {
        return err_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("保存临时文件失败: {e}"),
        );
    }

    info!("插件上传: {} ({} bytes)", file_name, bytes.len());

    let meta = match crate::engine::plugin_manager::install_plugin(&tmp_path) {
        Ok(m) => m,
        Err(e) => {
            let _ = std::fs::remove_file(&tmp_path);
            return err_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("安装失败: {e}"),
            );
        }
    };

    // 清理临时文件
    let _ = std::fs::remove_file(&tmp_path);

    ok_response(serde_json::json!({
        "success": true,
        "plugin": {
            "name": meta.name,
            "version": meta.version,
            "title": meta.title,
            "description": meta.description,
            "mcp_count": meta.mcp_mappings.len(),
            "template_count": meta.templates.len(),
        }
    }))
}

// ═══════════════════════════════════════════════════════════
// Browser element picker handlers
// ═══════════════════════════════════════════════════════════

use axum::extract::Json as AxumJson;

#[derive(Debug, Deserialize)]
pub struct PickStartBody {
    pub url: Option<String>,
}

pub async fn browser_pick_start(AxumJson(body): AxumJson<PickStartBody>) -> Response {
    let params = serde_json::json!({ "url": body.url });
    match crate::nodes::browser::send_sidecar_action("pick_start", &params).await {
        Ok(val) => ok_response(val),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("pick_start failed: {e}")),
    }
}

pub async fn browser_pick_next() -> Response {
    match crate::nodes::browser::send_sidecar_action("pick_next", &serde_json::json!({})).await {
        Ok(val) => ok_response(val),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("pick_next failed: {e}")),
    }
}

pub async fn browser_pick_stop() -> Response {
    match crate::nodes::browser::send_sidecar_action("pick_stop", &serde_json::json!({})).await {
        Ok(val) => ok_response(val),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("pick_stop failed: {e}")),
    }
}
