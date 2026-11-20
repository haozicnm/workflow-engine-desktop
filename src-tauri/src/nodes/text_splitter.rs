// nodes/text_splitter.rs — 文本分块节点（用于 RAG 文档处理）
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Default)]
pub struct TextSplitterNode;

#[async_trait]
impl NodeExecutor for TextSplitterNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "text_splitter".into(),
            version: "1.0".into(),
            display_name: "文本分块".into(),
            description: "将长文本按 chunk_size 分割为多个块（用于 RAG 文档处理）".into(),
            category: "ai".into(),
            inputs: vec![
                crate::nodes::traits::PortDef { label: "text".into(), data_type: "text".into(), required: false },
            ],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "chunks".into(), data_type: "json".into(), required: false },
                crate::nodes::traits::PortDef { label: "count".into(), data_type: "number".into(), required: false },
            ],
            config_schema: json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "要分块的文本" },
                    "chunk_size": { "type": "number", "default": 1000, "description": "每块最大字符数" },
                    "chunk_overlap": { "type": "number", "default": 200, "description": "块之间重叠字符数" },
                    "separator": { "type": "string", "default": "\n", "description": "优先分割符" }
                }
            }),
            params: vec![],
        }
    }

    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<Value> {
        let config = &step.config;
        let text = ctx.input_ports.get("text").and_then(|v| v.as_str()).map(String::from)
            .or_else(|| config.get("text").and_then(|v| v.as_str()).map(String::from))
            .ok_or_else(|| anyhow!("text_splitter: 缺少 text"))?;

        let raw_chunk_size = config.get("chunk_size").and_then(|v| v.as_u64()).unwrap_or(1000) as usize;
        let raw_chunk_overlap = config.get("chunk_overlap").and_then(|v| v.as_u64()).unwrap_or(200) as usize;
        let separator = config.get("separator").and_then(|v| v.as_str()).unwrap_or("\n");

        // chunk_size = 0 或 chunk_overlap >= chunk_size 时步进量会变成 0，
        // 导致 `end - chunk_overlap` 下溢：debug 下 panic，release 下回绕成巨大值，
        // start 直接越过 total → 循环静默结束、丢失剩余文本。
        let chunk_size = raw_chunk_size.max(1);
        let chunk_overlap = if raw_chunk_overlap >= chunk_size {
            chunk_size / 4
        } else {
            raw_chunk_overlap
        };

        let chunks = split_text(&text, chunk_size, chunk_overlap, separator);
        let count = chunks.len();
        Ok(json!({"chunks": chunks, "count": count}))
    }
}

/// 按字符（而非字节）语义把文本切成若干块，并优先在 `separator` 处断开。
///
/// 原实现混用了两套下标：`str::rfind` 返回**字节**偏移，却被当作 `chars`
/// 向量的下标参与算术（`chars[start..start + last_sep + separator.len()]`）。
/// 对非 ASCII 文本，轻则分块边界错乱，重则 "range end index N out of range" panic；
/// 而 `chunk_overlap >= chunk_size` 时 `end - chunk_overlap` 会 usize 下溢
/// （debug panic / release 静默截断丢文本）。
///
/// 这里统一用字节下标 + 显式 char 边界对齐，并保证起点严格前进。
fn split_text(text: &str, chunk_size: usize, chunk_overlap: usize, separator: &str) -> Vec<String> {
    let chunk_size = chunk_size.max(1);
    let chunk_overlap = if chunk_overlap >= chunk_size {
        chunk_size / 4
    } else {
        chunk_overlap
    };

    let total = text.len();
    let mut chunks: Vec<String> = Vec::new();

    if total <= chunk_size {
        chunks.push(text.to_string());
        return chunks;
    }

    let mut start = 0usize;
    while start < total {
        let mut end = floor_char_boundary(text, (start + chunk_size).min(total));
        if end <= start {
            // chunk_size 小于当前字符的字节长度时，向下取整会退回 start；
            // 强制至少前进一个字符，否则会死循环。
            end = char_boundary_after(text, start);
        }

        // 尝试在 separator 处断开
        if end < total {
            if let Some(rel) = text[start..end].rfind(separator) {
                if rel > chunk_size / 4 {
                    let actual_end = start + rel + separator.len();
                    if actual_end > start && actual_end <= total {
                        chunks.push(text[start..actual_end].to_string());
                        start = advance_from(text, actual_end, chunk_overlap);
                        continue;
                    }
                }
            }
        }
        chunks.push(text[start..end].to_string());
        if end >= total {
            break;
        }
        start = advance_from(text, end, chunk_overlap);
    }

    chunks
}

