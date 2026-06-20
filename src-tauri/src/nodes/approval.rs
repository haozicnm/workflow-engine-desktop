// nodes/approval.rs — 审批节点（channel 暂停/恢复 + SQLite 持久化 + 超时行为）
//
// 流程：
//   1. 查 SQLite 是否已有决策 → 有则直接返回
//   2. 解析 recommended（支持 {{step_x.branch}} 等变量引用）
//   3. 首次创建 → 写入 SQLite（pending）→ 注册到 ApprovalStore → 等待
//   4. 收到决策/超时 → 更新 SQLite → 返回结果
//
// 配置：
//   title: "审批标题"
//   message: "审批消息，支持 {{step_x.field}}"
//   options: "同意,拒绝"
//   recommended: "{{step_check.branch}}"  — 支持变量引用，由上游条件节点提供
//   require_review: true（默认）| false（自动用 recommended 决策）
//   timeout: 300（秒）
//   timeout_action: "recommended"（默认）| "reject" | "approve"
//   timeout_behavior: "auto"（默认，超时执行推荐）| "manual"（必须人工审批）
//
// 输出：
//   { decision: "选项名", comment: "...", item: {...}, auto?: true }

use crate::engine::approval_store::{ApprovalDecision, ApprovalEntry};
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;

#[derive(Default)]
pub struct ApprovalNode;

