// nodes/file.rs — 文件操作节点（v3: 每个操作独立 executor）
//
// 原 FileNode 使用 action 参数分发，现拆分为：
//   file_read   — 读取文件
//   file_write  — 写入文件
//   file_list   — 列出目录
//   file_delete — 删除文件/目录
//   file_exists — 检查文件存在

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::sync::Arc;
use tracing::info;

// ═══════════════════════════════════════
// Shared helpers
// ═══════════════════════════════════════

/// 获取 path 参数
fn get_path(config: &serde_json::Value) -> Result<&str> {
    config
        .get("path")
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
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_read".into(),
            version: "1.0".into(),
            display_name: "读取文件".into(),
            description: "读取文件内容为文本".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object", "required": ["path"] }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;
        let encoding = config
            .get("encoding")
            .and_then(|v| v.as_str())
            .unwrap_or("text");

        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| anyhow!("读取文件失败 [{}]: {}", path, e))?;

        info!(
            "读取文件: {} ({} bytes, encoding={})",
            path,
            bytes.len(),
            encoding
        );

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
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_write".into(),
            version: "1.0".into(),
            display_name: "写入文件".into(),
            description: "写入文本内容到文件".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object", "required": ["path"] }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;
        let content = config
            .get("content")
            .ok_or_else(|| anyhow!("write 缺少 content 参数"))?;
        let encoding = config
            .get("encoding")
            .and_then(|v| v.as_str())
            .unwrap_or("text");

        let bytes: Vec<u8> = match encoding {
            "base64" => {
                let b64_str = content
                    .as_str()
                    .ok_or_else(|| anyhow!("base64 编码需要 content 为字符串"))?;
                BASE64
                    .decode(b64_str)
                    .map_err(|e| anyhow!("base64 解码失败: {}", e))?
            }
            _ => match content {
                serde_json::Value::String(s) => s.as_bytes().to_vec(),
                other => other.to_string().as_bytes().to_vec(),
            },
        };

        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| anyhow!("创建父目录失败 [{}]: {}", parent.display(), e))?;
            }
        }

        tokio::fs::write(path, &bytes)
            .await
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
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_list".into(),
            version: "1.0".into(),
            display_name: "列出文件".into(),
            description: "列出目录中的文件".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object", "required": ["path"] }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;
        let recursive = config
            .get("recursive")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let extensions: Option<Vec<&str>> = config
            .get("extensions")
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
    base: &str,
    current: &str,
    recursive: bool,
    extensions: &Option<Vec<&str>>,
    entries: &mut Vec<serde_json::Value>,
) -> Result<()> {
    let mut read_dir = tokio::fs::read_dir(current)
        .await
        .map_err(|e| anyhow!("读取目录失败 [{}]: {}", current, e))?;

    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| anyhow!("遍历目录失败 [{}]: {}", current, e))?
    {
        let entry_path = entry.path();
        let metadata = entry
            .metadata()
            .await
            .map_err(|e| anyhow!("获取元数据失败 [{}]: {}", entry_path.display(), e))?;

        let relative = entry_path
            .strip_prefix(base)
            .unwrap_or(&entry_path)
            .to_string_lossy()
            .to_string();

        let ext = entry_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        if let Some(ref exts) = extensions {
            if !exts.is_empty()
                && metadata.is_file()
                && !exts.contains(&ext)
                && (!recursive || !metadata.is_dir())
            {
                continue;
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
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_delete".into(),
            version: "1.0".into(),
            display_name: "删除文件".into(),
            description: "删除指定文件".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object", "required": ["path"] }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;
        let meta = tokio::fs::metadata(path)
            .await
            .map_err(|e| anyhow!("文件不存在 [{}]: {}", path, e))?;

        if meta.is_dir() {
            let recursive = config
                .get("recursive")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if !recursive {
                return Err(anyhow!("删除目录需要 recursive: true [{}]", path));
            }
            tokio::fs::remove_dir_all(path)
                .await
                .map_err(|e| anyhow!("删除目录失败 [{}]: {}", path, e))?;
            info!("删除目录: {}", path);
        } else {
            tokio::fs::remove_file(path)
                .await
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
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_exists".into(),
            version: "1.0".into(),
            display_name: "文件存在".into(),
            description: "检查文件是否存在".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object", "required": ["path"] }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
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

// ═══════════════════════════════════════
// file_append — 追加内容到文件末尾
// ═══════════════════════════════════════

#[derive(Default)]
pub struct FileAppendNode;

#[async_trait]
impl NodeExecutor for FileAppendNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_append".into(),
            version: "1.0".into(),
            display_name: "追加文件".into(),
            description: "追加内容到文件末尾".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object", "required": ["path"] }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;
        let content = config
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("append 缺少 content 参数"))?;

        // 确保父目录存在
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| anyhow!("创建父目录失败 [{}]: {}", parent.display(), e))?;
            }
        }

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .await
            .map_err(|e| anyhow!("打开文件失败 [{}]: {}", path, e))?;

        file.write_all(content.as_bytes())
            .await
            .map_err(|e| anyhow!("追加写入失败 [{}]: {}", path, e))?;

        info!("追加文件: {} ({} bytes)", path, content.len());
        Ok(serde_json::json!({ "path": path, "appended": content.len() }))
    }
}

// ═══════════════════════════════════════
// file_mkdir — 创建目录
// ═══════════════════════════════════════

#[derive(Default)]
pub struct FileMkdirNode;

#[async_trait]
impl NodeExecutor for FileMkdirNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_mkdir".into(),
            version: "1.0".into(),
            display_name: "创建目录".into(),
            description: "递归创建目录".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object", "required": ["path"] }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;

        tokio::fs::create_dir_all(path)
            .await
            .map_err(|e| anyhow!("创建目录失败 [{}]: {}", path, e))?;

        info!("创建目录: {}", path);
        Ok(serde_json::json!({ "path": path, "created": true }))
    }
}

