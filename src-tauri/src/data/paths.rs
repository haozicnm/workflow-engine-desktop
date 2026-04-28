// data/paths.rs — 统一数据目录解析（便携模式 + 安装模式）
use std::path::PathBuf;

/// 解析数据目录
///
/// - 便携模式：exe 所在目录下有 `data/` 或 `portable.flag` → exe_dir/data/
/// - 安装模式：系统 AppData (%APPDATA%/Roaming on Windows)
pub fn resolve_data_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let portable = dir.join("data");
            if portable.exists() || dir.join("portable.flag").exists() {
                return portable;
            }
        }
    }
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("workflow-engine")
}

/// 解析日志目录
pub fn resolve_log_dir() -> PathBuf {
    resolve_data_dir().join("logs")
}