#[async_trait]
impl NodeExecutor for ApprovalNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "approval".into(),
            version: "1.0".into(),
            display_name: "人工审批".into(),
            description: "暂停工作流等待人工审批决策，支持超时自动处理和持久化".into(),
            category: "流程控制".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "decision".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "comment".into(), data_type: "string".into(), required: false },
                crate::nodes::traits::PortDef { label: "item".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "title": {"type": "string", "description": "审批标题"},
                    "message": {"type": "string", "description": "审批消息，支持 {{step_x.field}} 变量"},
                    "options": {"type": "string", "description": "选项列表，逗号分隔"},
                    "recommended": {"type": "string", "description": "推荐决策"},
                    "require_review": {"type": "boolean", "description": "是否需要人工审核"},
                    "timeout": {"type": "number", "description": "超时秒数"},
                    "timeout_action": {"type": "string", "enum": ["recommended", "reject", "approve"]},
                    "timeout_behavior": {"type": "string", "enum": ["auto", "manual"]}
                }
            }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let config = &step.config;
        let approval_id = format!("approval:{}:{}", ctx.run_id, step.id);

        // ── 读取配置（所有字符串字段都经过 resolve_config 支持变量引用）──
        let resolved = ctx.resolve_config(config);

        let title = resolved
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("人工审批")
            .to_string();

        let message = resolved
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("请审批此操作")
            .to_string();

        let options_str = resolved
            .get("options")
            .and_then(|v| v.as_str())
            .unwrap_or("同意,拒绝");
        let options: Vec<String> = options_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        // recommended 支持变量引用：{{step_check.branch}} → "true"/"false"/"同意" 等
        let recommended = resolved
            .get("recommended")
            .and_then(|v| v.as_str())
            .unwrap_or("同意")
            .to_string();

        let require_review = resolved
            .get("require_review")
            .and_then(|v| {
                if let Some(b) = v.as_bool() {
                    Some(b)
                } else {
                    v.as_str().map(|s| s == "true")
                }
            })
            .unwrap_or(true);

        let timeout_secs = resolved
            .get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(300);

        let timeout_action = resolved
            .get("timeout_action")
            .and_then(|v| v.as_str())
            .unwrap_or("recommended")
            .to_string();

        let timeout_behavior = resolved
            .get("timeout_behavior")
            .and_then(|v| v.as_str())
            .unwrap_or("auto")
            .to_string();
        let is_manual_timeout = timeout_behavior == "manual";

        let item = collect_upstream_item(ctx, config);

        // ── 不需要人工审核 → 直接用 recommended ──
        if !require_review {
            info!(
                "审批节点 '{}' 自动决策（无需审核）→ {}",
                step.name, recommended
            );
            return Ok(json!({
                "decision": recommended,
                "comment": "自动决策（无需审核）",
                "item": item,
                "auto": true,
            }));
        }

        // ── 查 SQLite 是否已有决策（重启恢复场景） ──
        if let Some(existing) = executor.db.get_approval(&approval_id) {
            match existing.status.as_str() {
                "decided" => {
                    info!(
                        "审批节点 '{}' SQLite 已有决策: {} → 直接返回",
                        step.name,
                        existing.decision.as_deref().unwrap_or("?")
                    );
                    let _ = executor.db.delete_approval(&approval_id);
                    return Ok(json!({
                        "decision": existing.decision.as_deref().unwrap_or("拒绝"),
                        "comment": existing.comment,
                        "item": item,
                        "auto": false,
                    }));
                }
                "pending" => {
                    if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&existing.created_at)
                    {
                        let elapsed = chrono::Utc::now()
                            .signed_duration_since(created.with_timezone(&chrono::Utc));
                        if elapsed.num_seconds() as u64 >= existing.timeout_secs as u64 {
                            let auto_option = resolve_timeout_action(
                                &existing.timeout_action,
                                &existing.recommended,
                            );
                            info!(
                                "审批节点 '{}' 重启后发现已超时 → 自动 {}",
                                step.name, auto_option
                            );
                            executor
                                .db
                                .update_approval_decision(
                                    &approval_id,
                                    &auto_option,
                                    Some("超时自动执行（重启恢复）"),
                                )
                                .ok();
                            let _ = executor.db.delete_approval(&approval_id);
                            return Ok(json!({
                                "decision": auto_option,
                                "comment": "超时自动执行（重启恢复）",
                                "item": item,
                                "auto": true,
                            }));
                        }
                    }
                    info!("审批节点 '{}' 重启后发现仍在等待中 → 重新注册", step.name);
                }
                _ => {}
            }
        }

        // ── 首次创建：写入 SQLite ──
        let item_str = item.as_ref().map(|v| v.to_string());
        let options_joined = options.join(",");
        let _ = executor.db.insert_approval(
            &approval_id,
            &ctx.run_id,
            &step.id,
            &title,
            &message,
            item_str.as_deref(),
            Some(&options_joined),
            &recommended,
            timeout_secs as i64,
            &timeout_action,
            &chrono::Utc::now().to_rfc3339(),
        );

        // ── 注册到 ApprovalStore 并等待 ──
        let entry = ApprovalEntry {
            id: approval_id.clone(),
            run_id: ctx.run_id.clone(),
            step_id: step.id.clone(),
            title,
            message,
            item: item.clone(),
            options: options.clone(),
            recommended: recommended.clone(),
            timeout_secs: if is_manual_timeout {
                u64::MAX
            } else {
                timeout_secs
            },
            timeout_action: timeout_action.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            recommendation_reason: None,
        };

        let mut rx = executor.approval_store.register(entry).await;
        info!(
            "审批节点 '{}' 已注册，等待用户决策 (id: {}, timeout_behavior: {})",
            step.name, approval_id, timeout_behavior
        );

        // ── 等待决策或超时 ──
        let decision = if is_manual_timeout {
            match rx.recv().await {
                Some(d) => {
                    info!("审批节点 '{}' 收到用户决策: {}", step.name, d.option);
                    executor
                        .db
                        .update_approval_decision(&approval_id, &d.option, d.comment.as_deref())
                        .ok();
                    let _ = executor.db.delete_approval(&approval_id);
                    d
                }
                None => {
                    info!("审批节点 '{}' channel 关闭，回退到拒绝", step.name);
                    ApprovalDecision {
                        option: "拒绝".to_string(),
                        comment: Some("审批通道异常关闭".to_string()),
                    }
                }
            }
        } else {
            tokio::select! {
                Some(d) = rx.recv() => {
                    info!("审批节点 '{}' 收到用户决策: {}", step.name, d.option);
                    executor.db.update_approval_decision(&approval_id, &d.option, d.comment.as_deref()).ok();
                    let _ = executor.db.delete_approval(&approval_id);
                    d
                }
                _ = tokio::time::sleep(std::time::Duration::from_secs(timeout_secs)) => {
                    let auto_option = resolve_timeout_action(&timeout_action, &recommended);
                    info!("审批节点 '{}' 超时，自动执行: {}", step.name, auto_option);
                    executor.db.update_approval_decision(&approval_id, &auto_option, Some(&format!("超时自动执行（{}秒）", timeout_secs))).ok();
                    let _ = executor.db.delete_approval(&approval_id);
                    executor.approval_store.decide(&approval_id, ApprovalDecision {
                        option: auto_option.clone(),
                        comment: Some(format!("超时自动执行（{}秒）", timeout_secs)),
                    }).await.ok();
                    ApprovalDecision {
                        option: auto_option,
                        comment: Some(format!("超时自动执行（{}秒）", timeout_secs)),
                    }
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

fn resolve_timeout_action(timeout_action: &str, recommended: &str) -> String {
    match timeout_action {
        "recommended" => recommended.to_string(),
        "reject" => "拒绝".to_string(),
        "approve" => "同意".to_string(),
        other => other.to_string(),
    }
}

fn collect_upstream_item(ctx: &ExecutionContext, config: &Value) -> Option<Value> {
    if let Some(source) = config.get("data_source").and_then(|v| v.as_str()) {
        if let Some(output) = ctx.get_output(source) {
            return Some(output.clone());
        }
    }
    ctx.step_outputs.values().last().cloned()
}
