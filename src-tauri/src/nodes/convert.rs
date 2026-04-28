// nodes/convert.rs — 类型转换节点
//
// 支持操作：
//   to_text   值转文本:      {action: "to_text", input: ...}
//   to_number 文本转数字:    {action: "to_number", input: "123"}
//   to_json   文本转 JSON:   {action: "to_json", input: '{"key": "value"}'}
//   to_csv    数组转 CSV:    {action: "to_csv", input: [...], delimiter: ","}
//   to_html   转 HTML:       {action: "to_html", input: ..., template: "..."}
//   to_base64 文本转 base64:  {action: "to_base64", input: "hello"}

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use tracing::info;

#[derive(Default)]
pub struct ConvertNode;

#[async_trait]
impl NodeExecutor for ConvertNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("转换节点缺少 action 参数"))?;

        // 获取输入值（已由 executor resolve 模板变量）
        let input = config.get("input")
            .ok_or_else(|| anyhow!("转换节点缺少 input 参数"))?;

        match action {
            "to_text" => to_text(input),
            "to_number" => to_number(input),
            "to_json" => to_json(input),
            "to_csv" => to_csv(input, config),
            "to_html" => to_html(input, config),
            "to_base64" => to_base64(input),
            _ => Err(anyhow!(
                "未知的转换操作: {}（支持 to_text/to_number/to_json/to_csv/to_html/to_base64）",
                action
            )),
        }
    }
}

/// 值转文本（JSON.stringify）
fn to_text(input: &serde_json::Value) -> Result<serde_json::Value> {
    let pretty = false; // 默认紧凑输出

    let result = if pretty {
        serde_json::to_string_pretty(input)
            .map_err(|e| anyhow!("序列化失败: {}", e))?
    } else {
        serde_json::to_string(input)
            .map_err(|e| anyhow!("序列化失败: {}", e))?
    };

    Ok(serde_json::json!({
        "action": "to_text",
        "result": result,
    }))
}

/// 文本转数字
fn to_number(input: &serde_json::Value) -> Result<serde_json::Value> {
    // 已经是数字
    if input.is_number() {
        return Ok(serde_json::json!({
            "action": "to_number",
            "result": input.clone(),
        }));
    }

    let text = match input {
        serde_json::Value::String(s) => s.trim(),
        other => &other.to_string(),
    };

    // 尝试解析为整数
    if let Ok(i) = text.parse::<i64>() {
        return Ok(serde_json::json!({
            "action": "to_number",
            "input_type": "string",
            "result": i,
        }));
    }

    // 尝试解析为浮点数
    if let Ok(f) = text.parse::<f64>() {
        return Ok(serde_json::json!({
            "action": "to_number",
            "input_type": "string",
            "result": f,
        }));
    }

    Err(anyhow!("无法将 '{}' 转换为数字", text))
}

/// JSON 文本解析
fn to_json(input: &serde_json::Value) -> Result<serde_json::Value> {
    // 如果已经是对象/数组/数字/布尔，直接返回
    if input.is_object() || input.is_array() || input.is_number() || input.is_boolean() {
        return Ok(serde_json::json!({
            "action": "to_json",
            "result": input.clone(),
        }));
    }

    let text = match input {
        serde_json::Value::String(s) => s.as_str(),
        other => {
            return Err(anyhow!("to_json 需要字符串输入，收到了: {}", other));
        }
    };

    let parsed: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| anyhow!("JSON 解析失败: {}", e))?;

    info!("JSON 解析成功");

    Ok(serde_json::json!({
        "action": "to_json",
        "result": parsed,
    }))
}

/// 数组转 CSV
fn to_csv(input: &serde_json::Value, config: &serde_json::Value) -> Result<serde_json::Value> {
    let delimiter = config.get("delimiter")
        .and_then(|v| v.as_str())
        .unwrap_or(",");

    let include_header = config.get("include_header")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let arr = input.as_array()
        .ok_or_else(|| anyhow!("to_csv 需要数组输入"))?;

    if arr.is_empty() {
        return Ok(serde_json::json!({
            "action": "to_csv",
            "result": "",
            "row_count": 0,
        }));
    }

    // 收集所有列名
    let mut columns: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for item in arr {
        if let Some(obj) = item.as_object() {
            for key in obj.keys() {
                if seen.insert(key.clone()) {
                    columns.push(key.clone());
                }
            }
        }
    }

    let mut csv = String::new();

    // 表头
    if include_header && !columns.is_empty() {
        csv.push_str(&columns.iter()
            .map(|c| csv_escape(c, delimiter))
            .collect::<Vec<_>>()
            .join(delimiter));
        csv.push('\n');
    }

    // 数据行
    for item in arr {
        let row: Vec<String> = columns.iter().map(|col| {
            match item.get(col) {
                Some(serde_json::Value::String(s)) => csv_escape(s, delimiter),
                Some(serde_json::Value::Null) => String::new(),
                Some(other) => csv_escape(&other.to_string(), delimiter),
                None => String::new(),
            }
        }).collect();
        csv.push_str(&row.join(delimiter));
        csv.push('\n');
    }

    info!("CSV 转换: {} 行, {} 列", arr.len(), columns.len());

    Ok(serde_json::json!({
        "action": "to_csv",
        "result": csv.trim_end(),
        "row_count": arr.len(),
        "column_count": columns.len(),
        "columns": columns,
    }))
}

