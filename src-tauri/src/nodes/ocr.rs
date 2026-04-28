// nodes/ocr.rs — OCR 文字识别节点
// 使用 Python sidecar 进行 OCR，支持屏幕截图 + 文字识别
use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use anyhow::{Result, anyhow};

#[derive(Default)]
pub struct OcrNode;

#[async_trait]
impl NodeExecutor for OcrNode {
    async fn execute(&self, step: &Step, _ctx: &mut ExecutionContext, _executor: &Arc<StepExecutor>) -> Result<serde_json::Value> {
        let config = &step.config;
        let action = config.get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("read");

        match action {
            // ─── 读取屏幕文字 ───
            "read" => {
                let region = config.get("region"); // 可选：{x, y, width, height}
                let lang = config.get("lang").and_then(|v| v.as_str()).unwrap_or("chi_sim+eng");

                // 截图 → OCR（通过 Python sidecar）
                let mut params = serde_json::json!({"lang": lang});
                if let Some(r) = region {
                    if let Some(obj) = r.as_object() {
                        if let Some(params_obj) = params.as_object_mut() {
                            for (k, v) in obj {
                                params_obj.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }

                let result = crate::nodes::browser::send_sidecar_action("ocr", &params).await?;
                Ok(serde_json::json!({"action":"read","result":result}))
            }

            // ─── 在屏幕上查找文字位置 ───
            "find_text" => {
                let text = config.get("text").and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("find_text 需要 text 参数"))?;
                let params = serde_json::json!({"text": text});
                let result = crate::nodes::browser::send_sidecar_action("find_text", &params).await?;
                Ok(serde_json::json!({"action":"find_text","text":text,"result":result}))
            }

            _ => Err(anyhow!("未知 OCR 操作: {}", action))
        }
    }
}
