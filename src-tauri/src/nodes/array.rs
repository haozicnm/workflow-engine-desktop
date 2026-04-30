// nodes/array.rs — 数组操作节点
//
// 支持操作：
//   filter    按条件过滤:  {action: "filter", source: [...], condition: {field, op, value}}
//   sort      排序:        {action: "sort", source: [...], field: "name", order: "asc"}
//   dedup     去重:        {action: "dedup", source: [...], field: "name"}
//   paginate  分页:        {action: "paginate", source: [...], page: 1, page_size: 10}
//   map       映射:        {action: "map", source: [...], template: "{{__item.name}}"}
//   join      连接:        {action: "join", source: [...], separator: ","}
//   reduce    聚合:        {action: "reduce", source: [...], aggregator: "sum", field: "price"}
//
// source 支持：
//   - 内联数组: ["a", "b", "c"]
//   - 变量引用: "output.step_id" 或 "output.step_id.field"
//   - 上下文变量名: "my_array"

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use tracing::info;

#[derive(Default)]
pub struct ArrayNode;

#[async_trait]
impl NodeExecutor for ArrayNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("数组节点缺少 action 参数"))?;

        // 解析 source：可能是内联数组、output.step_id 引用或变量名
        let source = resolve_array_source(
            config.get("source").unwrap_or(&serde_json::Value::Null),
            ctx,
        )?;

        match action {
            "filter" => array_filter(&source, config),
            "sort" => array_sort(&source, config),
            "dedup" => array_dedup(&source, config),
            "paginate" => array_paginate(&source, config),
            "map" => array_map(&source, config, ctx),
            "join" => array_join(&source, config),
            "reduce" => array_reduce(&source, config),
            _ => Err(anyhow!(
                "未知的数组操作: {}（支持 filter/sort/dedup/paginate/map/join/reduce）",
                action
            )),
        }
    }
}

/// 从 config 中解析数组 source
///
/// 支持三种格式：
///   1. 内联数组: source: ["a", "b"]
///   2. output 引用: source: "output.step_id" 或 "output.step_id.data.items"
///   3. 变量名: source: "my_var"
fn resolve_array_source(source: &serde_json::Value, ctx: &ExecutionContext) -> Result<Vec<serde_json::Value>> {
    match source {
        // 内联数组
        serde_json::Value::Array(arr) => Ok(arr.clone()),

        // 字符串引用
        serde_json::Value::String(ref_str) => {
            // output.step_id 引用
            if let Some(path) = ref_str.strip_prefix("output.") {
                let parts: Vec<&str> = path.splitn(2, '.').collect();
                let step_id = parts[0];
                let field = parts.get(1);

                let output = ctx.get_output(step_id)
                    .ok_or_else(|| anyhow!("步骤输出不存在: step_{}", step_id))?;

                if let Some(f) = field {
                    // 支持嵌套字段如 "output.step_id.data.items"
                    let val = resolve_nested_field(output, f);
                    val.as_array()
                        .cloned()
                        .ok_or_else(|| anyhow!("字段 '{}' 不是数组", f))
                } else {
                    output.as_array()
                        .cloned()
                        .ok_or_else(|| anyhow!("步骤输出不是数组: step_{}", step_id))
                }
            } else {
                // 上下文变量
                ctx.variables.get(ref_str)
                    .and_then(|v| v.as_array())
                    .cloned()
                    .or_else(|| {
                        ctx.variables.get(ref_str)
                            .and_then(|v| {
                                // 如果是对象，尝试包裹为单元素数组
                                if v.is_object() {
                                    Some(vec![v.clone()])
                                } else {
                                    None
                                }
                            })
                    })
                    .ok_or_else(|| anyhow!("变量不存在或不是数组: {}", ref_str))
            }
        }

        other => Err(anyhow!("无效的 source 类型: {}（需要数组或 output/变量引用）", other)),
    }
}

/// 解析嵌套字段路径 "data.items"
fn resolve_nested_field(val: &serde_json::Value, path: &str) -> serde_json::Value {
    let mut current = val.clone();
    for part in path.split('.') {
        current = current.get(part).cloned().unwrap_or(serde_json::Value::Null);
    }
    current
}

