// nodes/word.rs — Word 读写节点（拆分为细粒度操作，基于 zip/xml 解析 docx 格式）
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::io::{Read, Write};
use std::sync::Arc;

// ═══════════════════════════════════════════
// Word 通用节点（兼容旧版）
// ═══════════════════════════════════════════
#[derive(Default)]
pub struct WordNode;

#[async_trait]
impl NodeExecutor for WordNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Word 节点缺少 action 参数"))?;
        let file_path = config
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Word 节点缺少 path 参数"))?;

        match action {
            "read" => word_read(file_path).await,
            "write" => word_write(file_path, config).await,
            "append" => word_append(file_path, config).await,
            "replace" => word_replace(file_path, config).await,
            _ => Err(anyhow!("未知的 Word 操作: {}", action)),
        }
    }
}

// ═══════════════════════════════════════════
// 拆分节点
// ═══════════════════════════════════════════

// ── word_read ──
#[derive(Default)]
pub struct WordReadNode;

#[async_trait]
impl NodeExecutor for WordReadNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let path = step
            .config
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("读取文档需要 path 参数"))?;
        word_read(path).await
    }
}

// ── word_write ──
#[derive(Default)]
pub struct WordWriteNode;

#[async_trait]
impl NodeExecutor for WordWriteNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = config
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("写入文档需要 path 参数"))?;
        let mode = config
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("overwrite");

        let content = config.get("content");
        let paragraphs = match content {
            Some(c) => match c {
                // content 是字符串 → 单段落
                serde_json::Value::String(s) => serde_json::json!([s]),
                // content 是数组 → 多段落
                serde_json::Value::Array(_) => c.clone(),
                _ => serde_json::json!([c.to_string()]),
            },
            None => return Err(anyhow!("写入文档需要 content 参数")),
        };

        let cfg = serde_json::json!({ "paragraphs": paragraphs });
        match mode {
            "append" => word_append(path, &cfg).await,
            _ => word_write(path, &cfg).await,
        }
    }
}

// ── word_create ──
#[derive(Default)]
pub struct WordCreateNode;

#[async_trait]
impl NodeExecutor for WordCreateNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = config
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("创建文档需要 path 参数"))?;
        let title = config.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let content = config.get("content");

        let mut paras: Vec<serde_json::Value> = Vec::new();
        // 如果有标题，加一个标题段落
        if !title.is_empty() {
            paras.push(serde_json::json!({
                "type": "heading",
                "level": 1,
                "runs": [{ "text": title, "bold": true, "size": 32 }]
            }));
        }
        // 如果有内容
        if let Some(c) = content {
            match c {
                serde_json::Value::String(s) => {
                    paras.push(serde_json::json!(s));
                }
                serde_json::Value::Array(arr) => {
                    paras.extend(arr.clone());
                }
                _ => {
                    paras.push(serde_json::json!(c.to_string()));
                }
            }
        }

        let cfg = serde_json::json!({ "paragraphs": paras });
        word_write(path, &cfg).await
    }
}

// ── word_replace ──
#[derive(Default)]
pub struct WordReplaceNode;

#[async_trait]
impl NodeExecutor for WordReplaceNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let path = config
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("查找替换需要 path 参数"))?;
        let find = config
            .get("find")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("查找替换需要 find 参数"))?;
        let replace = config.get("replace").and_then(|v| v.as_str()).unwrap_or("");
        let count = config.get("count").and_then(|v| v.as_u64()).unwrap_or(0);

        let mut replacements = serde_json::Map::new();
        replacements.insert(find.to_string(), serde_json::json!(replace));

        let cfg = serde_json::json!({
            "replacements": replacements,
            "count": count,
        });
        word_replace(path, &cfg).await
    }
}

// ── word_merge ──
#[derive(Default)]
pub struct WordMergeNode;

