// engine/executor.rs — 步骤执行器
//
// v3: 所有节点独立 executor，不再使用 action 参数分发。
//   每个操作一个 struct，一个注册条目。

use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::collections::HashMap;
use serde_json::Value;
use anyhow::Result;
use tracing::{info, debug};

pub struct StepExecutor {
    executors: HashMap<String, Box<dyn NodeExecutor>>,
    pub approval_store: Arc<crate::engine::approval_store::ApprovalStore>,
    pub db: Arc<crate::data::db::Database>,
}

macro_rules! register {
    ($map:expr, $key:expr, $ctor:expr) => {
        $map.insert($key.to_string(), Box::new($ctor));
    };
}

/// 容器节点注册宏 — 自动加 _container 后缀，与 parser::CONTAINER_TYPES 保持同步
macro_rules! register_containers {
    ($map:expr, $( $type:literal => $ctor:expr ),* $(,)?) => {
        $(
            $map.insert(concat!($type, "_container").to_string(), Box::new($ctor));
        )*
    };
}

impl StepExecutor {
    pub fn new(approval_store: Arc<crate::engine::approval_store::ApprovalStore>, db: Arc<crate::data::db::Database>) -> Arc<Self> {
        let mut executors: HashMap<String, Box<dyn NodeExecutor>> = HashMap::new();

        // ── P0 核心节点 ──
        register!(executors, "http", crate::nodes::http::HttpNode);
        register!(executors, "script", crate::nodes::script::ScriptNode);
        register!(executors, "condition", crate::nodes::condition::ConditionNode);

        // ── 数据处理节点（v3: 独立 executor） ──
        register!(executors, "data_set", crate::nodes::data::DataSetNode);
        register!(executors, "data_get", crate::nodes::data::DataGetNode);
        register!(executors, "data_length", crate::nodes::data::DataLengthNode);
        register!(executors, "data_default", crate::nodes::data::DataDefaultNode);
        register!(executors, "data_merge", crate::nodes::data::DataMergeNode);

        // ── 文件节点（v3: 独立 executor） ──
        
        // ── 剪贴板节点（v3: 独立 executor） ──
        register!(executors, "clipboard_read", crate::nodes::clipboard::ClipboardReadNode);
        register!(executors, "clipboard_write", crate::nodes::clipboard::ClipboardWriteNode);
        register!(executors, "clipboard", crate::nodes::clipboard_container::ClipboardContainerNode);

        // ── 正则节点（v3: 独立 executor） ──
        register!(executors, "regex_extract", crate::nodes::regex::RegexExtractNode);
        register!(executors, "regex_replace", crate::nodes::regex::RegexReplaceNode);
        register!(executors, "regex_match", crate::nodes::regex::RegexMatchNode);

        // ── 数组节点（v3: 独立 executor） ──
        register!(executors, "array_filter", crate::nodes::array::ArrayFilterNode);
        register!(executors, "array_sort", crate::nodes::array::ArraySortNode);
        register!(executors, "array_dedup", crate::nodes::array::ArrayDedupNode);
        register!(executors, "array_paginate", crate::nodes::array::ArrayPaginateNode);
        register!(executors, "array_map", crate::nodes::array::ArrayMapNode);
        register!(executors, "array_join", crate::nodes::array::ArrayJoinNode);
        register!(executors, "array_reduce", crate::nodes::array::ArrayReduceNode);

        // ── 转换节点（v3: 独立 executor） ──
        register!(executors, "convert_to_text", crate::nodes::convert::ConvertToTextNode);
        register!(executors, "convert_to_number", crate::nodes::convert::ConvertToNumberNode);
        register!(executors, "convert_to_json", crate::nodes::convert::ConvertToJsonNode);
        register!(executors, "convert_to_csv", crate::nodes::convert::ConvertToCsvNode);
        register!(executors, "convert_to_html", crate::nodes::convert::ConvertToHtmlNode);
        register!(executors, "convert_to_base64", crate::nodes::convert::ConvertToBase64Node);

        // ── 新增数据节点 ──
        register!(executors, "json_parse", crate::nodes::json_parse::JsonParseNode);
        register!(executors, "text_template", crate::nodes::text_template::TextTemplateNode);

        // ── 容器节点（由 register_containers! 统一注册，与 parser::CONTAINER_TYPES 对应）──
        register_containers!(executors,
            "browser" => crate::nodes::browser_container::BrowserContainerNode,
            "excel"   => crate::nodes::excel_container::ExcelContainerNode,
            "word"    => crate::nodes::word_container::WordContainerNode,
            "logic"   => crate::nodes::condition::ConditionNode,
            "file"    => crate::nodes::file_container::FileContainerNode,
        );
        register!(executors, "condition", crate::nodes::condition::ConditionNode);

        // ── 其他节点 ──
        register!(executors, "notify", crate::nodes::notify::NotifyNode);
        register!(executors, "approval", crate::nodes::approval::ApprovalNode);
        register!(executors, "loop", crate::nodes::loop_node::LoopNode);
        register!(executors, "while", crate::nodes::while_node::WhileNode);
        register!(executors, "cursor", crate::nodes::cursor::CursorNode);
        register!(executors, "parallel", crate::nodes::parallel::ParallelNode);
        register!(executors, "map", crate::nodes::map::MapNode);
        register!(executors, "web_scrape", crate::nodes::web_scrape::WebScrapeNode);
        register!(executors, "mouse_keyboard", crate::nodes::mouse_keyboard::MouseKeyboardNode);
        register!(executors, "window", crate::nodes::window::WindowNode);
        register!(executors, "sub_workflow", crate::nodes::sub_workflow::SubWorkflowNode);
        register!(executors, "delay", crate::nodes::delay::DelayNode);
        register!(executors, "ocr", crate::nodes::ocr::OcrNode);
        register!(executors, "recording", crate::nodes::recording::RecordingNode);
        register!(executors, "print", crate::nodes::print::PrintNode);
        register!(executors, "shell", crate::nodes::shell::ShellNode);

        for type_name in executors.keys() {
            debug!("节点注册: {}", type_name);
        }

        Arc::new(StepExecutor { executors, approval_store, db })
    }