/// 按条件过滤数组
///
/// condition 支持：
///   {"field": "price", "op": ">=", "value": 100}
///   op: == / != / > / >= / < / <= / contains / starts_with / ends_with / is_null / is_not_null
fn array_filter(source: &[serde_json::Value], config: &serde_json::Value) -> Result<serde_json::Value> {
    let condition = config.get("condition")
        .ok_or_else(|| anyhow!("filter 操作缺少 condition 参数"))?;

    let field = condition.get("field").and_then(|v| v.as_str());
    let op = condition.get("op").and_then(|v| v.as_str()).unwrap_or("==");

    let result: Vec<serde_json::Value> = source.iter()
        .filter(|item| eval_condition(item, condition, field, op))
        .cloned()
        .collect();

    info!("数组过滤: {} → {} 条", source.len(), result.len());

    Ok(serde_json::json!({
        "action": "filter",
        "source_count": source.len(),
        "result_count": result.len(),
        "result": result,
    }))
}

/// 评估单个条件
fn eval_condition(
    item: &serde_json::Value,
    condition: &serde_json::Value,
    field: Option<&str>,
    op: &str,
) -> bool {
    let item_val = if let Some(f) = field {
        // 支持嵌套字段 "a.b.c"
        let parts: Vec<&str> = f.split('.').collect();
        let mut current = item;
        for part in parts {
            match current.get(part) {
                Some(v) => current = v,
                None => return false,
            }
        }
        current
    } else {
        item
    };

    match op {
        "is_null" => item_val.is_null(),
        "is_not_null" => !item_val.is_null(),
        "is_empty" => match item_val {
            serde_json::Value::String(s) => s.is_empty(),
            serde_json::Value::Array(a) => a.is_empty(),
            serde_json::Value::Null => true,
            _ => false,
        },
        "is_not_empty" => !eval_condition(item, condition, field, "is_empty"),
        _ => {
            let target = condition.get("value");
            compare_values(item_val, target, op)
        }
    }
}

/// 比较两个 JSON 值
fn compare_values(a: &serde_json::Value, b: Option<&serde_json::Value>, op: &str) -> bool {
    let b = match b {
        Some(v) => v,
        None => return false,
    };

    // 字符串比较
    if let (Some(sa), Some(sb)) = (a.as_str(), b.as_str()) {
        return match op {
            "==" => sa == sb,
            "!=" => sa != sb,
            "contains" => sa.contains(sb),
            "starts_with" => sa.starts_with(sb),
            "ends_with" => sa.ends_with(sb),
            _ => false,
        };
    }

    // 数值比较
    if let (Some(na), Some(nb)) = (a.as_f64(), b.as_f64()) {
        return match op {
            "==" => (na - nb).abs() < f64::EPSILON,
            "!=" => (na - nb).abs() >= f64::EPSILON,
            ">" => na > nb,
            ">=" => na >= nb,
            "<" => na < nb,
            "<=" => na <= nb,
            _ => false,
        };
    }

    // 布尔比较
    if let (Some(ba), Some(bb)) = (a.as_bool(), b.as_bool()) {
        return match op {
            "==" => ba == bb,
            "!=" => ba != bb,
            _ => false,
        };
    }

    // 类型不同时用字符串比较
    match op {
        "==" => *a == *b,
        "!=" => *a != *b,
        _ => false,
    }
}

/// 按字段排序
fn array_sort(source: &[serde_json::Value], config: &serde_json::Value) -> Result<serde_json::Value> {
    let field = config.get("field")
        .and_then(|v| v.as_str());
    let order = config.get("order")
        .and_then(|v| v.as_str())
        .unwrap_or("asc");

    let mut result: Vec<serde_json::Value> = source.to_vec();
    let ascending = order == "asc";

    result.sort_by(|a, b| {
        let va = extract_field(a, field);
        let vb = extract_field(b, field);
        let ord = compare_json(va, vb);
        if ascending { ord } else { ord.reverse() }
    });

    info!("数组排序: {} 条, field={:?}, order={}", result.len(), field, order);

    Ok(serde_json::json!({
        "action": "sort",
        "source_count": source.len(),
        "field": field,
        "order": order,
        "result": result,
    }))
}