/// 向下取到最近的 char 边界
fn floor_char_boundary(s: &str, idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    let mut i = idx;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// 严格大于 `idx` 的下一个 char 边界（`idx` 本身是边界时前进一个字符）
fn char_boundary_after(s: &str, idx: usize) -> usize {
    let mut i = idx + 1;
    while i < s.len() && !s.is_char_boundary(i) {
        i += 1;
    }
    i.min(s.len())
}

/// 从 `from` 回退 `overlap` 字节，并保证结果严格大于上一次的起点
/// （防止 overlap 不合法导致 start 不前进 / 下溢）
fn advance_from(s: &str, from: usize, overlap: usize) -> usize {
    let back = from.saturating_sub(overlap);
    let back = floor_char_boundary(s, back);
    if back == 0 || back >= from {
        from
    } else {
        back
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_well_formed(text: &str, chunks: &[String]) {
        assert!(!chunks.is_empty(), "至少要产出一个块");
        for c in chunks {
            assert!(!c.is_empty(), "不应产出空块");
        }
        // 循环只在 end >= total 时退出 → 首块是前缀，末块是后缀
        assert!(text.starts_with(&chunks[0]));
        assert!(
            text.ends_with(chunks.last().unwrap()),
            "末块必须覆盖到文本结尾（否则等于丢文本）"
        );
    }

    #[test]
    fn short_text_is_a_single_chunk() {
        let chunks = split_text("hello", 100, 10, "\n");
        assert_eq!(chunks, vec!["hello".to_string()]);
    }

    #[test]
    fn splits_ascii_on_separator() {
        let line = "abcdefghij\n"; // 11 bytes
        let text = line.repeat(20);
        let chunks = split_text(&text, 30, 5, "\n");
        assert_well_formed(&text, &chunks);
        assert!(chunks.len() > 1);
    }

    /// 回归：旧实现把 rfind 的**字节**偏移当成 chars 下标，
    /// 这段文本会触发 "range end index 1801 out of range" panic。
    #[test]
    fn multibyte_text_does_not_panic() {
        let text = format!("{}{}{}", "中".repeat(600), "\n", "中".repeat(600));
        let chunks = split_text(&text, 1000, 200, "\n");
        assert_well_formed(&text, &chunks);
    }

    /// 回归：chunk_overlap >= chunk_size 会让 `end - chunk_overlap` 下溢
    #[test]
    fn overlap_larger_than_chunk_size_is_clamped() {
        let text = "中".repeat(500);
        let chunks = split_text(&text, 100, 200, "\n");
        assert_well_formed(&text, &chunks);
        assert!(chunks.len() > 1);
    }

    /// chunk_size 小于一个字符的字节长度时必须仍然前进（不能死循环/空块）
    #[test]
    fn chunk_size_smaller_than_one_char_terminates() {
        let text = "中文测试内容".to_string();
        let chunks = split_text(&text, 1, 0, "\n");
        assert_well_formed(&text, &chunks);
        assert_eq!(chunks.concat(), text);
    }

    /// chunk_size = 0 会被夹到 1，不能 panic
    #[test]
    fn zero_chunk_size_is_clamped() {
        let text = "abcdef".to_string();
        let chunks = split_text(&text, 0, 0, "\n");
        assert_well_formed(&text, &chunks);
        assert_eq!(chunks.concat(), text);
    }

    #[test]
    fn multibyte_chunks_are_valid_utf8_boundaries() {
        let text = "🎉".repeat(50); // 4 bytes per char
        let chunks = split_text(&text, 9, 4, "\n");
        assert_well_formed(&text, &chunks);
        // 每块都必须是完整的字符（能重新拼回原文的子串即可，切片本身已保证）
        for c in &chunks {
            assert!(text.contains(c.as_str()));
        }
    }
}
