// commands/system.rs — 系统操作命令
use tauri::State;
use serde::{Deserialize, Serialize};
use crate::App;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub theme: String,
    pub language: String,
    pub auto_start: bool,
    pub log_level: String,
    pub python_path: Option<String>,
}

#[tauri::command]
pub async fn system_check_browser() -> Result<serde_json::Value, String> {
    // 检查 Python 和 Playwright 是否可用
    let python = which::which("python3")
        .or_else(|_| which::which("python"))
        .ok();

    match python {
        Some(path) => Ok(serde_json::json!({
            "available": true,
            "python_path": path.to_string_lossy(),
        })),
        None => Ok(serde_json::json!({
            "available": false,
            "python_path": null,
        })),
    }
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
    config.save().map_err(|e| e.to_string())
}
