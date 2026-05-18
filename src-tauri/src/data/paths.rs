// data/paths.rs — 统一数据目录解析（便携模式 + 安装模式）
use std::path::PathBuf;

/// 解析数据目录
///
/// - 便携模式：exe 所在目录下有 `data/` 或 `portable.flag` → exe_dir/data/
/// - 安装模式：系统 AppData (%APPDATA%/Roaming on Windows)
///
/// 目录名使用 bundle identifier (com.workflow-engine.desktop)
/// 与 NSIS 卸载程序的「清除数据」路径一致。
pub fn resolve_data_dir() -> PathBuf {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let portable = dir.join("data");
            if portable.exists() || dir.join("portable.flag").exists() {
                return portable;
            }
        }
    }
    let new_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("com.workflow-engine.desktop");

    // 迁移：从旧目录 (workflow-engine) 搬到新目录
    let old_dir = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("workflow-engine");
    if old_dir.exists() && !new_dir.exists() {
        if let Some(parent) = new_dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        // 尝试重命名；如果跨文件系统失败则回退新目录不迁移
        let _ = std::fs::rename(&old_dir, &new_dir);
    }

    new_dir
}

/// 解析日志目录
pub fn resolve_log_dir() -> PathBuf {
    resolve_data_dir().join("logs")
}
