// nodes/map.rs — 声明式数组映射节点（替代 script 节点做数据转换）
//
// 用法示例：
//   type: map
//   config:
//     source: output.step_loop.results   # 数据来源（step 输出中的数组字段）
//     template:                          # 每个元素的映射模板
//       cell: "C{{__index1}}"
//       value: "{{__item.step_get_text.result}}"
//
// 变量：
//   {{__item}}    当前元素
//   {{__index}}   当前索引（0-based）
//   {{__index1}}  当前索引（1-based）
//   {{__item.xxx}} 嵌套字段访问

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Default)]
pub struct MapNode;

#[async_trait]
impl NodeExecutor for MapNode {
    /// map 节点的 template 包含 {{__item}} 等迭代变量，
    /// 必须在迭代期间由节点自行解析，不能由 executor 提前 resolve。
    fn resolve_config_self(&self) -> bool {
        true
    }

    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "map".into(),
            version: "1.0".into(),
            display_name: "数组映射".into(),
            description: "声明式数组映射转换节点，使用模板将每个元素映射为新的值".into(),
            category: "数据".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "array".into(), required: false },
            ],
            config_schema: serde_json::json!({
                "type": "object",
                "required": ["source", "template"],
                "properties": {
                    "source": {"type": "string", "description": "数据源，如 output.step_id.results"},
                    "template": {"type": "object", "description": "映射模板，支持 {{__item}} {{__index}} 变量"}
                }
            }),
        }
    }

    async fn execute(
        &self,
        step: &Step,
        ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        let config = &step.config;

        // 获取源数组：支持内联数组 或 字符串引用
        let source_raw = config
            .get("source")
            .ok_or_else(|| anyhow!("map 节点缺少 source 参数"))?;
        let items: Vec<serde_json::Value> = if let Some(arr) = source_raw.as_array() {
            arr.clone()
        } else if let Some(s) = source_raw.as_str() {
            resolve_array(s, ctx)?
        } else {
            return Err(anyhow!("map source 必须是数组或字符串引用"));
        };

        // 获取映射模板
        let template = config
            .get("template")
            .ok_or_else(|| anyhow!("map 节点缺少 template 参数"))?
            .clone();

        // 逐个映射
        let results: Vec<serde_json::Value> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                ctx.set_var("__item".to_string(), item.clone());
                ctx.set_var("__index".to_string(), serde_json::json!(i));
                ctx.set_var("__index1".to_string(), serde_json::json!(i + 1));
                ctx.resolve_config(&template)
            })
            .collect();

        // 清理临时变量
        ctx.set_var("__item".to_string(), serde_json::Value::Null);
        ctx.set_var("__index".to_string(), serde_json::Value::Null);
        ctx.set_var("__index1".to_string(), serde_json::Value::Null);

        Ok(serde_json::json!(results))
    }
}

/// 从 source 路径解析出数组
/// 支持格式：
///   "output.step_id"          → step 输出（必须是数组）
///   "output.step_id.field"    → step 输出中的嵌套字段
///   "output.step_id.arr[n]"   → 索引访问
fn resolve_array(source: &str, ctx: &ExecutionContext) -> Result<Vec<serde_json::Value>> {
    let path = source
        .strip_prefix("output.")
        .ok_or_else(|| anyhow!("source 必须以 'output.' 开头，如 'output.step_id.results'"))?;

    let parts: Vec<&str> = path.split('.').collect();
    let step_id = parts[0];

    let mut current = ctx
        .get_output(step_id)
        .ok_or_else(|| anyhow!("找不到步骤 '{}' 的输出", step_id))?;

    // 遍历嵌套路径
    for part in &parts[1..] {
        // 支持 arr[0] 索引访问
        if let Some(bracket_pos) = part.find('[') {
            let field = &part[..bracket_pos];
            let idx_str = part[bracket_pos + 1..].trim_end_matches(']');
            let idx: usize = idx_str
                .parse()
                .map_err(|_| anyhow!("无效的数组索引: '{}'", idx_str))?;
            current = current
                .get(field)
                .and_then(|arr| arr.get(idx))
                .ok_or_else(|| anyhow!("路径 '{}.{}' 访问失败", step_id, part))?;
        } else {
            current = current
                .get(*part)
                .ok_or_else(|| anyhow!("路径 '{}' 中找不到字段 '{}'", source, part))?;
        }
    }

    current
        .as_array()
        .cloned()
        .ok_or_else(|| anyhow!("路径 '{}' 的结果不是数组", source))
}
