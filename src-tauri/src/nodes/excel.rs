// nodes/excel.rs — Excel 读写节点
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};
use calamine::Reader;

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

        Ok(serde_json::json!({
            "sheets": workbook.sheet_names(),
        }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 提取指定列为扁平数组（替代 script 节点提取列数据）
async fn excel_extract_column(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let sheet_name = config.get("sheet").and_then(|v| v.as_str()).unwrap_or("Sheet1").to_string();
    let column = config.get("column")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("extract_column 需要 column 参数（如 'A'、'B' 或 0、1）"))?
        .to_string();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(&path)
            .map_err(|e| anyhow!("打开 Excel 文件失败: {}", e))?;

        let range = workbook.worksheet_range(&sheet_name)
            .map_err(|e| anyhow!("读取工作表 '{}' 失败: {}", sheet_name, e))?;

        let col_idx = parse_column_index(&column)?;

        let values: Vec<serde_json::Value> = range.rows()
            .filter_map(|row| row.get(col_idx))
            .map(cell_to_json)
            .collect();

        Ok(serde_json::json!(values))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 解析列标识符：'A'→0, 'B'→1, ..., 'Z'→25, 'AA'→26；也支持数字 "0"→0
fn parse_column_index(column: &str) -> Result<usize> {
    // 先尝试数字
    if let Ok(idx) = column.parse::<usize>() {
        return Ok(idx);
    }
    // 字母格式
    let mut idx: usize = 0;
    for ch in column.chars() {
        if !ch.is_ascii_uppercase() {
            return Err(anyhow!("无效的列标识符: '{}'（支持 A-Z 或数字）", column));
        }
        idx = idx * 26 + (ch as usize - 'A' as usize + 1);
    }
    if idx == 0 {
        return Err(anyhow!("无效的列标识符: '{}'", column));
    }
    Ok(idx - 1) // 转为 0-indexed
}

/// 写入 Excel（创建新文件或覆盖）
async fn excel_write(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let sheet_name = config.get("sheet")
        .and_then(|v| v.as_str())
        .unwrap_or("Sheet1")
        .to_string();
    let data = config.get("data").and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("write 需要 data 参数（二维数组）"))?
        .clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet_name)?;

        let mut total_rows = 0u32;
        for (r, row) in data.iter().enumerate() {
            if let Some(cells) = row.as_array() {
                for (c, cell) in cells.iter().enumerate() {
                    match cell {
                        serde_json::Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                worksheet.write_number(r as u32, c as u16, i as f64)?;
                            } else if let Some(f) = n.as_f64() {
                                worksheet.write_number(r as u32, c as u16, f)?;
                            }
                        }
                        serde_json::Value::String(s) => {
                            worksheet.write_string(r as u32, c as u16, s)?;
                        }
                        serde_json::Value::Bool(b) => {
                            worksheet.write_boolean(r as u32, c as u16, *b)?;
                        }
                        _ => {}
                    }
                }
            }
            total_rows = r as u32 + 1;
        }

        workbook.save(&path)?;
        Ok(serde_json::json!({
            "path": path,
            "sheet": sheet_name,
            "rows_written": total_rows,
        }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 追加行到 Excel
async fn excel_append(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let sheet_name = config.get("sheet")
        .and_then(|v| v.as_str())
        .unwrap_or("Sheet1")
        .to_string();
    let data = config.get("data").and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("append 需要 data 参数（二维数组）"))?
        .clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        // 读取现有数据
        let mut existing: calamine::Xlsx<_> = calamine::open_workbook(&path)?;
        let next_row = if let Ok(range) = existing.worksheet_range(&sheet_name) {
            range.height() as u32
        } else {
            0
        };

        // 重新写入（读旧数据 + 追加新数据）
        let old_rows: Vec<Vec<serde_json::Value>> = if let Ok(range) = existing.worksheet_range(&sheet_name) {
            range.rows().map(|row| row.iter().map(cell_to_json).collect()).collect()
        } else {
            Vec::new()
        };

        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet_name)?;

        // 写旧数据
        for (r, row) in old_rows.iter().enumerate() {
            for (c, cell) in row.iter().enumerate() {
                write_json_cell(worksheet, r as u32, c as u16, cell)?;
            }
        }
        // 写新数据
        for (r, row) in data.iter().enumerate() {
            if let Some(cells) = row.as_array() {
                for (c, cell) in cells.iter().enumerate() {
                    write_json_cell(worksheet, next_row + r as u32, c as u16, cell)?;
                }
            }
        }

        workbook.save(&path)?;
        Ok(serde_json::json!({
            "path": path,
            "sheet": sheet_name,
            "appended_rows": data.len(),
            "start_row": next_row,
        }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 更新 Excel 中的指定单元格（读取现有文件 → 修改 → 写回）
async fn excel_update(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let sheet_name = config.get("sheet")
        .and_then(|v| v.as_str())
        .unwrap_or("Sheet1")
        .to_string();
    let updates = config.get("updates").and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("update 需要 updates 参数（[{{cell: 'B2', value: '...'}}]）"))?
        .clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        // 1. 读取现有数据
        let mut reader: calamine::Xlsx<_> = calamine::open_workbook(&path)
            .map_err(|e| anyhow!("打开 Excel 文件失败: {}", e))?;

        let range = reader.worksheet_range(&sheet_name)
            .map_err(|e| anyhow!("读取工作表 '{}' 失败: {}", sheet_name, e))?;

        let mut data: Vec<Vec<String>> = range.rows()
            .map(|row| row.iter().map(|c| match c {
                calamine::Data::String(s) => s.clone(),
                calamine::Data::Int(i) => i.to_string(),
                calamine::Data::Float(f) => f.to_string(),
                calamine::Data::Bool(b) => b.to_string(),
                _ => String::new(),
            }).collect())
            .collect();

        // 2. 应用更新
        let mut updated_count = 0;
        for update in &updates {
            let cell_ref = update.get("cell").and_then(|v| v.as_str())
                .ok_or_else(|| anyhow!("update 项缺少 cell 参数"))?;
            let value = update.get("value")
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_default();

            let (row, col) = parse_cell_ref(cell_ref)?;

            // 确保行数足够
            while data.len() <= row {
                data.push(Vec::new());
            }
            // 确保列数足够
            while data[row].len() <= col {
                data[row].push(String::new());
            }

            data[row][col] = value;
            updated_count += 1;
        }

        // 3. 写回文件
        let mut workbook = rust_xlsxwriter::Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&sheet_name)?;

        for (r, row) in data.iter().enumerate() {
            for (c, val) in row.iter().enumerate() {
                if !val.is_empty() {
                    // 尝试解析为数字
                    if let Ok(n) = val.parse::<f64>() {
                        worksheet.write_number(r as u32, c as u16, n)?;
                    } else {
                        worksheet.write_string(r as u32, c as u16, val)?;
                    }
                }
            }
        }

        workbook.save(&path)?;
        Ok(serde_json::json!({
            "path": path,
            "sheet": sheet_name,
            "updated_cells": updated_count,
        }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 解析单元格引用 "B2" → (row=1, col=1)（0-indexed）
fn parse_cell_ref(cell_ref: &str) -> Result<(usize, usize)> {
    let cell_ref = cell_ref.trim();
    let mut col_str = String::new();
    let mut row_str = String::new();
    let mut in_col = true;

    for ch in cell_ref.chars() {
        if ch.is_ascii_alphabetic() && in_col {
            col_str.push(ch.to_ascii_uppercase());
        } else if ch.is_ascii_digit() {
            in_col = false;
            row_str.push(ch);
        } else {
            return Err(anyhow!("无效的单元格引用: {}", cell_ref));
        }
    }

    if col_str.is_empty() || row_str.is_empty() {
        return Err(anyhow!("无效的单元格引用: {}", cell_ref));
    }

    let mut col: usize = 0;
    for ch in col_str.chars() {
        col = col * 26 + (ch as usize - 'A' as usize + 1);
    }
    col -= 1; // 0-indexed

    let row: usize = row_str.parse::<usize>()
        .map_err(|_| anyhow!("无效的行号: {}", row_str))? - 1; // 0-indexed

    Ok((row, col))
}

/// calamine CellValue → JSON
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

/// 写入 JSON 值到 Excel 单元格
fn write_json_cell(ws: &mut rust_xlsxwriter::Worksheet, row: u32, col: u16, val: &serde_json::Value) -> Result<()> {
    match val {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                ws.write_number(row, col, i as f64)?;
            } else if let Some(f) = n.as_f64() {
                ws.write_number(row, col, f)?;
            }
        }
        serde_json::Value::String(s) => {
            ws.write_string(row, col, s)?;
        }
        serde_json::Value::Bool(b) => {
            ws.write_boolean(row, col, *b)?;
        }
        _ => {}
    }
    Ok(())
}
