// nodes/s3_node.rs — S3/MinIO/阿里云 OSS 对象存储节点
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;

#[derive(Default)]
pub struct S3Node;

#[async_trait]
impl NodeExecutor for S3Node {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "s3".into(),
            version: "1.0".into(),
            display_name: "S3 对象存储".into(),
            description: "S3/MinIO/阿里云 OSS 对象存储操作（upload/download/list/delete）".into(),
            category: "data".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "content".into(), data_type: "text".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "url".into(), data_type: "text".into(), required: false },
                crate::nodes::traits::PortDef { label: "objects".into(), data_type: "json".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "action": { "type": "string", "enum": ["upload", "download", "list", "delete", "presign"], "default": "upload" },
                    "bucket": { "type": "string", "required": true },
                    "key": { "type": "string", "description": "对象 key" },
                    "content": { "type": "string", "description": "upload 的内容" },
                    "prefix": { "type": "string", "default": "", "description": "list 的前缀" },
                    "api_url": { "type": "string", "description": "S3 兼容 API 地址" },
                    "access_key": { "type": "string" },
                    "secret_key": { "type": "string" }
                },
                "required": ["action", "bucket"]
            }),
            params: vec![],
        }
    }

    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<Value> {
        let config = &step.config;
        let action = config.get("action").and_then(|v| v.as_str()).unwrap_or("upload");
        let bucket = config.get("bucket").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("s3: 缺少 bucket"))?;
        let api_url = config.get("api_url").and_then(|v| v.as_str())
            .unwrap_or("http://localhost:9000");
        let _access_key = config.get("access_key").and_then(|v| v.as_str()).unwrap_or("");
        let _secret_key = config.get("secret_key").and_then(|v| v.as_str()).unwrap_or("");

        let client = reqwest::Client::new();

        match action {
            "upload" => {
                let key = config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("s3 upload: 缺少 key"))?;
                let content = ctx.input_ports.get("content").and_then(|v| v.as_str()).map(String::from)
                    .or_else(|| config.get("content").and_then(|v| v.as_str()).map(String::from))
                    .ok_or_else(|| anyhow!("s3 upload: 缺少 content"))?;
                let url = format!("{}/{}/{}", api_url, bucket, key);
                info!("S3 上传: {}/{}", bucket, key);
                let resp = client.put(&url)
                    .body(content)
                    .send().await.map_err(|e| anyhow!("S3 上传失败: {}", e))?;
                let status = resp.status();
                if status.is_success() {
                    Ok(json!({"uploaded": true, "url": url}))
                } else {
                    Err(anyhow!("S3 上传失败: HTTP {}", status))
                }
            }
            "download" => {
                let key = config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("s3 download: 缺少 key"))?;
                let url = format!("{}/{}/{}", api_url, bucket, key);
                info!("S3 下载: {}/{}", bucket, key);
                let resp = client.get(&url).send().await.map_err(|e| anyhow!("S3 下载失败: {}", e))?;
                let status = resp.status();
                if status.is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    Ok(json!({"content": body, "key": key}))
                } else {
                    Err(anyhow!("S3 下载失败: HTTP {}", status))
                }
            }
            "list" => {
                let prefix = config.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
                let url = format!("{}/{}?prefix={}", api_url, bucket, prefix);
                info!("S3 列表: {}/{}", bucket, prefix);
                let resp = client.get(&url).send().await.map_err(|e| anyhow!("S3 列表失败: {}", e))?;
                let status = resp.status();
                if status.is_success() {
                    let body = resp.text().await.unwrap_or_default();
                    Ok(json!({"objects": body, "bucket": bucket}))
                } else {
                    Err(anyhow!("S3 列表失败: HTTP {}", status))
                }
            }
            "delete" => {
                let key = config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("s3 delete: 缺少 key"))?;
                let url = format!("{}/{}/{}", api_url, bucket, key);
                info!("S3 删除: {}/{}", bucket, key);
                let resp = client.delete(&url).send().await.map_err(|e| anyhow!("S3 删除失败: {}", e))?;
                let status = resp.status();
                if status.is_success() {
                    Ok(json!({"deleted": true, "key": key}))
                } else {
                    Err(anyhow!("S3 删除失败: HTTP {}", status))
                }
            }
            "presign" => {
                let key = config.get("key").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("s3 presign: 缺少 key"))?;
                let url = format!("{}/{}/{}", api_url, bucket, key);
                Ok(json!({"url": url, "key": key}))
            }
            _ => Err(anyhow!("s3: 不支持的操作 '{}'", action)),
        }
    }
}
