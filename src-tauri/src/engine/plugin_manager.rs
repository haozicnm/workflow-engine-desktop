// engine/plugin_manager.rs — Workflow Engine 插件系统
//
// .wfplug 包安装/卸载/列表管理
// 所有数据统一在 ~/.config/workflow-engine/ 下

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

/// 统一配置目录：~/.config/workflow-engine/
pub fn workflow_engine_dir() -> PathBuf {
    if let Ok(d) = std::env::var("WORKFLOW_ENGINE_HOME") {
        return PathBuf::from(d);
    }
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("workflow-engine")
}

pub fn plugins_dir() -> PathBuf {
    workflow_engine_dir().join("plugins")
}

pub fn mcp_external_path() -> PathBuf {
    workflow_engine_dir().join("mcp-external.json")
}

pub fn templates_library_dir() -> PathBuf {
    workflow_engine_dir().join("workflows").join("library")
}

/// 旧路径（迁移用）
pub fn legacy_hermes_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hermes")
}

// ── Data types ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub mcp_mappings: Vec<McpMappingEntry>,
    #[serde(default)]
    pub templates: Vec<String>,
    #[serde(default)]
    pub requires: Option<RequiresBlock>,
    #[serde(default)]
    pub dependencies: Option<DependenciesBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpMappingEntry {
    #[serde(rename = "type")]
    pub node_type: String,
    pub script: String,
    pub tool: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequiresBlock {
    #[serde(default, rename = "workflow_engine")]
    pub workflow_engine: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependenciesBlock {
    #[serde(default)]
    pub debian: Option<DebianDeps>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebianDeps {
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default, rename = "offline_dir")]
    pub offline_dir: Option<String>,
}

/// mcp-external.json 的条目（扩展字段 _plugin 追踪来源）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpExternalEntry {
    #[serde(rename = "type")]
    pub node_type: String,
    pub script: String,
    pub tool: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub _plugin: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpExternalConfig {
    #[serde(default)]
    pub mappings: Vec<McpExternalEntry>,
}

// ── Install ────────────────────────────────────────────────

pub fn install_plugin(wfplug_path: &Path) -> Result<PluginMeta> {
    // 1. 解压 .wfplug (zip)
    let file = fs::File::open(wfplug_path)
        .with_context(|| format!("无法打开插件包: {}", wfplug_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| "无法解析 .wfplug 包（不是有效的 zip 文件）")?;

    // 2. 读取 plugin.json
    let mut plugin_json_bytes = Vec::new();
    let plugin_json_entry = archive.by_name("plugin.json")
        .with_context(|| "插件包缺少 plugin.json")?;
    std::io::copy(&mut plugin_json_entry.take(10_000_000), &mut plugin_json_bytes)?;
    let meta: PluginMeta = serde_json::from_slice(&plugin_json_bytes)
        .with_context(|| "plugin.json 格式错误")?;

    // 3. 校验必填字段
    if meta.name.is_empty() || meta.version.is_empty() {
        bail!("plugin.json 缺少 name 或 version");
    }

    // 4. 版本兼容检查
    if let Some(ref req) = meta.requires {
        if let Some(ref we_req) = req.workflow_engine {
            // 简单版本检查：当前版本 >= 要求版本
            let current = env!("CARGO_PKG_VERSION");
            if !version_ge(current, we_req) {
                bail!(
                    "插件 {} 需要 workflow-engine >= {}，当前版本: {}",
                    meta.name, we_req, current
                );
            }
        }
    }

    let plugin_dir = plugins_dir().join(&meta.name);

    // 5. 检查是否已安装（覆盖模式：先卸载旧版本）
    if plugin_dir.exists() {
        tracing::info!("插件 {} 已存在，覆盖安装", meta.name);
        fs::remove_dir_all(&plugin_dir)
            .with_context(|| format!("无法删除旧插件目录: {}", plugin_dir.display()))?;
    }

    fs::create_dir_all(&plugin_dir)?;

    // 6. 解压所有文件到插件目录
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let entry_path = entry.mangled_name();
        let relative_path = entry_path.to_string_lossy();

        // 跳过目录项和 plugin.json（已读）
        if entry.is_dir() || relative_path == "plugin.json" {
            continue;
        }

        let dest = plugin_dir.join(relative_path.as_ref());
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut out = fs::File::create(&dest)?;
        std::io::copy(&mut entry, &mut out)?;
    }

    // 7. 安装离线依赖（如果有）
    if let Some(ref deps) = meta.dependencies {
        if let Some(ref deb) = deps.debian {
            if let Some(ref offline) = deb.offline_dir {
                let offline_path = plugin_dir.join(offline);
                if offline_path.exists() {
                    install_offline_debs(&offline_path, &deb.packages)?;
                }
            }
        }
    }

    // 8. 合并 MCP 映射到 mcp-external.json
    merge_mcp_mappings(&meta)?;

    // 9. 导入模板到 workflows/library/
    import_templates(&meta, &plugin_dir)?;

    tracing::info!("插件 {} v{} 安装成功", meta.name, meta.version);
    Ok(meta)
}

// ── Uninstall ──────────────────────────────────────────────

pub fn uninstall_plugin(name: &str) -> Result<()> {
    let plugin_dir = plugins_dir().join(name);

    if !plugin_dir.exists() {
        bail!("插件 {} 未安装", name);
    }

    // 1. 读取 plugin.json 获取模板清单
    let plugin_json_path = plugin_dir.join("plugin.json");
    let meta: PluginMeta = if plugin_json_path.exists() {
        serde_json::from_str(&fs::read_to_string(&plugin_json_path)?)?
    } else {
        PluginMeta {
            name: name.to_string(),
            version: String::new(),
            title: String::new(),
            description: String::new(),
            author: String::new(),
            icon: String::new(),
            mcp_mappings: vec![],
            templates: vec![],
            requires: None,
            dependencies: None,
        }
    };

    // 2. 从 mcp-external.json 移除该插件的 MCP 映射
    remove_mcp_mappings(name)?;

    // 3. 删除该插件导入的模板
    let library_dir = templates_library_dir();
    for template in &meta.templates {
        let tmpl_path = library_dir.join(template);
        if tmpl_path.exists() {
            fs::remove_file(&tmpl_path)?;
        }
    }

    // 4. 删除插件目录
    fs::remove_dir_all(&plugin_dir)?;

    tracing::info!("插件 {} 已卸载", name);
    Ok(())
}

// ── List ───────────────────────────────────────────────────

pub fn list_plugins() -> Result<Vec<PluginMeta>> {
    let dir = plugins_dir();
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut plugins = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            let plugin_json = entry.path().join("plugin.json");
            if plugin_json.exists() {
                if let Ok(content) = fs::read_to_string(&plugin_json) {
                    if let Ok(meta) = serde_json::from_str::<PluginMeta>(&content) {
                        plugins.push(meta);
                    }
                }
            }
        }
    }
    Ok(plugins)
}

