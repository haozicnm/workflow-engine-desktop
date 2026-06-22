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

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Default)]
pub struct CursorNode;

/// 游标存储结构
#[derive(serde::Serialize, serde::Deserialize)]
struct CursorState {
    index: usize,
    total: usize,
    items_hash: u64, // 简单校验数据是否变化
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
    crate::data::paths::resolve_data_dir()
        .join("cursors")
        .join(format!("{}.json", step_id))
}

/// 读取游标状态
fn parse_body_steps(step: &Step) -> Result<Vec<Step>> {
    // 优先 body_steps
    if let Some(ref body) = step.body_steps {
        if !body.is_empty() {
            return Ok(body.clone());
        }
    }
    // 回退 config.body
    let steps: Vec<Step> =
        serde_json::from_value(step.config.get("body").cloned().unwrap_or(json!([])))
            .map_err(|e| anyhow!("cursor: body 解析失败: {}", e))?;
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
    CursorState {
        index: 0,
        total: 0,
        items_hash: 0,
    }
}

/// 保存游标状态
fn save_cursor(step_id: &str, state: &CursorState) {
    let path = cursor_path(step_id);
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            tracing::warn!("cursor: 创建目录失败 {}: {}", parent.display(), e);
            return;
        }
    }
    match serde_json::to_string_pretty(state) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                tracing::warn!("cursor: 保存游标失败 {}: {}", path.display(), e);
            }
        }
        Err(e) => tracing::warn!("cursor: 序列化失败: {}", e),
    }
}

#[async_trait]
impl NodeExecutor for CursorNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "cursor".into(),
            version: "1.0".into(),
            display_name: "游标迭代".into(),
            description: "每次工作流执行只处理一项数据，游标跨次持久化。适用于逐条处理 Excel 行、分页 API 等场景".into(),
            category: "流程控制".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "done".into(), data_type: "boolean".into(), required: false },
                crate::nodes::traits::PortDef { label: "item".into(), data_type: "any".into(), required: false },
                crate::nodes::traits::PortDef { label: "index".into(), data_type: "number".into(), required: false },
                crate::nodes::traits::PortDef { label: "total".into(), data_type: "number".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "required": ["items"],
                "properties": {
                    "items": {"type": "array", "description": "数据源数组"},
                    "body": {"type": "array", "description": "每项要执行的步骤"}
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
        let items_value = step
            .config
            .get("items")
            .ok_or_else(|| anyhow!("cursor 节点缺少 items 参数"))?;
        let items = crate::engine::common::resolve_iteration_items(items_value, ctx, "cursor")?;
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
            cursor = CursorState {
                index: 0,
                total,
                items_hash,
            };
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
        ctx.set_var(
            "cursor".to_string(),
            json!({
                "current": current,
                "index": cursor.index,
                "index1": cursor.index + 1,
                "total": total,
            }),
        );

        // 执行 body — 迭代变量已设入 ctx，容器/节点各自解析模板
        for body_step in &body_steps {
            let output = executor.execute(body_step, ctx).await?;
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