/// 提取字段值（支持嵌套 "a.b.c"）
fn extract_field<'a>(item: &'a serde_json::Value, field: Option<&str>) -> &'a serde_json::Value {
    match field {
        Some(f) => {
            let mut current = item;
            for part in f.split('.') {
                match current.get(part) {
                    Some(v) => current = v,
                    None => return &serde_json::Value::Null,
                }
            }
            current
        }
        None => item,
    }
}

/// JSON 值排序比较
fn compare_json(a: &serde_json::Value, b: &serde_json::Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (a, b) {
        (serde_json::Value::Number(na), serde_json::Value::Number(nb)) => {
            na.as_f64().partial_cmp(&nb.as_f64()).unwrap_or(Ordering::Equal)
        }
        (serde_json::Value::String(sa), serde_json::Value::String(sb)) => {
            sa.cmp(sb)
        }
        (serde_json::Value::Bool(ba), serde_json::Value::Bool(bb)) => {
            ba.cmp(bb)
        }
        (serde_json::Value::Null, _) => Ordering::Less,
        (_, serde_json::Value::Null) => Ordering::Greater,
        _ => a.to_string().cmp(&b.to_string()),
    }
}

/// 按字段去重
fn array_dedup(source: &[serde_json::Value], config: &serde_json::Value) -> Result<serde_json::Value> {
    let field = config.get("field")
        .and_then(|v| v.as_str());

    let mut seen = std::collections::HashSet::new();
    let mut result: Vec<serde_json::Value> = Vec::new();

    for item in source {
        let key = match field {
            Some(f) => {
                let val = extract_field(item, Some(f));
                format_key(val)
            }
            None => format_key(item),
        };

        if seen.insert(key) {
            result.push(item.clone());
        }
    }

    info!("数组去重: {} → {} 条, field={:?}", source.len(), result.len(), field);

    Ok(serde_json::json!({
        "action": "dedup",
        "source_count": source.len(),
        "result_count": result.len(),
        "field": field,
        "result": result,
    }))
}

/// 格式化 JSON 值为去重 key
fn format_key(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "__null__".to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

/// 分页
fn array_paginate(source: &[serde_json::Value], config: &serde_json::Value) -> Result<serde_json::Value> {
    let page = config.get("page")
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .max(1) as usize;

    let page_size = config.get("page_size")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .max(1) as usize;

    let total = source.len();
    let total_pages = total.div_ceil(page_size);
    let start = ((page - 1) * page_size).min(total);
    let end = (start + page_size).min(total);

    let page_items: Vec<serde_json::Value> = source[start..end].to_vec();

    info!("数组分页: page={}/{}, {} 条", page, total_pages, page_items.len());

    Ok(serde_json::json!({
        "action": "paginate",
        "page": page,
        "page_size": page_size,
        "total": total,
        "total_pages": total_pages,
        "count": page_items.len(),
        "result": page_items,
    }))
}

/// 数组映射（模板替换）
///
/// 对每个元素应用模板，支持 {{__item}} 和 {{__index}} 占位符
fn array_map(
    source: &[serde_json::Value],
    config: &serde_json::Value,
    _ctx: &ExecutionContext,
) -> Result<serde_json::Value> {
    let template = config.get("template")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("map 操作缺少 template 参数"))?;

    // 可选的映射字段提取
    let fields: Option<Vec<&str>> = config.get("fields")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|f| f.as_str()).collect());

    let result: Vec<serde_json::Value> = source.iter().enumerate().map(|(idx, item)| {
        apply_map_template(template, item, idx, fields.as_deref())
    }).collect();

    info!("数组映射: {} 条", result.len());

    Ok(serde_json::json!({
        "action": "map",
        "source_count": source.len(),
        "template": template,
        "result": result,
    }))
}

