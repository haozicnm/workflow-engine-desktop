// nodes/file.rs — 文件操作节点
//
// 支持操作：
//   read    读取文件:  {action: "read", path: "/tmp/a.txt", encoding: "text"|"base64"}
//   write   写入文件:  {action: "write", path: "/tmp/a.txt", content: "hello"}
//   list    列出目录:  {action: "list", path: "/tmp"}
//   delete  删除文件:  {action: "delete", path: "/tmp/a.txt"}
//   exists  检查存在:  {action: "exists", path: "/tmp/a.txt"}
//
// 支持模板变量 {{xxx}}（path 和 content 由 executor 在执行前 resolve）

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::info;

#[derive(Default)]
pub struct FileNode;

#[async_trait]
impl NodeExecutor for FileNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("文件节点缺少 action 参数"))?;

        match action {
            "read" => file_read(config).await,
            "write" => file_write(config).await,
            "list" => file_list(config).await,
            "delete" => file_delete(config).await,
            "exists" => file_exists(config).await,
            _ => Err(anyhow!(
                "未知的文件操作: {}（支持 read/write/list/delete/exists）",
                action
            )),
        }
    }
}

/// 读取文件内容
async fn file_read(config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = config.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("read 操作缺少 path 参数"))?;

    let encoding = config.get("encoding")
        .and_then(|v| v.as_str())
        .unwrap_or("text");

    let bytes = tokio::fs::read(path).await
        .map_err(|e| anyhow!("读取文件失败 [{}]: {}", path, e))?;

    info!("读取文件: {} ({} bytes, encoding={})", path, bytes.len(), encoding);

    match encoding {
        "base64" | "raw" => {
            let encoded = BASE64.encode(&bytes);
            Ok(serde_json::json!({
                "action": "read",
                "path": path,
                "encoding": "base64",
                "size": bytes.len(),
                "raw": encoded,
            }))
        }
        _ => {
            let content = String::from_utf8(bytes)
                .map_err(|e| anyhow!("文件不是有效的 UTF-8 文本 [{}]: {}。请使用 encoding: 'base64' 读取二进制文件", path, e))?;
            Ok(serde_json::json!({
                "action": "read",
                "path": path,
                "encoding": "text",
                "size": content.len(),
                "file_content": content,
            }))
        }
    }
}

/// 写入文件内容
async fn file_write(config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = config.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("write 操作缺少 path 参数"))?;

    let content = config.get("content")
        .ok_or_else(|| anyhow!("write 操作缺少 content 参数"))?;

    // 支持文本或 base64 编码内容
    let encoding = config.get("encoding")
        .and_then(|v| v.as_str())
        .unwrap_or("text");

    let bytes: Vec<u8> = match encoding {
        "base64" => {
            let b64_str = content.as_str()
                .ok_or_else(|| anyhow!("base64 编码需要 content 为字符串"))?;
            BASE64.decode(b64_str)
                .map_err(|e| anyhow!("base64 解码失败: {}", e))?
        }
        _ => {
            // 如果 content 是字符串直接转 bytes，否则 JSON 序列化
            match content {
                serde_json::Value::String(s) => s.as_bytes().to_vec(),
                other => other.to_string().as_bytes().to_vec(),
            }
        }
    };

    // 确保父目录存在
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| anyhow!("创建父目录失败 [{}]: {}", parent.display(), e))?;
        }
    }

    tokio::fs::write(path, &bytes).await
        .map_err(|e| anyhow!("写入文件失败 [{}]: {}", path, e))?;

    info!("写入文件: {} ({} bytes)", path, bytes.len());

    Ok(serde_json::json!({
        "action": "write",
        "path": path,
        "size": bytes.len(),
        "written": true,
    }))
}

/// 列出目录内容
async fn file_list(config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = config.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("list 操作缺少 path 参数"))?;

    let recursive = config.get("recursive")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let extensions: Option<Vec<&str>> = config.get("extensions")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter()
            .filter_map(|v| v.as_str())
            .collect());

    let mut entries: Vec<serde_json::Value> = Vec::new();
    collect_entries(path, path, recursive, &extensions, &mut entries).await?;

    Ok(serde_json::json!({
        "action": "list",
        "path": path,
        "count": entries.len(),
        "files": entries,
    }))
}

async fn collect_entries(
    base: &str,
    current: &str,
    recursive: bool,
    extensions: &Option<Vec<&str>>,
    entries: &mut Vec<serde_json::Value>,
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
            .unwrap_or(&entry_path)
            .to_string_lossy()
            .to_string();

        let ext = entry_path.extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        // 按扩展名过滤
        if let Some(ref exts) = extensions {
            if !exts.is_empty() && metadata.is_file() && !exts.contains(&ext) {
                if recursive && metadata.is_dir() {
                    // 目录始终探索
                } else {
                    continue;
                }
            }
        }

        let entry_info = serde_json::json!({
            "name": entry.file_name().to_string_lossy(),
            "path": relative,
            "is_dir": metadata.is_dir(),
            "size": if metadata.is_file() { metadata.len() } else { 0 },
            "modified": metadata.modified().ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs()),
        });

        entries.push(entry_info);

        if recursive && metadata.is_dir() {
            let sub = entry_path.to_string_lossy().to_string();
            Box::pin(collect_entries(base, &sub, recursive, extensions, entries)).await?;
        }
    }

    Ok(())
}

/// 删除文件或目录
async fn file_delete(config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = config.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("delete 操作缺少 path 参数"))?;

    let meta = tokio::fs::metadata(path).await
        .map_err(|e| anyhow!("文件不存在 [{}]: {}", path, e))?;

    if meta.is_dir() {
        let recursive = config.get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
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

    Ok(serde_json::json!({
        "action": "delete",
        "path": path,
        "deleted": true,
    }))
}

/// 检查文件是否存在
async fn file_exists(config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = config.get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("exists 操作缺少 path 参数"))?;

    let exists = tokio::fs::try_exists(path).await.unwrap_or(false);

    let result = if exists {
        let meta = tokio::fs::metadata(path).await.ok();
        serde_json::json!({
            "action": "exists",
            "path": path,
            "exists": true,
            "is_dir": meta.as_ref().map(|m| m.is_dir()).unwrap_or(false),
            "size": meta.as_ref().map(|m| m.len()).unwrap_or(0),
        })
    } else {
        serde_json::json!({
            "action": "exists",
            "path": path,
            "exists": false,
        })
    };

    Ok(result)
}
