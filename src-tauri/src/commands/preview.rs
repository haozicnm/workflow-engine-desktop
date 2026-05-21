// commands/preview.rs — 节点预览 + trajectory/bundle IPC 命令
//
// 为前端 PreviewPanel 提供数据：
//   preview_excel / preview_word — 文件预览
//   get_trajectory / get_bundle_files / read_bundle_file — 执行轨迹 + 快照

use crate::nodes::excel;
use crate::nodes::word;
use crate::engine::preview;
use serde_json::json;

/// 预览 Excel 文件内容（不执行工作流，仅读取数据）
#[tauri::command]
pub async fn preview_excel(
    path: String,
    sheet: Option<String>,
) -> Result<serde_json::Value, String> {
    let config = json!({
        "path": &path,
        "sheet": sheet,
        "action": "read",
    });
    excel::excel_read(&path, &config)
        .await
        .map_err(|e| format!("Excel 预览失败: {}", e))
}

/// 预览 Word 文档内容（不执行工作流，仅读取文本）
#[tauri::command]
pub async fn preview_word(
    path: String,
) -> Result<serde_json::Value, String> {
    word::word_read(&path)
        .await
        .map_err(|e| format!("Word 预览失败: {}", e))
}

/// 获取某次运行的所有步骤预览 (trajectory)
#[tauri::command]
pub fn get_trajectory(run_id: String) -> Result<Vec<preview::StepPreview>, String> {
    Ok(preview::read_trajectory(&run_id))
}

/// 获取某步骤的 bundle 文件列表
#[tauri::command]
pub fn get_bundle_files(run_id: String, step_id: String) -> Result<Vec<String>, String> {
    let bundle_dir = preview::preview_dir(&run_id).join("bundles").join(&step_id);
    if !bundle_dir.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    match std::fs::read_dir(&bundle_dir) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    files.push(name.to_string());
                }
            }
        }
        Err(e) => return Err(format!("读取 bundle 目录失败: {}", e)),
    }
    files.sort();
    Ok(files)
}

/// 读取 bundle 文件内容
#[tauri::command]
pub fn read_bundle_file(run_id: String, step_id: String, filename: String) -> Result<String, String> {
    let path = preview::preview_dir(&run_id)
        .join("bundles")
        .join(&step_id)
        .join(&filename);
    // 安全检查：防止路径遍历攻击
    if !path.starts_with(preview::preview_dir(&run_id)) {
        return Err("非法文件路径".to_string());
    }
    std::fs::read_to_string(&path)
        .map_err(|e| format!("读取文件失败: {}", e))
}