#[async_trait]
impl NodeExecutor for WordMergeNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let output = config
            .get("output")
            .and_then(|v| v.as_str())
            .unwrap_or("合并文档.docx");
        let paths_str = config.get("paths").and_then(|v| v.as_str()).unwrap_or("");
        let files = config.get("files");

        let path_list: Vec<String> = if !paths_str.is_empty() {
            paths_str.split(',').map(|s| s.trim().to_string()).collect()
        } else if let Some(arr) = files.and_then(|v| v.as_array()) {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        } else {
            return Err(anyhow!("合并文档需要 paths 或 files 参数"));
        };

        word_merge_files(&path_list, output).await
    }
}

// ═══════════════════════════════════════════
// 数据模型
// ═══════════════════════════════════════════

#[derive(Debug, Clone)]
struct Run {
    text: String,
    bold: bool,
    italic: bool,
    underline: bool,
    size: Option<u32>,
    color: Option<String>,
}

#[derive(Debug, Clone)]
enum Block {
    Paragraph {
        heading_level: Option<u32>,
        runs: Vec<Run>,
    },
    Table {
        rows: Vec<Vec<String>>,
    },
    PageBreak,
}

impl Run {
    fn from_json(v: &serde_json::Value) -> Result<Self> {
        let obj = v.as_object().ok_or_else(|| anyhow!("run 必须是对象"))?;
        let text = obj
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("run 缺少 text 字段"))?
            .to_string();
        let bold = obj.get("bold").and_then(|v| v.as_bool()).unwrap_or(false);
        let italic = obj.get("italic").and_then(|v| v.as_bool()).unwrap_or(false);
        let underline = obj
            .get("underline")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let size = obj.get("size").and_then(|v| v.as_u64()).map(|v| v as u32);
        let color = obj
            .get("color")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Ok(Run {
            text,
            bold,
            italic,
            underline,
            size,
            color,
        })
    }
}

fn parse_paragraphs(paragraphs: &[serde_json::Value]) -> Result<Vec<Block>> {
    let mut blocks = Vec::new();
    for p in paragraphs {
        match p {
            serde_json::Value::String(s) => {
                blocks.push(Block::Paragraph {
                    heading_level: None,
                    runs: vec![Run {
                        text: s.clone(),
                        bold: false,
                        italic: false,
                        underline: false,
                        size: None,
                        color: None,
                    }],
                });
            }
            serde_json::Value::Object(obj) => {
                let block_type = obj
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("paragraph");
                match block_type {
                    "paragraph" | "heading" => {
                        let heading_level = if block_type == "heading" {
                            obj.get("level").and_then(|v| v.as_u64()).map(|v| v as u32)
                        } else {
                            None
                        };
                        let runs_json = obj
                            .get("runs")
                            .and_then(|v| v.as_array())
                            .ok_or_else(|| anyhow!("段落/标题需要 runs 数组"))?;
                        let runs: Vec<Run> = runs_json
                            .iter()
                            .map(Run::from_json)
                            .collect::<Result<Vec<_>>>()?;
                        if runs.is_empty() {
                            return Err(anyhow!("段落/标题的 runs 不能为空"));
                        }
                        blocks.push(Block::Paragraph {
                            heading_level,
                            runs,
                        });
                    }
                    "table" => {
                        let rows_json = obj
                            .get("rows")
                            .and_then(|v| v.as_array())
                            .ok_or_else(|| anyhow!("表格需要 rows 数组"))?;
                        let rows: Vec<Vec<String>> = rows_json
                            .iter()
                            .map(|row| {
                                row.as_array()
                                    .ok_or_else(|| anyhow!("表格每行必须是数组"))
                                    .map(|arr| {
                                        arr.iter()
                                            .map(|c| {
                                                c.as_str().unwrap_or(&c.to_string()).to_string()
                                            })
                                            .collect()
                                    })
                            })
                            .collect::<Result<Vec<_>>>()?;
                        if rows.is_empty() {
                            return Err(anyhow!("表格 rows 不能为空"));
                        }
                        blocks.push(Block::Table { rows });
                    }
                    "pagebreak" => {
                        blocks.push(Block::PageBreak);
                    }
                    _ => return Err(anyhow!("未知的段落类型: {}", block_type)),
                }
            }
            _ => return Err(anyhow!("paragraphs 数组元素必须是字符串或对象")),
        }
    }
    Ok(blocks)
}

