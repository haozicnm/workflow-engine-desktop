// engine/placeholder.rs — 两阶段 resolve 的占位符机制
//
// 解决问题：容器节点跳过全局 resolve_config 以避免类型破坏，
// 但导致每个容器自己实现 resolve，实现不一致。
//
// 方案：占位符替换 → 反序列化 → 后处理替换

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::HashMap;
use crate::engine::context::ExecutionContext;

/// 占位符映射
pub struct PlaceholderMap {
    /// index → 原始表达式（如 "step_1.rows"）
    map: HashMap<usize, String>,
    /// 下一个可用的 index
    next_index: usize,
}

impl Default for PlaceholderMap {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaceholderMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
            next_index: 0,
        }
    }

    /// 生成占位符字符串
    fn make_placeholder(index: usize) -> String {
        format!("__WF_PH_{}__", index)
    }

    /// 检查字符串是否是占位符，返回 index
    pub fn parse_placeholder(s: &str) -> Option<usize> {
        if s.starts_with("__WF_PH_") && s.ends_with("__") {
            let inner = &s[8..s.len() - 2];
            inner.parse().ok()
        } else {
            None
        }
    }

    /// 扫描 JSON，替换 {{...}} 为占位符
    ///
    /// 递归遍历 JSON 树，找到所有 `{{expr}}` 格式的字符串，
    /// 替换为 `__WF_PH_{index}__`，并记录映射。
    pub fn scan_and_replace(&mut self, config: &mut Value) -> Result<()> {
        self.scan_value(config);
        Ok(())
    }

    fn scan_value(&mut self, value: &mut Value) {
        match value {
            Value::String(s) => {
                if let Some(replaced) = self.scan_string(s) {
                    *value = Value::String(replaced);
                }
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.scan_value(item);
                }
            }
            Value::Object(map) => {
                for (_, v) in map.iter_mut() {
                    self.scan_value(v);
                }
            }
            _ => {}
        }
    }

    /// 扫描字符串，替换 {{...}} 为占位符
    /// 只处理已知的变量引用（如 step_1.rows、params.title），不处理未知的变量引用（如 DATE）
    /// 返回 Some(新字符串) 如果有替换，None 如果没有
    fn scan_string(&mut self, s: &str) -> Option<String> {
        let mut result = String::new();
        let mut remaining = s;
        let mut changed = false;

        while let Some(start) = remaining.find("{{") {
            if let Some(end) = remaining[start + 2..].find("}}") {
                let expr = remaining[start + 2..start + 2 + end].trim().to_string();
                // 只处理已知的变量引用格式
                if Self::is_known_variable(&expr) {
                    let index = self.next_index;
                    self.next_index += 1;
                    self.map.insert(index, expr);

                    result.push_str(&remaining[..start]);
                    result.push_str(&Self::make_placeholder(index));
                    remaining = &remaining[start + 2 + end + 2..];
                    changed = true;
                } else {
                    // 未知变量引用，跳过
                    result.push_str(&remaining[..start + 2 + end + 2]);
                    remaining = &remaining[start + 2 + end + 2..];
                }
            } else {
                break;
            }
        }

        if changed {
            result.push_str(remaining);
            Some(result)
        } else {
            None
        }
    }

    /// 检查是否是已知的变量引用格式
    /// 已知格式：step_xxx.yyy、params.xxx、vars.xxx 等
    /// 注意：__item、__index 等延迟注入变量不在此列
    fn is_known_variable(expr: &str) -> bool {
        // step_xxx.yyy 格式（支持数组索引：step_1.items[0].name）
        if expr.starts_with("step_") && (expr.contains('.') || expr.contains('[')) {
            return true;
        }
        // params.xxx 格式
        if expr.starts_with("params.") {
            return true;
        }
        // vars.xxx 格式
        if expr.starts_with("vars.") {
            return true;
        }
        // 其他格式不处理
        false
    }

    /// 后处理：替换占位符为实际值
    ///
    /// 递归遍历 Value，找到所有占位符字符串，替换为 ExecutionContext 中的实际值。
    /// 这是公共接口，用于 executor 层处理 condition_group 等特殊字段。
    pub fn resolve_value(&self, value: &mut Value, ctx: &ExecutionContext) -> Result<()> {
        self.resolve_value_inner(value, ctx)
    }

    fn resolve_value_inner(&self, value: &mut Value, ctx: &ExecutionContext) -> Result<()> {
        match value {
            Value::String(s) => {
                // 先检查是否是纯占位符
                let placeholder_index = Self::parse_placeholder(s);
                let is_mixed = s.contains("__WF_PH_");

                if let Some(index) = placeholder_index {
                    // 纯占位符：直接替换为实际值（保持原始类型）
                    // 数组/对象/数字都会保持原样
                    if let Some(expr) = self.map.get(&index) {
                        let resolved = self.resolve_expression(expr, ctx)?;
                        *value = resolved;
                    }
                } else if is_mixed {
                    // 混合字符串：替换占位符为字符串形式
                    let new_s = self.resolve_mixed_string(s, ctx)?;
                    if new_s != *s {
                        *value = Value::String(new_s);
                    }
                }
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.resolve_value_inner(item, ctx)?;
                }
            }
            Value::Object(map) => {
                for (_, v) in map.iter_mut() {
                    self.resolve_value_inner(v, ctx)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// 解析表达式，返回实际值
    fn resolve_expression(&self, expr: &str, ctx: &ExecutionContext) -> Result<Value> {
        // 尝试作为变量引用解析
        if let Some(val) = ctx.resolve_var(expr) {
            return Ok(val.clone());
        }
        // 尝试作为 Rhai 表达式解析
        ctx.eval_expr(expr)
            .map_err(|e| anyhow!("表达式解析失败: {}", e))
    }

    /// 处理混合字符串（如 "用户 __WF_PH_0__ 的分数"）
    fn resolve_mixed_string(&self, s: &str, ctx: &ExecutionContext) -> Result<String> {
        let mut result = String::new();
        let mut remaining = s;

        while let Some(start) = remaining.find("__WF_PH_") {
            if let Some(end) = remaining[start..].find("__") {
                let placeholder = &remaining[start..start + end + 2];
                if let Some(index) = Self::parse_placeholder(placeholder) {
                    if let Some(expr) = self.map.get(&index) {
                        let resolved = self.resolve_expression(expr, ctx)?;
                        let resolved_str = match resolved {
                            Value::String(s) => s,
                            Value::Number(n) => n.to_string(),
                            Value::Bool(b) => b.to_string(),
                            Value::Null => String::new(),
                            other => serde_json::to_string(&other).unwrap_or_default(),
                        };
                        result.push_str(&remaining[..start]);
                        result.push_str(&resolved_str);
                        remaining = &remaining[start + end + 2..];
                        continue;
                    }
                }
                // 不是有效占位符，跳过
                result.push_str(&remaining[..start + end + 2]);
                remaining = &remaining[start + end + 2..];
            } else {
                break;
            }
        }

        result.push_str(remaining);
        Ok(result)
    }

    /// 获取映射中的条目数
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// 检查映射是否为空
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_and_replace_simple() {
        let mut ph = PlaceholderMap::new();
        let mut config = serde_json::json!({
            "value": "{{step_1.rows}}",
            "name": "test"
        });

        ph.scan_and_replace(&mut config).unwrap();

        assert_eq!(config["value"].as_str().unwrap(), "__WF_PH_0__");
        assert_eq!(config["name"].as_str().unwrap(), "test");
        assert_eq!(ph.len(), 1);
    }

    #[test]
    fn test_scan_and_replace_mixed() {
        let mut ph = PlaceholderMap::new();
        let mut config = serde_json::json!({
            "template": "用户 {{params.name}} 的分数是 {{step_1.score}}"
        });

        ph.scan_and_replace(&mut config).unwrap();

        let template = config["template"].as_str().unwrap();
        assert!(template.contains("__WF_PH_0__"));
        assert!(template.contains("__WF_PH_1__"));
        assert_eq!(ph.len(), 2);
    }

    #[test]
    fn test_scan_and_replace_nested() {
        let mut ph = PlaceholderMap::new();
        let mut config = serde_json::json!({
            "actions": [
                {"id": "a1", "config": {"value": "{{step_1.data}}"}}
            ]
        });

        ph.scan_and_replace(&mut config).unwrap();

        assert_eq!(
            config["actions"][0]["config"]["value"].as_str().unwrap(),
            "__WF_PH_0__"
        );
        assert_eq!(ph.len(), 1);
    }

    #[test]
    fn test_parse_placeholder() {
        assert_eq!(PlaceholderMap::parse_placeholder("__WF_PH_0__"), Some(0));
        assert_eq!(PlaceholderMap::parse_placeholder("__WF_PH_42__"), Some(42));
        assert_eq!(PlaceholderMap::parse_placeholder("not_a_placeholder"), None);
        assert_eq!(PlaceholderMap::parse_placeholder("__WF_PH_abc__"), None);
    }
}
