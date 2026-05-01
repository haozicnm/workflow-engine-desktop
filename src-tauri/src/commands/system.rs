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
}

#[tauri::command]
pub async fn system_check_browser() -> Result<serde_json::Value, String> {
    // ─── Python 检测 ───
    // 1. 内置 Python
    let bundled = if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let python = dir.join("embed").join("python.exe");
            if python.exists() { Some(python.to_string_lossy().to_string()) } else { None }
        } else { None }
    } else { None };

    // 2. 系统 PATH 中的 Python
    let system_python = which::which("python3")
        .or_else(|_| which::which("python"))
        .ok()
        .map(|p| p.to_string_lossy().to_string());

    // 3. Windows 目录扫描（不依赖 PATH，始终执行）
    #[cfg(target_os = "windows")]
    let scanned_python: Option<String> = {
        use std::path::PathBuf;
        let candidates: [(String, PathBuf); 3] = [
            (std::env::var("LOCALAPPDATA").unwrap_or_default(),
             PathBuf::from(std::env::var("LOCALAPPDATA").unwrap_or_default()).join("Programs").join("Python")),
            (std::env::var("PROGRAMFILES").unwrap_or_default(),
             PathBuf::from(std::env::var("PROGRAMFILES").unwrap_or_default()).join("Python")),
            ("C:\\".into(),
             PathBuf::from("C:\\Python")),
        ];
        let mut found: Vec<PathBuf> = Vec::new();
        for (_label, base) in &candidates {
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
        found.sort_by(|a, b| b.cmp(a)); // 降序取最新版本
        found.into_iter().next().map(|p| p.to_string_lossy().to_string())
    };
    #[cfg(not(target_os = "windows"))]
    let scanned_python: Option<String> = None;

    // 取最优 Python
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

    let python_available = bundled.is_some() || best_python.is_some();

    Ok(serde_json::json!({
        "python_available": python_available,
        "bundled_python": bundled,
        "system_python": best_python,
        "has_edge": has_edge,
        "has_chrome": has_chrome,
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
