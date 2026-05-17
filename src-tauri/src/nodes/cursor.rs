// nodes/cursor.rs — 游标迭代节点
// 每次工作流执行只处理一项数据，游标跨次持久化。
// 适用场景：Excel 行/列逐条处理、分页 API 调用等。
//
// 与 loop_node 的区别：
// - loop_node: 一次性遍历全部 items
// - cursor:     每次只取一项，游标持久化，直到全部处理完
//
// 配置：
//   items: 数据源数组或引用（如 "{{read_excel.data}}"）
//   body:  每项要执行的步骤
//
// 输出：
//   未完成: { done: false, item: ..., index: N, total: M, remaining: M-N-1 }
//   已完成: { done: true, total: M }  — 游标自动重置

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use std::path::PathBuf;

#[derive(Default)]
pub struct CursorNode;

/// 游标存储结构
#[derive(serde::Serialize, serde::Deserialize)]
struct CursorState {
    index: usize,
    total: usize,
    items_hash: u64,  // 简单校验数据是否变化
}

/// 计算 items 的简单 hash
fn hash_items(items: &[Value]) -> u64 {
    let json_str = serde_json::to_string(items).unwrap_or_default();
    let mut h: u64 = 5381;
    for b in json_str.bytes() {
        h = h.wrapping_mul(33).wrapping_add(b as u64);
    }
    h
}

/// 获取游标文件路径
fn cursor_path(step_id: &str) -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".workflow-engine").join("cursors").join(format!("{}.json", step_id))
}

/// 解析 items
fn resolve_items(items_value: &Value, ctx: &ExecutionContext) -> Result<Vec<Value>> {
    if let Some(arr) = items_value.as_array() {
        return Ok(arr.clone());
    }
    if let Some(s) = items_value.as_str() {
        // 尝试将 JSON 字符串解析为数组
        if let Ok(parsed) = serde_json::from_str::<Value>(s) {
            if let Some(arr) = parsed.as_array() {
                return Ok(arr.clone());
            }
        }
        if let Some(key) = s.strip_prefix("output.") {
            return ctx.get_output(key)
                .and_then(|v| v.as_array())
                .cloned()
                .ok_or_else(|| anyhow!("cursor: items 引用 '{}' 无法解析为数组", s));
        }
        return ctx.get_output(s)
            .and_then(|v| v.as_array())
            .cloned()
            .or_else(|| ctx.variables.get(s).and_then(|v| v.as_array()).cloned())
            .ok_or_else(|| anyhow!("cursor: items '{}' 不是数组", s));
    }
    Err(anyhow!("cursor: items 必须是数组或引用"))
}

/// 解析 body 步骤：优先从 step.body_steps（编辑器 UI 设置），回退到 config.body（手动编辑）
fn parse_body_steps(step: &Step) -> Result<Vec<Step>> {
    // 优先 body_steps
    if let Some(ref body) = step.body_steps {
        if !body.is_empty() {
            return Ok(body.clone());
        }
    }
    // 回退 config.body
    let steps: Vec<Step> = serde_json::from_value(
        step.config.get("body").cloned().unwrap_or(json!([]))
    ).map_err(|e| anyhow!("cursor: body 解析失败: {}", e))?;
    if steps.is_empty() {
        return Err(anyhow!("cursor: body 不能为空"));
    }
    Ok(steps)
}

/// 读取游标状态
fn read_cursor(step_id: &str) -> CursorState {
    let path = cursor_path(step_id);
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(state) = serde_json::from_str::<CursorState>(&content) {
                return state;
            }
        }
    }
    CursorState { index: 0, total: 0, items_hash: 0 }
}

/// 保存游标状态
fn save_cursor(step_id: &str, state: &CursorState) {
    let path = cursor_path(step_id);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(&path, json);
    }
}

#[async_trait]
impl NodeExecutor for CursorNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        executor: &Arc<StepExecutor>,
    ) -> Result<Value> {
        let items_value = step.config.get("items")
            .ok_or_else(|| anyhow!("cursor 节点缺少 items 参数"))?;
        let items = resolve_items(items_value, ctx)?;
        let body_steps = parse_body_steps(step)?;
        let total = items.len();
        let items_hash = hash_items(&items);

        if total == 0 {
            return Ok(json!({ "done": true, "total": 0 }));
        }

        // 读游标
        let mut cursor = read_cursor(&step.id);

        // 数据变化检测：hash 不匹配就重置
        if cursor.items_hash != 0 && cursor.items_hash != items_hash {
            tracing::info!("cursor: 数据已变化，重置游标 (step={})", step.id);
            cursor = CursorState { index: 0, total, items_hash };
        }
        cursor.total = total;
        cursor.items_hash = items_hash;

        // 全部完成 → 重置并返回
        if cursor.index >= total {
            cursor.index = 0;
            save_cursor(&step.id, &cursor);
            return Ok(json!({ "done": true, "total": total }));
        }

        // 取当前项
        let current = items[cursor.index].clone();
        ctx.set_var("__item".to_string(), current.clone());
        ctx.set_var("__index".to_string(), json!(cursor.index));
        ctx.set_var("__index1".to_string(), json!(cursor.index + 1));
        // 友好别名：{{cursor.current}} / {{cursor.index}}
        ctx.set_var("cursor".to_string(), json!({
            "current": current,
            "index": cursor.index,
            "index1": cursor.index + 1,
            "total": total,
        }));

        // 执行 body
        for body_step in &body_steps {
            let mut resolved = body_step.clone();
            resolved.config = ctx.resolve_config(&body_step.config);
            let output = executor.execute(&resolved, ctx).await?;
            ctx.set_output(&body_step.id, output);
        }

        // 递进游标
        let done = cursor.index + 1 >= total;
        cursor.index += 1;
        save_cursor(&step.id, &cursor);

        Ok(json!({
            "done": done,
            "item": current,
            "index": cursor.index - 1,
            "total": total,
            "remaining": if done { 0 } else { total - cursor.index }
        }))
    }
}
