// nodes/word.rs — Word 读写节点（基于 zip/xml 解析 docx 格式，支持富文本/表格/标题）
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use std::io::{Read, Write};
use anyhow::{Result, anyhow};

#[derive(Default)]
pub struct WordNode;

#[async_trait]
impl NodeExecutor for WordNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Word 节点缺少 action 参数"))?;
        let file_path = config.get("path").and_then(|v| v.as_str())
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

// ─── 数据模型 ───

/// 一个文本 run（段落内的带格式文本片段）
#[derive(Debug, Clone)]
struct Run {
    text: String,
    bold: bool,
    italic: bool,
    underline: bool,
    size: Option<u32>,       // half-points (e.g. 24 = 12pt)
    color: Option<String>,   // hex RGB without # (e.g. "FF0000")
}

/// 段落/表格/分页块
#[derive(Debug, Clone)]
enum Block {
    Paragraph {
        heading_level: Option<u32>, // 1-6 for headings, None for normal paragraph
        runs: Vec<Run>,
    },
    Table {
        rows: Vec<Vec<String>>,
    },
    PageBreak,
}

impl Run {
    fn from_json(v: &serde_json::Value) -> Result<Self> {
        let obj = v.as_object()
            .ok_or_else(|| anyhow!("run 必须是对象"))?;
        let text = obj.get("text").and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("run 缺少 text 字段"))?
            .to_string();
        let bold = obj.get("bold").and_then(|v| v.as_bool()).unwrap_or(false);
        let italic = obj.get("italic").and_then(|v| v.as_bool()).unwrap_or(false);
        let underline = obj.get("underline").and_then(|v| v.as_bool()).unwrap_or(false);
        let size = obj.get("size").and_then(|v| v.as_u64()).map(|v| v as u32);
        let color = obj.get("color").and_then(|v| v.as_str()).map(|s| s.to_string());
        Ok(Run { text, bold, italic, underline, size, color })
    }
}

