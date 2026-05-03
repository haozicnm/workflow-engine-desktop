// nodes/excel.rs — Excel 读写节点（拆分为细粒度操作 + 内存筛选排序 + CSV 互转）
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use calamine::Reader;

// ═══════════════════════════════════════════
// Excel 通用节点（兼容旧版，保留所有 action）
// ═══════════════════════════════════════════
#[derive(Default)]
pub struct ExcelNode;

#[async_trait]
impl NodeExecutor for ExcelNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Excel 节点缺少 action 参数"))?;
        let file_path = config.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Excel 节点缺少 path 参数"))?;

        match action {
            "read" => excel_read(file_path, config).await,
            "write" => excel_write(file_path, config).await,
            "append" => excel_append(file_path, config).await,
            "update" => excel_update(file_path, config).await,
            "sheets" => excel_sheets(file_path).await,
            "extract_column" => excel_extract_column(file_path, config).await,
            _ => Err(anyhow!("未知的 Excel 操作: {}", action)),
        }
    }
}

// ═══════════════════════════════════════════
// 拆分节点（每个对应一个操作）
// ═══════════════════════════════════════════

// ── excel_read ──
#[derive(Default)]
pub struct ExcelReadNode;

#[async_trait]
impl NodeExecutor for ExcelReadNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = config.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("读取表格需要 path 参数"))?;
        let sheet = config.get("sheet").and_then(|v| v.as_str());
        let cfg = if let Some(s) = sheet {
            serde_json::json!({ "sheet": s })
        } else {
            serde_json::json!({})
        };
        excel_read(path, &cfg).await
    }
}

// ── excel_write ──
#[derive(Default)]
pub struct ExcelWriteNode;

#[async_trait]
impl NodeExecutor for ExcelWriteNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = config.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("写入表格需要 path 参数"))?;
        let sheet = config.get("sheet").and_then(|v| v.as_str()).unwrap_or("Sheet1");
        let write_mode = config.get("write_mode").and_then(|v| v.as_str()).unwrap_or("overwrite");
        let data = config.get("data");
        match write_mode {
            "append" => {
                let cfg = serde_json::json!({ "sheet": sheet, "data": data });
                excel_append(path, &cfg).await
            }
            _ => {
                let cfg = serde_json::json!({ "sheet": sheet, "data": data });
                excel_write(path, &cfg).await
            }
        }
    }
}

// ── excel_create ──
#[derive(Default)]
pub struct ExcelCreateNode;

#[async_trait]
impl NodeExecutor for ExcelCreateNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = config.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("创建表格需要 path 参数"))?;
        let sheet = config.get("sheet").and_then(|v| v.as_str()).unwrap_or("Sheet1");
        let headers = config.get("headers").and_then(|v| v.as_str());
        let data = config.get("data");

        let cfg = serde_json::json!({ "sheet": sheet, "data": data, "headers": headers });
        excel_create(path, &cfg).await
    }
}

// ── excel_filter ──
#[derive(Default)]
pub struct ExcelFilterNode;

#[async_trait]
impl NodeExecutor for ExcelFilterNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let column = config.get("column").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("筛选数据需要 column 参数"))?;
        let op = config.get("op").and_then(|v| v.as_str()).unwrap_or("==");
        let value = config.get("value").and_then(|v| v.as_str()).unwrap_or("");
        let data = config.get("data")
            .ok_or_else(|| anyhow!("筛选数据需要 data 参数"))?;

        excel_filter_in_memory(data, column, op, value)
    }
}

// ── excel_sort ──
#[derive(Default)]
pub struct ExcelSortNode;

#[async_trait]
impl NodeExecutor for ExcelSortNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let column = config.get("column").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("排序数据需要 column 参数"))?;
        let order = config.get("order").and_then(|v| v.as_str()).unwrap_or("asc");
        let data = config.get("data")
            .ok_or_else(|| anyhow!("排序数据需要 data 参数"))?;

        excel_sort_in_memory(data, column, order)
    }
}

// ── excel_append ──
#[derive(Default)]
pub struct ExcelAppendNode;

#[async_trait]
impl NodeExecutor for ExcelAppendNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = config.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("追加行需要 path 参数"))?;
        let sheet = config.get("sheet").and_then(|v| v.as_str()).unwrap_or("Sheet1");
        let data = config.get("data");

        let cfg = serde_json::json!({ "sheet": sheet, "data": data });
        excel_append(path, &cfg).await
    }
}

// ── excel_csv_convert ── CSV ↔ Excel 互转
#[derive(Default)]
pub struct ExcelCsvNode;