// ─── XML 生成 ───

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn run_to_xml(run: &Run) -> String {
    let mut rpr = String::new();
    if run.bold {
        rpr.push_str("<w:b/><w:bCs/>");
    }
    if run.italic {
        rpr.push_str("<w:i/><w:iCs/>");
    }
    if run.underline {
        rpr.push_str(r#"<w:u w:val="single"/>"#);
    }
    if let Some(size) = run.size {
        rpr.push_str(&format!(
            r#"<w:sz w:val="{}"/><w:szCs w:val="{}"/>"#,
            size, size
        ));
    }
    if let Some(ref color) = run.color {
        rpr.push_str(&format!(r#"<w:color w:val="{}"/>"#, color));
    }
    let rpr_elem = if rpr.is_empty() {
        String::new()
    } else {
        format!("<w:rPr>{}</w:rPr>", rpr)
    };
    format!(
        r#"<w:r>{}<w:t xml:space="preserve">{}</w:t></w:r>"#,
        rpr_elem,
        escape_xml(&run.text)
    )
}

fn paragraph_to_xml(block: &Block) -> String {
    match block {
        Block::Paragraph {
            heading_level,
            runs,
        } => {
            let mut ppr = String::new();
            if let Some(level) = heading_level {
                let style = format!("Heading{}", (*level).min(6));
                ppr.push_str(&format!(r#"<w:pStyle w:val="{}"/>"#, style));
            }
            let ppr_elem = if ppr.is_empty() {
                String::new()
            } else {
                format!("<w:pPr>{}</w:pPr>", ppr)
            };
            let runs_xml: String = runs.iter().map(run_to_xml).collect();
            format!("<w:p>{}{}</w:p>", ppr_elem, runs_xml)
        }
        Block::PageBreak => r#"<w:p><w:r><w:br w:type="page"/></w:r></w:p>"#.to_string(),
        Block::Table { rows } => table_to_xml(rows),
    }
}

fn table_to_xml(rows: &[Vec<String>]) -> String {
    let mut xml = String::from(
        "<w:tbl><w:tblPr><w:tblStyle w:val=\"TableGrid\"/><w:tblW w:w=\"0\" w:type=\"auto\"/>",
    );
    xml.push_str(
        "<w:tblBorders><w:top w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>",
    );
    xml.push_str("<w:left w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>");
    xml.push_str("<w:bottom w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>");
    xml.push_str("<w:right w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>");
    xml.push_str("<w:insideH w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>");
    xml.push_str("<w:insideV w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>");
    xml.push_str("</w:tblBorders></w:tblPr>");
    for row in rows {
        xml.push_str("<w:tr>");
        for cell in row {
            xml.push_str(&format!(
                r#"<w:tc><w:p><w:r><w:t xml:space="preserve">{}</w:t></w:r></w:p></w:tc>"#,
                escape_xml(cell)
            ));
        }
        xml.push_str("</w:tr>");
    }
    xml.push_str("</w:tbl>");
    xml
}

fn blocks_to_body_xml(blocks: &[Block]) -> String {
    blocks.iter().map(paragraph_to_xml).collect()
}

fn build_document_xml_from_blocks(blocks: &[Block]) -> String {
    let body = blocks_to_body_xml(blocks);
    format!(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:body>{}</w:body>
</w:document>"#,
        body
    )
}

// ─── 操作函数 ───

pub async fn word_read(path: &str) -> Result<serde_json::Value> {
    let path = path.to_string();
    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let file = std::fs::File::open(&path)?;
        let mut archive = zip::ZipArchive::new(file).map_err(|e| anyhow!("打开 docx 失败: {}", e))?;
        let mut doc_xml = String::new();
        archive.by_name("word/document.xml")
            .map_err(|e| anyhow!("docx 格式错误: {}", e))?
            .read_to_string(&mut doc_xml)?;
        let paragraphs = extract_paragraphs_text(&doc_xml);
        Ok(serde_json::json!({ "path": path, "paragraphs": paragraphs, "paragraph_count": paragraphs.len() }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

pub async fn word_write(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let paragraphs = config
        .get("paragraphs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("write 需要 paragraphs 参数"))?
        .clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let blocks = parse_paragraphs(&paragraphs)?;
        let doc_xml = build_document_xml_from_blocks(&blocks);
        create_docx(&path, &doc_xml)?;
        Ok(serde_json::json!({ "path": path, "paragraphs_written": blocks.len() }))
    })
    .await
    .map_err(|e| anyhow!("任务执行失败: {}", e))?
}

async fn word_append(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let paragraphs = config
        .get("paragraphs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("append 需要 paragraphs 参数"))?
        .clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let existing_xml = {
            let file = std::fs::File::open(&path)?;
            let mut archive =
                zip::ZipArchive::new(file).map_err(|e| anyhow!("打开 docx 失败: {}", e))?;
            let mut xml = String::new();
            archive
                .by_name("word/document.xml")?
                .read_to_string(&mut xml)?;
            xml
        };
        let blocks = parse_paragraphs(&paragraphs)?;
        let new_xml = blocks_to_body_xml(&blocks);
        let body_close = "</w:body>";
        let insert_pos = existing_xml
            .rfind(body_close)
            .ok_or_else(|| anyhow!("docx XML 格式异常：找不到 </w:body>"))?;
        let mut combined_xml = String::with_capacity(existing_xml.len() + new_xml.len());
        combined_xml.push_str(&existing_xml[..insert_pos]);
        combined_xml.push_str(&new_xml);
        combined_xml.push_str(&existing_xml[insert_pos..]);
        rewrite_docx_document_xml(&path, &combined_xml)?;
        Ok(serde_json::json!({ "path": path, "appended": blocks.len() }))
    })
    .await
    .map_err(|e| anyhow!("任务执行失败: {}", e))?
}

pub async fn word_replace(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let output_path = config
        .get("output")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| {
            let p = std::path::Path::new(&path);
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
            let parent = p.parent().and_then(|p| p.to_str()).unwrap_or(".");
            format!(
                "{}_output.docx",
                std::path::Path::new(parent).join(stem).display()
            )
        });
    let replacements = config
        .get("replacements")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow!("replace 需要 replacements 参数"))?
        .clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let existing_xml = {
            let file = std::fs::File::open(&path)?;
            let mut archive =
                zip::ZipArchive::new(file).map_err(|e| anyhow!("打开 docx 失败: {}", e))?;
            let mut xml = String::new();
            archive
                .by_name("word/document.xml")?
                .read_to_string(&mut xml)?;
            xml
        };
        let mut replaced_xml = existing_xml.clone();
        let mut replaced_count = 0;
        for (placeholder, value) in &replacements {
            let val_str = value.as_str().unwrap_or("");
            let val_owned = if val_str.is_empty() {
                value.to_string()
            } else {
                val_str.to_string()
            };
            let escaped_val = escape_xml(&val_owned);
            let escaped_placeholder = escape_xml(placeholder);

            // Replace raw text
            let count1 = replaced_xml.matches(placeholder).count();
            if count1 > 0 {
                replaced_xml = replaced_xml.replace(placeholder, val_str);
                replaced_count += count1;
            }
            // Replace XML-escaped
            let count2 = replaced_xml.matches(&escaped_placeholder).count();
            if count2 > 0 {
                replaced_xml = replaced_xml.replace(&escaped_placeholder, &escaped_val);
                replaced_count += count2;
            }
        }
        create_docx_from_existing(&path, &output_path, &replaced_xml)?;
        Ok(serde_json::json!({ "path": output_path, "replaced": replaced_count }))
    })
    .await
    .map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 合并多个 docx 文件
pub async fn word_merge_files(paths: &[String], output: &str) -> Result<serde_json::Value> {
    let paths = paths.to_vec();
    let output = output.to_string();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let mut all_paragraphs: Vec<String> = Vec::new();
        let mut total = 0u32;

        for path in &paths {
            let file = std::fs::File::open(path)
                .map_err(|e| anyhow!("打开文件 {} 失败: {}", path, e))?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| anyhow!("打开 docx {} 失败: {}", path, e))?;
            let mut xml = String::new();
            archive.by_name("word/document.xml")
                .map_err(|e| anyhow!("docx {} 格式错误: {}", path, e))?
                .read_to_string(&mut xml)?;
            let paras = extract_paragraphs_text(&xml);
            total += paras.len() as u32;
            all_paragraphs.extend(paras);
            // 文档间加分页
            all_paragraphs.push("\n--- 分页 ---\n".to_string());
        }

        // 用合并段落生成新 docx
        let paragraphs: Vec<serde_json::Value> = all_paragraphs.iter()
            .map(|s| serde_json::json!(s)).collect();
        let blocks = parse_paragraphs(&paragraphs)?;
        let doc_xml = build_document_xml_from_blocks(&blocks);
        create_docx(&output, &doc_xml)?;

        Ok(serde_json::json!({ "path": output, "merged_files": paths.len(), "total_paragraphs": total }))
    }).await.map_err(|e| anyhow!("合并文档失败: {}", e))?
}