/// 应用 map 模板到单个元素
fn apply_map_template(
    template: &str,
    item: &serde_json::Value,
    index: usize,
    fields: Option<&[&str]>,
) -> serde_json::Value {
    // 仅"{{__item}}" → 直接返回 item
    if template.trim() == "{{__item}}" {
        return item.clone();
    }
    if template.trim() == "{{__index}}" {
        return serde_json::json!(index);
    }

    // 模板插值
    let mut result = template.to_string();
    result = result.replace("{{__index}}", &index.to_string());

    // 替换 {{__item.xxx}} 引用
    let re = regex::Regex::new(r"\{\{__item\.(\w+(?:\.\w+)*)\}\}")
        .expect("正则表达式 {{__item.xxx}} 编译失败");
    result = re.replace_all(&result, |caps: &regex::Captures| {
        let path = caps.get(1)
            .expect("捕获组1应在正则匹配中存在")
            .as_str();
        let val = extract_field(item, Some(path));
        match val {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }).to_string();

    // 替换 {{__item}} 整个引用
    result = result.replace("{{__item}}", &item.to_string());

    // 如果指定了 fields，只提取指定字段
    if let Some(field_list) = fields {
        if field_list.is_empty() {
            return serde_json::Value::String(result);
        }
        let mut obj = serde_json::Map::new();
        for f in field_list {
            let val = extract_field(item, Some(f));
            obj.insert(f.to_string(), val.clone());
        }
        return serde_json::Value::Object(obj);
    }

    serde_json::Value::String(result)
}

/// 数组连接
fn array_join(source: &[serde_json::Value], config: &serde_json::Value) -> Result<serde_json::Value> {
    let separator = config.get("separator")
        .and_then(|v| v.as_str())
        .unwrap_or(",");

    let field = config.get("field")
        .and_then(|v| v.as_str());

    let parts: Vec<String> = source.iter().map(|item| {
        let val = match field {
            Some(f) => extract_field(item, Some(f)),
            None => item,
        };
        match val {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Null => String::new(),
            other => other.to_string(),
        }
    }).collect();

    let result = parts.join(separator);
    let len = result.len();

    Ok(serde_json::json!({
        "action": "join",
        "source_count": source.len(),
        "separator": separator,
        "field": field,
        "length": len,
        "result": result,
    }))
}

/// 数组聚合（reduce）
///
/// aggregator: sum / avg / min / max / count
fn array_reduce(source: &[serde_json::Value], config: &serde_json::Value) -> Result<serde_json::Value> {
    let aggregator = config.get("aggregator")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("reduce 操作缺少 aggregator 参数（sum/avg/min/max/count）"))?;

    let field = config.get("field")
        .and_then(|v| v.as_str());

    // count 不需要 field
    if aggregator == "count" {
        return Ok(serde_json::json!({
            "action": "reduce",
            "aggregator": "count",
            "source_count": source.len(),
            "result": source.len(),
        }));
    }

    // 提取数值
    let values: Vec<f64> = source.iter()
        .filter_map(|item| {
            let val = match field {
                Some(f) => extract_field(item, Some(f)),
                None => item,
            };
            val.as_f64()
                .or_else(|| val.as_str().and_then(|s| s.parse::<f64>().ok()))
                .or_else(|| {
                    if val.is_i64() {
                        val.as_i64().map(|i| i as f64)
                    } else {
                        None
                    }
                })
        })
        .collect();

    if values.is_empty() {
        return Ok(serde_json::json!({
            "action": "reduce",
            "aggregator": aggregator,
            "source_count": source.len(),
            "field": field,
            "result": serde_json::Value::Null,
        }));
    }

    let result = match aggregator {
        "sum" => values.iter().sum::<f64>(),
        "avg" => values.iter().sum::<f64>() / values.len() as f64,
        "min" => values.iter().cloned().fold(f64::INFINITY, f64::min),
        "max" => values.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        _ => return Err(anyhow!("不支持的聚合方式: {}（支持 sum/avg/min/max/count）", aggregator)),
    };

    info!("数组聚合: {}={}, {} 条", aggregator, result, values.len());

    Ok(serde_json::json!({
        "action": "reduce",
        "aggregator": aggregator,
        "source_count": source.len(),
        "field": field,
        "value_count": values.len(),
        "result": result,
    }))
}
