// nodes/file_save.rs — 保存文件节点
//
// 将数据写入文件，支持 JSON/YAML/CSV/TXT/binary 格式

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
pub struct FileSaveNode;

#[async_trait]
impl NodeExecutor for FileSaveNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = config.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 path 参数"))?;
        let data = config.get("data").ok_or_else(|| anyhow!("缺少 data 参数"))?;
        let format = config.get("format").and_then(|v| v.as_str()).unwrap_or("auto");
        let encoding = config.get("encoding").and_then(|v| v.as_str()).unwrap_or("utf-8");

        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| anyhow!("创建父目录失败 [{}]: {}", parent.display(), e))?;
            }
        }

        match encoding {
            "base64" => {
                let b64_str = data.as_str().ok_or_else(|| anyhow!("base64 编码需要 data 为字符串"))?;
                let bytes = BASE64.decode(b64_str).map_err(|e| anyhow!("base64 解码失败: {}", e))?;
                tokio::fs::write(path, &bytes).await
                    .map_err(|e| anyhow!("写入文件失败 [{}]: {}", path, e))?;
                info!("保存文件(base64): {} ({} bytes)", path, bytes.len());
                Ok(serde_json::json!({ "path": path, "size": bytes.len(), "written": true }))
            }
            "binary" => {
                let bytes = match data {
                    serde_json::Value::Array(arr) => {
                        arr.iter().filter_map(|v| v.as_u64()).map(|n| n as u8).collect::<Vec<u8>>()
                    }
                    _ => return Err(anyhow!("binary 需要 data 为字节数组 [0-255]")),
                };
                tokio::fs::write(path, &bytes).await
                    .map_err(|e| anyhow!("写入文件失败 [{}]: {}", path, e))?;
                info!("保存文件(binary): {} ({} bytes)", path, bytes.len());
                Ok(serde_json::json!({ "path": path, "size": bytes.len(), "written": true }))
            }
            _ => {
                let content = match format {
                    "json" => serde_json::to_string_pretty(data).map_err(|e| anyhow!("JSON 序列化失败: {}", e))?,
                    "csv" => data.as_array()
                        .map(|arr| json_array_to_csv(arr))
                        .unwrap_or_else(|| data.to_string()),
                    "yaml" => {
                        // 简单对象转 YAML（无外部依赖）
                        json_to_simple_yaml(data)
                    }
                    "txt" | "text" | _ => match data {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    },
                };
                tokio::fs::write(path, &content).await
                    .map_err(|e| anyhow!("写入文件失败 [{}]: {}", path, e))?;
                info!("保存文件: {} ({} chars)", path, content.len());
                Ok(serde_json::json!({ "path": path, "size": content.len(), "written": true }))
            }
        }
    }
}

fn json_array_to_csv(arr: &[serde_json::Value]) -> String {
    let mut cols: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for item in arr {
        if let Some(obj) = item.as_object() {
            for key in obj.keys() { if seen.insert(key.clone()) { cols.push(key.clone()); } }
        }
    }
    let mut csv = cols.join(",");
    csv.push('\n');
    for item in arr {
        let row: Vec<String> = cols.iter().map(|col| {
            match item.get(col) {
                Some(serde_json::Value::String(s)) => csv_escape(s),
                Some(serde_json::Value::Null) => String::new(),
                Some(other) => csv_escape(&other.to_string()),
                None => String::new(),
            }
        }).collect();
        csv.push_str(&row.join(","));
        csv.push('\n');
    }
    csv.trim_end().to_string()
}

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else { s.to_string() }
}

fn json_to_simple_yaml(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::Object(obj) => {
            let mut out = String::new();
            for (k, v) in obj {
                match v {
                    serde_json::Value::String(s) => out.push_str(&format!("{}: \"{}\"\n", k, s)),
                    serde_json::Value::Number(n) => out.push_str(&format!("{}: {}\n", k, n)),
                    serde_json::Value::Bool(b) => out.push_str(&format!("{}: {}\n", k, b)),
                    serde_json::Value::Null => out.push_str(&format!("{}: ~\n", k)),
                    other => out.push_str(&format!("{}: {}\n", k, other)),
                }
            }
            out
        }
        serde_json::Value::Array(arr) => arr.iter().map(|v| format!("- {}", v)).collect::<Vec<_>>().join("\n"),
        other => other.to_string(),
    }
}