// ─── docx 辅助函数 ───

fn create_docx_from_existing(src_path: &str, dst_path: &str, new_document_xml: &str) -> Result<()> {
    let src_file = std::fs::File::open(src_path)?;
    let mut src_archive =
        zip::ZipArchive::new(src_file).map_err(|e| anyhow!("打开源 docx 失败: {}", e))?;
    let dst_file = std::fs::File::create(dst_path)?;
    let mut dst_zip = zip::ZipWriter::new(dst_file);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for i in 0..src_archive.len() {
        let mut entry = src_archive
            .by_index(i)
            .map_err(|e| anyhow!("读取 zip 条目失败: {}", e))?;
        let name = entry.name().to_string();
        if name == "word/document.xml" {
            dst_zip.start_file(&name, options)?;
            dst_zip.write_all(new_document_xml.as_bytes())?;
        } else {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            dst_zip.start_file(&name, options)?;
            dst_zip.write_all(&buf)?;
        }
    }
    dst_zip.finish()?;
    Ok(())
}

fn rewrite_docx_document_xml(path: &str, new_document_xml: &str) -> Result<()> {
    let tmp_path = format!("{}.tmp", path);
    create_docx_from_existing(path, &tmp_path, new_document_xml)?;
    std::fs::rename(&tmp_path, path).map_err(|e| anyhow!("重命名临时文件失败: {}", e))?;
    Ok(())
}

