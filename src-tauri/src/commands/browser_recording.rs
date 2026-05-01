// commands/browser_recording.rs — 浏览器录制控制命令
//
// 提供 Tauri 前端可直接调用的录制开始/停止/元素选取命令，
// 复用 crate::nodes::browser::send_sidecar_action 与录制转换器。

use crate::engine::recording_converter::{self, RecordedAction};

/// 开始浏览器录制
///
/// 前端调用后，sidecar 浏览器开始记录用户的交互操作（点击、输入、导航等）。
#[tauri::command]
pub async fn browser_recording_start() -> Result<serde_json::Value, String> {
    crate::nodes::browser::send_sidecar_action("recording_start", &serde_json::json!({}))
        .await
        .map_err(|e| e.to_string())
}

/// 停止浏览器录制并转换为工作流 JSON
///
/// 停止录制后获取所有记录的操作，通过 recording_converter 转换为
/// 结构化的工作流 JSON（包含 YAML、步骤概要等），供前端渲染预览。
#[tauri::command]
pub async fn browser_recording_stop() -> Result<serde_json::Value, String> {
    let result = crate::nodes::browser::send_sidecar_action("recording_stop", &serde_json::json!({}))
        .await
        .map_err(|e| e.to_string())?;

    // 从 sidecar 返回中提取操作列表
    let actions = result
        .get("actions")
        .cloned()
        .unwrap_or_else(|| serde_json::json!([]));

    // 反序列化为 RecordedAction 列表
    let recorded_actions: Vec<RecordedAction> = actions
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|a| serde_json::from_value(a.clone()).ok())
                .collect()
        })
        .unwrap_or_default();

    // 转换为 v2.0 JSON 工作流（nodes + edges）
    let workflow = recording_converter::convert_to_json(&recorded_actions, "browser");

    Ok(workflow)
}

/// 浏览器元素选取器
///
/// 激活浏览器的元素选择模式，用户在浏览器中点击元素后返回该元素的 CSS 选择器。
#[tauri::command]
pub async fn browser_pick_element() -> Result<serde_json::Value, String> {
    crate::nodes::browser::send_sidecar_action("pick", &serde_json::json!({}))
        .await
        .map_err(|e| e.to_string())
}
