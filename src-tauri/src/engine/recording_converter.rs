// engine/recording_converter.rs — 录制操作 → 工作流 YAML 转换器
//
// 将浏览器/桌面录制产生的原始操作列表转换为结构化工作流 YAML。
// 核心价值：用户录完操作后一键生成可编辑的工作流。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 单条录制操作
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecordedAction {
    /// 操作类型: click / fill / navigate / scroll / type / hotkey / select / wait
    #[serde(rename = "type")]
    pub action_type: String,
    /// CSS 选择器（浏览器操作）
    pub selector: Option<String>,
    /// 输入值
    pub value: Option<String>,
    /// 坐标
    pub x: Option<f64>,
    pub y: Option<f64>,
    /// URL（导航操作）
    pub url: Option<String>,
    /// 按键组合（热键操作）
    pub keys: Option<String>,
    /// 滚动量
    pub amount: Option<i64>,
    /// 鼠标按键
    pub button: Option<String>,
    /// 时间戳（毫秒）
    pub timestamp: Option<u64>,
    /// 来源: browser / desktop
    pub source: Option<String>,
    /// 页面标题（浏览器操作）
    pub page_title: Option<String>,
}

/// 录制来源
#[derive(Debug, Clone, PartialEq)]
pub enum RecordingSource {
    Browser,
    Desktop,
    Mixed,
}

/// 步骤模板（生成 YAML 步骤的中间表示）
#[derive(Debug, Clone)]
struct StepTemplate {
    id: String,
    name: String,
    step_type: String,
    config: Value,
    /// 原始操作索引列表（用于追溯）
    action_indices: Vec<usize>,
}

