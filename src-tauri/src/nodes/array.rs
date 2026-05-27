// nodes/array.rs — 数组操作节点（v3: 每个操作独立 executor）
//
// array_filter   — 条件过滤
// array_sort     — 排序
// array_dedup    — 去重
// array_paginate — 分页
// array_map      — 映射
// array_join     — 连接
// array_reduce   — 聚合

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;
use tracing::info;

// ── Shared helpers ──

fn resolve_array_source(
    source: &serde_json::Value,
    ctx: &ExecutionContext,
) -> Result<Vec<serde_json::Value>> {
    match source {
        serde_json::Value::Array(arr) => Ok(arr.clone()),
        serde_json::Value::String(ref_str) => {
            if let Some(path) = ref_str.strip_prefix("output.") {
                let parts: Vec<&str> = path.splitn(2, '.').collect();
                let step_id = parts[0];
                let field = parts.get(1);
                let output = ctx
                    .get_output(step_id)
                    .ok_or_else(|| anyhow!("步骤输出不存在: step_{}", step_id))?;
                if let Some(f) = field {
                    let val = resolve_nested_field(output, f);
                    val.as_array()
                        .cloned()
                        .ok_or_else(|| anyhow!("字段 '{}' 不是数组", f))
                } else {
                    output
                        .as_array()
                        .cloned()
                        .ok_or_else(|| anyhow!("步骤输出不是数组: step_{}", step_id))
                }
            } else {
                ctx.variables
                    .get(ref_str)
                    .and_then(|v| v.as_array())
                    .cloned()
                    .or_else(|| {
                        ctx.variables.get(ref_str).and_then(|v| {
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
        other => Err(anyhow!(
            "无效的 source 类型: {}（需要数组或 output/变量引用）",
            other
        )),
    }
}

fn resolve_nested_field(val: &serde_json::Value, path: &str) -> serde_json::Value {
    let mut current = val.clone();
    for part in path.split('.') {
        current = current
            .get(part)
            .cloned()
            .unwrap_or(serde_json::Value::Null);
    }
    current
}

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

fn compare_values(a: &serde_json::Value, b: Option<&serde_json::Value>, op: &str) -> bool {
    let b = match b {
        Some(v) => v,
        None => return false,
    };
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
    if let (Some(ba), Some(bb)) = (a.as_bool(), b.as_bool()) {
        return match op {
            "==" => ba == bb,
            "!=" => ba != bb,
            _ => false,
        };
    }
    match op {
        "==" => *a == *b,
        "!=" => *a != *b,
        _ => false,
    }
}

fn eval_condition(
    item: &serde_json::Value,
    condition: &serde_json::Value,
    field: Option<&str>,
    op: &str,
) -> bool {
    let item_val = if let Some(f) = field {
        let mut current = item;
        for part in f.split('.') {
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
        "is_empty" => {
            matches!(item_val, serde_json::Value::String(s) if s.is_empty())
                || matches!(item_val, serde_json::Value::Array(a) if a.is_empty())
                || item_val.is_null()
        }
        "is_not_empty" => !eval_condition(item, condition, field, "is_empty"),
        _ => compare_values(item_val, condition.get("value"), op),
    }
}

fn compare_json(a: &serde_json::Value, b: &serde_json::Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (a, b) {
        (serde_json::Value::Number(na), serde_json::Value::Number(nb)) => na
            .as_f64()
            .partial_cmp(&nb.as_f64())
            .unwrap_or(Ordering::Equal),
        (serde_json::Value::String(sa), serde_json::Value::String(sb)) => sa.cmp(sb),
        (serde_json::Value::Bool(ba), serde_json::Value::Bool(bb)) => ba.cmp(bb),
        (serde_json::Value::Null, _) => Ordering::Less,
        (_, serde_json::Value::Null) => Ordering::Greater,
        _ => a.to_string().cmp(&b.to_string()),
    }
}

fn format_key(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "__null__".to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

// ═══════════════════════════════════════
// array_filter — 条件过滤
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ArrayFilterNode;

#[async_trait]
impl NodeExecutor for ArrayFilterNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let source = resolve_array_source(
            config.get("source").unwrap_or(&serde_json::Value::Null),
            ctx,
        )?;
        let condition = config
            .get("condition")
            .ok_or_else(|| anyhow!("filter 缺少 condition 参数"))?;
        let field = condition.get("field").and_then(|v| v.as_str());
        let op = condition.get("op").and_then(|v| v.as_str()).unwrap_or("==");

        let result: Vec<serde_json::Value> = source
            .iter()
            .filter(|item| eval_condition(item, condition, field, op))
            .cloned()
            .collect();

        info!("数组过滤: {} → {} 条", source.len(), result.len());
        Ok(
            serde_json::json!({ "source_count": source.len(), "result_count": result.len(), "result": result }),
        )
    }
}

// ═══════════════════════════════════════
// array_sort — 排序
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ArraySortNode;

#[async_trait]
impl NodeExecutor for ArraySortNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let source = resolve_array_source(
            config.get("source").unwrap_or(&serde_json::Value::Null),
            ctx,
        )?;
        let field = config.get("field").and_then(|v| v.as_str());
        let order = config
            .get("order")
            .and_then(|v| v.as_str())
            .unwrap_or("asc");

        let mut result: Vec<serde_json::Value> = source.to_vec();
        let ascending = order == "asc";
        result.sort_by(|a, b| {
            let va = extract_field(a, field);
            let vb = extract_field(b, field);
            let ord = compare_json(va, vb);
            if ascending {
                ord
            } else {
                ord.reverse()
            }
        });

        info!(
            "数组排序: {} 条, field={:?}, order={}",
            result.len(),
            field,
            order
        );
        Ok(
            serde_json::json!({ "source_count": source.len(), "field": field, "order": order, "result": result }),
        )
    }
}

// ═══════════════════════════════════════
// array_dedup — 去重
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ArrayDedupNode;

#[async_trait]
impl NodeExecutor for ArrayDedupNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let source = resolve_array_source(
            config.get("source").unwrap_or(&serde_json::Value::Null),
            ctx,
        )?;
        let field = config.get("field").and_then(|v| v.as_str());

        let mut seen = std::collections::HashSet::new();
        let mut result: Vec<serde_json::Value> = Vec::new();
        for item in &source {
            let key = match field {
                Some(f) => format_key(extract_field(item, Some(f))),
                None => format_key(item),
            };
            if seen.insert(key) {
                result.push(item.clone());
            }
        }

        info!(
            "数组去重: {} → {} 条, field={:?}",
            source.len(),
            result.len(),
            field
        );
        Ok(
            serde_json::json!({ "source_count": source.len(), "result_count": result.len(), "field": field, "result": result }),
        )
    }
}

// ═══════════════════════════════════════
// array_paginate — 分页
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ArrayPaginateNode;

#[async_trait]
impl NodeExecutor for ArrayPaginateNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let source = resolve_array_source(
            config.get("source").unwrap_or(&serde_json::Value::Null),
            ctx,
        )?;
        let page = config
            .get("page")
            .and_then(|v| v.as_u64())
            .unwrap_or(1)
            .max(1) as usize;
        let page_size = config
            .get("page_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(10)
            .max(1) as usize;
        let total = source.len();
        let total_pages = total.div_ceil(page_size);
        let start = ((page - 1) * page_size).min(total);
        let end = (start + page_size).min(total);
        let page_items: Vec<serde_json::Value> = source[start..end].to_vec();

        info!(
            "数组分页: page={}/{}, {} 条",
            page,
            total_pages,
            page_items.len()
        );
        Ok(
            serde_json::json!({ "page": page, "page_size": page_size, "total": total, "total_pages": total_pages, "count": page_items.len(), "result": page_items }),
        )
    }
}

// ═══════════════════════════════════════
// array_map — 映射
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ArrayMapNode;

#[async_trait]
impl NodeExecutor for ArrayMapNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let source = resolve_array_source(
            config.get("source").unwrap_or(&serde_json::Value::Null),
            ctx,
        )?;
        let template = config
            .get("template")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("map 缺少 template 参数"))?;
        let fields: Option<Vec<&str>> = config
            .get("fields")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|f| f.as_str()).collect());

        let result: Vec<serde_json::Value> = source
            .iter()
            .enumerate()
            .map(|(idx, item)| apply_map_template(template, item, idx, fields.as_deref()))
            .collect();

        info!("数组映射: {} 条", result.len());
        Ok(
            serde_json::json!({ "source_count": source.len(), "template": template, "result": result }),
        )
    }
}

