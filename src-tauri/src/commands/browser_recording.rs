// commands/browser_recording.rs — 浏览器录制控制命令
//
// 提供 Tauri 前端可直接调用的录制开始/停止/元素选取命令，
// 复用 crate::nodes::browser::send_sidecar_action 与录制转换器。

use crate::engine::recording_converter::{self, RecordedAction};

/// 开始浏览器录制
///
/// 前端调用后，sidecar 浏览器开始记录用户的交互操作（点击、输入、导航等）。
/// 如果传入 url，会先导航到该页面再开始录制。
#[tauri::command]
pub async fn browser_recording_start(url: Option<String>) -> Result<serde_json::Value, String> {
    // 如果提供了 url，先导航到目标页面
    if let Some(ref target_url) = url {
        if !target_url.is_empty() {
            crate::nodes::browser::send_sidecar_action(
                "navigate",
                &serde_json::json!({ "url": target_url, "wait_until": "load" }),
            )
            .await
            .map_err(|e| format!("导航失败: {}", e))?;
        }
    }

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
                .filter_map(|a| {
                    match serde_json::from_value(a.clone()) {
                        Ok(v) => Some(v),
                        Err(e) => { tracing::warn!("跳过无法解析的录制操作: {}", e); None }
                    }
                })
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
/// URL 直接传给 sidecar 的 pick 操作，由 Python 端内部处理导航和等待。
#[tauri::command]
pub async fn browser_pick_element(url: Option<String>) -> Result<serde_json::Value, String> {
    let mut params = serde_json::json!({});
    if let Some(ref target_url) = url {
        if !target_url.is_empty() {
            params["url"] = serde_json::json!(target_url);
        }
    }

    crate::nodes::browser::send_sidecar_action("pick", &params)
        .await
        .map_err(|e| e.to_string())
}

/// 开始连续拾取模式
///
/// 打开浏览器（如未启动），导航到 URL（如提供），注入元素选择器 JS。
/// 之后可反复调用 browser_pick_next 等待用户点选元素。
#[tauri::command]
pub async fn browser_pick_session_start(url: Option<String>) -> Result<serde_json::Value, String> {
    let mut params = serde_json::json!({});
    if let Some(ref target_url) = url {
        if !target_url.is_empty() {
            params["url"] = serde_json::json!(target_url);
        }
    }

    crate::nodes::browser::send_sidecar_action("pick_start", &params)
        .await
        .map_err(|e| e.to_string())
}

/// 连续拾取：等待用户点选下一个元素
///
/// 在 pick_start 之后调用，阻塞直到用户在浏览器中点击一个元素。
/// 返回 { selector: "..." }。浏览器保持打开，可再次调用。
#[tauri::command]
pub async fn browser_pick_next() -> Result<serde_json::Value, String> {
    crate::nodes::browser::send_sidecar_action("pick_next", &serde_json::json!({}))
        .await
        .map_err(|e| e.to_string())
}

/// 结束连续拾取模式
#[tauri::command]
pub async fn browser_pick_session_stop() -> Result<serde_json::Value, String> {
    crate::nodes::browser::send_sidecar_action("pick_stop", &serde_json::json!({}))
        .await
        .map_err(|e| e.to_string())
}