/// 录制操作 → 工作流转换结果
#[derive(Debug, Clone, Serialize)]
pub struct ConversionResult {
    /// 生成的工作流 YAML 字符串
    pub yaml: String,
    /// 步骤数量
    pub step_count: usize,
    /// 操作数量
    pub action_count: usize,
    /// 合并优化数量（被合并的重复/相邻操作）
    pub merged_count: usize,
    /// 步骤概要列表（用于前端预览）
    pub step_summary: Vec<StepSummary>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StepSummary {
    pub id: String,
    pub name: String,
    pub step_type: String,
    pub description: String,
}

/// 将录制操作列表转换为工作流 YAML
///
/// 转换策略：
/// 1. 按来源分组（浏览器 vs 桌面）
/// 2. 每个操作映射为对应节点类型
/// 3. 智能合并（连续 fill → 合并；连续 click 同元素 → 去重；导航后自动分组）
/// 4. 生成带注释的 YAML
pub fn convert_actions_to_workflow(
    actions: &[RecordedAction],
    workflow_name: &str,
    source: RecordingSource,
) -> ConversionResult {
    let total_actions = actions.len();
    if total_actions == 0 {
        return ConversionResult {
            yaml: String::new(),
            step_count: 0,
            action_count: 0,
            merged_count: 0,
            step_summary: vec![],
        };
    }

    // 步骤 1：每个操作映射为步骤模板
    let raw_templates: Vec<StepTemplate> = actions
        .iter()
        .enumerate()
        .map(|(i, action)| action_to_template(action, i))
        .collect();

    // 步骤 2：智能合并
    let merged_templates = merge_templates(&raw_templates, &source);

    let merged_count = raw_templates.len() - merged_templates.len();
    let step_count = merged_templates.len();

    // 步骤 3：生成 YAML
    let yaml = generate_yaml(&merged_templates, workflow_name);

    // 步骤 4：生成概要
    let step_summary: Vec<StepSummary> = merged_templates
        .iter()
        .map(|t| StepSummary {
            id: t.id.clone(),
            name: t.name.clone(),
            step_type: t.step_type.clone(),
            description: step_description(&t.step_type, &t.config),
        })
        .collect();

    ConversionResult {
        yaml,
        step_count,
        action_count: total_actions,
        merged_count,
        step_summary,
    }
}

// ═══════════════════════════════════════════════
// v2.0 JSON 输出（nodes + edges）
// ═══════════════════════════════════════════════

/// 将录制操作列表转换为工作流 JSON（v2.0 格式：nodes + edges 数组）
///
/// 输出格式：
/// ```json
/// {
///   "name": "录制工作流",
///   "description": "由录制生成的浏览器自动化",
///   "nodes": [
///     {"id": "n1", "type": "browser_navigate", "label": "打开页面", "position": {"x": 60, "y": 60}, "config": {"url": "...", "wait_until": "load"}},
///     {"id": "n2", "type": "browser_click", "label": "点击元素", "position": {"x": 380, "y": 60}, "config": {"selector": "#btn"}}
///   ],
///   "edges": [
///     {"id": "e_n1_n2", "source": "n1", "target": "n2", "sourceHandle": "page", "targetHandle": "selector"}
///   ]
/// }
/// ```
///
/// 节点类型映射：
///   navigate → browser_navigate（输出 pin: page）
///   click    → browser_click（输入: selector, 输出: data）
///   fill     → browser_fill（输入: selector + value, 输出: data）
///   type     → browser_fill（同 fill）
///   scroll   → browser_scroll
///   wait     → browser_wait
///   hotkey   → browser（万能节点，action="evaluate"）
///
/// 布局：水平排列，320px 间距，60px 边距
pub fn convert_to_json(actions: &[RecordedAction], source: &str) -> Value {
    let mut nodes: Vec<Value> = Vec::new();
    let mut edges: Vec<Value> = Vec::new();

    let margin: f64 = 60.0;
    let spacing: f64 = 320.0;

    // 步骤 1：每个操作 → 节点
    for (i, action) in actions.iter().enumerate() {
        let id = format!("n{}", i + 1);
        let x = margin + (i as f64) * spacing;
        let y = margin;

        let (node_type, label, config) = action_to_node(action);

        nodes.push(serde_json::json!({
            "id": id,
            "type": node_type,
            "label": label,
            "position": {"x": x, "y": y},
            "config": config,
        }));
    }

    // 步骤 2：生成顺序连线
    for i in 1..actions.len() {
        let source_id = format!("n{}", i);
        let target_id = format!("n{}", i + 1);
        let (source_handle, target_handle) = get_handles(&actions[i - 1], &actions[i]);

        edges.push(serde_json::json!({
            "id": format!("e_{}_{}", source_id, target_id),
            "source": source_id,
            "target": target_id,
            "sourceHandle": source_handle,
            "targetHandle": target_handle,
        }));
    }

    serde_json::json!({
        "name": "录制工作流",
        "description": format!("由录制生成的浏览器自动化 — 来源: {}", source),
        "nodes": nodes,
        "edges": edges,
    })
}

/// 单条操作 → 节点类型、标签、配置
fn action_to_node(action: &RecordedAction) -> (&'static str, String, Value) {
    match action.action_type.as_str() {
        "navigate" => {
            let url = action.url.clone().unwrap_or_default();
            (
                "browser_navigate",
                "打开页面".to_string(),
                serde_json::json!({
                    "url": url,
                    "wait_until": "load",
                }),
            )
        }
        "click" => {
            let label = if let Some(ref sel) = action.selector {
                format!("点击 {}", sel)
            } else if let (Some(x), Some(y)) = (action.x, action.y) {
                format!("点击 ({}, {})", x, y)
            } else {
                "点击元素".to_string()
            };
            (
                "browser_click",
                label,
                serde_json::json!({
                    "selector": action.selector.clone().unwrap_or_default(),
                }),
            )
        }
        "fill" | "type" => {
            let value = action.value.clone().unwrap_or_default();
            let label = if let Some(ref sel) = action.selector {
                format!("填写 {}", sel)
            } else {
                "填写输入".to_string()
            };
            (
                "browser_fill",
                label,
                serde_json::json!({
                    "selector": action.selector.clone().unwrap_or_default(),
                    "value": value,
                }),
            )
        }
        "scroll" => {
            let amount = action.amount.unwrap_or(3);
            (
                "browser_scroll",
                format!("滚动 x{}", amount),
                serde_json::json!({
                    "direction": if amount > 0 { "down" } else { "up" },
                    "amount": amount.abs(),
                }),
            )
        }
        "wait" => (
            "browser_wait",
            "等待".to_string(),
            serde_json::json!({
                "selector": action.selector.clone().unwrap_or_default(),
                "timeout": 10000,
            }),
        ),
        "hotkey" => {
            let keys = action.keys.clone().unwrap_or_default();
            (
                "browser",
                format!("快捷键 {}", keys),
                serde_json::json!({
                    "action": "evaluate",
                    "expression": format!("await page.keyboard.press('{}')", keys),
                }),
            )
        }
        _ => {
            // 未知类型 → 万能 browser 节点（保留操作数据供用户手动调整）
            (
                "browser",
                "未知操作".to_string(),
                serde_json::json!({
                    "action": "evaluate",
                    "expression": "",
                    "_raw": action,
                }),
            )
        }
    }
}

/// 获取两个连续操作之间的连接手柄名称
///
/// 上游输出手柄：
///   navigate → "page"
///   其他     → "data"
///
/// 下游输入手柄：
///   navigate        → "url"
///   click/fill/wait → "selector"
///   其他            → "selector"
fn get_handles(from: &RecordedAction, to: &RecordedAction) -> (&'static str, &'static str) {
    let source_handle = match from.action_type.as_str() {
        "navigate" => "page",
        _ => "data",
    };

    let target_handle = match to.action_type.as_str() {
        "navigate" => "url",
        "click" | "fill" | "type" | "wait" | "scroll" => "selector",
        _ => "selector",
    };

    (source_handle, target_handle)
}

/// 单个操作 → 步骤模板
fn action_to_template(action: &RecordedAction, index: usize) -> StepTemplate {
    let id = format!("step_{}", index + 1);
    match action.action_type.as_str() {
        "navigate" => {
            let url = action.url.clone().unwrap_or_default();
            StepTemplate {
                id: id.clone(),
                name: "打开页面".to_string(),
                step_type: "browser".to_string(),
                config: serde_json::json!({
                    "action": "navigate",
                    "params": {
                        "url": url,
                        "wait_until": "load"
                    }
                }),
                action_indices: vec![index],
            }
        }
        "click" => {
            let desc = if let Some(ref sel) = action.selector {
                format!("点击 {}", sel)
            } else if let (Some(x), Some(y)) = (action.x, action.y) {
                format!("点击 ({}, {})", x, y)
            } else {
                "点击".to_string()
            };

            // 浏览器点击 → browser 节点；桌面点击 → mouse_keyboard 节点
            let (step_type, config) = if action.source.as_deref() == Some("desktop") {
                (
                    "mouse_keyboard".to_string(),
                    serde_json::json!({
                        "action": "click",
                        "x": action.x.unwrap_or(0.0),
                        "y": action.y.unwrap_or(0.0),
                        "button": action.button.as_deref().unwrap_or("left"),
                    }),
                )
            } else {
                (
                    "browser".to_string(),
                    serde_json::json!({
                        "action": "click",
                        "params": {
                            "selector": action.selector.clone().unwrap_or_default(),
                        }
                    }),
                )
            };

            StepTemplate {
                id: id.clone(),
                name: desc,
                step_type,
                config,
                action_indices: vec![index],
            }
        }
        "fill" | "type" => {
            let value = action.value.clone().unwrap_or_default();
            let desc = if let Some(ref sel) = action.selector {
                format!("输入 {} → {}", sel, truncate(&value, 30))
            } else {
                format!("输入文本: {}", truncate(&value, 30))
            };

            let (step_type, config) = if action.source.as_deref() == Some("desktop") {
                (
                    "mouse_keyboard".to_string(),
                    serde_json::json!({
                        "action": "type",
                        "text": value,
                        "delay_ms": 30,
                    }),
                )
            } else {
                (
                    "browser".to_string(),
                    serde_json::json!({
                        "action": "fill",
                        "params": {
                            "selector": action.selector.clone().unwrap_or_default(),
                            "value": value,
                            "clear": true,
                        }
                    }),
                )
            };

            StepTemplate {
                id: id.clone(),
                name: desc,
                step_type,
                config,
                action_indices: vec![index],
            }
        }
        "scroll" => {
            let amount = action.amount.unwrap_or(3);
            let desc = format!("滚动 x{}", amount);

            let (step_type, config) = if action.source.as_deref() == Some("desktop") {
                (
                    "mouse_keyboard".to_string(),
                    serde_json::json!({
                        "action": "scroll",
                        "amount": amount,
                    }),
                )
            } else {
                (
                    "browser".to_string(),
                    serde_json::json!({
                        "action": "scroll_to",
                        "params": {
                            "to": if amount > 0 { "bottom" } else { "top" },
                            "times": amount.abs(),
                        }
                    }),
                )
            };

            StepTemplate {
                id: id.clone(),
                name: desc,
                step_type,
                config,
                action_indices: vec![index],
            }
        }
        "hotkey" => {
            let keys = action.keys.clone().unwrap_or_default();
            StepTemplate {
                id: id.clone(),
                name: format!("快捷键 {}", keys),
                step_type: "mouse_keyboard".to_string(),
                config: serde_json::json!({
                    "action": "hotkey",
                    "keys": keys,
                }),
                action_indices: vec![index],
            }
        }
        "select" => StepTemplate {
            id: id.clone(),
            name: format!("下拉选择 {}", action.selector.as_deref().unwrap_or("")),
            step_type: "browser".to_string(),
            config: serde_json::json!({
                "action": "select",
                "params": {
                    "selector": action.selector.clone().unwrap_or_default(),
                    "value": action.value.clone().unwrap_or_default(),
                }
            }),
            action_indices: vec![index],
        },
        "wait" => StepTemplate {
            id: id.clone(),
            name: "等待".to_string(),
            step_type: "browser".to_string(),
            config: serde_json::json!({
                "action": "wait",
                "params": {
                    "selector": action.selector.clone().unwrap_or_default(),
                    "timeout_ms": 10000,
                }
            }),
            action_indices: vec![index],
        },
        "screenshot" => StepTemplate {
            id: id.clone(),
            name: "截图".to_string(),
            step_type: "browser".to_string(),
            config: serde_json::json!({
                "action": "screenshot",
                "params": {
                    "full_page": true,
                }
            }),
            action_indices: vec![index],
        },
        // 未知类型 → 脚本节点（保留原始数据供用户调整）
        _ => StepTemplate {
            id: id.clone(),
            name: format!("操作 #{}", index + 1),
            step_type: "data".to_string(),
            config: serde_json::json!({
                "action": "set",
                "key": format!("recorded_action_{}", index + 1),
                "value": serde_json::to_value(action).unwrap_or(Value::Null),
            }),
            action_indices: vec![index],
        },
    }
}

/// 智能合并相邻步骤模板
///
/// 合并规则：
///   - 连续多个 fill → 合并为一个"批量填表"步骤
///   - 连续 click 同一元素（去重）
///   - navigate 后的 fill/click 自动分组到该页面步骤
fn merge_templates(templates: &[StepTemplate], source: &RecordingSource) -> Vec<StepTemplate> {
    if templates.is_empty() {
        return vec![];
    }

    let mut merged: Vec<StepTemplate> = Vec::new();
    let mut i = 0;

    while i < templates.len() {
        let current = &templates[i];

        // 规则 1：连续 fill 合并
        if current.step_type == "browser" && is_fill_action(&current.config) {
            let mut fills: Vec<Value> = vec![extract_fill_params(&current.config)];
            let mut indices = current.action_indices.clone();
            let mut j = i + 1;

            while j < templates.len() && is_fill_action(&templates[j].config) {
                fills.push(extract_fill_params(&templates[j].config));
                indices.extend(&templates[j].action_indices);
                j += 1;
            }

            if fills.len() == 1 {
                merged.push(current.clone());
            } else {
                merged.push(StepTemplate {
                    id: current.id.clone(),
                    name: format!("批量填表 ({} 个字段)", fills.len()),
                    step_type: "browser".to_string(),
                    config: serde_json::json!({
                        "action": "fill",
                        "params": {
                            "fields": fills,
                        }
                    }),
                    action_indices: indices,
                });
            }
            i = j;
            continue;
        }

        // 规则 2：导航后跟的操作 → 归组到导航页面
        if is_navigate_action(&current.config) && i + 1 < templates.len() {
            let _page_url = get_navigate_url(&current.config);
            let mut desc_parts: Vec<String> = vec![];
            let mut next_idx = i + 1;

            // 收集导航后同页面的操作描述
            while next_idx < templates.len()
                && !is_navigate_action(&templates[next_idx].config)
                && next_idx - i <= 5
            {
                desc_parts.push(templates[next_idx].name.clone());
                next_idx += 1;
            }

            if !desc_parts.is_empty() && source != &RecordingSource::Desktop {
                let mut new_config = current.config.clone();
                if let Some(obj) = new_config.as_object_mut() {
                    obj.insert(
                        "page_actions".to_string(),
                        serde_json::json!(desc_parts.join(" → ")),
                    );
                }
            }

            merged.push(current.clone());
            i += 1;
            continue;
        }

        // 默认：不合并
        merged.push(current.clone());
        i += 1;
    }

    merged
}

fn is_fill_action(config: &Value) -> bool {
    config.get("action").and_then(|v| v.as_str()) == Some("fill")
}

fn is_navigate_action(config: &Value) -> bool {
    config.get("action").and_then(|v| v.as_str()) == Some("navigate")
}

fn get_navigate_url(config: &Value) -> String {
    config
        .get("params")
        .and_then(|v| v.get("url"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn extract_fill_params(config: &Value) -> Value {
    config.get("params").cloned().unwrap_or(Value::Null)
}

/// 生成工作流 YAML
fn generate_yaml(templates: &[StepTemplate], workflow_name: &str) -> String {
    let mut yaml = String::new();

    // 头部
    yaml.push_str(&format!(
        "# 工作流: {}\n# 由录制操作自动生成\n# 步骤数: {}\n\n",
        workflow_name,
        templates.len()
    ));
    yaml.push_str(&format!("name: \"{}\"\n", workflow_name));
    yaml.push_str(&format!(
        "description: \"录制于 {}，共 {} 个步骤\"\n",
        chrono::Utc::now().format("%Y-%m-%d %H:%M"),
        templates.len()
    ));
    yaml.push_str("steps:\n");

    // 步骤列表
    for (i, template) in templates.iter().enumerate() {
        yaml.push_str(&format!("  - id: \"{}\"\n", template.id));
        yaml.push_str(&format!("    name: \"{}\"\n", template.name));
        yaml.push_str(&format!("    type: \"{}\"\n", template.step_type));

        // config 转为 YAML 格式
        let config_yaml = json_to_yaml_string(&template.config, "      ");
        if !config_yaml.is_empty() {
            yaml.push_str("    config:\n");
            yaml.push_str(&config_yaml);
        }

        // 默认顺序连接（非最后一个步骤）
        if i < templates.len() - 1 {
            yaml.push_str(&format!("    next: \"{}\"\n", templates[i + 1].id));
        }

        yaml.push('\n');
    }

    // 变量区（预留）
    yaml.push_str("variables:\n");
    yaml.push_str("  workflow_name: \"");
    yaml.push_str(workflow_name);
    yaml.push_str("\"\n");

    yaml
}

/// JSON Value → 缩进 YAML 字符串（简化版）
fn json_to_yaml_string(value: &Value, indent: &str) -> String {
    match value {
        Value::Object(map) => {
            let mut result = String::new();
            for (k, v) in map {
                match v {
                    Value::String(s) => {
                        // 包含特殊字符的字符串加引号
                        if s.contains(':') || s.contains('#') || s.contains('{') || s.contains('}')
                        {
                            result.push_str(&format!("{}{}: \"{}\"\n", indent, k, s));
                        } else if s.is_empty() {
                            result.push_str(&format!("{}{}: \"\"\n", indent, k));
                        } else {
                            result.push_str(&format!("{}{}: {}\n", indent, k, s));
                        }
                    }
                    Value::Number(n) => {
                        result.push_str(&format!("{}{}: {}\n", indent, k, n));
                    }
                    Value::Bool(b) => {
                        result.push_str(&format!(
                            "{}{}: {}\n",
                            indent,
                            k,
                            if *b { "true" } else { "false" }
                        ));
                    }
                    Value::Object(_) => {
                        result.push_str(&format!("{}{}:\n", indent, k));
                        result.push_str(&json_to_yaml_string(v, &format!("{}  ", indent)));
                    }
                    Value::Array(arr) => {
                        result.push_str(&format!("{}{}:\n", indent, k));
                        for item in arr {
                            result.push_str(&format!("{}  - ", indent));
                            if let Value::String(s) = item {
                                result.push_str(s);
                                result.push('\n');
                            } else {
                                result.push_str(&format!("{}\n", item));
                            }
                        }
                    }
                    Value::Null => {
                        result.push_str(&format!("{}{}: null\n", indent, k));
                    }
                }
            }
            result
        }
        Value::String(s) => {
            format!("{}{}\n", indent, s)
        }
        _ => format!("{}{}\n", indent, value),
    }
}

/// 生成步骤的人类可读描述
fn step_description(step_type: &str, config: &Value) -> String {
    match step_type {
        "browser" => {
            let action = config.get("action").and_then(|v| v.as_str()).unwrap_or("");
            match action {
                "navigate" => {
                    let url = config
                        .get("params")
                        .and_then(|p| p.get("url"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    format!("打开页面: {}", truncate(url, 50))
                }
                "click" => {
                    let sel = config
                        .get("params")
                        .and_then(|p| p.get("selector"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    format!("点击: {}", truncate(sel, 40))
                }
                "fill" => {
                    let sel = config
                        .get("params")
                        .and_then(|p| p.get("selector"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let val = config
                        .get("params")
                        .and_then(|p| p.get("value"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    format!("填写 {} ← {}", truncate(sel, 20), truncate(val, 20))
                }
                _ => format!("浏览器: {}", action),
            }
        }
        "mouse_keyboard" => {
            let action = config.get("action").and_then(|v| v.as_str()).unwrap_or("");
            match action {
                "click" => {
                    let x = config.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let y = config.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    format!("鼠标点击 ({:.0}, {:.0})", x, y)
                }
                "type" => {
                    let text = config.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    format!("输入文本: {}", truncate(text, 30))
                }
                "hotkey" => {
                    let keys = config.get("keys").and_then(|v| v.as_str()).unwrap_or("");
                    format!("快捷键: {}", keys)
                }
                "scroll" => {
                    let amount = config.get("amount").and_then(|v| v.as_i64()).unwrap_or(0);
                    format!("滚动 x{}", amount)
                }
                _ => format!("桌面操作: {}", action),
            }
        }
        "data" => "数据处理".to_string(),
        _ => step_type.to_string(),
    }
}

/// 截断字符串用于显示
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        format!("{}…", &s.chars().take(max_len).collect::<String>())
    }
}

// ═══════════════════════════════════════════════
// 测试
// ═══════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_actions() {
        let result = convert_actions_to_workflow(&[], "test", RecordingSource::Browser);
        assert_eq!(result.step_count, 0);
        assert_eq!(result.action_count, 0);
    }

    #[test]
    fn test_single_navigate() {
        let actions = vec![RecordedAction {
            action_type: "navigate".to_string(),
            url: Some("https://example.com".to_string()),
            selector: None,
            value: None,
            x: None,
            y: None,
            keys: None,
            amount: None,
            button: None,
            timestamp: Some(1000),
            source: Some("browser".to_string()),
            page_title: None,
        }];
        let result = convert_actions_to_workflow(&actions, "测试", RecordingSource::Browser);
        assert_eq!(result.step_count, 1);
        assert!(result.yaml.contains("navigate"));
        assert!(result.yaml.contains("https://example.com"));
    }

    #[test]
    fn test_merge_consecutive_fills() {
        let actions = vec![
            RecordedAction {
                action_type: "fill".to_string(),
                selector: Some("#username".to_string()),
                value: Some("admin".to_string()),
                source: Some("browser".to_string()),
                url: None,
                x: None,
                y: None,
                keys: None,
                amount: None,
                button: None,
                timestamp: Some(1000),
                page_title: None,
            },
            RecordedAction {
                action_type: "fill".to_string(),
                selector: Some("#password".to_string()),
                value: Some("123456".to_string()),
                source: Some("browser".to_string()),
                url: None,
                x: None,
                y: None,
                keys: None,
                amount: None,
                button: None,
                timestamp: Some(2000),
                page_title: None,
            },
        ];
        let result = convert_actions_to_workflow(&actions, "填表", RecordingSource::Browser);
        assert_eq!(result.step_count, 1);
        assert!(result.step_summary[0].name.contains("批量填表"));
        assert!(result.yaml.contains("fields"));
    }

    #[test]
    fn test_desktop_click() {
        let actions = vec![RecordedAction {
            action_type: "click".to_string(),
            x: Some(500.0),
            y: Some(300.0),
            button: Some("left".to_string()),
            source: Some("desktop".to_string()),
            selector: None,
            value: None,
            url: None,
            keys: None,
            amount: None,
            timestamp: Some(1000),
            page_title: None,
        }];
        let result = convert_actions_to_workflow(&actions, "桌面点击", RecordingSource::Desktop);
        assert_eq!(result.step_count, 1);
        assert!(result.yaml.contains("mouse_keyboard"));
        assert!(result.yaml.contains("x: 500"));
    }
}