/// CSV 字段转义
fn csv_escape(value: &str, delimiter: &str) -> String {
    if value.contains(delimiter) || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// 转 HTML（简单表格/列表）
fn to_html(input: &serde_json::Value, config: &serde_json::Value) -> Result<serde_json::Value> {
    let template = config.get("template")
        .and_then(|v| v.as_str());

    // 如果提供了模板，将整个 input 作为 __data 变量替换
    if let Some(tpl) = template {
        let html = tpl.replace("{{__data}}", &input.to_string());
        return Ok(serde_json::json!({
            "action": "to_html",
            "method": "template",
            "result": html,
        }));
    }

    // 默认：数组 → HTML 表格，对象 → HTML 列表
    let html = match input {
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                "<table></table>".to_string()
            } else {
                json_array_to_html_table(arr)
            }
        }
        serde_json::Value::Object(obj) => {
            json_object_to_html_list(obj)
        }
        serde_json::Value::String(s) => {
            format!("<span>{}</span>", html_escape(s))
        }
        other => {
            format!("<pre>{}</pre>", html_escape(&other.to_string()))
        }
    };

    Ok(serde_json::json!({
        "action": "to_html",
        "method": "auto",
        "result": html,
    }))
}

/// JSON 数组 → HTML 表格
fn json_array_to_html_table(arr: &[serde_json::Value]) -> String {
    // 收集所有列
    let mut columns: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for item in arr {
        if let Some(obj) = item.as_object() {
            for key in obj.keys() {
                if seen.insert(key.clone()) {
                    columns.push(key.clone());
                }
            }
        }
    }

    if columns.is_empty() {
        // 普通数组，转列表
        let items: String = arr.iter()
            .map(|v| format!("<li>{}</li>", html_escape(&v.to_string())))
            .collect::<Vec<_>>()
            .join("");
        return format!("<ul>{}</ul>", items);
    }

    // 对象数组，转表格
    let header: String = columns.iter()
        .map(|c| format!("<th>{}</th>", html_escape(c)))
        .collect::<Vec<_>>()
        .join("");

    let rows: String = arr.iter().map(|item| {
        let cells: String = columns.iter().map(|col| {
            let val = item.get(col).map(|v| match v {
                serde_json::Value::String(s) => html_escape(s),
                serde_json::Value::Null => String::new(),
                other => html_escape(&other.to_string()),
            }).unwrap_or_default();
            format!("<td>{}</td>", val)
        }).collect::<Vec<_>>().join("");
        format!("<tr>{}</tr>", cells)
    }).collect::<Vec<_>>().join("");

    format!("<table><thead><tr>{}</tr></thead><tbody>{}</tbody></table>", header, rows)
}

/// JSON 对象 → HTML 描述列表
fn json_object_to_html_list(obj: &serde_json::Map<String, serde_json::Value>) -> String {
    let items: String = obj.iter().map(|(k, v)| {
        let val = match v {
            serde_json::Value::String(s) => html_escape(s),
            serde_json::Value::Null => String::new(),
            other => html_escape(&other.to_string()),
        };
        format!("<dt>{}</dt><dd>{}</dd>", html_escape(k), val)
    }).collect::<Vec<_>>().join("");

    format!("<dl>{}</dl>", items)
}

/// HTML 转义
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// 文本转 Base64
fn to_base64(input: &serde_json::Value) -> Result<serde_json::Value> {
    let text = match input {
        serde_json::Value::String(s) => s.as_str(),
        other => {
            // 非字符串：先 JSON 序列化再编码
            let json = serde_json::to_string(other)
                .map_err(|e| anyhow!("序列化失败: {}", e))?;
            return Ok(serde_json::json!({
                "action": "to_base64",
                "result": BASE64.encode(json.as_bytes()),
            }));
        }
    };

    Ok(serde_json::json!({
        "action": "to_base64",
        "result": BASE64.encode(text.as_bytes()),
    }))
}
