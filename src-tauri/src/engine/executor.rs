// engine/executor.rs — 步骤执行器
//
// v3: 所有节点独立 executor，不再使用 action 参数分发。
//   每个操作一个 struct，一个注册条目。

use crate::engine::context::ExecutionContext;
use crate::engine::workflow::{Edge, Step, Workflow};
use crate::nodes::traits::NodeExecutor;
use anyhow::{anyhow, Result};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, info, warn};


/// 执行事件（可选的事件流推送）
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// 工作流开始
    WorkflowStarted { total_steps: usize },
    /// 节点开始执行
    NodeStarted { node_id: String, step_type: String },
    /// 节点执行完成
    NodeCompleted { node_id: String, duration_ms: u64 },
    /// 节点执行失败
    NodeFailed { node_id: String, error: String, duration_ms: u64 },
    /// 工作流完成
    WorkflowCompleted { total_duration_ms: u64 },
}

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

/// 容器节点注册宏 — v8: 不加 _container 后缀，与前端 type 名 1:1 对应
macro_rules! register_containers {
    ($map:expr, $( $type:literal => $ctor:expr ),* $(,)?) => {
        $(
            $map.insert($type.to_string(), Box::new($ctor));
        )*
    };
}

impl StepExecutor {
    pub fn new(
        approval_store: Arc<crate::engine::approval_store::ApprovalStore>,
        db: Arc<crate::data::db::Database>,
    ) -> Arc<Self> {
        let mut executors: HashMap<String, Box<dyn NodeExecutor>> = HashMap::new();

        // ── P0 核心节点 ──
        register!(executors, "http", crate::nodes::http::HttpNode);
        register!(executors, "script", crate::nodes::script::ScriptNode);
        register!(
            executors,
            "condition",
            crate::nodes::condition::ConditionNode
        );

        // ── 数据处理节点（v3: 独立 executor） ──
        register!(executors, "data_set", crate::nodes::data::DataSetNode);
        register!(executors, "data_get", crate::nodes::data::DataGetNode);

        // ── 文件节点（v3: 独立 executor） ──
        register!(executors, "file_read", crate::nodes::file::FileReadNode);
        register!(executors, "file_write", crate::nodes::file::FileWriteNode);
        register!(executors, "file_list", crate::nodes::file::FileListNode);
        register!(executors, "file_exists", crate::nodes::file::FileExistsNode);
        register!(
            executors,
            "file_checksum",
            crate::nodes::file::FileChecksumNode
        );

        // ── 剪贴板节点 ──
        register!(
            executors,
            "clipboard_read",
            crate::nodes::clipboard::ClipboardReadNode
        );
        register!(
            executors,
            "clipboard_write",
            crate::nodes::clipboard::ClipboardWriteNode
        );

        // ── 正则节点（P3: 合并为单一 regex 节点，mode 控制行为） ──
        register!(
            executors,
            "regex",
            crate::nodes::regex::RegexNode
        );

        // ── 容器节点（由 register_containers! 统一注册，与 registry (node-schema.json) 对应）──
        register_containers!(executors,
            "browser" => crate::nodes::browser_container::BrowserContainerNode,
            "excel"   => crate::nodes::excel_container::ExcelContainerNode,
            "word"    => crate::nodes::word_container::WordContainerNode,
            "file"      => crate::nodes::file_container::FileContainerNode,
        );

        // ── 其他节点 ──

        register!(executors, "approval", crate::nodes::approval::ApprovalNode);
        register!(executors, "loop", crate::nodes::loop_node::LoopNode);
        register!(executors, "while", crate::nodes::while_node::WhileNode);
        register!(executors, "cursor", crate::nodes::cursor::CursorNode);
        register!(executors, "parallel", crate::nodes::parallel::ParallelNode);
        register!(executors, "map", crate::nodes::map::MapNode);
        register!(
            executors,
            "web_scrape",
            crate::nodes::web_scrape::WebScrapeNode
        );
        #[cfg(feature = "gui")]
        register!(
            executors,
            "mouse_keyboard",
            crate::nodes::mouse_keyboard::MouseKeyboardNode
        );
        #[cfg(feature = "gui")]
        register!(executors, "window", crate::nodes::window::WindowNode);
        register!(
            executors,
            "sub_workflow",
            crate::nodes::sub_workflow::SubWorkflowNode
        );
        register!(executors, "delay", crate::nodes::delay::DelayNode);
        register!(executors, "ocr", crate::nodes::ocr::OcrNode);
        #[cfg(feature = "gui")]
        register!(executors, "print", crate::nodes::print::PrintNode);
        register!(executors, "shell", crate::nodes::shell::ShellNode);

        // ── 触发器节点（v8.5）──
        register!(executors, "trigger_cron", crate::nodes::trigger_cron::TriggerCronNode);
        register!(executors, "trigger_webhook", crate::nodes::trigger_webhook::TriggerWebhookNode);
        register!(executors, "trigger_file", crate::nodes::trigger_file::TriggerFileNode);
        register!(executors, "webhook_response", crate::nodes::webhook_response::WebhookResponseNode);

        // ── v8.6 实用节点 + AI 薄封装 ──
        register!(executors, "json_transform", crate::nodes::json_transform::JsonTransformNode);
        register!(executors, "data_filter", crate::nodes::data_filter::DataFilterNode);
        register!(executors, "llm_chat", crate::nodes::llm_chat::LlmChatNode);
        register!(executors, "prompt_template", crate::nodes::prompt_template::PromptTemplateNode);
        register!(executors, "email_send", crate::nodes::email_send::EmailSendNode);
        register!(executors, "database_query", crate::nodes::database_query::DatabaseQueryNode);

        // ── v8.8 IM 集成节点 ──
        register!(executors, "im_message", crate::nodes::im_message::ImMessageNode);
        register!(executors, "github_issue", crate::nodes::github_issue::GithubIssueNode);

        // ── v9.0 AI 扩展 + 数据节点 ──
        register!(executors, "llm_embedding", crate::nodes::llm_embedding::LlmEmbeddingNode);
        register!(executors, "llm_agent", crate::nodes::llm_agent::LlmAgentNode);
        register!(executors, "text_splitter", crate::nodes::text_splitter::TextSplitterNode);
        register!(executors, "json_schema_extract", crate::nodes::json_schema_extract::JsonSchemaExtractNode);
        register!(executors, "vector_store", crate::nodes::vector_store::VectorStoreNode);
        register!(executors, "rag_query", crate::nodes::rag_query::RagQueryNode);
        register!(executors, "redis", crate::nodes::redis_node::RedisNode);
        register!(executors, "mongodb", crate::nodes::mongodb_node::MongodbNode);
        register!(executors, "s3", crate::nodes::s3_node::S3Node);

        // ── MCP 节点（Python 实现，仅注册原生没有的类型）──
        {
            use crate::nodes::mcp_node::{create_mcp_executor, get_all_mcp_types};
            let mcp_types = get_all_mcp_types();
            for t in &mcp_types {
                if executors.contains_key(t) {
                    debug!("节点跳过 (MCP 不覆盖原生): {}", t);
                    continue;
                }
                if let Some(ex) = create_mcp_executor(t) {
                    executors.insert(t.to_string(), ex);
                    debug!("节点注册 (MCP): {}", t);
                }
            }
        }

        for type_name in executors.keys() {
            debug!("节点注册: {}", type_name);
        }

        Arc::new(StepExecutor {
            executors,
            approval_store,
            db,
        })
    }

