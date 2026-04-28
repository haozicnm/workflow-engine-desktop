// nodes/regex.rs — 正则处理节点
//
// 支持操作：
//   extract   提取捕获组:  {action: "extract", pattern: "...", input: "..."}
//   replace   替换匹配:    {action: "replace", pattern: "...", replacement: "...", input: "..."}
//   match     查找匹配:    {action: "match", pattern: "...", input: "..."}
//
// 使用 Rust regex crate 进行正则匹配。

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use regex::Regex;
use tracing::info;

#[derive(Default)]
pub struct RegexNode;

#[async_trait]
impl NodeExecutor for RegexNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("正则节点缺少 action 参数"))?;

        match action {
            "extract" => regex_extract(config),
            "replace" => regex_replace(config),
            "match" => regex_match(config),
            _ => Err(anyhow!(
                "未知的正则操作: {}（支持 extract/replace/match）",
                action
            )),
        }
    }
}

/// 编译正则表达式
fn compile_pattern(config: &serde_json::Value) -> Result<Regex> {
    let pattern = config.get("pattern")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("正则操作缺少 pattern 参数"))?;

    Regex::new(pattern)
        .map_err(|e| anyhow!("正则表达式编译失败: {}", e))
}

/// 获取输入文本
fn get_input(config: &serde_json::Value) -> Result<String> {
    config.get("input")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| config.get("input").map(|v| v.to_string()))
        .ok_or_else(|| anyhow!("正则操作缺少 input 参数"))
}

/// 提取捕获组
///
/// 返回格式：
/// {
///   "action": "extract",
///   "pattern": "...",
///   "captures": [
///     ["full_match", "group1", "group2", ...],  // 每个匹配的捕获组
///     ...
///   ],
///   "count": N
/// }
fn regex_extract(config: &serde_json::Value) -> Result<serde_json::Value> {
    let re = compile_pattern(config)?;
    let input = get_input(config)?;
    let global = config.get("global")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut captures: Vec<Vec<String>> = Vec::new();

    if global {
        for caps in re.captures_iter(&input) {
            let groups: Vec<String> = caps.iter()
                .map(|m| m.map(|m| m.as_str().to_string()).unwrap_or_default())
                .collect();

            // 可选：按名称组织捕获组
            if let Some(named) = config.get("named").and_then(|v| v.as_bool()) {
                if named {
                    let mut named_map = serde_json::Map::new();
                    for name in re.capture_names().flatten() {
                        if let Some(m) = caps.name(name) {
                            named_map.insert(name.to_string(), serde_json::Value::String(m.as_str().to_string()));
                        }
                    }
                    captures.push(groups);
                    // 同时附加命名捕获
                    let _ = named_map;
                } else {
                    captures.push(groups);
                }
            } else {
                captures.push(groups);
            }
        }
    } else {
        // 仅第一个匹配
        if let Some(caps) = re.captures(&input) {
            let groups: Vec<String> = caps.iter()
                .map(|m| m.map(|m| m.as_str().to_string()).unwrap_or_default())
                .collect();
            captures.push(groups);
        }
    }

    let count = captures.len();
    info!("正则提取: pattern={}, 匹配 {} 组", re.as_str(), count);

    Ok(serde_json::json!({
        "action": "extract",
        "pattern": re.as_str(),
        "captures": captures,
        "count": count,
    }))
}

/// 替换匹配内容
///
/// 返回格式：
/// {
///   "action": "replace",
///   "pattern": "...",
///   "replacement": "...",
///   "result": "替换后的文本",
///   "count": N
/// }
fn regex_replace(config: &serde_json::Value) -> Result<serde_json::Value> {
    let re = compile_pattern(config)?;
    let input = get_input(config)?;

    let replacement = config.get("replacement")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("replace 操作缺少 replacement 参数"))?;

    let global = config.get("global")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

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

    info!("正则替换: pattern={}, {} 处替换", re.as_str(), count);

    Ok(serde_json::json!({
        "action": "replace",
        "pattern": re.as_str(),
        "replacement": replacement,
        "result": result,
        "count": count,
    }))
}

/// 展开替换模板中的 $1, $2 等引用
fn expand_replacement(template: &str, caps: &regex::Captures) -> String {
    let mut result = String::new();
    let mut chars = template.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '$' {
            if let Some(&next) = chars.peek() {
                if next.is_ascii_digit() {
                    chars.next();
                    let index = next.to_digit(10).unwrap() as usize;
                    if let Some(m) = caps.get(index) {
                        result.push_str(m.as_str());
                    }
                    continue;
                }
                if next == '{' {
                    chars.next(); // skip '{'
                    let mut name = String::new();
                    while let Some(&nc) = chars.peek() {
                        if nc == '}' {
                            chars.next();
                            break;
                        }
                        chars.next();
                        name.push(nc);
                    }
                    if let Some(m) = caps.name(&name) {
                        result.push_str(m.as_str());
                    }
                    continue;
                }
            }
        }
        result.push(c);
    }

    result
}

/// 查找所有匹配
///
/// 返回格式：
/// {
///   "action": "match",
///   "pattern": "...",
///   "matches": [
///     {"text": "matched", "start": 0, "end": 5},
///     ...
///   ],
///   "count": N,
///   "is_match": true
/// }
fn regex_match(config: &serde_json::Value) -> Result<serde_json::Value> {
    let re = compile_pattern(config)?;
    let input = get_input(config)?;

    let global = config.get("global")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut matches: Vec<serde_json::Value> = Vec::new();

    if global {
        for m in re.find_iter(&input) {
            matches.push(serde_json::json!({
                "text": m.as_str(),
                "start": m.start(),
                "end": m.end(),
            }));
        }
    } else {
        if let Some(m) = re.find(&input) {
            matches.push(serde_json::json!({
                "text": m.as_str(),
                "start": m.start(),
                "end": m.end(),
            }));
        }
    }

    let count = matches.len();
    let is_match = count > 0;

    info!("正则匹配: pattern={}, {} 处匹配", re.as_str(), count);

    Ok(serde_json::json!({
        "action": "match",
        "pattern": re.as_str(),
        "matches": matches,
        "count": count,
        "is_match": is_match,
    }))
}