fn extract_paragraphs_text(xml: &str) -> Vec<String> {
    let mut paragraphs = Vec::new();
    let mut in_p = false;
    let mut current_text = String::new();
    for (i, ch) in xml.char_indices() {
        if ch == '<' {
            let tag_end = xml[i..].find('>').map(|p| i + p).unwrap_or(xml.len());
            let tag = &xml[i..=tag_end];
            if tag.starts_with("<w:p>") || tag.starts_with("<w:p ") {
                in_p = true;
                current_text.clear();
            } else if tag == "</w:p>" {
                if in_p {
                    paragraphs.push(current_text.clone());
                }
                in_p = false;
                current_text.clear();
            }
        } else if in_p {
            let before = &xml[..i];
            let last_open = before.rfind("<w:t").unwrap_or(0);
            let last_close_tag = before.rfind("</w:t>").map(|p| p + 6).unwrap_or(0);
            if last_open > last_close_tag || (last_close_tag == 0 && before.contains("<w:t")) {
                current_text.push(ch);
            }
        }
    }
    paragraphs
}

fn create_docx(path: &str, document_xml: &str) -> Result<()> {
    let file = std::fs::File::create(path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // [Content_Types].xml
    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#)?;

    // _rels/.rels
    zip.start_file("_rels/.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/></Relationships>"#)?;

    // word/_rels/document.xml.rels
    zip.start_file("word/_rels/document.xml.rels", options)?;
    zip.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"></Relationships>"#)?;

    // word/document.xml
    zip.start_file("word/document.xml", options)?;
    zip.write_all(document_xml.as_bytes())?;

    zip.finish()?;
    Ok(())
}