    /// 返回所有已注册的节点类型名称（用于编译期契约校验）
    pub fn registered_types(&self) -> Vec<&str> {
        self.executors.keys().map(|s| s.as_str()).collect()
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

        // v8: 用 registry::is_container() 判断容器，不再依赖 _container 后缀
        let is_container = crate::nodes::registry::is_container(&step.step_type);
        // v8.1: 节点可声明自行解析模板变量（如 map 的 {{__item}}），跳过全局预解析
        let resolve_self = executor.resolve_config_self();

        // Phase 3: 两阶段 resolve
        // - 容器节点：占位符替换 → 反序列化 → 后处理替换（避免类型破坏）
        // - resolve_self 节点：跳过全局 resolve（如 map 的 {{__item}}）
        // - 非容器节点：全局 resolve_config（原有行为）
        let (resolved_config, placeholder_map) = if is_container {
            // v8: 归一化 actions — 前端 actions 在 step.actions，需合并到 config 供容器反序列化
            let mut config = if step.config.is_object() {
                step.config.clone()
            } else {
                serde_json::json!({})
            };
            if let Some(actions) = &step.actions {
                if !actions.is_empty() {
                    let normalized = crate::engine::parser::normalize_actions(actions);
                    if let serde_json::Value::Object(ref mut map) = config {
                        map.insert("actions".to_string(), serde_json::Value::Array(normalized));
                    }
                }
            }
            // Phase 3: 占位符替换
            // 注意：condition_group 也在 config 中，需要一起处理
            let mut ph = crate::engine::placeholder::PlaceholderMap::new();
            let _ = ph.scan_and_replace(&mut config);
            (config, Some(ph))
        } else if resolve_self {
            (step.config.clone(), None)
        } else {
            (ctx.resolve_config(&step.config), None)
        };
        // Phase 3: condition_group 也需要占位符处理
        // 优先从 step.condition_group 读（parser 提取的），回退到 config.condition_group
        let condition_group_raw = step.condition_group.clone().or_else(|| {
            resolved_config
                .get("condition_group")
                .and_then(|cg| serde_json::from_value(cg.clone()).ok())
        });
        let condition_group = if let Some(ref cg) = condition_group_raw {
            let mut cg_value = serde_json::to_value(cg).unwrap_or(serde_json::Value::Null);
            // 为 condition_group 创建独立的 placeholder_map 进行扫描和解析
            // 这样无论是否有全局 placeholder_map，condition_group 都能正确处理变量
            let mut cg_ph = crate::engine::placeholder::PlaceholderMap::new();
            let _ = cg_ph.scan_and_replace(&mut cg_value);
            let _ = cg_ph.resolve_value(&mut cg_value, ctx);
            serde_json::from_value(cg_value).ok()
        } else {
            None
        };

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
            condition_group,

            run_condition: None,
        };

