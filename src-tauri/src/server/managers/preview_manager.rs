// server/managers/preview_manager.rs — 预览 handler
//
// 从 handlers.rs 提取的预览相关 handler 函数和类型。

use axum::{
    extract::Path,
    response::{Response, Json},
    http::StatusCode,
};
use serde::Deserialize;

use crate::server::handlers::{ok_response, err_response};

// ═══════════════════════════════════════════════════════════
// Request body types
// ═══════════════════════════════════════════════════════════

#[derive(Debug, Deserialize)]
pub struct PreviewExcelBody {
    pub path: String,
    pub sheet: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PreviewWordBody {
    pub path: String,
}

#[derive(Debug, Deserialize)]
pub struct WebScrapePreviewBody {
    pub url: String,
    pub headless: Option<bool>,
    pub viewport_width: Option<u32>,
    pub viewport_height: Option<u32>,
}

// ═══════════════════════════════════════════════════════════
// 预览 handler
// ═══════════════════════════════════════════════════════════

pub async fn preview_excel(
    Json(body): Json<PreviewExcelBody>,
) -> Response {
    let config = serde_json::json!({
        "path": &body.path,
        "sheet": body.sheet,
        "action": "read",
    });
    match crate::nodes::excel::excel_read(&body.path, &config).await {
        Ok(v) => ok_response(v),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Excel 预览失败: {e}")),
    }
}

pub async fn preview_word(
    Json(body): Json<PreviewWordBody>,
) -> Response {
    match crate::nodes::word::word_read(&body.path).await {
        Ok(v) => ok_response(v),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("Word 预览失败: {e}")),
    }
}

pub async fn get_trajectory(
    Path(run_id): Path<String>,
) -> Response {
    let trajectory = crate::engine::preview::read_trajectory(&run_id);
    ok_response(trajectory)
}

pub async fn get_bundle_files(
    Path((run_id, step_id)): Path<(String, String)>,
) -> Response {
    let bundle_dir = crate::engine::preview::preview_dir(&run_id).join("bundles").join(&step_id);
    if !bundle_dir.exists() {
        return ok_response(Vec::<String>::new());
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
        Err(e) => return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("读取 bundle 目录失败: {e}")),
    }
    files.sort();
    ok_response(files)
}

pub async fn read_bundle_file(
    Path((run_id, step_id, filename)): Path<(String, String, String)>,
) -> Response {
    let preview_dir = crate::engine::preview::preview_dir(&run_id);
    let path = preview_dir.join("bundles").join(&step_id).join(&filename);
    if !path.starts_with(&preview_dir) {
        return err_response(StatusCode::FORBIDDEN, "非法文件路径");
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => ok_response(serde_json::json!({ "content": content })),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("读取文件失败: {e}")),
    }
}

// ═══════════════════════════════════════════════════════════
// Web scrape preview handler
// ═══════════════════════════════════════════════════════════

pub async fn web_scrape_preview(
    Json(body): Json<WebScrapePreviewBody>,
) -> Response {
    use crate::nodes::browser;

    let headless = body.headless.unwrap_or(true);

    let mut launch_params = serde_json::json!({
        "headless": headless,
        "channel": "auto",
    });
    if let (Some(w), Some(h)) = (body.viewport_width, body.viewport_height) {
        launch_params["viewport"] = serde_json::json!({"width": w, "height": h});
    }

    if let Err(e) = browser::send_sidecar_action("launch", &launch_params).await {
        return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("启动浏览器失败: {e}. 预览需要浏览器环境"));
    }

    let preview_params = serde_json::json!({
        "url": body.url,
        "wait_until": "networkidle",
    });

    match browser::send_sidecar_action("preview", &preview_params).await {
        Ok(result) => ok_response(result),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("页面预览失败: {e}")),
    }
}
