use axum::{
    extract::{Multipart, Path, Query},
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
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

pub async fn sidecar_health() -> Response {
    let state = crate::nodes::webbridge::get_state();
    let connected = state.is_connected().await;
    let info = state.get_info().await;
    ok_response(serde_json::json!({
        "webbridge_connected": connected,
        "webbridge_info": info,
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
        "capabilities": info.as_ref().map(|i| &i.capabilities).unwrap_or(&vec![]),
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
    config.working_dir = body.working_dir;
    // P1: 可选子配置（前端未传则保留旧值）
    if let Some(t) = body.timeouts {
        config.timeouts = t;
    }
    if let Some(l) = body.logging {
        config.logging = l;
    }
    if let Some(e) = body.execution {
        config.execution = e;
    }
    info!("设置已更新");
    match config.save() {
        Ok(()) => ok_response(serde_json::json!({ "success": true })),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub async fn system_check_browser() -> Response {
    // WebBridge 作为唯一浏览器后端，检查连接状态
    let state = crate::nodes::webbridge::get_state();
    let webbridge_connected = state.is_connected().await;
    let webbridge_info = state.get_info().await;

    ok_response(serde_json::json!({
        "webbridge_connected": webbridge_connected,
        "webbridge_info": webbridge_info,
        "ready": webbridge_connected,
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
        Err(e) => return err_response(StatusCode::BAD_REQUEST, format!("读取上传文件失败: {e}")),
    } {
        let name = field.name().unwrap_or("").to_string();
        if name == "file" || name == "wfplug" {
            file_name = field.file_name().unwrap_or("plugin.wfplug").to_string();
            match field.bytes().await {
                Ok(b) => file_bytes = Some(b.to_vec()),
                Err(e) => {
                    return err_response(StatusCode::BAD_REQUEST, format!("读取文件内容失败: {e}"))
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
            return err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("安装失败: {e}"));
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

pub async fn browser_pick_start(AxumJson(_body): AxumJson<PickStartBody>) -> Response {
    // WebBridge: 请使用 snapshot 功能代替元素拾取
    err_response(
        StatusCode::NOT_IMPLEMENTED,
        "pick_start 已弃用，请使用 WebBridge snapshot 功能代替",
    )
}

pub async fn browser_pick_next() -> Response {
    err_response(
        StatusCode::NOT_IMPLEMENTED,
        "pick_next 已弃用，请使用 WebBridge snapshot 功能代替",
    )
}

pub async fn browser_pick_stop() -> Response {
    err_response(
        StatusCode::NOT_IMPLEMENTED,
        "pick_stop 已弃用，请使用 WebBridge snapshot 功能代替",
    )
}

/// POST /api/browser/snapshot — 获取页面无障碍树快照（带 @eN ref）
pub async fn browser_snapshot() -> Response {
    // 检查 WebBridge 是否可用
    if !crate::nodes::webbridge::is_available().await {
        return err_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "WebBridge 扩展未连接，请安装并刷新页面",
        );
    }
    match crate::nodes::webbridge::send_command("snapshot", serde_json::json!({})).await {
        Ok(val) => ok_response(val),
        Err(e) => err_response(StatusCode::INTERNAL_SERVER_ERROR, format!("snapshot 失败: {}", e)),
    }
}

// ═══════════════════════════════════════════════════════════
// Blocks 自描述 API (P0)
// ═══════════════════════════════════════════════════════════

/// GET /api/blocks?category=core&q=web
/// 列出所有 block 摘要（type, label, category, desc, icon, tags）
/// 支持按分类过滤和关键词搜索
pub async fn blocks_list(Query(params): Query<std::collections::HashMap<String, String>>) -> Response {
    let category_filter = params.get("category").map(|s| s.as_str());
    let query = params.get("q").map(|s| s.as_str());

    let blocks: Vec<serde_json::Value> = if let Some(q) = query {
        // 搜索模式：按标签/名称/描述搜索
        crate::nodes::registry::search_by_tags(q)
            .into_iter()
            .filter(|n| category_filter.is_none_or(|cat| n.category == cat))
            .map(|n| {
                serde_json::json!({
                    "type": n.node_type,
                    "label": n.label,
                    "category": n.category,
                    "desc": n.description,
                    "icon": n.icon,
                    "tags": n.tags,
                })
            })
            .collect()
    } else {
        crate::nodes::registry::all_nodes()
            .into_iter()
            .filter(|n| category_filter.is_none_or(|cat| n.category == cat))
            .map(|n| {
                serde_json::json!({
                    "type": n.node_type,
                    "label": n.label,
                    "category": n.category,
                    "desc": n.description,
                    "icon": n.icon,
                    "tags": n.tags,
                })
            })
            .collect()
    };
    ok_response(serde_json::json!({ "blocks": blocks, "count": blocks.len() }))
}

/// GET /api/blocks/categories
/// 返回所有分类及节点数量
pub async fn blocks_categories() -> Response {
    let categories: Vec<serde_json::Value> = crate::nodes::registry::categories()
        .into_iter()
        .map(|(name, count)| serde_json::json!({ "name": name, "count": count }))
        .collect();
    ok_response(serde_json::json!({ "categories": categories }))
}

/// GET /api/blocks/:type
/// 获取某个 block 的完整详情（含 params schema、tags、validation、visible_when、examples、action_definitions）
pub async fn blocks_get(Path(node_type): Path<String>) -> Response {
    match crate::nodes::registry::get_node(&node_type) {
        Some(manifest) => ok_response(serde_json::json!({
            "type": manifest.node_type,
            "label": manifest.label,
            "category": manifest.category,
            "desc": manifest.description,
            "icon": manifest.icon,
            "is_container": manifest.is_container,
            "inputs": manifest.inputs,
            "outputs": manifest.outputs,
            "params": manifest.params,
            "tags": manifest.tags,
            "action_definitions": manifest.action_definitions,
        })),
        None => err_response(StatusCode::NOT_FOUND, format!("节点类型 '{}' 不存在", node_type)),
    }
}
