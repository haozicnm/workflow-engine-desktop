// engine/common.rs — 引擎公共函数（loop / cursor 共用）

use crate::engine::context::ExecutionContext;
use anyhow::{anyhow, Result};
use serde_json::Value;

/// 解析迭代 items（loop / cursor 共用）
///
/// 支持：
///   1. 直接数组 `["a", "b"]`
///   2. JSON 编码的字符串数组 `"[\"a\",\"b\"]"`
///   3. 引用字符串（`output.step_x` 或变量名）
///
/// `node_label` 用于错误消息前缀，如 "cursor" 或 "循环"
pub fn resolve_iteration_items(
    items_value: &Value,
    ctx: &ExecutionContext,
    node_label: &str,
) -> Result<Vec<Value>> {
    if let Some(arr) = items_value.as_array() {
        return Ok(arr.clone());
    }
    if let Some(s) = items_value.as_str() {
        // 先解析变量引用
        let resolved = ctx.resolve_config(items_value);
        let resolved_s = resolved.as_str().unwrap_or(s);

        // 尝试将 JSON 字符串解析为数组
        if let Ok(parsed) = serde_json::from_str::<Value>(resolved_s) {
            if let Some(arr) = parsed.as_array() {
                return Ok(arr.clone());
            }
        }
        // 如果解析后是数组直接返回
        if let Some(arr) = resolved.as_array() {
            return Ok(arr.clone());
        }
        // 尝试 output.xxx 格式
        if let Some(key) = resolved_s.strip_prefix("output.") {
            return ctx
                .get_output(key)
                .and_then(|v| v.as_array())
                .cloned()
                .ok_or_else(|| anyhow!("{}: items 引用 '{}' 无法解析为数组", node_label, s));
        }
        return ctx
            .get_output(resolved_s)
            .and_then(|v| v.as_array())
            .cloned()
            .or_else(|| {
                ctx.variables
                    .get(resolved_s)
                    .and_then(|v| v.as_array())
                    .cloned()
            })
            .ok_or_else(|| anyhow!("{}: items '{}' 不是数组", node_label, s));
    }
    Err(anyhow!("{}: items 必须是数组或引用", node_label))
}
