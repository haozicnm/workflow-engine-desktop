// engine/collect.rs — 循环节点的 collect/table 后处理（公共模块）
// 供 loop_node / while_node 共用，避免代码重复
use serde_json::Value;

/// 从循环的单轮迭代中构建临时上下文（用于 collect/table 路径解析）
/// 包含 __item、__index、__index1 以及该轮 body 步骤的所有输出
pub fn build_iter_context(item: &Value, index: usize, iter_outputs: &Value) -> Value {
    let mut ctx = serde_json::Map::new();
    ctx.insert("__item".to_string(), item.clone());
    ctx.insert("__index".to_string(), Value::Number(index.into()));
    ctx.insert("__index1".to_string(), Value::Number((index + 1).into()));
    // 把该轮 body 步骤输出平铺到上下文中
    if let Some(outputs) = iter_outputs.as_object() {
        for (k, v) in outputs {
            ctx.insert(k.clone(), v.clone());
        }
    }
    Value::Object(ctx)
}

/// 在给定上下文中按 . 分隔路径取值，支持 name[index] 数组索引
/// 例: "step_api.body.items[2].name" → items[2].name
/// 例: "__item.title" → 当前迭代项的 title 字段
pub fn resolve_path(path: &str, context: &Value) -> Value {
    let mut current = context;
    for part in path.split('.') {
        if part.is_empty() {
            continue;
        }
        // 处理 name[index] 格式
        let (name, idx) = if let Some(pos) = part.find('[') {
            let name = &part[..pos];
            let idx_str = &part[pos + 1..part.len().min(pos + 10)].trim_end_matches(']');
            (name, idx_str.parse::<usize>().ok())
        } else {
            (part, None)
        };
        if !name.is_empty() {
            match current.get(name) {
                Some(v) => current = v,
                None => return Value::Null,
            }
        }
        if let Some(idx) = idx {
            current = current.get(idx).unwrap_or(&Value::Null);
        }
    }
    current.clone()
}

/// 对循环输出应用 collect 后处理
///
/// # 参数
/// - `output`: 可变引用，结果会被写入 `output["collected"]`
/// - `collect_cfg`: 格式 `{ key: "path", ... }`，从每轮迭代按路径取值汇集成数组
/// - `items`: 原始迭代数组
/// - `results`: 每轮 body 执行输出（与 items 等长）
///
/// # 结果
/// ```json
/// { "collected": { "names": [...], "values": [...] } }
/// ```
pub fn apply_collect(
    output: &mut Value,
    collect_cfg: &Value,
    items: &[Value],
    results: &[Value],
) {
    let Some(map) = collect_cfg.as_object() else { return };
    let mut collected = serde_json::Map::new();

    for (key, path_val) in map {
        if let Some(path) = path_val.as_str() {
            let values: Vec<Value> = items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let ictx = build_iter_context(item, i, &results[i]);
                    resolve_path(path, &ictx)
                })
                .collect();
            collected.insert(key.clone(), Value::Array(values));
        }
    }

    if let Some(obj) = output.as_object_mut() {
        obj.insert("collected".to_string(), Value::Object(collected));
    }
}

/// 对循环输出应用 table 后处理
///
/// # 参数
/// - `output`: 可变引用，结果会被写入 `output["table"]`
/// - `table_cfg`: 格式 `[{ header: "列名", field: "path" }, ...]`
/// - `items`: 原始迭代数组
/// - `results`: 每轮 body 执行输出
///
/// # 结果
/// ```json
/// { "table": { "headers": [...], "rows": [[...], ...], "data": [[headers], [...], ...] } }
/// ```
/// `data = [headers, row0, row1, ...]` — 可直接喂给 Excel write 节点
pub fn apply_table(
    output: &mut Value,
    table_cfg: &Value,
    items: &[Value],
    results: &[Value],
) {
    let Some(cols) = table_cfg.as_array() else { return };

    let mut headers = Vec::new();
    let mut field_paths = Vec::new();

    for col in cols {
        if let Some(cm) = col.as_object() {
            if let Some(h) = cm.get("header").and_then(|v| v.as_str()) {
                headers.push(Value::String(h.to_string()));
            }
            if let Some(f) = cm.get("field").and_then(|v| v.as_str()) {
                field_paths.push(f.to_string());
            }
        }
    }

    let mut rows = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let ictx = build_iter_context(item, i, &results[i]);
        let row: Vec<Value> = field_paths
            .iter()
            .map(|p| resolve_path(p, &ictx))
            .collect();
        rows.push(Value::Array(row));
    }

    // data = [headers, row0, row1, ...]
    let mut data = vec![Value::Array(headers.clone())];
    data.extend(rows.clone());

    if let Some(obj) = output.as_object_mut() {
        obj.insert(
            "table".to_string(),
            serde_json::json!({
                "headers": headers,
                "rows": rows,
                "data": data,
            }),
        );
    }
}
