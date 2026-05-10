// nodes/approval.rs — 审批节点（channel 暂停/恢复版）
//
// 流程：
//   require_review=true  → 注册到 ApprovalStore → tokio::select! 等待决策或超时
//   require_review=false → 直接用推荐选项返回
//
// 输出：
//   { decision: "选项名", comment: "...", item: {...}, auto?: true }

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use crate::engine::approval_store::{ApprovalEntry, ApprovalDecision};
use std::sync::Arc;
use anyhow::Result;
use serde_json::{Value, json};
use tracing::info;

#[derive(Default)]
pub struct ApprovalNode;

#[async_trait]
impl NodeExecutor for ApprovalNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config = &step.config;

        // ── 读取配置 ──
        let title = config.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("人工审批")
            .to_string();

        let message_template = config.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("请审批此操作")
            .to_string();

        // 渲染消息模板（解析 {{变量}}）
        let message = {
            let tmp_val = json!(message_template);
            let config_val = json!({ "_msg": tmp_val });
            let resolved = ctx.resolve_config(&config_val);
            resolved.get("_msg")
                .and_then(|v| v.as_str())
                .unwrap_or(&message_template)
                .to_string()
        };

        // 解析选项（逗号分隔）
        let options_str = config.get("options")
            .and_then(|v| v.as_str())
            .unwrap_or("同意,拒绝");
        let options: Vec<String> = options_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let recommended = config.get("recommended")
            .and_then(|v| v.as_str())
            .unwrap_or("同意")
            .to_string();

        let require_review = config.get("require_review")
            .and_then(|v| {
                if let Some(b) = v.as_bool() { Some(b) }
                else if let Some(s) = v.as_str() { Some(s == "true") }
                else { None }
            })
            .unwrap_or(true);

        let timeout_secs = config.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(300);

        let timeout_action = config.get("timeout_action")
            .and_then(|v| v.as_str())
            .unwrap_or("recommended")
            .to_string();

        // ── 收集上游数据（只取上一步输出） ──
        let item = collect_upstream_item(ctx, config);

        // ── 不需要人工审核 → 直接用推荐选项 ──
        if !require_review {
            info!("审批节点 '{}' 自动决策（无需审核）→ {}", step.name, recommended);
            return Ok(json!({
                "decision": recommended,
                "comment": "自动决策（无需审核）",
                "item": item,
                "auto": true,
            }));
        }

        // ── 需要人工审核 → 注册到 ApprovalStore 并等待 ──
        let approval_id = format!("approval:{}:{}", ctx.run_id, step.id);
        let entry = ApprovalEntry {
            id: approval_id.clone(),
            run_id: ctx.run_id.clone(),
            step_id: step.id.clone(),
            title: title.clone(),
            message: message.clone(),
            item: item.clone(),
            options: options.clone(),
            recommended: recommended.clone(),
            timeout_secs,
            timeout_action: timeout_action.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        let mut rx = executor.approval_store.register(entry).await;
        info!("审批节点 '{}' 已注册，等待用户决策 (id: {})", step.name, approval_id);

        // ── 等待决策或超时 ──
        let decision = tokio::select! {
            Some(d) = rx.recv() => {
                info!("审批节点 '{}' 收到用户决策: {}", step.name, d.option);
                d
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                // 超时处理
                let auto_option = match timeout_action.as_str() {
                    "recommended" => recommended.clone(),
                    "reject" => "拒绝".to_string(),
                    "approve" => "同意".to_string(),
                    other => other.to_string(),
                };
                info!("审批节点 '{}' 超时，自动执行: {}", step.name, auto_option);
                // 从 store 清理
                executor.approval_store.decide(&approval_id, ApprovalDecision {
                    option: auto_option.clone(),
                    comment: Some(format!("超时自动执行（{}秒）", timeout_secs)),
                }).await.ok();
                ApprovalDecision {
                    option: auto_option,
                    comment: Some(format!("超时自动执行（{}秒）", timeout_secs)),
                }
            }
        };

        let is_auto = decision.comment.as_deref().unwrap_or("").contains("超时");

        Ok(json!({
            "decision": decision.option,
            "comment": decision.comment,
            "item": item,
            "auto": is_auto,
        }))
    }
}

/// 收集上游数据作为审批上下文
fn collect_upstream_item(ctx: &ExecutionContext, config: &Value) -> Option<Value> {
    // 如果配置指定了 data_source，用指定的
    if let Some(source) = config.get("data_source").and_then(|v| v.as_str()) {
        if let Some(output) = ctx.get_output(source) {
            return Some(output.clone());
        }
    }
    // 否则取上一步的输出
    let last_output = ctx.step_outputs.values().last();
    last_output.cloned()
}
