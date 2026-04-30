// engine/executor.rs — 步骤执行器
//
// 节点注册方式：
//   所有节点通过 register() 显式注册，对应 nodes/registry.rs 中的清单。
//   新增节点时在此添加一行 register() 调用即可。

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

        // ── 通过 registry 清单对应注册 ──
        // P0 核心节点
        register!(executors, "http", crate::nodes::http::HttpNode);
        register!(executors, "data", crate::nodes::data::DataNode);
        register!(executors, "script", crate::nodes::script::ScriptNode);
        register!(executors, "condition", crate::nodes::condition::ConditionNode);

        // P2 文件节点
        register!(executors, "excel", crate::nodes::excel::ExcelNode);
        register!(executors, "word", crate::nodes::word::WordNode);

        // P2.5+ 新节点
        register!(executors, "browser", crate::nodes::browser::BrowserNode);
        register!(executors, "notify", crate::nodes::notify::NotifyNode);
        register!(executors, "approval", crate::nodes::approval::ApprovalNode);

        // 循环/并行节点
        register!(executors, "loop", crate::nodes::loop_node::LoopNode);
        register!(executors, "while", crate::nodes::while_node::WhileNode);
        register!(executors, "parallel", crate::nodes::parallel::ParallelNode);

        // v0.7.0 声明式数据节点
        register!(executors, "map", crate::nodes::map::MapNode);

        // v1.1 声明式网页抓取节点
        register!(executors, "web_scrape", crate::nodes::web_scrape::WebScrapeNode);

        // v2.0 AI 节点

        // DAG 执行引擎节点
        register!(executors, "mouse_keyboard", crate::nodes::mouse_keyboard::MouseKeyboardNode);
        register!(executors, "window", crate::nodes::window::WindowNode);
        register!(executors, "sub_workflow", crate::nodes::sub_workflow::SubWorkflowNode);
        register!(executors, "delay", crate::nodes::delay::DelayNode);
        register!(executors, "ocr", crate::nodes::ocr::OcrNode);
        register!(executors, "recording", crate::nodes::recording::RecordingNode);

        // v2 通用节点
        register!(executors, "file", crate::nodes::file::FileNode);
        register!(executors, "clipboard", crate::nodes::clipboard::ClipboardNode);
        register!(executors, "regex", crate::nodes::regex::RegexNode);
        register!(executors, "array", crate::nodes::array::ArrayNode);
        register!(executors, "convert", crate::nodes::convert::ConvertNode);
        register!(executors, "print", crate::nodes::print::PrintNode);

        // 记录已注册的节点类型
        for type_name in executors.keys() {
            debug!("节点注册: {}", type_name);
        }

        Arc::new(StepExecutor { executors })
    }

    /// 执行一个步骤（统一入口，所有节点类型通过 trait 分发）
    ///
    /// 优化：先 resolve config（变量替换），避免克隆整个 config HashMap 后再丢弃。
    /// 只克隆轻量的非 config 字段（id, name 等字符串），config 直接使用解析结果。
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

        // Resolve config from reference first — avoids cloning the large config HashMap,
        // only to immediately discard it. The resolved config IS the final result.
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
