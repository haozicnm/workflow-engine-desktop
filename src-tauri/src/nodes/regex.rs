// nodes/regex.rs — 正则处理节点（v3: 每个操作独立 executor）
//
// regex_extract — 提取捕获组
// regex_replace — 替换匹配
// regex_match   — 查找匹配

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use regex::Regex;
use tracing::info;

// ── Shared helpers ──

fn compile_pattern(config: &serde_json::Value) -> Result<Regex> {
    let pattern = config.get("pattern")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("缺少 pattern 参数"))?;
    Regex::new(pattern).map_err(|e| anyhow!("正则表达式编译失败: {}", e))
}

fn get_input(config: &serde_json::Value) -> Result<String> {
    config.get("input")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| config.get("input").map(|v| v.to_string()))
        .ok_or_else(|| anyhow!("缺少 input 参数"))
}

fn expand_replacement(template: &str, caps: &regex::Captures) -> String {
    let mut result = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '$' {
            if let Some(&next) = chars.peek() {
                if next.is_ascii_digit() {
                    chars.next();
                    let index = next.to_digit(10).expect("ASCII数字转换失败") as usize;
                    if let Some(m) = caps.get(index) { result.push_str(m.as_str()); }
                    continue;
                }
                if next == '{' {
                    chars.next();
                    let mut name = String::new();
                    while let Some(&nc) = chars.peek() {
                        if nc == '}' { chars.next(); break; }
                        chars.next();
                        name.push(nc);
                    }
                    if let Some(m) = caps.name(&name) { result.push_str(m.as_str()); }
                    continue;
                }
            }
        }
        result.push(c);
    }
    result
}

// ═══════════════════════════════════════
// regex_extract — 提取捕获组
// ═══════════════════════════════════════

#[derive(Default)]
pub struct RegexExtractNode;

#[async_trait]
impl NodeExecutor for RegexExtractNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let re = compile_pattern(config)?;
        let input = get_input(config)?;
        let global = config.get("global").and_then(|v| v.as_bool()).unwrap_or(true);

        let mut captures: Vec<serde_json::Value> = Vec::new();
        if global {
            for caps in re.captures_iter(&input) {
                let groups: Vec<String> = caps.iter()
                    .map(|m| m.map(|m| m.as_str().to_string()).unwrap_or_default())
                    .collect();
                captures.push(serde_json::Value::Array(
                    groups.into_iter().map(serde_json::Value::String).collect()
                ));
            }
        } else if let Some(caps) = re.captures(&input) {
            let groups: Vec<String> = caps.iter()
                .map(|m| m.map(|m| m.as_str().to_string()).unwrap_or_default())
                .collect();
            captures.push(serde_json::Value::Array(
                groups.into_iter().map(serde_json::Value::String).collect()
            ));
        }

        let count = captures.len();
        info!("正则提取: pattern={}, {} 组", re.as_str(), count);
        Ok(serde_json::json!({ "pattern": re.as_str(), "captures": captures, "count": count }))
    }
}

// ═══════════════════════════════════════
// regex_replace — 替换匹配
// ═══════════════════════════════════════

#[derive(Default)]
pub struct RegexReplaceNode;

#[async_trait]
impl NodeExecutor for RegexReplaceNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let re = compile_pattern(config)?;
        let input = get_input(config)?;
        let replacement = config.get("replacement")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("缺少 replacement 参数"))?;
        let global = config.get("global").and_then(|v| v.as_bool()).unwrap_or(true);

        let (result, count) = if global {
            let mut n = 0;
            let result = re.replace_all(&input, |caps: &regex::Captures| {
                n += 1;
                expand_replacement(replacement, caps)
            }).to_string();
            (result, n)
        } else {
            let result = re.replace(&input, replacement).to_string();
            let count = if re.is_match(&input) { 1 } else { 0 };
            (result, count)
        };

        info!("正则替换: pattern={}, {} 处", re.as_str(), count);
        Ok(serde_json::json!({ "pattern": re.as_str(), "replacement": replacement, "result": result, "count": count }))
    }
}

// ═══════════════════════════════════════
// regex_match — 查找匹配
// ═══════════════════════════════════════

#[derive(Default)]
pub struct RegexMatchNode;

#[async_trait]
impl NodeExecutor for RegexMatchNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let re = compile_pattern(config)?;
        let input = get_input(config)?;
        let global = config.get("global").and_then(|v| v.as_bool()).unwrap_or(true);

        let mut matches: Vec<serde_json::Value> = Vec::new();
        if global {
            for m in re.find_iter(&input) {
                matches.push(serde_json::json!({ "text": m.as_str(), "start": m.start(), "end": m.end() }));
            }
        } else if let Some(m) = re.find(&input) {
            matches.push(serde_json::json!({ "text": m.as_str(), "start": m.start(), "end": m.end() }));
        }

        let count = matches.len();
        info!("正则匹配: pattern={}, {} 处", re.as_str(), count);
        Ok(serde_json::json!({ "pattern": re.as_str(), "matches": matches, "count": count, "is_match": count > 0 }))
    }
}
