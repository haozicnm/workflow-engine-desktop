// commands/plugin.rs — 插件系统 Tauri 命令
use tauri::State;
use crate::App;
use crate::engine::plugin_manager;

#[tauri::command]
pub async fn plugin_pick_file() -> Result<Option<String>, String> {
    let handle = rfd::AsyncFileDialog::new()
        .add_filter("插件包", &["wfplug"])
        .pick_file()
        .await;
    Ok(handle.map(|f| f.path().to_string_lossy().to_string()))
}

#[tauri::command]
pub async fn plugin_install(
    app: State<'_, App>,
    wfplug_path: String,
) -> Result<serde_json::Value, String> {
    let path = std::path::Path::new(&wfplug_path);
    let meta = plugin_manager::install_plugin(path)
        .map_err(|e| format!("安装失败: {e}"))?;

    Ok(serde_json::json!({
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

#[tauri::command]
pub async fn plugin_uninstall(
    app: State<'_, App>,
    name: String,
) -> Result<serde_json::Value, String> {
    plugin_manager::uninstall_plugin(&name)
        .map_err(|e| format!("卸载失败: {e}"))?;

    Ok(serde_json::json!({
        "success": true,
        "message": format!("插件 {} 已卸载", name),
    }))
}

#[tauri::command]
pub async fn plugin_list(
    app: State<'_, App>,
) -> Result<serde_json::Value, String> {
    let plugins = plugin_manager::list_plugins()
        .map_err(|e| format!("获取插件列表失败: {e}"))?;

    let items: Vec<serde_json::Value> = plugins.iter().map(|p| {
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
    }).collect();

    Ok(serde_json::json!({ "plugins": items }))
}