fn apply_map_template(
    template: &str,
    item: &serde_json::Value,
    index: usize,
    fields: Option<&[&str]>,
) -> serde_json::Value {
    if template.trim() == "{{__item}}" {
        return item.clone();
    }
    if template.trim() == "{{__index}}" {
        return serde_json::json!(index);
    }

    let mut result = template.to_string();
    result = result.replace("{{__index}}", &index.to_string());

    use std::sync::LazyLock;
    static RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"\{\{__item\.(\w+(?:\.\w+)*)\}\}").expect("regex compile")
    });
    result = RE
        .replace_all(&result, |caps: &regex::Captures| {
            let path = caps.get(1).expect("capture").as_str();
            let val = extract_field(item, Some(path));
            match val {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            }
        })
        .to_string();
    result = result.replace("{{__item}}", &item.to_string());

    if let Some(field_list) = fields {
        if !field_list.is_empty() {
            let mut obj = serde_json::Map::new();
            for f in field_list {
                obj.insert(f.to_string(), extract_field(item, Some(f)).clone());
            }
            return serde_json::Value::Object(obj);
        }
    }
    serde_json::Value::String(result)
}

// ═══════════════════════════════════════
// array_join — 连接
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ArrayJoinNode;

#[async_trait]
impl NodeExecutor for ArrayJoinNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let source = resolve_array_source(
            config.get("source").unwrap_or(&serde_json::Value::Null),
            ctx,
        )?;
        let separator = config
            .get("separator")
            .and_then(|v| v.as_str())
            .unwrap_or(",");
        let field = config.get("field").and_then(|v| v.as_str());

        let parts: Vec<String> = source
            .iter()
            .map(|item| {
                let val = match field {
                    Some(f) => extract_field(item, Some(f)),
                    None => item,
                };
                match val {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Null => String::new(),
                    other => other.to_string(),
                }
            })
            .collect();

        let result = parts.join(separator);
        Ok(
            serde_json::json!({ "source_count": source.len(), "separator": separator, "field": field, "length": result.len(), "result": result }),
        )
    }
}

