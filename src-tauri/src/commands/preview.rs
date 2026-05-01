// commands/preview.rs — 节点预览命令
//
// 为前端 PreviewPanel 提供文件预览数据：
//   preview_excel(path, sheet?) → { sheet, rows, cols, data: [[...]] }
//   preview_word(path)          → { paragraphs: [...], paragraph_count }

use crate::nodes::excel;
use crate::nodes::word;
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
