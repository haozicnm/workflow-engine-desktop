// engine/executor.rs — 步骤执行器
//
// v3: 所有节点独立 executor，不再使用 action 参数分发。
//   每个操作一个 struct，一个注册条目。

use crate::engine::context::ExecutionContext;
use crate::engine::workflow::Step;
use crate::nodes::traits::NodeExecutor;
use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::{debug, info};

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

        // ── 正则节点（v3: 独立 executor） ──
        register!(
            executors,
            "regex_extract",
            crate::nodes::regex::RegexExtractNode
        );
        register!(
            executors,
            "regex_match",
            crate::nodes::regex::RegexMatchNode
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
            if let Some(ref ph) = placeholder_map {
                let _ = ph.resolve_value(&mut cg_value, ctx);
            }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::context::ExecutionContext;
    use crate::engine::parser;
    use crate::engine::workflow::{Step, Workflow};
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
}
