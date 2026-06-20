// nodes/file_container.rs — 统一文件操作容器
//
// 将分散的 file_read/write/list/delete/exists 统一为一个容器节点，
// 支持在单个步骤中执行多个文件操作，返回按 action id 索引的结果。
//
// 支持的动作：
//   read   — 读取文件
//   write  — 写入文件
//   append — 追加内容
//   copy   — 复制文件
//   move   — 移动/重命名
//   delete — 删除文件/目录
//   list   — 列出目录
//   exists — 检查存在
//   glob   — 按模式查找文件
//   grep   — 在文件中搜索内容

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};

// ─── 数据结构 ───

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAction {
    pub id: String,
    #[serde(rename = "type")]
    pub action_type: String,
    #[serde(default)]
    pub config: HashMap<String, Value>,
}

// ─── NodeExecutor ───

#[derive(Default)]
pub struct FileContainerNode;

#[async_trait]
impl NodeExecutor for FileContainerNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "file_container".into(),
            version: "1.0".into(),
            display_name: "文件操作容器".into(),
            description: "在一个步骤中执行多个文件操作（读/写/复制/移动/删除/列出/搜索），按 action ID 索引结果".into(),
            category: "文件".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "results".into(), data_type: "object".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "required": ["actions"],
                "properties": {
                    "actions": {"type": "array", "description": "文件操作列表"}
                }
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let actions: Vec<FileAction> = step
            .config
            .get("actions")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .ok_or_else(|| anyhow!("file 容器缺少 actions 参数"))?;

        // Phase 3: 占位符机制已在 executor 层处理，不需要容器内部 resolve

        let mut results = serde_json::Map::new();

        for action in &actions {
            let result = execute_file_action(action).await;
            match result {
                Ok(val) => {
                    info!("文件操作成功: {} ({})", action.action_type, action.id);
                    results.insert(action.id.clone(), val);
                }
                Err(e) => {
                    warn!(
                        "文件操作失败: {} ({}) — {}",
                        action.action_type, action.id, e
                    );
                    results.insert(
                        action.id.clone(),
                        json!({
                            "error": e.to_string(),
                        }),
                    );
                }
            }
        }

        results.insert("_container_type".to_string(), json!("file"));
        results.insert("_step_name".to_string(), json!(step.name.clone()));
        Ok(Value::Object(results))
    }
}

// ─── 动作分发 ───

async fn execute_file_action(action: &FileAction) -> Result<Value> {
    match action.action_type.as_str() {
        "read" => file_read(&action.config).await,
        "write" => file_write(&action.config).await,
        "append" => file_append(&action.config).await,
        "copy" => file_copy(&action.config).await,
        "move" | "rename" => file_move(&action.config).await,
        "delete" => file_delete(&action.config).await,
        "list" => file_list(&action.config).await,
        "exists" => file_exists(&action.config).await,
        "glob" => file_glob(&action.config).await,
        "grep" => file_grep(&action.config).await,
        _ => Err(anyhow!("未知文件操作: {}", action.action_type)),
    }
}

fn get_str<'a>(config: &'a HashMap<String, Value>, key: &str) -> Result<&'a str> {
    config
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("缺少 {} 参数", key))
}

fn get_str_opt<'a>(config: &'a HashMap<String, Value>, key: &str) -> Option<&'a str> {
    config.get(key).and_then(|v| v.as_str())
}

// ─── 文件操作实现 ───

async fn file_read(config: &HashMap<String, Value>) -> Result<Value> {
    let path = get_str(config, "path")?;
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| anyhow!("读取失败 [{}]: {}", path, e))?;
    let metadata = tokio::fs::metadata(path).await.ok();
    Ok(json!({
        "path": path,
        "content": content,
        "size": metadata.as_ref().map(|m| m.len()),
        "lines": content.lines().count(),
    }))
}

async fn file_write(config: &HashMap<String, Value>) -> Result<Value> {
    let path = get_str(config, "path")?;
    let content = get_str(config, "content")?;
    if let Some(parent) = std::path::Path::new(path).parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::write(path, content)
        .await
        .map_err(|e| anyhow!("写入失败 [{}]: {}", path, e))?;
    let size = content.len();
    info!("写入文件: {} ({} bytes)", path, size);
    Ok(json!({ "path": path, "size": size }))
}

async fn file_append(config: &HashMap<String, Value>) -> Result<Value> {
    let path = get_str(config, "path")?;
    let content = get_str(config, "content")?;
    let newline = config
        .get("newline")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    use tokio::io::AsyncWriteExt;

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await
        .map_err(|e| anyhow!("打开文件失败 [{}]: {}", path, e))?;
    if newline && !content.ends_with('\n') {
        file.write_all(format!("{}\n", content).as_bytes()).await?;
    } else {
        file.write_all(content.as_bytes()).await?;
    }
    let metadata = tokio::fs::metadata(path).await.ok();
    Ok(json!({ "path": path, "size": metadata.map(|m| m.len()) }))
}

async fn file_copy(config: &HashMap<String, Value>) -> Result<Value> {
    let from = get_str(config, "from")?;
    let to = get_str(config, "to")?;
    if let Some(parent) = std::path::Path::new(to).parent() {
        tokio::fs::create_dir_all(parent).await.ok();
    }
    tokio::fs::copy(from, to)
        .await
        .map_err(|e| anyhow!("复制失败 [{} -> {}]: {}", from, to, e))?;
    info!("复制文件: {} -> {}", from, to);
    Ok(json!({ "from": from, "to": to }))
}

