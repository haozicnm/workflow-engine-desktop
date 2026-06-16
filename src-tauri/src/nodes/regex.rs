// nodes/regex.rs — 正则处理节点（P3: 合并为单一 regex 节点）
//
// regex — 通过 mode 参数控制行为:
//   mode = "extract" (默认) — 提取捕获组
//   mode = "match"           — 测试匹配

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use regex::Regex;
use std::sync::Arc;
use tracing::info;

// ── Shared helpers ──

fn compile_pattern(config: &serde_json::Value) -> Result<Regex> {
    let pattern = config
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("缺少 pattern 参数"))?;
    Regex::new(pattern).map_err(|e| anyhow!("正则表达式编译失败: {}", e))
}

fn get_input(config: &serde_json::Value) -> Result<String> {
    config
        .get("input")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| config.get("input").map(|v| v.to_string()))
        .ok_or_else(|| anyhow!("缺少 input 参数"))
}

// ═══════════════════════════════════════
// regex — 统一正则节点（P3 合并）
// ═══════════════════════════════════════

#[derive(Default)]
pub struct RegexNode;

#[async_trait]
impl NodeExecutor for RegexNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "regex".into(),
            version: "1.0".into(),
            display_name: "正则表达式".into(),
            description: "使用正则表达式提取或匹配文本内容".into(),
            category: "数据".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "matches".into(), data_type: "array".into(), required: false },
                crate::nodes::traits::PortDef { label: "captures".into(), data_type: "array".into(), required: false },
                crate::nodes::traits::PortDef { label: "count".into(), data_type: "number".into(), required: false },
                crate::nodes::traits::PortDef { label: "is_match".into(), data_type: "bool".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "required": ["pattern", "input"],
                "properties": {
                    "pattern": {"type": "string", "description": "正则表达式模式"},
                    "input": {"type": "string", "description": "输入文本"},
                    "mode": {"type": "string", "enum": ["extract", "match"], "description": "操作模式", "default": "extract"},
                    "global": {"type": "boolean", "description": "是否全局匹配", "default": true}
                }
            }),
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;
        let re = compile_pattern(config)?;
        let input = get_input(config)?;
        let global = config
            .get("global")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let mode = config
            .get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("extract");

        match mode {
            "match" => {
                // ── match 模式：查找匹配位置 ──
                let mut matches: Vec<serde_json::Value> = Vec::new();
                if global {
                    for m in re.find_iter(&input) {
                        matches.push(
                            serde_json::json!({ "text": m.as_str(), "start": m.start(), "end": m.end() }),
                        );
                    }
                } else if let Some(m) = re.find(&input) {
                    matches.push(
                        serde_json::json!({ "text": m.as_str(), "start": m.start(), "end": m.end() }),
                    );
                }

                let count = matches.len();
                info!("正则匹配: pattern={}, {} 处", re.as_str(), count);
                Ok(
                    serde_json::json!({ "pattern": re.as_str(), "mode": "match", "matches": matches, "count": count, "is_match": count > 0 }),
                )
            }
            _ => {
                // ── extract 模式（默认）：提取捕获组 ──
                let mut captures: Vec<serde_json::Value> = Vec::new();
                if global {
                    for caps in re.captures_iter(&input) {
                        let groups: Vec<String> = caps
                            .iter()
                            .map(|m| m.map(|m| m.as_str().to_string()).unwrap_or_default())
                            .collect();
                        captures.push(serde_json::Value::Array(
                            groups.into_iter().map(serde_json::Value::String).collect(),
                        ));
                    }
                } else if let Some(caps) = re.captures(&input) {
                    let groups: Vec<String> = caps
                        .iter()
                        .map(|m| m.map(|m| m.as_str().to_string()).unwrap_or_default())
                        .collect();
                    captures.push(serde_json::Value::Array(
                        groups.into_iter().map(serde_json::Value::String).collect(),
                    ));
                }

                let count = captures.len();
                info!("正则提取: pattern={}, {} 组", re.as_str(), count);
                Ok(serde_json::json!({ "pattern": re.as_str(), "mode": "extract", "captures": captures, "count": count, "is_match": count > 0 }))
            }
        }
    }
}