/// 将 paragraphs JSON 数组解析为 Block 列表，支持向后兼容纯字符串
fn parse_paragraphs(paragraphs: &[serde_json::Value]) -> Result<Vec<Block>> {
    let mut blocks = Vec::new();
    for p in paragraphs {
        match p {
            // 纯字符串 → 普通段落
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
            // 对象 → 解析类型
            serde_json::Value::Object(obj) => {
                let block_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("paragraph");
                match block_type {
                    "paragraph" | "heading" => {
                        let heading_level = if block_type == "heading" {
                            obj.get("level").and_then(|v| v.as_u64()).map(|v| v as u32)
                        } else {
                            None
                        };
                        let runs_json = obj.get("runs")
                            .and_then(|v| v.as_array())
                            .ok_or_else(|| anyhow!("段落/标题需要 runs 数组"))?;
                        let runs: Vec<Run> = runs_json.iter()
                            .map(Run::from_json)
                            .collect::<Result<Vec<_>>>()?;
                        if runs.is_empty() {
                            return Err(anyhow!("段落/标题的 runs 不能为空"));
                        }
                        blocks.push(Block::Paragraph { heading_level, runs });
                    }
                    "table" => {
                        let rows_json = obj.get("rows")
                            .and_then(|v| v.as_array())
                            .ok_or_else(|| anyhow!("表格需要 rows 数组"))?;
                        let rows: Vec<Vec<String>> = rows_json.iter()
                            .map(|row| {
                                row.as_array()
                                    .ok_or_else(|| anyhow!("表格每行必须是数组"))
                                    .map(|arr| arr.iter().map(|c| {
                                        c.as_str().unwrap_or(&c.to_string()).to_string()
                                    }).collect())
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
                    _ => {
                        return Err(anyhow!("未知的段落类型: {}", block_type));
                    }
                }
            }
            _ => {
                return Err(anyhow!("paragraphs 数组元素必须是字符串或对象"));
            }
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
        rpr.push_str("<w:u w:val=\"single\"/>");
    }
    if let Some(size) = run.size {
        rpr.push_str(&format!("<w:sz w:val=\"{}\"/><w:szCs w:val=\"{}\"/>", size, size));
    }
    if let Some(ref color) = run.color {
        rpr.push_str(&format!("<w:color w:val=\"{}\"/>", color));
    }

    let rpr_elem = if rpr.is_empty() {
        String::new()
    } else {
        format!("<w:rPr>{}</w:rPr>", rpr)
    };

    format!(
        "<w:r>{}<w:t xml:space=\"preserve\">{}</w:t></w:r>",
        rpr_elem,
        escape_xml(&run.text)
    )
}

fn paragraph_to_xml(block: &Block) -> String {
    match block {
        Block::Paragraph { heading_level, runs } => {
            let mut ppr = String::new();
            if let Some(level) = heading_level {
                // Heading 1 → Heading 6
                let style = format!("Heading{}", (*level).min(6));
                ppr.push_str(&format!("<w:pStyle w:val=\"{}\"/>", style));
            }
            let ppr_elem = if ppr.is_empty() {
                String::new()
            } else {
                format!("<w:pPr>{}</w:pPr>", ppr)
            };

            let runs_xml: String = runs.iter().map(run_to_xml).collect();
            format!("<w:p>{}{}</w:p>", ppr_elem, runs_xml)
        }
        Block::PageBreak => {
            "<w:p><w:r><w:br w:type=\"page\"/></w:r></w:p>".to_string()
        }
        Block::Table { rows } => {
            table_to_xml(rows)
        }
    }
}

fn table_to_xml(rows: &[Vec<String>]) -> String {
    let mut xml = String::new();
    xml.push_str("<w:tbl>");
    xml.push_str("<w:tblPr>");
    xml.push_str("<w:tblStyle w:val=\"TableGrid\"/>");
    xml.push_str("<w:tblW w:w=\"0\" w:type=\"auto\"/>");
    xml.push_str("<w:tblBorders>");
    xml.push_str("<w:top w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>");
    xml.push_str("<w:left w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>");
    xml.push_str("<w:bottom w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>");
    xml.push_str("<w:right w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>");
    xml.push_str("<w:insideH w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>");
    xml.push_str("<w:insideV w:val=\"single\" w:sz=\"4\" w:space=\"0\" w:color=\"auto\"/>");
    xml.push_str("</w:tblBorders>");
    xml.push_str("</w:tblPr>");

    for row in rows {
        xml.push_str("<w:tr>");
        for cell in row {
            xml.push_str("<w:tc>");
            xml.push_str(&format!(
                "<w:p><w:r><w:t xml:space=\"preserve\">{}</w:t></w:r></w:p>",
                escape_xml(cell)
            ));
            xml.push_str("</w:tc>");
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

/// 读取 docx 文件中的所有段落文本（纯文本，保持不变）
async fn word_read(path: &str) -> Result<serde_json::Value> {
    let path = path.to_string();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let file = std::fs::File::open(&path)?;
        let mut archive = zip::ZipArchive::new(file)
            .map_err(|e| anyhow!("打开 docx 文件失败: {}", e))?;

        let mut doc_xml = String::new();
        {
            let mut xml_file = archive.by_name("word/document.xml")
                .map_err(|e| anyhow!("docx 格式错误（缺少 word/document.xml）: {}", e))?;
            xml_file.read_to_string(&mut doc_xml)?;
        }

        let paragraphs = extract_paragraphs_text(&doc_xml);

        Ok(serde_json::json!({
            "path": path,
            "paragraphs": paragraphs,
            "paragraph_count": paragraphs.len(),
        }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 创建新 docx 文件（支持富文本）
async fn word_write(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let paragraphs = config.get("paragraphs").and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("write 需要 paragraphs 参数"))?
        .clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        let blocks = parse_paragraphs(&paragraphs)?;
        let doc_xml = build_document_xml_from_blocks(&blocks);
        create_docx(&path, &doc_xml)?;

        Ok(serde_json::json!({
            "path": path,
            "paragraphs_written": blocks.len(),
        }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 追加段落到现有 docx（在 </w:body> 前插入新段落 XML，保留原有格式）
async fn word_append(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let paragraphs = config.get("paragraphs").and_then(|v| v.as_array())
        .ok_or_else(|| anyhow!("append 需要 paragraphs 参数"))?
        .clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        // 1. 读取现有 document.xml
        let existing_xml = {
            let file = std::fs::File::open(&path)?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| anyhow!("打开 docx 失败: {}", e))?;
            let mut xml = String::new();
            archive.by_name("word/document.xml")
                .map_err(|e| anyhow!("docx 格式错误: {}", e))?
                .read_to_string(&mut xml)?;
            xml
        };

        // 2. 解析新段落为 blocks 并生成 XML
        let blocks = parse_paragraphs(&paragraphs)?;
        let new_xml = blocks_to_body_xml(&blocks);

        // 3. 在 </w:body> 前插入新段落 XML
        let body_close = "</w:body>";
        let insert_pos = existing_xml.rfind(body_close)
            .ok_or_else(|| anyhow!("docx XML 格式异常：找不到 </w:body>"))?;

        let mut combined_xml = String::with_capacity(existing_xml.len() + new_xml.len());
        combined_xml.push_str(&existing_xml[..insert_pos]);
        combined_xml.push_str(&new_xml);
        combined_xml.push_str(&existing_xml[insert_pos..]);

        // 4. 重写文件（保留其他 zip 条目不变）
        rewrite_docx_document_xml(&path, &combined_xml)?;

        Ok(serde_json::json!({
            "path": path,
            "appended": blocks.len(),
        }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

/// 替换 docx 中的占位符（在原始 XML 上做字符串替换，保留原有格式）
async fn word_replace(path: &str, config: &serde_json::Value) -> Result<serde_json::Value> {
    let path = path.to_string();
    let output_path = config.get("output")
        .and_then(|v| v.as_str())
        .map(String::from)
        .unwrap_or_else(|| {
            let p = std::path::Path::new(&path);
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("output");
            let parent = p.parent().and_then(|p| p.to_str()).unwrap_or(".");
            format!("{}_output.docx", std::path::Path::new(parent).join(stem).display())
        });
    let replacements = config.get("replacements")
        .and_then(|v| v.as_object())
        .ok_or_else(|| anyhow!("replace 需要 replacements 参数（{{placeholder: value}}）"))?
        .clone();

    tokio::task::spawn_blocking(move || -> Result<serde_json::Value> {
        // 1. 读取现有 document.xml
        let existing_xml = {
            let file = std::fs::File::open(&path)?;
            let mut archive = zip::ZipArchive::new(file)
                .map_err(|e| anyhow!("打开 docx 失败: {}", e))?;
            let mut xml = String::new();
            archive.by_name("word/document.xml")
                .map_err(|e| anyhow!("docx 格式错误: {}", e))?
                .read_to_string(&mut xml)?;
            xml
        };

        // 2. 在原始 XML 上做字符串替换
        let mut replaced_xml = existing_xml.clone();
        let mut replaced_count = 0;
        for (placeholder, value) in &replacements {
            let val_owned = value.to_string();
            let val_str = value.as_str().unwrap_or(&val_owned);
            // XML 中的文本已经被 escape，所以我们同时匹配未 escape 和已 escape 的版本
            let escaped_val = escape_xml(val_str);
            let escaped_placeholder = escape_xml(placeholder);

            // 替换原始 XML 文本
            let count1 = replaced_xml.matches(placeholder).count();
            if count1 > 0 {
                replaced_xml = replaced_xml.replace(placeholder, val_str);
                replaced_count += count1;
            }

            // 替换 XML-escaped 版本
            let count2 = replaced_xml.matches(&escaped_placeholder).count();
            if count2 > 0 {
                replaced_xml = replaced_xml.replace(&escaped_placeholder, &escaped_val);
                replaced_count += count2;
            }
        }

        // 3. 生成输出 docx
        let output = output_path.clone();
        create_docx_from_existing(&path, &output, &replaced_xml)?;

        Ok(serde_json::json!({
            "path": output,
            "replaced": replaced_count,
        }))
    }).await.map_err(|e| anyhow!("任务执行失败: {}", e))?
}

// ─── docx 辅助函数 ───

/// 从现有 docx 读取所有条目，替换 document.xml，写入新文件
fn create_docx_from_existing(src_path: &str, dst_path: &str, new_document_xml: &str) -> Result<()> {
    let src_file = std::fs::File::open(src_path)?;
    let mut src_archive = zip::ZipArchive::new(src_file)
        .map_err(|e| anyhow!("打开源 docx 失败: {}", e))?;

    let dst_file = std::fs::File::create(dst_path)?;
    let mut dst_zip = zip::ZipWriter::new(dst_file);
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for i in 0..src_archive.len() {
        let mut entry = src_archive.by_index(i)
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

/// 重写 docx 中的 document.xml，保留其他条目不变
fn rewrite_docx_document_xml(path: &str, new_document_xml: &str) -> Result<()> {
    let tmp_path = format!("{}.tmp", path);
    create_docx_from_existing(path, &tmp_path, new_document_xml)?;
    std::fs::rename(&tmp_path, path)
        .map_err(|e| anyhow!("重命名临时文件失败: {}", e))?;
    Ok(())
}

/// 从 XML 中提取纯文本段落（word_read 用）
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
    let options = zip::write::FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("[Content_Types].xml", options)?;
    zip.write_all(CONTENT_TYPES_XML.as_bytes())?;

    zip.start_file("_rels/.rels", options)?;
    zip.write_all(RELS_XML.as_bytes())?;

    zip.start_file("word/document.xml", options)?;
    zip.write_all(document_xml.as_bytes())?;

    zip.start_file("word/_rels/document.xml.rels", options)?;
    zip.write_all(DOC_RELS_XML.as_bytes())?;

    zip.finish()?;
    Ok(())
}

const CONTENT_TYPES_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
</Types>"#;

const RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"#;

const DOC_RELS_XML: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
</Relationships>"#;