#[async_trait]
impl NodeExecutor for ExcelCsvNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = config.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("CSV 转换需要 path 参数"))?;
        let direction = config.get("direction").and_then(|v| v.as_str()).unwrap_or("csv_to_xlsx");
        let output = config.get("output").and_then(|v| v.as_str());
        let delimiter = config.get("delimiter").and_then(|v| v.as_str()).unwrap_or(",");

        excel_csv_convert(path, output, direction, delimiter).await
    }
}

// ═══════════════════════════════════════════
// 操作函数
// ═══════════════════════════════════════════

/// 读取 Excel 工作表数据
pub async fn excel_read(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let sheet_name = config.get("sheet").and_then(|v| v.as_str()).map(String::from);

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let mut workbook = calamine::open_workbook::<calamine::Xlsx<_>, _>(&path)
            .map_err(|e| anyhow!("打开 Excel 文件失败: {}", e))?;

        let sheet = if let Some(s) = &sheet_name {
            s.clone()
        } else {
            workbook.sheet_names().first()
                .ok_or_else(|| anyhow!("Excel 文件没有工作表"))?
                .clone()
        };

        let range = workbook.worksheet_range(&sheet)
            .map_err(|e| anyhow!("读取工作表 '{}' 失败: {}", sheet, e))?;

        let mut rows = Vec::new();
        for row in range.rows() {
            let cells: Vec<serde_json::Value> = row.iter().map(cell_to_json).collect();
            rows.push(serde_json::Value::Array(cells));
        }

        Ok(serde_json::json!({
            "sheet": sheet,
            "rows": range.height(),
            "cols": range.width(),
            "data": rows,
        }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 读取工作表名称列表
async fn excel_sheets(path: &str) -> Result<serde_json::Value> {
    let path = path.to_string();
    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let workbook: calamine::Xlsx<_> = calamine::open_workbook(&path)
            .map_err(|e| anyhow!("打开 Excel 文件失败: {}", e))?;
        Ok(serde_json::json!({ "sheets": workbook.sheet_names() }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 提取指定列为扁平数组
async fn excel_extract_column(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let sheet_name = config.get("sheet").and_then(|v| v.as_str()).unwrap_or("Sheet1").to_string();
    let column = config.get("column").and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("extract_column 需要 column 参数"))?.to_string();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(&path)
            .map_err(|e| anyhow!("打开 Excel 文件失败: {}", e))?;
        let range = workbook.worksheet_range(&sheet_name)
            .map_err(|e| anyhow!("读取工作表 '{}' 失败: {}", sheet_name, e))?;
        let col_idx = parse_column_index(&column)?;
        let values: Vec<serde_json::Value> = range.rows()
            .filter_map(|row| row.get(col_idx)).map(cell_to_json).collect();
        Ok(serde_json::json!(values))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 写入 Excel（创建新文件或覆盖）
async fn excel_write(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let sheet_name = config.get("sheet").and_then(|v| v.as_str()).unwrap_or("Sheet1").to_string();
    let data = config.get("data").and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("write 需要 data 参数（二维数组）"))?.clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet_name)?;
        let mut total_rows = 0u32;
        for (r, row) in data.iter().enumerate() {
            if let Some(cells) = row.as_array() {
                for (c, cell) in cells.iter().enumerate() {
                    write_json_cell(worksheet, r as u32, c as u16, cell)?;
                }
            }
            total_rows = r as u32 + 1;
        }
        workbook.save(&path)?;
        Ok(serde_json::json!({ "path": path, "sheet": sheet_name, "rows_written": total_rows }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 创建新 Excel（写空文件或带表头）
async fn excel_create(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let sheet_name = config.get("sheet").and_then(|v| v.as_str()).unwrap_or("Sheet1").to_string();
    let headers = config.get("headers").and_then(|v| v.as_str()).map(String::from);
    let data = config.get("data").and_then(|v| v.as_array()).cloned();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet_name)?;

        let mut row = 0u32;
        // 写表头
        if let Some(ref h) = headers {
            let cols: Vec<&str> = h.split(',').map(|s| s.trim()).collect();
            for (c, col_name) in cols.iter().enumerate() {
                worksheet.write_string(row, c as u16, *col_name)?;
            }
            row += 1;
        }
        // 写数据
        if let Some(rows) = data {
            for (r, data_row) in rows.iter().enumerate() {
                if let Some(cells) = data_row.as_array() {
                    for (c, cell) in cells.iter().enumerate() {
                        write_json_cell(worksheet, row + r as u32, c as u16, cell)?;
                    }
                }
            }
        }

        workbook.save(&path)?;
        Ok(serde_json::json!({ "path": path, "sheet": sheet_name, "created": true }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 追加行到 Excel
async fn excel_append(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let sheet_name = config.get("sheet").and_then(|v| v.as_str()).unwrap_or("Sheet1").to_string();
    let data = config.get("data").and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("append 需要 data 参数"))?.clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let mut existing: calamine::Xlsx<_> = calamine::open_workbook(&path)?;
        let next_row = if let Ok(range) = existing.worksheet_range(&sheet_name) {
            range.height() as u32
        } else { 0 };

        let old_rows: Vec<Vec<serde_json::Value>> = if let Ok(range) = existing.worksheet_range(&sheet_name) {
            range.rows().map(|row| row.iter().map(cell_to_json).collect()).collect()
        } else { Vec::new() };

        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet_name)?;

        for (r, row) in old_rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                write_json_cell(worksheet, r as u32, c as u16, cell)?;
            }
        }
        for (r, row) in data.iter().enumerate() {
            if let Some(cells) = row.as_array() {
                for (c, cell) in cells.iter().enumerate() {
                    write_json_cell(worksheet, next_row + r as u32, c as u16, cell)?;
                }
            }
        }

        workbook.save(&path)?;
        Ok(serde_json::json!({ "path": path, "sheet": sheet_name, "appended_rows": data.len(), "start_row": next_row }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 更新 Excel 单元格
async fn excel_update(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let sheet_name = config.get("sheet").and_then(|v| v.as_str()).unwrap_or("Sheet1").to_string();
    let updates = config.get("updates").and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("update 需要 updates 参数"))?.clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let mut reader: calamine::Xlsx<_> = calamine::open_workbook(&path)?;
        let range = reader.worksheet_range(&sheet_name)
            .map_err(|e| anyhow!("读取工作表失败: {}", e))?;
        let mut data: Vec<Vec<String>> = range.rows().map(|row| row.iter().map(|c| match c {
            calamine::Data::String(s) => s.clone(),
            calamine::Data::Int(i) => i.to_string(),
            calamine::Data::Float(f) => f.to_string(),
            calamine::Data::Bool(b) => b.to_string(),
            _ => String::new(),
        }).collect()).collect();

        let mut updated_count = 0;
        for update in &updates {
            let cell_ref = update.get("cell").and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("update 项缺少 cell 参数"))?;
            let value = update.get("value").map(|v| v.to_string()).unwrap_or_default();
            let (row, col) = parse_cell_ref(cell_ref)?;
            while data.len() <= row { data.push(Vec::new()); }
            while data[row].len() <= col { data[row].push(String::new()); }
            data[row][col] = value;
            updated_count += 1;
        }

        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet_name)?;
        for (r, row) in data.iter().enumerate() {
            for (c, val) in row.iter().enumerate() {
                if !val.is_empty() {
                    if let Ok(n) = val.parse::<f64>() {
                        worksheet.write_number(r as u32, c as u16, n)?;
                    } else {
                        worksheet.write_string(r as u32, c as u16, val)?;
                    }
                }
            }
        }
        workbook.save(&path)?;
        Ok(serde_json::json!({ "path": path, "sheet": sheet_name, "updated_cells": updated_count }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

// ─── 内存操作（不读文件，直接对传入的 JSON 数据操作） ───

/// 内存筛选：对 JSON 二维数组按列条件过滤
fn excel_filter_in_memory(data: &serde_json::Value, column: &str, op: &str, value: &str) -> Result<serde_json::Value> {
    let rows = data.as_array().ok_or_else(|| anyhow!("筛选需要 data 参数（二维数组）"))?;
    if rows.is_empty() {
        return Ok(serde_json::json!({ "data": [], "filtered_count": 0, "total": 0 }));
    }

    let col_idx = parse_column_index(column)
        .or_else(|_| {
            // 如果 column 不是字母，尝试按 header 名称查找
            let first_row = rows[0].as_array();
            match first_row {
                Some(cells) => cells.iter().position(|c| c.as_str() == Some(column))
                    .ok_or_else(|| anyhow!("未找到列 '{}'", column)),
                None => Err(anyhow!("无法解析列 '{}'", column)),
            }
        })?;

    let filtered: Vec<serde_json::Value> = rows.iter().filter(|row| {
        let cells = row.as_array();
        match cells.and_then(|c| c.get(col_idx)) {
            None => false,
            Some(cell) => {
                let cell_str = match cell {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => String::new(),
                };
                match op {
                    "==" => cell_str == value,
                    "!=" => cell_str != value,
                    ">" => cell_str.parse::<f64>().ok().zip(value.parse::<f64>().ok()).map_or(false, |(a, b)| a > b),
                    "<" => cell_str.parse::<f64>().ok().zip(value.parse::<f64>().ok()).map_or(false, |(a, b)| a < b),
                    ">=" => cell_str.parse::<f64>().ok().zip(value.parse::<f64>().ok()).map_or(false, |(a, b)| a >= b),
                    "<=" => cell_str.parse::<f64>().ok().zip(value.parse::<f64>().ok()).map_or(false, |(a, b)| a <= b),
                    "contains" => cell_str.contains(value),
                    "starts_with" => cell_str.starts_with(value),
                    _ => false,
                }
            }
        }
    }).cloned().collect();

    let total = rows.len();
    let count = filtered.len();
    Ok(serde_json::json!({ "data": filtered, "filtered_count": count, "total": total }))
}

/// 内存排序：对 JSON 二维数组按列排序
fn excel_sort_in_memory(data: &serde_json::Value, column: &str, order: &str) -> Result<serde_json::Value> {
    let rows = data.as_array().ok_or_else(|| anyhow!("排序需要 data 参数（二维数组）"))?.clone();
    if rows.len() <= 1 {
        return Ok(serde_json::json!({ "data": rows, "sorted_count": rows.len() }));
    }

    let col_idx = parse_column_index(column)
        .or_else(|_| {
            let first_row = rows[0].as_array();
            match first_row {
                Some(cells) => cells.iter().position(|c| c.as_str() == Some(column))
                    .ok_or_else(|| anyhow!("未找到列 '{}'", column)),
                None => Err(anyhow!("无法解析列 '{}'", column)),
            }
        })?;

    let mut sorted = rows.clone();
    let desc = order == "desc";
    sorted.sort_by(|a, b| {
        let ca = a.as_array().and_then(|c| c.get(col_idx));
        let cb = b.as_array().and_then(|c| c.get(col_idx));
        let cmp = compare_cells(ca, cb);
        if desc { cmp.reverse() } else { cmp }
    });

    Ok(serde_json::json!({ "data": sorted, "sorted_count": sorted.len() }))
}

fn compare_cells(a: Option<&serde_json::Value>, b: Option<&serde_json::Value>) -> std::cmp::Ordering {
    match (a, b) {
        (None, None) => std::cmp::Ordering::Equal,
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (Some(a), Some(b)) => {
            let sa = match a { serde_json::Value::String(s) => s.clone(), other => other.to_string() };
            let sb = match b { serde_json::Value::String(s) => s.clone(), other => other.to_string() };
            if let (Ok(na), Ok(nb)) = (sa.parse::<f64>(), sb.parse::<f64>()) {
                na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
            } else {
                sa.cmp(&sb)
            }
        }
    }
}

/// CSV ↔ Excel 互转
async fn excel_csv_convert(path: &str, output: Option<&str>, direction: &str, delimiter: &str) -> Result<serde_json::Value> {
    let path = path.to_string();
    let output = output.map(String::from);
    let direction = direction.to_string();
    let delim = delimiter.to_string();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        match direction.as_str() {
            "csv_to_xlsx" => {
                let output_path = output.unwrap_or_else(|| path.replace(".csv", ".xlsx"));
                let mut reader = csv::ReaderBuilder::new()
                    .delimiter(delim.bytes().next().unwrap_or(b','))
                    .from_path(&path)
                    .map_err(|e| anyhow!("打开 CSV 失败: {}", e))?;

                let mut workbook = rust_xlsxwriter::Workbook::new();
                let worksheet = workbook.add_worksheet();
                worksheet.set_name("Sheet1")?;

                let mut row = 0u32;
                for result in reader.records() {
                    let record = result.map_err(|e| anyhow!("CSV 行解析失败: {}", e))?;
                    for (c, field) in record.iter().enumerate() {
                        if let Ok(n) = field.parse::<f64>() {
                            worksheet.write_number(row, c as u16, n)?;
                        } else {
                            worksheet.write_string(row, c as u16, field)?;
                        }
                    }
                    row += 1;
                }
                workbook.save(&output_path)?;
                Ok(serde_json::json!({ "path": output_path, "rows": row, "direction": "csv_to_xlsx" }))
            }
            "xlsx_to_csv" => {
                let output_path = output.unwrap_or_else(|| path.replace(".xlsx", ".csv"));
                let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(&path)
                    .map_err(|e| anyhow!("打开 Excel 失败: {}", e))?;
                let sheet = workbook.sheet_names().first()
                    .ok_or_else(|| anyhow!("Excel 无工作表"))?.clone();
                let range = workbook.worksheet_range(&sheet)
                    .map_err(|e| anyhow!("读取工作表失败: {}", e))?;

                let mut writer = csv::Writer::from_path(&output_path)
                    .map_err(|e| anyhow!("创建 CSV 失败: {}", e))?;
                let mut count = 0u32;
                for row in range.rows() {
                    let cells: Vec<String> = row.iter().map(|c| match c {
                        calamine::Data::String(s) => s.clone(),
                        calamine::Data::Int(i) => i.to_string(),
                        calamine::Data::Float(f) => f.to_string(),
                        calamine::Data::Bool(b) => b.to_string(),
                        _ => String::new(),
                    }).collect();
                    writer.write_record(&cells).map_err(|e| anyhow!("CSV 写入失败: {}", e))?;
                    count += 1;
                }
                writer.flush().map_err(|e| anyhow!("CSV flush 失败: {}", e))?;
                Ok(serde_json::json!({ "path": output_path, "rows": count, "direction": "xlsx_to_csv" }))
            }
            _ => Err(anyhow!("未知的转换方向: {}（支持 csv_to_xlsx / xlsx_to_csv）", direction)),
        }
    }).await.map_err(|e| anyhow!("CSV 转换失败: {}", e))?
}

// ─── 辅助函数 ───

fn parse_column_index(column: &str) -> Result<usize> {
    if let Ok(idx) = column.parse::<usize>() { return Ok(idx); }
    let mut idx: usize = 0;
    for ch in column.chars() {
        if !ch.is_ascii_uppercase() {
            return Err(anyhow!("无效的列标识符: '{}'", column));
        }
        idx = idx * 26 + (ch as usize - 'A' as usize + 1);
    }
    if idx == 0 { return Err(anyhow!("无效的列标识符: '{}'", column)); }
    Ok(idx - 1)
}

fn parse_cell_ref(cell_ref: &str) -> Result<(usize, usize)> {
    let cell_ref = cell_ref.trim();
    let mut col_str = String::new();
    let mut row_str = String::new();
    let mut in_col = true;
    for ch in cell_ref.chars() {
        if ch.is_ascii_alphabetic() && in_col { col_str.push(ch.to_ascii_uppercase()); }
        else if ch.is_ascii_digit() { in_col = false; row_str.push(ch); }
        else { return Err(anyhow!("无效的单元格引用: {}", cell_ref)); }
    }
    if col_str.is_empty() || row_str.is_empty() { return Err(anyhow!("无效的单元格引用: {}", cell_ref)); }
    let mut col: usize = 0;
    for ch in col_str.chars() { col = col * 26 + (ch as usize - 'A' as usize + 1); }
    col -= 1;
    let row: usize = row_str.parse::<usize>()
        .map_err(|_| anyhow!("无效的行号: {}", row_str))?;
    Ok((row - 1, col))
}

fn cell_to_json(cell: &calamine::Data) -> serde_json::Value {
    match cell {
        calamine::Data::Int(i) => serde_json::json!(i),
        calamine::Data::Float(f) => serde_json::json!(f),
        calamine::Data::String(s) => serde_json::Value::String(s.clone()),
        calamine::Data::Bool(b) => serde_json::Value::Bool(*b),
        calamine::Data::DateTime(v) => serde_json::json!(v.as_f64()),
        calamine::Data::DateTimeIso(s) => serde_json::Value::String(s.clone()),
        calamine::Data::DurationIso(s) => serde_json::Value::String(s.clone()),
        calamine::Data::Error(e) => serde_json::Value::String(format!("#ERROR: {:?}", e)),
        calamine::Data::Empty => serde_json::Value::Null,
    }
}

fn write_json_cell(ws: &mut rust_xlsxwriter::Worksheet, row: u32, col: u16, val: &serde_json::Value) -> Result<()> {
    match val {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() { ws.write_number(row, col, i as f64)?; }
            else if let Some(f) = n.as_f64() { ws.write_number(row, col, f)?; }
        }
        serde_json::Value::String(s) => { ws.write_string(row, col, s)?; }
        serde_json::Value::Bool(b) => { ws.write_boolean(row, col, *b)?; }
        _ => {}
    }
    Ok(())
}