// ═══════════════════════════════════════
// array_reduce — 聚合
// ═══════════════════════════════════════

#[derive(Default)]
pub struct ArrayReduceNode;

#[async_trait]
impl NodeExecutor for ArrayReduceNode {
    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let source = resolve_array_source(
            config.get("source").unwrap_or(&serde_json::Value::Null),
            ctx,
        )?;
        let aggregator = config
            .get("aggregator")
            .and_then(|v| v.as_str())
            .unwrap_or("count");
        let field = config.get("field").and_then(|v| v.as_str());

        let result = match aggregator {
            "count" => serde_json::json!(source.len()),
            "sum" => {
                let sum: f64 = source
                    .iter()
                    .filter_map(|item| {
                        let val = extract_field(item, field);
                        val.as_f64()
                            .or_else(|| val.as_str().and_then(|s| s.parse::<f64>().ok()))
                    })
                    .sum();
                serde_json::json!(sum)
            }
            "avg" => {
                let vals: Vec<f64> = source
                    .iter()
                    .filter_map(|item| {
                        let val = extract_field(item, field);
                        val.as_f64()
                            .or_else(|| val.as_str().and_then(|s| s.parse::<f64>().ok()))
                    })
                    .collect();
                if vals.is_empty() {
                    serde_json::json!(0.0)
                } else {
                    serde_json::json!(vals.iter().sum::<f64>() / vals.len() as f64)
                }
            }
            "min" => {
                let min = source
                    .iter()
                    .filter_map(|item| {
                        let val = extract_field(item, field);
                        val.as_f64()
                            .or_else(|| val.as_str().and_then(|s| s.parse::<f64>().ok()))
                    })
                    .fold(f64::NAN, f64::min);
                serde_json::json!(if min.is_nan() { 0.0 } else { min })
            }
            "max" => {
                let max = source
                    .iter()
                    .filter_map(|item| {
                        let val = extract_field(item, field);
                        val.as_f64()
                            .or_else(|| val.as_str().and_then(|s| s.parse::<f64>().ok()))
                    })
                    .fold(f64::NAN, f64::max);
                serde_json::json!(if max.is_nan() { 0.0 } else { max })
            }
            "first" => source.first().cloned().unwrap_or(serde_json::Value::Null),
            "last" => source.last().cloned().unwrap_or(serde_json::Value::Null),
            _ => {
                return Err(anyhow!(
                    "未知的聚合操作: {}（支持 count/sum/avg/min/max/first/last）",
                    aggregator
                ))
            }
        };

        info!(
            "数组聚合: {} → {:?}, {} 条",
            aggregator,
            field,
            source.len()
        );
        Ok(
            serde_json::json!({ "aggregator": aggregator, "field": field, "source_count": source.len(), "result": result }),
        )
    }
}
