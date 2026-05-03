// nodes/convert.rs — 类型转换节点（v3: 每个转换独立 executor）
//
// convert_to_text   — 值转文本
// convert_to_number — 文本转数字
// convert_to_json   — 文本转 JSON
// convert_to_csv    — 数组转 CSV
// convert_to_html   — 转 HTML
// convert_to_base64 — 文本转 Base64

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::info;

// ── Helpers ──

fn get_input(config: &serde_json::Value) -> Result<&serde_json::Value> {
    config.get("input").ok_or_else(|| anyhow!("缺少 input 参数"))
}

fn csv_escape(value: &str, delimiter: &str) -> String {
    if value.contains(delimiter) || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else { value.to_string() }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

// ═══════════════════════════════════════
// convert_to_text
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ConvertToTextNode;

#[async_trait]
impl NodeExecutor for ConvertToTextNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let input = get_input(&step.config)?;
        let result = serde_json::to_string(input).map_err(|e| anyhow!("序列化失败: {}", e))?;
        Ok(serde_json::json!({ "result": result }))
    }
}

// ═══════════════════════════════════════
// convert_to_number
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ConvertToNumberNode;

#[async_trait]
impl NodeExecutor for ConvertToNumberNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let input = get_input(&step.config)?;
        if input.is_number() { return Ok(serde_json::json!({ "result": input.clone() })); }
        let text = match input { serde_json::Value::String(s) => s.trim(), other => &other.to_string() };
        if let Ok(i) = text.parse::<i64>() { return Ok(serde_json::json!({ "result": i })); }
        if let Ok(f) = text.parse::<f64>() { return Ok(serde_json::json!({ "result": f })); }
        Err(anyhow!("无法将 '{}' 转换为数字", text))
    }
}

// ═══════════════════════════════════════
// convert_to_json
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ConvertToJsonNode;

#[async_trait]
impl NodeExecutor for ConvertToJsonNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let input = get_input(&step.config)?;
        if input.is_object() || input.is_array() || input.is_number() || input.is_boolean() {
            return Ok(serde_json::json!({ "result": input.clone() }));
        }
        let text = input.as_str().ok_or_else(|| anyhow!("to_json 需要字符串输入"))?;
        let parsed: serde_json::Value = serde_json::from_str(text).map_err(|e| anyhow!("JSON 解析失败: {}", e))?;
        info!("JSON 解析成功");
        Ok(serde_json::json!({ "result": parsed }))
    }
}

// ═══════════════════════════════════════
// convert_to_csv
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ConvertToCsvNode;

#[async_trait]
impl NodeExecutor for ConvertToCsvNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let input = get_input(config)?;
        let delimiter = config.get("delimiter").and_then(|v| v.as_str()).unwrap_or(",");
        let include_header = config.get("include_header").and_then(|v| v.as_bool()).unwrap_or(true);
        let arr = input.as_array().ok_or_else(|| anyhow!("to_csv 需要数组输入"))?;

        if arr.is_empty() { return Ok(serde_json::json!({ "result": "", "row_count": 0 })); }

        let mut columns: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for item in arr {
            if let Some(obj) = item.as_object() {
                for key in obj.keys() { if seen.insert(key.clone()) { columns.push(key.clone()); } }
            }
        }

        let mut csv = String::new();
        if include_header && !columns.is_empty() {
            csv.push_str(&columns.iter().map(|c| csv_escape(c, delimiter)).collect::<Vec<_>>().join(delimiter));
            csv.push('\n');
        }
        for item in arr {
            let row: Vec<String> = columns.iter().map(|col| match item.get(col) {
                Some(serde_json::Value::String(s)) => csv_escape(s, delimiter),
                Some(serde_json::Value::Null) => String::new(),
                Some(other) => csv_escape(&other.to_string(), delimiter),
                None => String::new(),
            }).collect();
            csv.push_str(&row.join(delimiter));
            csv.push('\n');
        }

        info!("CSV 转换: {} 行, {} 列", arr.len(), columns.len());
        Ok(serde_json::json!({ "result": csv.trim_end(), "row_count": arr.len(), "column_count": columns.len(), "columns": columns }))
    }
}

// ═══════════════════════════════════════
// convert_to_html
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ConvertToHtmlNode;

#[async_trait]
impl NodeExecutor for ConvertToHtmlNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let input = get_input(config)?;
        let template = config.get("template").and_then(|v| v.as_str());

        if let Some(tpl) = template {
            let html = tpl.replace("{{__data}}", &input.to_string());
            return Ok(serde_json::json!({ "method": "template", "result": html }));
        }

        let html = match input {
            serde_json::Value::Array(arr) if !arr.is_empty() => {
                let mut cols: Vec<String> = Vec::new();
                let mut seen = std::collections::HashSet::new();
                for item in arr {
                    if let Some(obj) = item.as_object() {
                        for key in obj.keys() { if seen.insert(key.clone()) { cols.push(key.clone()); } }
                    }
                }
                if cols.is_empty() {
                    let items: String = arr.iter().map(|v| format!("<li>{}</li>", html_escape(&v.to_string()))).collect::<Vec<_>>().join("");
                    format!("<ul>{}</ul>", items)
                } else {
                    let header: String = cols.iter().map(|c| format!("<th>{}</th>", html_escape(c))).collect::<Vec<_>>().join("");
                    let rows: String = arr.iter().map(|item| {
                        let cells: String = cols.iter().map(|col| {
                            let val = item.get(col).map(|v| match v { serde_json::Value::String(s) => html_escape(s), serde_json::Value::Null => String::new(), other => html_escape(&other.to_string()) }).unwrap_or_default();
                            format!("<td>{}</td>", val)
                        }).collect::<Vec<_>>().join("");
                        format!("<tr>{}</tr>", cells)
                    }).collect::<Vec<_>>().join("");
                    format!("<table><thead><tr>{}</tr></thead><tbody>{}</tbody></table>", header, rows)
                }
            }
            serde_json::Value::Object(obj) => {
                let items: String = obj.iter().map(|(k, v)| {
                    let val = match v { serde_json::Value::String(s) => html_escape(s), serde_json::Value::Null => String::new(), other => html_escape(&other.to_string()) };
                    format!("<dt>{}</dt><dd>{}</dd>", html_escape(k), val)
                }).collect::<Vec<_>>().join("");
                format!("<dl>{}</dl>", items)
            }
            serde_json::Value::String(s) => format!("<span>{}</span>", html_escape(s)),
            other => format!("<pre>{}</pre>", html_escape(&other.to_string())),
        };

        Ok(serde_json::json!({ "method": "auto", "result": html }))
    }
}

// ═══════════════════════════════════════
// convert_to_base64
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ConvertToBase64Node;

#[async_trait]
impl NodeExecutor for ConvertToBase64Node {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let input = get_input(&step.config)?;
        let result = match input {
            serde_json::Value::String(s) => BASE64.encode(s.as_bytes()),
            other => {
                let json = serde_json::to_string(other).map_err(|e| anyhow!("序列化失败: {}", e))?;
                BASE64.encode(json.as_bytes())
            }
        };
        Ok(serde_json::json!({ "result": result }))
    }
}