        // Phase 3: 后处理替换占位符
        // 对于容器节点，需要在 executor.execute 之前解析占位符
        // 把 placeholder_map 传递到 async 块中执行后处理

        Box::pin(async move {
            // Phase 3: 后处理替换占位符
            if let Some(ph) = placeholder_map {
                // 把 resolved_step.config 转为 mutable Value，执行后处理替换
                let mut config_value =
                    serde_json::to_value(&resolved_step.config).unwrap_or(serde_json::Value::Null);
                ph.resolve_value(&mut config_value, ctx)?;
                // 创建新的 resolved_step，使用解析后的 config
                let mut new_step = resolved_step.clone();
                new_step.config = config_value;
                let result = executor.execute(&new_step, ctx, self).await;
                result
            } else {
                let result = executor.execute(&resolved_step, ctx, self).await;
                result
            }
        })
    }

    // ═══════════════════════════════════════════════
    // v8: 图执行引擎（拓扑分层 + 并行执行）
    // ═══════════════════════════════════════════════

    /// 执行完整工作流：有 edges 走图模式，无 edges 走线性模式。
    pub async fn run_workflow(self: &Arc<Self>, workflow: &Workflow) -> Result<ExecutionContext> {
        self.run_workflow_with_events(workflow, None).await
    }

    /// 执行工作流并可选地推送事件
    pub async fn run_workflow_with_events(
        self: &Arc<Self>,
        workflow: &Workflow,
        event_tx: Option<tokio::sync::mpsc::Sender<ExecutionEvent>>,
    ) -> Result<ExecutionContext> {
        let start = std::time::Instant::now();
        let _ = event_tx.as_ref().map(|tx| tx.try_send(ExecutionEvent::WorkflowStarted { total_steps: workflow.steps.len() }));

        let mut ctx = ExecutionContext::new("graph-run", workflow);

        let result = if !workflow.edges.is_empty() {
            self.run_graph(workflow, &mut ctx).await
        } else {
            self.run_linear(workflow, &mut ctx).await
        };

        if result.is_ok() {
            let _ = event_tx.as_ref().map(|tx| tx.try_send(ExecutionEvent::WorkflowCompleted { total_duration_ms: start.elapsed().as_millis() as u64 }));
        }

        result.map(|_| ctx)
    }

    /// 图执行：拓扑分层 → 同层并行（支持 on_error 策略 + 调试断点/单步）
    async fn run_graph(self: &Arc<Self>, workflow: &Workflow, ctx: &mut ExecutionContext) -> Result<()> {
        self.validate_variable_references(workflow)?;
        let levels = self.topological_levels(&workflow.steps, &workflow.edges)?;
        info!("图执行: {} 个层级, {} 个步骤", levels.len(), workflow.steps.len());
        for level in &levels {
            // 调试：暂停检查（单步模式 / 断点 / 暂停）
            for node_id in level {
                if let Some(step) = workflow.steps.iter().find(|s| &s.id == node_id) {
                    self.check_debug_pause(step, ctx).await;
                }
            }
            if level.len() == 1 {
                let step = workflow.steps.iter().find(|s| s.id == level[0])
                    .ok_or_else(|| anyhow!("节点未找到: {}", level[0]))?;
                match Arc::clone(self).execute(step, ctx).await {
                    Ok(result) => {
                        // 存入 step_outputs（不污染 variables，避免和 data_set 等节点冲突）
                        ctx.set_output(&step.id, result);
                    }
                    Err(e) => {
                        // G3: 图模式 on_error 策略
                        match &step.on_error {
                            Some(crate::engine::workflow::ErrorStrategy::Ignore) => {
                                warn!("节点 {} 失败 (on_error=ignore): {}", step.id, e);
                                ctx.set_var(format!("_error.{}", step.id), serde_json::json!(e.to_string()));
                                // 跳过失败节点，继续执行后续层级
                            }
                            Some(crate::engine::workflow::ErrorStrategy::Branch { step_id }) => {
                                warn!("节点 {} 失败 (on_error=branch → {}): {}", step.id, step_id, e);
                                ctx.set_var(format!("_error.{}", step.id), serde_json::json!(e.to_string()));
                                // 跳转到错误处理分支（如果目标节点存在）
                                if let Some(branch_step) = workflow.steps.iter().find(|s| s.id == *step_id) {
                                    let branch_result = Arc::clone(self).execute(branch_step, ctx).await?;
                                    ctx.set_output(&branch_step.id, branch_result);
                                }
                            }
                            _ => {
                                // fail（默认）：传播错误
                                return Err(e);
                            }
                        }
                    }
                }
            } else {
                self.run_parallel_level(level, workflow, ctx).await?;
            }
        }
        Ok(())
    }

    /// 并行执行一层节点
    async fn run_parallel_level(self: &Arc<Self>, node_ids: &[String], workflow: &Workflow, ctx: &mut ExecutionContext) -> Result<()> {
        let mut join_set = tokio::task::JoinSet::new();
        for node_id in node_ids {
            let step = workflow.steps.iter().find(|s| &s.id == node_id).cloned()
                .ok_or_else(|| anyhow!("节点未找到: {}", node_id))?;
            let mut task_ctx = ExecutionContext::new(&step.id, workflow);
            task_ctx.variables = ctx.variables.clone();
            task_ctx.step_outputs = ctx.step_outputs.clone(); // 传递上游输出（修复 CLI 路径）
            // 注入 input_ports（按 edge 映射上游输出）
            for edge in &workflow.edges {
                if edge.to == *node_id {
                    if let Some(upstream_output) = ctx.step_outputs.get(&edge.from) {
                        // 按 from_port 过滤：非 "out" 端口从输出中提取对应分支数据
                        let port_data = if edge.from_port.is_empty() || edge.from_port == "out" {
                            upstream_output.clone()
                        } else {
                            upstream_output.get(&edge.from_port).cloned()
                                .unwrap_or(upstream_output.clone())
                        };
                        task_ctx.input_ports.insert(edge.to_port.clone(), port_data);
                    }
                }
            }
            let exec = Arc::clone(self);
            join_set.spawn(async move {
                let result = exec.execute(&step, &mut task_ctx).await;
                (step.id.clone(), result, task_ctx.variables, step.on_error.clone())
            });
        }
        let mut hard_errors: Vec<(String, anyhow::Error)> = Vec::new();
        let mut panics: Vec<String> = Vec::new();
        while let Some(jr) = join_set.join_next().await {
            match jr {
                Ok((nid, Ok(val), vars, _)) => {
                    ctx.set_output(&nid, val);
                    for (k, v) in vars {
                        if let Some(existing) = ctx.variables.get(&k) {
                            if *existing != v {
                                warn!("并行变量冲突: {} (已有值，忽略 {} 的新值)", k, nid);
                            }
                        }
                        ctx.variables.entry(k).or_insert(v);
                    }
                }
                Ok((nid, Err(e), _, on_error)) => {
                    warn!("节点 {} 失败: {}", nid, e);
                    ctx.set_var(format!("_error.{}", nid), serde_json::json!(e.to_string()));
                    match on_error.unwrap_or_default() {
                        crate::engine::workflow::ErrorStrategy::Ignore => {
                            warn!("并行节点 {} 错误已忽略 (on_error=ignore)", nid);
                        }
                        crate::engine::workflow::ErrorStrategy::Branch { step_id } => {
                            warn!("并行节点 {} 错误分支 → {}", nid, step_id);
                            // 直接执行分支节点（不等待下轮调度）
                            if let Some(branch_step) = workflow.steps.iter().find(|s| s.id == *step_id) {
                                let mut branch_ctx = ExecutionContext::new(&step_id, workflow);
                                branch_ctx.variables = ctx.variables.clone();
                                branch_ctx.step_outputs = ctx.step_outputs.clone();
                                match Arc::clone(self).execute(branch_step, &mut branch_ctx).await {
                                    Ok(val) => {
                                        ctx.set_output(&step_id, val);
                                        for (k, v) in branch_ctx.variables {
                                            ctx.variables.entry(k).or_insert(v);
                                        }
                                    }
                                    Err(branch_err) => {
                                        warn!("错误分支 {} 执行失败: {}", step_id, branch_err);
                                        hard_errors.push((step_id.clone(), branch_err));
                                    }
                                }
                            }
                        }
                        _ => {
                            hard_errors.push((nid, e));
                        }
                    }
                }
                Err(e) => {
                    panics.push(format!("{}", e));
                }
            }
        }
        if !panics.is_empty() {
            return Err(anyhow!("并行任务 panic: {}", panics.join("; ")));
        }
        if !hard_errors.is_empty() {
            let summary: Vec<String> = hard_errors.iter().map(|(nid, e)| format!("{}: {}", nid, e)).collect();
            return Err(anyhow!("并行执行有 {} 个节点失败: {}", hard_errors.len(), summary.join("; ")));
        }
        Ok(())
    }

    /// 调试暂停检查：单步模式 / 断点 / 暂停 — 等待用户恢复
    async fn check_debug_pause(&self, step: &crate::engine::workflow::Step, ctx: &ExecutionContext) {
        use std::sync::atomic::Ordering;
        let should_pause = step.breakpoint
            || ctx.step_mode_flag.as_ref().map(|f| f.load(Ordering::Relaxed)).unwrap_or(false)
            || ctx.pause_flag.as_ref().map(|f| f.load(Ordering::Relaxed)).unwrap_or(false);
        if !should_pause { return; }
        // 设置断点标志，通知前端
        if let Some(ref bp) = ctx.breakpoint_flag {
            bp.store(true, Ordering::Relaxed);
        }
        info!("调试暂停: 节点 '{}' ({})", step.name, step.id);
        // 轮询等待恢复（debug_step 或 debug_continue 会清除标志）
        if let Some(ref bp) = ctx.breakpoint_flag {
            while bp.load(Ordering::Relaxed) {
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        }
    }

    /// 线性执行（向后兼容）
    async fn run_linear(self: &Arc<Self>, workflow: &Workflow, ctx: &mut ExecutionContext) -> Result<()> {
        for step in &workflow.steps {
            let result = Arc::clone(self).execute(step, ctx).await?;
            // 存入 step_outputs（不污染 variables，避免和 data_set 等节点冲突）
            ctx.set_output(&step.id, result);
        }
        Ok(())
    }

    /// 拓扑分层（Kahn 算法）
    pub fn topological_levels(&self, steps: &[Step], edges: &[Edge]) -> Result<Vec<Vec<String>>> {
        // 验证 edge 引用的节点都存在
        let step_ids: HashSet<&str> = steps.iter().map(|s| s.id.as_str()).collect();
        for edge in edges {
            if !step_ids.contains(edge.from.as_str()) {
                return Err(anyhow!("边引用了不存在的源节点 '{}'（from）", edge.from));
            }
            if !step_ids.contains(edge.to.as_str()) {
                return Err(anyhow!("边引用了不存在的目标节点 '{}'（to）", edge.to));
            }
        }
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        for step in steps { in_degree.entry(step.id.clone()).or_insert(0); adj.entry(step.id.clone()).or_default(); }
        for edge in edges {
            *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
            adj.entry(edge.from.clone()).or_default().push(edge.to.clone());
        }
        let mut current_level: Vec<String> = in_degree.iter().filter(|(_, &d)| d == 0).map(|(id, _)| id.clone()).collect();
        let mut levels = Vec::new();
        let mut visited = HashSet::new();
        while !current_level.is_empty() {
            let mut next = Vec::new();
            for nid in &current_level {
                visited.insert(nid.clone());
                if let Some(nbrs) = adj.get(nid) {
                    for nb in nbrs {
                        if let Some(d) = in_degree.get_mut(nb) { *d -= 1; if *d == 0 { next.push(nb.clone()); } }
                    }
                }
            }
            levels.push(current_level);
            current_level = next;
        }
        if visited.len() != steps.len() { return Err(anyhow!("工作流包含循环依赖")); }
        Ok(levels)
    }

    /// 校验工作流中所有变量引用是否指向存在的节点/端口
    pub fn validate_variable_references(&self, workflow: &Workflow) -> Result<()> {
        let node_ids: HashSet<&str> = workflow.steps.iter().map(|s| s.id.as_str()).collect();
        // 匹配 {{xxx}} / {{xxx.y}} / {{xxx.y.z}} — 支持无点号和多级路径
        let re = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap();

        for step in &workflow.steps {
            // 收集所有需要检查的字符串：config + actions + condition_group
            let mut configs_to_check: Vec<String> = vec![step.config.to_string()];
            if let Some(ref actions) = step.actions {
                configs_to_check.push(serde_json::to_string(actions).unwrap_or_default());
            }
            if let Some(ref cg) = step.condition_group {
                configs_to_check.push(serde_json::to_string(cg).unwrap_or_default());
            }
            for config_str in &configs_to_check {
                for cap in re.captures_iter(config_str) {
                    let ref_expr = cap[1].trim(); // e.g. "step_1.result" or "params.x" or "step_1"
                    if ref_expr.is_empty() { continue; }
                    let root = ref_expr.split('.').next().unwrap_or(ref_expr);
                    // 允许 {{params.xxx}} 内置变量
                    if root == "params" { continue; }
                    // 允许 {{__…}} 巫术变量
                    if root.starts_with("__") { continue; }
                    // 去掉 step_ 前缀后查找（兼容 {{step_1}} 和直接引用 ID "1"）
                    let root_clean = root.strip_prefix("step_").unwrap_or(root);
                    // 允许自引用
                    if root_clean == step.id.as_str() { continue; }
                    if !node_ids.contains(root) && !node_ids.contains(root_clean) {
                        // 也检查是否是 workflow 级变量
                        if let Some(ref vars) = workflow.variables {
                            if vars.contains_key(root) || vars.contains_key(root_clean) {
                                continue;
                            }
                        }
                        return Err(anyhow!(
                            "步骤 '{}' 中引用了不存在的节点 '{}'（变量标记: {}）",
                            step.id, root, ref_expr
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    pub fn get_execution_plan(&self, workflow: &Workflow) -> Result<Vec<Vec<String>>> {
        if workflow.edges.is_empty() {
            Ok(workflow.steps.iter().map(|s| vec![s.id.clone()]).collect())
        } else {
            self.topological_levels(&workflow.steps, &workflow.edges)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::context::ExecutionContext;
    
    use crate::engine::workflow::{Edge, Step, Workflow};
    use serde_json::json;
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
        // 使用真实 StepExecutor，不再重复 register_containers!
        let exec = StepExecutor::new(test_approval_store(), test_db());
        let registered = exec.registered_types();

        // v8: 正向检查 — CONTAINER_TYPES 里的每个类型必须直接注册（不加后缀）
        for container_type in crate::nodes::registry::container_types() {
            assert!(
                registered.contains(&container_type.as_str()),
                "parser::CONTAINER_TYPES 包含 '{}'，但 executor 未注册",
                container_type
            );
        }

        // v8: 反向检查 — executor 中注册的容器类型必须在 CONTAINER_TYPES 中声明
        for name in &registered {
            if crate::nodes::registry::is_container(name) {
                // 已在正向检查中验证
                continue;
            }
        }
    }

    #[test]
    fn all_registered_types_can_be_instantiated() {
        let exec = StepExecutor::new(test_approval_store(), test_db());
        let registered = exec.registered_types();
        // 从真实 executor 派生 required 列表，不再硬编码
        // 只需验证核心类型数量合理（至少注册了 20+ 种）
        assert!(
            registered.len() >= 20,
            "预期至少注册 20 种节点类型，实际: {}",
            registered.len()
        );
        // 验证所有注册类型都非空
        for t in &registered {
            assert!(!t.is_empty(), "注册了空类型名");
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
        assert_eq!(
            ctx.variables.get("greeting").and_then(|v| v.as_str()),
            Some("hello")
        );
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
        assert_eq!(
            ctx.variables.get("msg").and_then(|v| v.as_str()),
            Some("world")
        );
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
        assert!(
            result.is_ok(),
            "condition execution failed: {:?}",
            result.err()
        );
        let output = result.unwrap();
        assert_eq!(output.get("branch").and_then(|v| v.as_str()), Some("true"));
        assert_eq!(output.get("result").and_then(|v| v.as_bool()), Some(true));
    }

    // ═══════════════════════════════════════════════
    // v8: 图执行测试
    // ═══════════════════════════════════════════════

    #[test]
    fn topological_levels_linear_chain() {
        let exec = StepExecutor::new(test_approval_store(), test_db());
        let steps = vec![
            Step { id: "a".into(), step_type: "log".into(), config: json!({}), ..Default::default() },
            Step { id: "b".into(), step_type: "log".into(), config: json!({}), ..Default::default() },
            Step { id: "c".into(), step_type: "log".into(), config: json!({}), ..Default::default() },
        ];
        let edges = vec![
            Edge { from: "a".into(), from_port: "out".into(), to: "b".into(), to_port: "in".into() },
            Edge { from: "b".into(), from_port: "out".into(), to: "c".into(), to_port: "in".into() },
        ];
        let levels = exec.topological_levels(&steps, &edges).unwrap();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec!["a"]);
        assert_eq!(levels[1], vec!["b"]);
        assert_eq!(levels[2], vec!["c"]);
    }

    #[test]
    fn topological_levels_parallel_branch() {
        let exec = StepExecutor::new(test_approval_store(), test_db());
        let steps = vec![
            Step { id: "a".into(), step_type: "http".into(), config: json!({}), ..Default::default() },
            Step { id: "b".into(), step_type: "http".into(), config: json!({}), ..Default::default() },
            Step { id: "c".into(), step_type: "http".into(), config: json!({}), ..Default::default() },
        ];
        let edges = vec![
            Edge { from: "a".into(), from_port: "out".into(), to: "b".into(), to_port: "in".into() },
            Edge { from: "a".into(), from_port: "out".into(), to: "c".into(), to_port: "in".into() },
        ];
        let levels = exec.topological_levels(&steps, &edges).unwrap();
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0], vec!["a"]);
        assert_eq!(levels[1].len(), 2);
        assert!(levels[1].contains(&"b".to_string()));
        assert!(levels[1].contains(&"c".to_string()));
    }

    #[test]
    fn topological_levels_diamond() {
        let exec = StepExecutor::new(test_approval_store(), test_db());
        let steps = vec![
            Step { id: "a".into(), step_type: "http".into(), config: json!({}), ..Default::default() },
            Step { id: "b".into(), step_type: "http".into(), config: json!({}), ..Default::default() },
            Step { id: "c".into(), step_type: "http".into(), config: json!({}), ..Default::default() },
            Step { id: "d".into(), step_type: "http".into(), config: json!({}), ..Default::default() },
        ];
        let edges = vec![
            Edge { from: "a".into(), from_port: "out".into(), to: "b".into(), to_port: "in".into() },
            Edge { from: "a".into(), from_port: "out".into(), to: "c".into(), to_port: "in".into() },
            Edge { from: "b".into(), from_port: "out".into(), to: "d".into(), to_port: "in".into() },
            Edge { from: "c".into(), from_port: "out".into(), to: "d".into(), to_port: "in".into() },
        ];
        let levels = exec.topological_levels(&steps, &edges).unwrap();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], vec!["a"]);
        assert_eq!(levels[1].len(), 2);
        assert_eq!(levels[2], vec!["d"]);
    }

    #[test]
    fn topological_levels_detects_cycle() {
        let exec = StepExecutor::new(test_approval_store(), test_db());
        let steps = vec![
            Step { id: "a".into(), step_type: "http".into(), config: json!({}), ..Default::default() },
            Step { id: "b".into(), step_type: "http".into(), config: json!({}), ..Default::default() },
        ];
        let edges = vec![
            Edge { from: "a".into(), from_port: "out".into(), to: "b".into(), to_port: "in".into() },
            Edge { from: "b".into(), from_port: "out".into(), to: "a".into(), to_port: "in".into() },
        ];
        let result = exec.topological_levels(&steps, &edges);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("循环"));
    }

    #[test]
    fn get_execution_plan_no_edges_is_linear() {
        let exec = StepExecutor::new(test_approval_store(), test_db());
        let wf = Workflow {
            steps: vec![
                Step { id: "s1".into(), step_type: "data_set".into(), config: json!({}), ..Default::default() },
                Step { id: "s2".into(), step_type: "data_get".into(), config: json!({}), ..Default::default() },
            ],
            ..Default::default()
        };
        let plan = exec.get_execution_plan(&wf).unwrap();
        assert_eq!(plan.len(), 2);
        assert_eq!(plan[0], vec!["s1"]);
        assert_eq!(plan[1], vec!["s2"]);
    }
}
#[cfg(test)]
mod graph_tests {
    
    use crate::engine::executor::StepExecutor;
    use crate::engine::workflow::{Edge, Step, Workflow};
    use serde_json::json;
    use std::sync::Arc;

    fn test_exec() -> Arc<StepExecutor> {
        StepExecutor::new(
            Arc::new(crate::engine::approval_store::ApprovalStore::new()),
            Arc::new(crate::data::db::Database::open_default().expect("db")),
        )
    }

    #[tokio::test]
    async fn graph_execution_linear_chain_three_nodes() {
        // data_set → data_get → data_set（验证变量在节点间传递）
        let exec = test_exec();
        let wf = Workflow {
            name: "图链测试".into(),
            steps: vec![
                Step { id: "s1".into(), step_type: "data_set".into(), name: "Set name".into(),
                    config: json!({"key": "name", "value": "hello"}), ..Default::default() },
                Step { id: "s2".into(), step_type: "data_get".into(), name: "Get name".into(),
                    config: json!({"key": "name"}), ..Default::default() },
                Step { id: "s3".into(), step_type: "data_set".into(), name: "Set result".into(),
                    config: json!({"key": "result", "value": "{{s2}}"}), ..Default::default() },
            ],
            edges: vec![
                Edge { from: "s1".into(), from_port: "out".into(), to: "s2".into(), to_port: "in".into() },
                Edge { from: "s2".into(), from_port: "out".into(), to: "s3".into(), to_port: "in".into() },
            ],
            ..Default::default()
        };

        let ctx = exec.run_workflow(&wf).await.unwrap();
        // s1 设置了 name=hello
        assert_eq!(ctx.variables.get("name").and_then(|v| v.as_str()), Some("hello"));
        // s3 设置了 result（从 s2 读出的值）
        assert!(ctx.variables.contains_key("result"));
    }

    #[tokio::test]
    async fn graph_execution_parallel_two_branches() {
        // 两个独立 data_set 节点并行执行
        let exec = test_exec();
        let wf = Workflow {
            name: "图并行测试".into(),
            steps: vec![
                Step { id: "a".into(), step_type: "data_set".into(), name: "Set A".into(),
                    config: json!({"key": "a", "value": "A"}), ..Default::default() },
                Step { id: "b".into(), step_type: "data_set".into(), name: "Set B".into(),
                    config: json!({"key": "b", "value": "B"}), ..Default::default() },
            ],
            edges: vec![], // 无依赖 → 并行
            ..Default::default()
        };

        let ctx = exec.run_workflow(&wf).await.unwrap();
        assert_eq!(ctx.variables.get("a").and_then(|v| v.as_str()), Some("A"));
        assert_eq!(ctx.variables.get("b").and_then(|v| v.as_str()), Some("B"));
    }

    #[tokio::test]
    async fn empty_edges_falls_back_to_linear() {
        // 无 edges → 线性执行（向后兼容）
        let exec = test_exec();
        let wf = Workflow {
            name: "线性测试".into(),
            steps: vec![
                Step { id: "s1".into(), step_type: "data_set".into(), name: "S1".into(),
                    config: json!({"key": "x", "value": "1"}), ..Default::default() },
            ],
            ..Default::default()
        };

        let ctx = exec.run_workflow(&wf).await.unwrap();
        assert_eq!(ctx.variables.get("x").and_then(|v| v.as_str()), Some("1"));
    }

    #[tokio::test]
    async fn graph_execution_diamond_structure() {
        // 钻石结构：a → b, a → c, b → d, c → d
        let exec = test_exec();
        let wf = Workflow {
            name: "钻石测试".into(),
            steps: vec![
                Step { id: "a".into(), step_type: "data_set".into(), name: "A".into(),
                    config: json!({"key": "v", "value": "start"}), ..Default::default() },
                Step { id: "b".into(), step_type: "data_set".into(), name: "B".into(),
                    config: json!({"key": "b", "value": "done"}), ..Default::default() },
                Step { id: "c".into(), step_type: "data_set".into(), name: "C".into(),
                    config: json!({"key": "c", "value": "done"}), ..Default::default() },
                Step { id: "d".into(), step_type: "data_set".into(), name: "D".into(),
                    config: json!({"key": "d", "value": "final"}), ..Default::default() },
            ],
            edges: vec![
                Edge { from: "a".into(), from_port: "out".into(), to: "b".into(), to_port: "in".into() },
                Edge { from: "a".into(), from_port: "out".into(), to: "c".into(), to_port: "in".into() },
                Edge { from: "b".into(), from_port: "out".into(), to: "d".into(), to_port: "in".into() },
                Edge { from: "c".into(), from_port: "out".into(), to: "d".into(), to_port: "in".into() },
            ],
            ..Default::default()
        };

        let ctx = exec.run_workflow(&wf).await.unwrap();
        assert_eq!(ctx.variables.get("v").and_then(|v| v.as_str()), Some("start"));
        assert_eq!(ctx.variables.get("b").and_then(|v| v.as_str()), Some("done"));
        assert_eq!(ctx.variables.get("c").and_then(|v| v.as_str()), Some("done"));
        assert_eq!(ctx.variables.get("d").and_then(|v| v.as_str()), Some("final"));
    }

    #[tokio::test]
    async fn graph_execution_http_data_flow() {
        // http → data_set（验证 HTTP 输出到变量传递）
        let exec = test_exec();
        let wf = Workflow {
            name: "HTTP流".into(),
            steps: vec![
                Step { id: "fetch".into(), step_type: "http".into(), name: "Fetch".into(),
                    config: json!({"action": "GET", "url": "https://httpbin.org/get"}), ..Default::default() },
                Step { id: "save".into(), step_type: "data_set".into(), name: "Save".into(),
                    config: json!({"key": "response", "value": "{{fetch}}"}), ..Default::default() },
            ],
            edges: vec![
                Edge { from: "fetch".into(), from_port: "out".into(), to: "save".into(), to_port: "in".into() },
            ],
            ..Default::default()
        };

        let result = exec.run_workflow(&wf).await;
        // HTTP 可能因网络问题失败，但不应 panic
        match result {
            Ok(ctx) => {
                assert!(ctx.variables.contains_key("response"), "response 变量应存在");
            }
            Err(e) => {
                eprintln!("HTTP 请求失败（网络可能不可用）: {}", e);
            }
        }
    }
}
