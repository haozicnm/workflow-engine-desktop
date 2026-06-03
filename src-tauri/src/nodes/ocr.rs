// nodes/ocr.rs — OCR 文字识别节点
// 通过 WebBridge 扩展进行截图，OCR 处理需外部完成
// TODO: OCR 能力应后续添加到 WebBridge 扩展中
use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Default)]
pub struct OcrNode;

#[async_trait]
impl NodeExecutor for OcrNode {
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
            .unwrap_or("read");

        match action {
            // ─── 读取屏幕文字 ───
            "read" => {
                let region = config.get("region"); // 可选：{x, y, width, height}
                let lang = config
                    .get("lang")
                    .and_then(|v| v.as_str())
                    .unwrap_or("chi_sim+eng");

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

                // OCR 能力需 WebBridge 扩展支持，当前截图仅返回原始图像
                // 如需文字识别，请使用 shell 节点调用 tesseract CLI：
                //   tesseract screenshot.png stdout
                let result = crate::nodes::webbridge::send_command("screenshot", params).await?;
                Ok(serde_json::json!({"action":"read","result":result,"warning":"OCR 文字识别尚未实现，仅返回截图。可通过 shell 节点调用 tesseract"}))
            }

            // ─── 在屏幕上查找文字位置 ───
            "find_text" => {
                let text = config
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow!("find_text 需要 text 参数"))?;
                let params = serde_json::json!({"text": text});
                // find_text 能力需 WebBridge 扩展支持
                let result = crate::nodes::webbridge::send_command("find_text", params).await?;
                Ok(serde_json::json!({"action":"find_text","result":result,"warning":"find_text 尚未完全实现"}))
            }

            _ => Err(anyhow!("未知 OCR 操作: {}", action)),
        }
    }
}
