// nodes/github_issue.rs — GitHub Issue/PR 节点
// 通过 GitHub REST API 创建 Issue 或 Pull Request
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Default)]
pub struct GithubIssueNode;

#[async_trait]
impl NodeExecutor for GithubIssueNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "github_issue".into(),
            version: "1.0".into(),
            display_name: "GitHub Issue".into(),
            description: "通过 GitHub REST API 创建 Issue 或 Pull Request".into(),
            category: "集成".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "body".into(), data_type: "string".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "url".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "number".into(), data_type: "number".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "token": { "type": "string", "description": "GitHub Personal Access Token" },
                    "repo": { "type": "string", "description": "仓库 (owner/repo)" },
                    "action": { "type": "string", "enum": ["create_issue", "create_pr"], "default": "create_issue" },
                    "title": { "type": "string", "description": "标题" },
                    "body": { "type": "string", "description": "正文" },
                    "labels": { "type": "array", "items": { "type": "string" }, "description": "标签列表" },
                    "base": { "type": "string", "description": "PR 目标分支 (create_pr 用)" },
                    "head": { "type": "string", "description": "PR 源分支 (create_pr 用)" }
                },
                "required": ["token", "repo", "title"]
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config = &step.config;
        let token = config.get("token").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("github_issue: 缺少 token"))?;
        let repo = config.get("repo").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("github_issue: 缺少 repo"))?;
        let action = config.get("action").and_then(|v| v.as_str()).unwrap_or("create_issue");
        let title = config.get("title").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("github_issue: 缺少 title"))?;

        let body = ctx.input_ports.values().next().and_then(|v| v.as_str()).map(String::from)
            .or_else(|| config.get("body").and_then(|v| v.as_str()).map(String::from))
            .unwrap_or_default();

        let client = reqwest::Client::new();

        let (url, payload) = match action {
            "create_issue" => {
                let labels = config.get("labels").and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>())
                    .unwrap_or_default();
                (
                    format!("https://api.github.com/repos/{}/issues", repo),
                    json!({ "title": title, "body": body, "labels": labels }),
                )
            }
            "create_pr" => {
                let base = config.get("base").and_then(|v| v.as_str()).unwrap_or("main");
                let head = config.get("head").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("github_issue: create_pr 需要 head 分支"))?;
                (
                    format!("https://api.github.com/repos/{}/pulls", repo),
                    json!({ "title": title, "body": body, "base": base, "head": head }),
                )
            }
            _ => return Err(anyhow!("github_issue: 不支持的操作 '{}'，支持: create_issue/create_pr", action)),
        };

        info!("GitHub {} → {} ({})", action, repo, title);

        let mut retries = 0;
        let max_retries = 3;
        loop {
            let resp = client.post(&url)
                .header("Authorization", format!("Bearer {}", token))
                .header("Accept", "application/vnd.github+json")
                .header("User-Agent", "WorkflowEngine")
                .json(&payload)
                .send()
                .await
                .map_err(|e| anyhow!("GitHub API 请求失败: {}", e))?;

            let status = resp.status();

            // 429 Rate Limited — 等待后重试
            if status.as_u16() == 429 && retries < max_retries {
                let retry_after = resp.headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(60);
                warn!("GitHub rate limited, retry in {}s ({}/{})", retry_after, retries + 1, max_retries);
                tokio::time::sleep(std::time::Duration::from_secs(retry_after)).await;
                retries += 1;
                continue;
            }

            let result: Value = resp.json().await.unwrap_or(json!({}));

            if status.is_success() {
                let issue_url = result.get("html_url").and_then(|v| v.as_str()).unwrap_or("");
                let number = result.get("number").and_then(|v| v.as_u64()).unwrap_or(0);
                info!("GitHub {} 成功: #{} {}", action, number, issue_url);
                return Ok(json!({ "url": issue_url, "number": number }));
            } else {
                return Err(anyhow!("GitHub API 返回错误 {}: {}", status, result));
            }
        }
    }
}
