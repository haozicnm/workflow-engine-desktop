// nodes/file.rs — 文件操作节点（v3: 每个操作独立 executor）
//
// 原 FileNode 使用 action 参数分发，现拆分为：
//   file_read   — 读取文件
//   file_write  — 写入文件
//   file_list   — 列出目录
//   file_delete — 删除文件/目录
//   file_exists — 检查文件存在

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::info;

// ═══════════════════════════════════════
// Shared helpers
// ═══════════════════════════════════════

/// 获取 path 参数
fn get_path(config: &serde_json::Value) -> Result<&str> {
    config.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("缺少 path 参数"))
}

// ═══════════════════════════════════════
// file_read — 读取文件
// ═══════════════════════════════════════

#[derive(Default)]
pub struct FileReadNode;

#[async_trait]
impl NodeExecutor for FileReadNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;
        let encoding = config.get("encoding").and_then(|v| v.as_str()).unwrap_or("text");

        let bytes = tokio::fs::read(path).await
            .map_err(|e| anyhow!("读取文件失败 [{}]: {}", path, e))?;

        info!("读取文件: {} ({} bytes, encoding={})", path, bytes.len(), encoding);

        match encoding {
            "base64" | "raw" => {
                let encoded = BASE64.encode(&bytes);
                Ok(serde_json::json!({
                    "path": path, "encoding": "base64",
                    "size": bytes.len(), "raw": encoded,
                }))
            }
            _ => {
                let content = String::from_utf8(bytes)
                    .map_err(|e| anyhow!("文件不是有效的 UTF-8 文本 [{}]: {}。请使用 encoding: 'base64' 读取二进制文件", path, e))?;
                Ok(serde_json::json!({
                    "path": path, "encoding": "text",
                    "size": content.len(), "content": content,
                }))
            }
        }
    }
}

// ═══════════════════════════════════════
// file_write — 写入文件
// ═══════════════════════════════════════

#[derive(Default)]
pub struct FileWriteNode;

#[async_trait]
impl NodeExecutor for FileWriteNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;
        let content = config.get("content").ok_or_else(|| anyhow!("write 缺少 content 参数"))?;
        let encoding = config.get("encoding").and_then(|v| v.as_str()).unwrap_or("text");

        let bytes: Vec<u8> = match encoding {
            "base64" => {
                let b64_str = content.as_str().ok_or_else(|| anyhow!("base64 编码需要 content 为字符串"))?;
                BASE64.decode(b64_str).map_err(|e| anyhow!("base64 解码失败: {}", e))?
            }
            _ => match content {
                serde_json::Value::String(s) => s.as_bytes().to_vec(),
                other => other.to_string().as_bytes().to_vec(),
            },
        };

        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| anyhow!("创建父目录失败 [{}]: {}", parent.display(), e))?;
            }
        }

        tokio::fs::write(path, &bytes).await
            .map_err(|e| anyhow!("写入文件失败 [{}]: {}", path, e))?;

        info!("写入文件: {} ({} bytes)", path, bytes.len());
        Ok(serde_json::json!({ "path": path, "size": bytes.len(), "written": true }))
    }
}

// ═══════════════════════════════════════
// file_list — 列出目录
// ═══════════════════════════════════════

#[derive(Default)]
pub struct FileListNode;

#[async_trait]
impl NodeExecutor for FileListNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;
        let recursive = config.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);
        let extensions: Option<Vec<&str>> = config.get("extensions")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect());

        let mut entries: Vec<serde_json::Value> = Vec::new();
        collect_entries(path, path, recursive, &extensions, &mut entries).await?;

        Ok(serde_json::json!({
            "path": path, "count": entries.len(), "files": entries,
        }))
    }
}

async fn collect_entries(
    base: &str, current: &str, recursive: bool,
    extensions: &Option<Vec<&str>>, entries: &mut Vec<serde_json::Value>,
) -> Result<()> {
    let mut read_dir = tokio::fs::read_dir(current).await
        .map_err(|e| anyhow!("读取目录失败 [{}]: {}", current, e))?;

    while let Some(entry) = read_dir.next_entry().await
        .map_err(|e| anyhow!("遍历目录失败 [{}]: {}", current, e))?
    {
        let entry_path = entry.path();
        let metadata = entry.metadata().await
            .map_err(|e| anyhow!("获取元数据失败 [{}]: {}", entry_path.display(), e))?;

        let relative = entry_path.strip_prefix(base)
            .unwrap_or(&entry_path).to_string_lossy().to_string();

        let ext = entry_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if let Some(ref exts) = extensions {
            if !exts.is_empty() && metadata.is_file() && !exts.contains(&ext) {
                if !recursive || !metadata.is_dir() { continue; }
            }
        }

        entries.push(serde_json::json!({
            "name": entry.file_name().to_string_lossy(),
            "path": relative,
            "is_dir": metadata.is_dir(),
            "size": if metadata.is_file() { metadata.len() } else { 0 },
            "modified": metadata.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
        }));

        if recursive && metadata.is_dir() {
            let sub = entry_path.to_string_lossy().to_string();
            Box::pin(collect_entries(base, &sub, recursive, extensions, entries)).await?;
        }
    }
    Ok(())
}

// ═══════════════════════════════════════
// file_delete — 删除文件/目录
// ═══════════════════════════════════════

#[derive(Default)]
pub struct FileDeleteNode;

#[async_trait]
impl NodeExecutor for FileDeleteNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;
        let meta = tokio::fs::metadata(path).await
            .map_err(|e| anyhow!("文件不存在 [{}]: {}", path, e))?;

        if meta.is_dir() {
            let recursive = config.get("recursive").and_then(|v| v.as_bool()).unwrap_or(false);
            if !recursive {
                return Err(anyhow!("删除目录需要 recursive: true [{}]", path));
            }
            tokio::fs::remove_dir_all(path).await
                .map_err(|e| anyhow!("删除目录失败 [{}]: {}", path, e))?;
            info!("删除目录: {}", path);
        } else {
            tokio::fs::remove_file(path).await
                .map_err(|e| anyhow!("删除文件失败 [{}]: {}", path, e))?;
            info!("删除文件: {}", path);
        }

        Ok(serde_json::json!({ "path": path, "deleted": true }))
    }
}

// ═══════════════════════════════════════
// file_exists — 检查文件存在
// ═══════════════════════════════════════

#[derive(Default)]
pub struct FileExistsNode;

#[async_trait]
impl NodeExecutor for FileExistsNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;
        let exists = tokio::fs::try_exists(path).await.unwrap_or(false);

        if exists {
            let meta = tokio::fs::metadata(path).await.ok();
            Ok(serde_json::json!({
                "path": path, "exists": true,
                "is_dir": meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
                "size": meta.as_ref().map(|m| m.len()).unwrap_or(0),
            }))
        } else {
            Ok(serde_json::json!({ "path": path, "exists": false }))
        }
    }
}