// ── MCP 合并 ───────────────────────────────────────────────

fn merge_mcp_mappings(meta: &PluginMeta) -> Result<()> {
    if meta.mcp_mappings.is_empty() {
        return Ok(());
    }

    let path = mcp_external_path();
    let mut config: McpExternalConfig = if path.exists() {
        serde_json::from_str(&fs::read_to_string(&path)?)?
    } else {
        McpExternalConfig { mappings: Vec::new() }
    };

    // 移除该插件旧的映射（覆盖安装）
    config.mappings.retain(|m| m._plugin.as_deref() != Some(&meta.name));

    // 添加新映射
    for mapping in &meta.mcp_mappings {
        config.mappings.push(McpExternalEntry {
            node_type: mapping.node_type.clone(),
            script: mapping.script.clone(),
            tool: mapping.tool.clone(),
            _plugin: Some(meta.name.clone()),
        });
    }

    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(&path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

fn remove_mcp_mappings(plugin_name: &str) -> Result<()> {
    let path = mcp_external_path();
    if !path.exists() {
        return Ok(());
    }

    let mut config: McpExternalConfig = serde_json::from_str(&fs::read_to_string(&path)?)?;
    config.mappings.retain(|m| m._plugin.as_deref() != Some(plugin_name));
    fs::write(&path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}

// ── 模板导入 ──────────────────────────────────────────────

fn import_templates(meta: &PluginMeta, plugin_dir: &Path) -> Result<()> {
    if meta.templates.is_empty() {
        return Ok(());
    }

    let library_dir = templates_library_dir();
    fs::create_dir_all(&library_dir)?;

    let templates_dir = plugin_dir.join("templates");
    if !templates_dir.exists() {
        return Ok(());
    }

    for template in &meta.templates {
        let src = templates_dir.join(template);
        if !src.exists() {
            tracing::warn!("模板文件不存在: {}", src.display());
            continue;
        }
        let dst = library_dir.join(template);
        fs::copy(&src, &dst)
            .with_context(|| format!("导入模板失败: {}", template))?;
    }

    Ok(())
}

// ── 离线依赖安装 ──────────────────────────────────────────

fn install_offline_debs(offline_dir: &Path, packages: &[String]) -> Result<()> {
    if !offline_dir.exists() {
        return Ok(());
    }

    // 检查是否已安装
    let mut all_installed = true;
    for pkg in packages {
        let status = std::process::Command::new("dpkg")
            .args(["-l", pkg])
            .output();
        if let Ok(output) = status {
            if output.status.success() {
                continue; // 已安装
            }
        }
        all_installed = false;
        break;
    }

    if all_installed {
        tracing::info!("离线依赖已全部安装，跳过");
        return Ok(());
    }

    // 按依赖顺序安装
    let debs: Vec<PathBuf> = fs::read_dir(offline_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "deb").unwrap_or(false))
        .collect();

    // lib → common → server → tools 顺序
    let order = ["libwbclient", "samba-libs", "samba-common", "samba_", "samba-common-bin"];
    let mut sorted = Vec::new();
    for prefix in &order {
        for deb in &debs {
            let name = deb.file_name().unwrap_or_default().to_string_lossy();
            if name.starts_with(prefix) && !sorted.contains(deb) {
                sorted.push(deb.clone());
            }
        }
    }

    for deb in &sorted {
        let output = std::process::Command::new("sudo")
            .args(["dpkg", "-i"])
            .arg(deb)
            .output()?;
        if !output.status.success() {
            tracing::warn!(
                "dpkg 安装 {} 失败: {}",
                deb.display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }

    // 修复依赖
    let _ = std::process::Command::new("sudo")
        .args(["apt-get", "install", "-f", "-y", "-qq"])
        .output();

    Ok(())
}

// ── 版本比较 ──────────────────────────────────────────────

fn version_ge(current: &str, required: &str) -> bool {
    let cur_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
    let req_parts: Vec<u32> = required.split('.').filter_map(|s| s.parse().ok()).collect();
    for i in 0..req_parts.len().max(cur_parts.len()) {
        let c = cur_parts.get(i).copied().unwrap_or(0);
        let r = req_parts.get(i).copied().unwrap_or(0);
        if c > r { return true; }
        if c < r { return false; }
    }
    true // equal
}

// ── 路径迁移 ──────────────────────────────────────────────

/// 首次启动时自动迁移 ~/.hermes/mcp-external.json → ~/.config/workflow-engine/
pub fn migrate_from_legacy() -> Result<()> {
    let legacy_mcp = legacy_hermes_dir().join("mcp-external.json");
    let new_mcp = mcp_external_path();

    // 如果新路径已有 mcp-external.json，不覆盖
    if new_mcp.exists() {
        return Ok(());
    }

    // 如果旧路径有 mcp-external.json，迁移
    if legacy_mcp.exists() {
        tracing::info!("检测到旧版 mcp-external.json，迁移至 {}", new_mcp.display());
        if let Some(parent) = new_mcp.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&legacy_mcp, &new_mcp)?;

        // 清理 _plugin 字段（旧数据可能没有）
        if let Ok(content) = fs::read_to_string(&new_mcp) {
            if let Ok(config) = serde_json::from_str::<McpExternalConfig>(&content) {
                fs::write(&new_mcp, serde_json::to_string_pretty(&config)?)?;
            }
        }
    }

    // 迁移 workflows/library/
    let legacy_library = legacy_hermes_dir().join("workflows").join("library");
    let new_library = templates_library_dir();
    if legacy_library.exists() && !new_library.exists() {
        tracing::info!("迁移模板库: {} → {}", legacy_library.display(), new_library.display());
        fs::create_dir_all(new_library.parent().unwrap())?;
        copy_dir_recursive(&legacy_library, &new_library)?;
    }

    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let dest = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &dest)?;
        } else {
            fs::copy(&entry.path(), &dest)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_ge() {
        assert!(version_ge("7.0.0", "7.0.0"));
        assert!(version_ge("7.1.0", "7.0.0"));
        assert!(version_ge("8.0.0", "7.0.0"));
        assert!(!version_ge("6.0.0", "7.0.0"));
        assert!(!version_ge("7.0.0", "7.1.0"));
        assert!(version_ge("7.0.1", "7.0.0"));
    }

    #[test]
    fn test_workflow_engine_dir() {
        let dir = workflow_engine_dir();
        assert!(dir.ends_with("workflow-engine"));
    }
}
