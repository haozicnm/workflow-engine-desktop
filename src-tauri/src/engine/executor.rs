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
}

macro_rules! register {
    ($map:expr, $key:literal, $ctor:expr) => {
        $map.insert($key.to_string(), Box::new($ctor));
    };
}

impl StepExecutor {
    pub fn new() -> Arc<Self> {
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
        register!(executors, "file_read", crate::nodes::file::FileReadNode);
        register!(executors, "file_write", crate::nodes::file::FileWriteNode);
        register!(executors, "file_list", crate::nodes::file::FileListNode);
        register!(executors, "file_delete", crate::nodes::file::FileDeleteNode);
        register!(executors, "file_exists", crate::nodes::file::FileExistsNode);
        register!(executors, "file_save", crate::nodes::file_save::FileSaveNode);

        // ── 剪贴板节点（v3: 独立 executor） ──
        register!(executors, "clipboard_read", crate::nodes::clipboard::ClipboardReadNode);
        register!(executors, "clipboard_write", crate::nodes::clipboard::ClipboardWriteNode);

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

        // ── P2 文件节点 ──
        register!(executors, "excel", crate::nodes::excel::ExcelNode);
        register!(executors, "excel_read", crate::nodes::excel::ExcelReadNode);
        register!(executors, "excel_write", crate::nodes::excel::ExcelWriteNode);
        register!(executors, "excel_create", crate::nodes::excel::ExcelCreateNode);
        register!(executors, "excel_filter", crate::nodes::excel::ExcelFilterNode);
        register!(executors, "excel_sort", crate::nodes::excel::ExcelSortNode);
        register!(executors, "excel_append", crate::nodes::excel::ExcelAppendNode);
        register!(executors, "excel_csv", crate::nodes::excel::ExcelCsvNode);
        register!(executors, "word", crate::nodes::word::WordNode);
        register!(executors, "word_read", crate::nodes::word::WordReadNode);
        register!(executors, "word_write", crate::nodes::word::WordWriteNode);
        register!(executors, "word_create", crate::nodes::word::WordCreateNode);
        register!(executors, "word_replace", crate::nodes::word::WordReplaceNode);
        register!(executors, "word_merge", crate::nodes::word::WordMergeNode);

        // ── 浏览器节点 ──
        register!(executors, "browser", crate::nodes::browser::BrowserNode);
        register!(executors, "browser_navigate", crate::nodes::browser::BrowserNavigateNode);
        register!(executors, "browser_click", crate::nodes::browser::BrowserClickNode);
        register!(executors, "browser_fill", crate::nodes::browser::BrowserFillNode);
        register!(executors, "browser_extract", crate::nodes::browser::BrowserExtractNode);
        register!(executors, "browser_screenshot", crate::nodes::browser::BrowserScreenshotNode);
        register!(executors, "browser_evaluate", crate::nodes::browser::BrowserEvaluateNode);
        register!(executors, "browser_scroll", crate::nodes::browser::BrowserScrollNode);
        register!(executors, "browser_wait", crate::nodes::browser::BrowserWaitNode);
        register!(executors, "browser_pdf", crate::nodes::browser::BrowserPdfNode);
        register!(executors, "browser_container", crate::nodes::browser_container::BrowserContainerNode);
        register!(executors, "word_container", crate::nodes::word_container::WordContainerNode);
        register!(executors, "excel_container", crate::nodes::excel_container::ExcelContainerNode);
        register!(executors, "logic_container", crate::nodes::condition::ConditionNode);
        register!(executors, "condition", crate::nodes::condition::ConditionNode);  // 向后兼容旧格式

        // ── 其他节点 ──
        register!(executors, "notify", crate::nodes::notify::NotifyNode);
        register!(executors, "approval", crate::nodes::approval::ApprovalNode);
        register!(executors, "loop", crate::nodes::loop_node::LoopNode);
        register!(executors, "while", crate::nodes::while_node::WhileNode);
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

        for type_name in executors.keys() {
            debug!("节点注册: {}", type_name);
        }

        Arc::new(StepExecutor { executors })
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
        };

        Box::pin(async move {
            executor.execute(&resolved_step, ctx, self).await
        })
    }
}
