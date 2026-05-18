// nodes/approval.rs — 审批节点（channel 暂停/恢复 + SQLite 持久化版 + 条件推荐 + 超时行为）
//
// 流程：
//   1. 查 SQLite 是否已有决策 → 有则直接返回
//   2. 评估 approval_conditions（如有）→ 动态设置 recommended
//   3. 首次创建 → 写入 SQLite（pending）→ 注册到 ApprovalStore → 等待
//   4. 收到决策/超时 → 更新 SQLite → 返回结果
//
// 新增配置：
//   approval_conditions: [{ id, left, op, right }] — 复用 LogicCondition 格式
//     所有条件通过 → recommended = options[0]（通常是"同意"）
//     任一条件不通过 → recommended = options[-1]（通常是"拒绝"）
//   timeout_behavior: "auto"（默认，超时执行推荐）| "manual"（必须人工审批，永不过期）
//
// 输出：
//   { decision: "选项名", comment: "...", item: {...}, auto?: true, recommendation_reason?: str }

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use crate::engine::approval_store::{ApprovalEntry, ApprovalDecision};
use crate::nodes::condition::eval_condition;
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
        let approval_id = format!("approval:{}:{}", ctx.run_id, step.id);

        // ── 读取配置 ──
        let title = config.get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("人工审批")
            .to_string();

        let message_template = config.get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("请审批此操作")
            .to_string();

        let message = {
            let tmp_val = json!(message_template);
            let config_val = json!({ "_msg": tmp_val });
            let resolved = ctx.resolve_config(&config_val);
            resolved.get("_msg")
                .and_then(|v| v.as_str())
                .unwrap_or(&message_template)
                .to_string()
        };

        let options_str = config.get("options")
            .and_then(|v| v.as_str())
            .unwrap_or("同意,拒绝");
        let options: Vec<String> = options_str
            .split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();

        let mut recommended = config.get("recommended")
            .and_then(|v| v.as_str())
            .unwrap_or("同意").to_string();

        let require_review = config.get("require_review")
            .and_then(|v| {
                if let Some(b) = v.as_bool() { Some(b) }
                else if let Some(s) = v.as_str() { Some(s == "true") }
                else { None }
            }).unwrap_or(true);

        let timeout_secs = config.get("timeout")
            .and_then(|v| v.as_u64()).unwrap_or(300);

        let timeout_action = config.get("timeout_action")
            .and_then(|v| v.as_str())
            .unwrap_or("recommended").to_string();

        // ── 新增：超时行为（auto/manual） ──
        let timeout_behavior = config.get("timeout_behavior")
            .and_then(|v| v.as_str())
            .unwrap_or("auto").to_string();
        let is_manual_timeout = timeout_behavior == "manual";

        let item = collect_upstream_item(ctx, config);

        // ── 新增：审批条件评估（动态推荐） ──
        let mut recommendation_reason = String::new();
        if let Some(conditions) = config.get("approval_conditions").and_then(|v| v.as_array()) {
            if !conditions.is_empty() {
                let mut all_pass = true;
                let mut results: Vec<String> = Vec::new();

                for cond in conditions {
                    let left_template = cond.get("left").and_then(|v| v.as_str()).unwrap_or("");
                    let op = cond.get("op").and_then(|v| v.as_str()).unwrap_or("equals");
                    let right_template = cond.get("right").and_then(|v| v.as_str()).unwrap_or("");

                    let left = ctx.resolve_config(&json!(left_template));
                    let right = ctx.resolve_config(&json!(right_template));

                    let pass = eval_condition(&left, op, &right);
                    results.push(format!(
                        "{} {} {} → {}",
                        left, op, right, if pass { "✓" } else { "✗" }
                    ));

                    if !pass {
                        all_pass = false;
                    }
                }

                // 全部通过 → 推荐同意（第一个选项）；任一不通过 → 推荐拒绝（最后一个选项）
                if all_pass {
                    recommended = options.first().cloned().unwrap_or_else(|| "同意".to_string());
                    recommendation_reason = format!("全部{}个条件通过 → 推荐「{}」", conditions.len(), recommended);
                } else {
                    recommended = options.last().cloned().unwrap_or_else(|| "拒绝".to_string());
                    recommendation_reason = format!(
                        "条件未全部通过 → 推荐「{}」\n详情: {}",
                        recommended,
                        results.join("; ")
                    );
                }
                info!("审批节点 '{}' 条件评估: {}", step.name, recommendation_reason);
            }
        }

        // ── 不需要人工审核 → 直接用推荐选项 ──
        if !require_review {
            info!("审批节点 '{}' 自动决策（无需审核）→ {}", step.name, recommended);
            return Ok(json!({
                "decision": recommended,
                "comment": if recommendation_reason.is_empty() {
                    "自动决策（无需审核）".to_string()
                } else {
                    format!("自动决策（无需审核）— {}", recommendation_reason)
                },
                "item": item,
                "auto": true,
                "recommendation_reason": recommendation_reason,
            }));
        }

        // ── 查 SQLite 是否已有决策（重启恢复场景） ──
        if let Some(existing) = executor.db.get_approval(&approval_id) {
            match existing.status.as_str() {
                "decided" => {
                    info!("审批节点 '{}' SQLite 已有决策: {} → 直接返回", step.name, existing.decision.as_deref().unwrap_or("?"));
                    let _ = executor.db.delete_approval(&approval_id);
                    return Ok(json!({
                        "decision": existing.decision.as_deref().unwrap_or("拒绝"),
                        "comment": existing.comment,
                        "item": item,
                        "auto": false,
                        "recommendation_reason": recommendation_reason,
                    }));
                }
                "pending" => {
                    // 检查是否已超时
                    if let Ok(created) = chrono::DateTime::parse_from_rfc3339(&existing.created_at) {
                        let elapsed = chrono::Utc::now().signed_duration_since(created.with_timezone(&chrono::Utc));
                        if elapsed.num_seconds() as u64 >= existing.timeout_secs as u64 {
                            let auto_option = resolve_timeout_action(&existing.timeout_action, &existing.recommended);
                            info!("审批节点 '{}' 重启后发现已超时 → 自动 {}", step.name, auto_option);
                            executor.db.update_approval_decision(&approval_id, &auto_option, Some("超时自动执行（重启恢复）")).ok();
                            let _ = executor.db.delete_approval(&approval_id);
                            return Ok(json!({
                                "decision": auto_option,
                                "comment": "超时自动执行（重启恢复）",
                                "item": item,
                                "auto": true,
                                "recommendation_reason": recommendation_reason,
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
        let options_str = options.join(",");
        let _ = executor.db.insert_approval(
            &approval_id, &ctx.run_id, &step.id, &title, &message,
            item_str.as_deref(), Some(&options_str), &recommended,
            timeout_secs as i64, &timeout_action, &chrono::Utc::now().to_rfc3339(),
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
            timeout_secs: if is_manual_timeout { u64::MAX } else { timeout_secs },
            timeout_action: timeout_action.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            recommendation_reason: if recommendation_reason.is_empty() {
                None
            } else {
                Some(recommendation_reason.clone())
            },
        };

        let mut rx = executor.approval_store.register(entry).await;
        info!("审批节点 '{}' 已注册，等待用户决策 (id: {}, timeout_behavior: {})",
            step.name, approval_id, timeout_behavior);

        // ── 等待决策或超时 ──
        let decision = if is_manual_timeout {
            // 手动模式：仅等待用户决策，永不过期
            match rx.recv().await {
                Some(d) => {
                    info!("审批节点 '{}' 收到用户决策: {}", step.name, d.option);
                    executor.db.update_approval_decision(&approval_id, &d.option, d.comment.as_deref()).ok();
                    let _ = executor.db.delete_approval(&approval_id);
                    d
                }
                None => {
                    // channel 关闭（异常情况）→ 回退到拒绝
                    info!("审批节点 '{}' channel 关闭，回退到拒绝", step.name);
                    ApprovalDecision {
                        option: "拒绝".to_string(),
                        comment: Some("审批通道异常关闭".to_string()),
                    }
                }
            }
        } else {
            // 自动模式：等待用户决策或超时
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
            "recommendation_reason": recommendation_reason,
        }))
    }
}

/// 解析超时动作
fn resolve_timeout_action(timeout_action: &str, recommended: &str) -> String {
    match timeout_action {
        "recommended" => recommended.to_string(),
        "reject" => "拒绝".to_string(),
        "approve" => "同意".to_string(),
        other => other.to_string(),
    }
}

/// 收集上游数据作为审批上下文
fn collect_upstream_item(ctx: &ExecutionContext, config: &Value) -> Option<Value> {
    if let Some(source) = config.get("data_source").and_then(|v| v.as_str()) {
        if let Some(output) = ctx.get_output(source) {
            return Some(output.clone());
        }
    }
    ctx.step_outputs.values().last().cloned()
}