// ═══════════════════════════════════════
// file_copy — 复制文件/目录
// ═══════════════════════════════════════

#[derive(Default)]
pub struct FileCopyNode;

#[async_trait]
impl NodeExecutor for FileCopyNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_copy".into(),
            version: "1.0".into(),
            display_name: "复制文件".into(),
            description: "复制文件或目录到目标路径".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object", "required": ["path", "dest"] }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let src = get_path(config)?;
        let dest = config
            .get("dest")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("copy 缺少 dest 参数"))?;

        let meta = tokio::fs::metadata(src)
            .await
            .map_err(|e| anyhow!("源路径不存在 [{}]: {}", src, e))?;

        if meta.is_dir() {
            // 递归复制目录
            copy_dir_recursive(src, dest).await?;
        } else {
            // 确保目标父目录存在
            if let Some(parent) = std::path::Path::new(dest).parent() {
                if !parent.as_os_str().is_empty() && !parent.exists() {
                    tokio::fs::create_dir_all(parent)
                        .await
                        .map_err(|e| anyhow!("创建父目录失败: {}", e))?;
                }
            }
            tokio::fs::copy(src, dest)
                .await
                .map_err(|e| anyhow!("复制文件失败 [{} -> {}]: {}", src, dest, e))?;
        }

        info!("复制: {} -> {}", src, dest);
        Ok(serde_json::json!({ "src": src, "dest": dest, "copied": true }))
    }
}

async fn copy_dir_recursive(src: &str, dest: &str) -> Result<()> {
    tokio::fs::create_dir_all(dest)
        .await
        .map_err(|e| anyhow!("创建目标目录失败 [{}]: {}", dest, e))?;

    let mut read_dir = tokio::fs::read_dir(src)
        .await
        .map_err(|e| anyhow!("读取源目录失败 [{}]: {}", src, e))?;

    while let Some(entry) = read_dir
        .next_entry()
        .await
        .map_err(|e| anyhow!("遍历目录失败: {}", e))?
    {
        let src_path = entry.path();
        let dest_path = std::path::Path::new(dest).join(entry.file_name());
        let meta = entry
            .metadata()
            .await
            .map_err(|e| anyhow!("获取元数据失败: {}", e))?;

        if meta.is_dir() {
            Box::pin(copy_dir_recursive(
                &src_path.to_string_lossy(),
                &dest_path.to_string_lossy(),
            ))
            .await?;
        } else {
            tokio::fs::copy(&src_path, &dest_path)
                .await
                .map_err(|e| anyhow!("复制失败: {}", e))?;
        }
    }
    Ok(())
}

// ═══════════════════════════════════════
// file_move — 移动/重命名文件
// ═══════════════════════════════════════

#[derive(Default)]
pub struct FileMoveNode;

#[async_trait]
impl NodeExecutor for FileMoveNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_move".into(),
            version: "1.0".into(),
            display_name: "移动文件".into(),
            description: "移动或重命名文件/目录".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object", "required": ["path", "dest"] }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let src = get_path(config)?;
        let dest = config
            .get("dest")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("move 缺少 dest 参数"))?;

        // 确保目标父目录存在
        if let Some(parent) = std::path::Path::new(dest).parent() {
            if !parent.as_os_str().is_empty() && !parent.exists() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| anyhow!("创建父目录失败: {}", e))?;
            }
        }

        tokio::fs::rename(src, dest)
            .await
            .map_err(|e| anyhow!("移动失败 [{} -> {}]: {}", src, dest, e))?;

        info!("移动: {} -> {}", src, dest);
        Ok(serde_json::json!({ "src": src, "dest": dest, "moved": true }))
    }
}

// ═══════════════════════════════════════
// file_glob — 通配符查找文件
// ═══════════════════════════════════════

#[derive(Default)]
pub struct FileGlobNode;

#[async_trait]
impl NodeExecutor for FileGlobNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_glob".into(),
            version: "1.0".into(),
            display_name: "搜索文件".into(),
            description: "使用通配符模式查找文件".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object", "required": ["pattern"] }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let pattern = config
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("glob 缺少 pattern 参数"))?;

        let paths: Vec<String> = glob::glob(pattern)
            .map_err(|e| anyhow!("无效的 glob 模式 [{}]: {}", pattern, e))?
            .filter_map(|entry| entry.ok())
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let count = paths.len();
        info!("glob [{}]: {} 匹配", pattern, count);
        Ok(serde_json::json!({ "pattern": pattern, "matches": paths, "count": count }))
    }
}

// ═══════════════════════════════════════
// file_checksum — 计算文件校验和
// ═══════════════════════════════════════

#[derive(Default)]
pub struct FileChecksumNode;

#[async_trait]
impl NodeExecutor for FileChecksumNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_checksum".into(),
            version: "1.0".into(),
            display_name: "文件校验和".into(),
            description: "计算文件的 MD5 和 SHA256 校验和".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object", "required": ["path"] }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = get_path(config)?;

        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| anyhow!("读取文件失败 [{}]: {}", path, e))?;

        use md5::Md5;
        use sha2::{Digest, Sha256};

        let md5_hash = {
            let mut hasher = Md5::new();
            hasher.update(&bytes);
            format!("{:x}", hasher.finalize())
        };

        let sha256_hash = {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            format!("{:x}", hasher.finalize())
        };

        info!(
            "checksum [{}]: md5={}, sha256={}",
            path, md5_hash, sha256_hash
        );
        Ok(serde_json::json!({
            "path": path,
            "size": bytes.len(),
            "md5": md5_hash,
            "sha256": sha256_hash,
        }))
    }
}