async fn file_move(config: &HashMap<String, Value>) -> Result<Value> {
    let from = get_str(config, "from")?;
    let to = get_str(config, "to")?;
    tokio::fs::rename(from, to)
        .await
        .map_err(|e| anyhow!("移动失败 [{} -> {}]: {}", from, to, e))?;
    info!("移动文件: {} -> {}", from, to);
    Ok(json!({ "from": from, "to": to }))
}

async fn file_delete(config: &HashMap<String, Value>) -> Result<Value> {
    let path = get_str(config, "path")?;
    let metadata = tokio::fs::metadata(path).await;
    match metadata {
        Ok(m) if m.is_dir() => {
            tokio::fs::remove_dir_all(path)
                .await
                .map_err(|e| anyhow!("删除目录失败 [{}]: {}", path, e))?;
        }
        Ok(_) => {
            tokio::fs::remove_file(path)
                .await
                .map_err(|e| anyhow!("删除文件失败 [{}]: {}", path, e))?;
        }
        Err(_) => return Err(anyhow!("文件不存在: {}", path)),
    }
    info!("删除: {}", path);
    Ok(json!({ "path": path }))
}

async fn file_list(config: &HashMap<String, Value>) -> Result<Value> {
    let path = get_str_opt(config, "path").unwrap_or(".");
    let pattern = get_str_opt(config, "pattern");
    let mut entries = vec![];
    let mut dir = tokio::fs::read_dir(path)
        .await
        .map_err(|e| anyhow!("读取目录失败 [{}]: {}", path, e))?;
    while let Some(entry) = dir
        .next_entry()
        .await
        .map_err(|e| anyhow!("遍历目录失败: {}", e))?
    {
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(pat) = pattern {
            if !glob_match(pat, &name) {
                continue;
            }
        }
        let ft = entry.file_type().await.ok();
        entries.push(json!({
            "name": name,
            "is_dir": ft.as_ref().map(|t| t.is_dir()).unwrap_or(false),
            "path": entry.path().to_string_lossy(),
        }));
    }
    entries.sort_by(|a, b| {
        let a_dir = a["is_dir"].as_bool().unwrap_or(false);
        let b_dir = b["is_dir"].as_bool().unwrap_or(false);
        b_dir.cmp(&a_dir).then_with(|| {
            a["name"]
                .as_str()
                .unwrap_or("")
                .cmp(b["name"].as_str().unwrap_or(""))
        })
    });
    Ok(json!({ "path": path, "entries": entries, "count": entries.len() }))
}

async fn file_exists(config: &HashMap<String, Value>) -> Result<Value> {
    let path = get_str(config, "path")?;
    let exists = tokio::fs::metadata(path).await.is_ok();
    Ok(json!({ "path": path, "exists": exists }))
}

async fn file_glob(config: &HashMap<String, Value>) -> Result<Value> {
    let pattern = get_str(config, "pattern")?;
    let base = get_str_opt(config, "path").unwrap_or(".");
    let mut results = vec![];

    // 优先使用 glob crate（如果可用），否则用 walkdir
    let walker = walkdir::WalkDir::new(base)
        .max_depth(
            config
                .get("max_depth")
                .and_then(|v| v.as_u64())
                .unwrap_or(10) as usize,
        )
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in walker {
        let name = entry.file_name().to_string_lossy().to_string();
        if glob_match(pattern, &name) {
            results.push(json!({
                "name": name,
                "path": entry.path().to_string_lossy(),
                "is_dir": entry.file_type().is_dir(),
            }));
        }
    }

    Ok(json!({ "pattern": pattern, "matches": results, "count": results.len() }))
}

async fn file_grep(config: &HashMap<String, Value>) -> Result<Value> {
    let pattern = get_str(config, "pattern")?;
    let path = get_str_opt(config, "path").unwrap_or(".");
    let file_pattern = get_str_opt(config, "file_pattern").unwrap_or("*");
    let max_results = config
        .get("max_results")
        .and_then(|v| v.as_u64())
        .unwrap_or(100) as usize;

    let mut results = vec![];
    let walker = walkdir::WalkDir::new(path)
        .max_depth(
            config
                .get("max_depth")
                .and_then(|v| v.as_u64())
                .unwrap_or(5) as usize,
        )
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file() && glob_match(file_pattern, &e.file_name().to_string_lossy())
        });

    for entry in walker {
        if results.len() >= max_results {
            break;
        }
        let file_path = entry.path();
        match tokio::fs::read_to_string(file_path).await {
            Ok(content) => {
                for (i, line) in content.lines().enumerate() {
                    if results.len() >= max_results {
                        break;
                    }
                    if line.contains(pattern) {
                        results.push(json!({
                            "file": file_path.to_string_lossy(),
                            "line": i + 1,
                            "content": line.trim(),
                        }));
                    }
                }
            }
            Err(_) => continue, // skip binary files
        }
    }

    Ok(json!({ "pattern": pattern, "matches": results, "count": results.len() }))
}

/// 简单的 glob 匹配（支持 * 通配符）
fn glob_match(pattern: &str, name: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') {
        return name == pattern;
    }

    let parts: Vec<&str> = pattern.split('*').collect();
    let mut remaining = name;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !remaining.starts_with(part) {
                return false;
            }
            remaining = &remaining[part.len()..];
        } else if i == parts.len() - 1 && !pattern.ends_with('*') {
            if !remaining.ends_with(part) {
                return false;
            }
            return true;
        } else {
            if let Some(pos) = remaining.find(part) {
                remaining = &remaining[pos + part.len()..];
            } else {
                return false;
            }
        }
    }
    true
}