    /// 执行一个步骤（统一入口，所有节点类型通过 trait 分发）
    pub fn execute<'a>(
        self: &'a Arc<Self>,
        step: &'a Step,
        ctx: &'a mut ExecutionContext,
    ) -> Pin<Box<dyn Future<Output = Result<Value>> + Send + 'a>> {
        let executor = match self.executors.get(&step.step_type) {
            Some(e) => {
                info!("执行节点: {} (类型: {})", step.name, step.step_type);
                e
            }
            None => {
                return Box::pin(async move {
                    Err(anyhow::anyhow!("未知节点类型: {}", step.step_type))
                });
            }
        };

        let resolved_config = ctx.resolve_config(&step.config);
        let resolved_step = Step {
            id: step.id.clone(),
            name: step.name.clone(),
            step_type: step.step_type.clone(),
            config: resolved_config,
            next: step.next.clone(),
            retry: step.retry.clone(),
            timeout: step.timeout,
            body_steps: step.body_steps.clone(),
            breakpoint: step.breakpoint,
            delay: step.delay,
            on_error: step.on_error.clone(),
            actions: step.actions.clone(),
            expanded: step.expanded,
            condition: step.condition.clone(),
            condition_group: step.condition_group.clone(),

            run_condition: None,
        };

        Box::pin(async move {
            executor.execute(&resolved_step, ctx, self).await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::parser;
    use crate::engine::context::ExecutionContext;
    use crate::engine::workflow::{Workflow, Step};
    use std::sync::Arc;

    fn test_db() -> Arc<crate::data::db::Database> {
        Arc::new(crate::data::db::Database::open_default().expect("open db"))
    }

    fn test_approval_store() -> Arc<crate::engine::approval_store::ApprovalStore> {
        Arc::new(crate::engine::approval_store::ApprovalStore::new())
    }

    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        tokio::runtime::Runtime::new().unwrap().block_on(f)
    }

    #[test]
    fn container_types_match_registrations() {
        let mut executors: HashMap<String, Box<dyn NodeExecutor>> = HashMap::new();
        register_containers!(executors,
            "browser" => crate::nodes::browser_container::BrowserContainerNode,
            "excel"   => crate::nodes::excel_container::ExcelContainerNode,
            "word"    => crate::nodes::word_container::WordContainerNode,
            "logic"   => crate::nodes::condition::ConditionNode,
            "file"    => crate::nodes::file_container::FileContainerNode,
        );

        for &container_type in parser::CONTAINER_TYPES {
            let expected_name = format!("{}_container", container_type);
            assert!(
                executors.contains_key(&expected_name),
                "parser::CONTAINER_TYPES 包含 '{}'，但 executor 未注册 '{}'",
                container_type, expected_name
            );
        }

        let registered_types: Vec<String> = executors.keys().cloned().collect();
        for name in &registered_types {
            let base = name.trim_end_matches("_container");
            assert!(
                parser::CONTAINER_TYPES.contains(&base),
                "executor 注册了 '{}'，但 parser::CONTAINER_TYPES 中未声明",
                name
            );
        }
    }

    #[test]
    fn all_registered_types_can_be_instantiated() {
        let exec = StepExecutor::new(test_approval_store(), test_db());
        // 验证至少注册了核心类型
        let required = &[
            "http", "script", "condition", "shell", "notify", "delay",
            "loop", "while", "cursor", "parallel", "map",
            "browser_container", "excel_container", "word_container", "file_container", "logic_container",
            "data_set", "data_get", "data_length", "data_default", "data_merge",
            "clipboard", "approval",
        ];
        for &t in required {
            assert!(exec.executors.contains_key(t), "缺少注册: {}", t);
        }
    }

    #[test]
    fn unknown_node_type_rejected() {
        let exec = StepExecutor::new(test_approval_store(), test_db());
        let step = Step {
            id: "s1".into(),
            step_type: "nonexistent_type_xyz".into(),
            config: serde_json::json!({}),
            ..Default::default()
        };
        let mut ctx = ExecutionContext::new("run-1", &Workflow::default());
        let result = block_on(exec.execute(&step, &mut ctx));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("未知节点类型"));
    }

    #[test]
    fn simple_node_config_is_resolved_before_execution() {
        let exec = StepExecutor::new(test_approval_store(), test_db());
        let mut workflow = Workflow::default();
        workflow.steps = vec![Step {
            id: "step_x".into(),
            step_type: "data_set".into(),
            config: serde_json::json!({"key": "greeting", "value": "hello"}),
            ..Default::default()
        }];
        let step = workflow.steps[0].clone();
        let mut ctx = ExecutionContext::new("run-1", &workflow);
        // data_set should set a variable
        let result = block_on(exec.execute(&step, &mut ctx));
        assert!(result.is_ok());
        assert_eq!(ctx.variables.get("greeting").and_then(|v| v.as_str()), Some("hello"));
    }

    #[test]
    fn variable_resolution_in_config_works() {
        let exec = StepExecutor::new(test_approval_store(), test_db());
        let mut workflow = Workflow::default();
        workflow.variables = Some({
            let mut m = std::collections::HashMap::new();
            m.insert("name".into(), serde_json::json!("world"));
            m
        });
        workflow.steps = vec![Step {
            id: "s1".into(),
            step_type: "data_set".into(),
            config: serde_json::json!({"key": "msg", "value": "{{name}}"}),
            ..Default::default()
        }];
        let step = workflow.steps[0].clone();
        let mut ctx = ExecutionContext::new("run-1", &workflow);
        let result = block_on(exec.execute(&step, &mut ctx));
        assert!(result.is_ok());
        assert_eq!(ctx.variables.get("msg").and_then(|v| v.as_str()), Some("world"));
    }

    #[test]
    fn condition_node_returns_branch_result() {
        let exec = StepExecutor::new(test_approval_store(), test_db());
        let step = Step {
            id: "c1".into(),
            step_type: "condition".into(),
            config: serde_json::json!({
                "condition_group": {
                    "combinator": "and",
                    "conditions": [
                        {"id": "c1", "left": "2", "op": "greater_than", "right": "1"}
                    ]
                }
            }),
            ..Default::default()
        };
        let mut ctx = ExecutionContext::new("run-1", &Workflow::default());
        let result = block_on(exec.execute(&step, &mut ctx));
        assert!(result.is_ok(), "condition execution failed: {:?}", result.err());
        let output = result.unwrap();
        assert_eq!(output.get("branch").and_then(|v| v.as_str()), Some("true"));
        assert_eq!(output.get("result").and_then(|v| v.as_bool()), Some(true));
    }
}
