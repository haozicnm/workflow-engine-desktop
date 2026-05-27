// cli.rs — 命令行接口
// 用法: workflow-engine.exe --cli <command> [args]

use crate::engine::{parser, scheduler};
use crate::App;
use clap::{Parser, Subcommand};
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[derive(Parser)]
#[command(name = "workflow-engine", about = "Workflow Engine CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// 列出所有工作流
    List {
        /// 输出 JSON 格式
        #[arg(long)]
        json: bool,
    },
    /// 运行工作流
    Run {
        id: String,
        /// 注入变量 (可多次使用: --var key=value --var key2=value2)
        #[arg(short = 'v', long = "var", value_parser = parse_var)]
        vars: Vec<(String, String)>,
    },
    /// 查看运行状态
    Status {
        run_id: String,
        /// 输出 JSON 格式
        #[arg(long)]
        json: bool,
    },
    /// 导出工作流 (JSON)
    Export {
        id: String,
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
    /// 导入工作流
    Import { file: String },
    /// 校验工作流文件
    Validate {
        file: String,
        /// 输出 JSON 格式
        #[arg(long)]
        json: bool,
    },
    /// 管理定时调度
    #[command(subcommand)]
    Schedule(ScheduleCommand),
    /// 管理标准化工作流模板库
    #[command(subcommand)]
    Library(LibraryCommand),
    /// 列出所有可用步骤类型和动作（JSON）
    Steps {
        /// 输出 JSON 格式（默认）
        #[arg(long)]
        json: bool,
    },
    /// 创建新的工作流文件
    New {
        /// 工作流名称
        name: String,
        /// 输出文件路径（默认：<name>.wf.json）
        #[arg(short = 'o', long)]
        output: Option<String>,
        /// 工作流描述
        #[arg(long, default_value = "")]
        description: String,
    },
    /// 管理工作流步骤
    #[command(subcommand)]
    Step(StepCommand),
    /// 管理容器动作（仅 browser/excel/word/file 容器）
    #[command(subcommand)]
    Action(ActionCommand),
    /// 查看工作流文件详情
    Show {
        /// 工作流文件路径
        file: String,
    },
    /// 从文件直接运行工作流（无需先导入）
    RunFile {
        /// 工作流文件路径
        file: String,
        /// 注入变量 (可多次使用: --var key=value --var key2=value2)
        #[arg(short = 'v', long = "var", value_parser = parse_var)]
        vars: Vec<(String, String)>,
    },
    /// 查看工作流执行预览
    Preview {
        /// run_id、"latest"（最近一次执行）、或 "list"（列出所有有预览的执行）
        #[arg(default_value = "latest")]
        run_or_action: String,
        /// 指定 step_id 查看该步详情（不指定则列出所有步骤）
        step_id: Option<String>,
        /// 输出 JSON 格式
        #[arg(long)]
        json: bool,
    },
    /// 从工作流生成 SKILL.md
    Skill {
        /// 工作流文件路径 或 ID
        file_or_id: String,
        /// 输出文件路径 (默认: stdout)
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
    /// 启动 HTTP 服务器（提供 API + 前端静态文件）
    Serve {
        /// 监听地址 (默认: 0.0.0.0:3000)
        #[arg(long, default_value = "0.0.0.0:3000")]
        bind: String,
        /// 静态文件目录 (默认: dist)
        #[arg(long, default_value = "dist")]
        static_dir: String,
    },
}

#[derive(Subcommand)]
pub enum StepCommand {
    /// 添加步骤
    Add {
        /// 工作流文件路径
        file: String,
        /// 步骤类型 (http|script|condition|browser_container|file|excel|word|...)
        #[arg(short = 't', long = "type")]
        step_type: String,
        /// 步骤标签
        #[arg(short = 'n', long)]
        name: String,
        /// 步骤配置 JSON
        #[arg(short = 'c', long, default_value = "{}")]
        config: String,
        /// 步骤 ID（默认自动生成 step_N）
        #[arg(long)]
        id: Option<String>,
        /// 插入位置（0-based，默认追加到末尾）
        #[arg(long)]
        position: Option<usize>,
    },
    /// 列出步骤
    List {
        /// 工作流文件路径
        file: String,
    },
    /// 删除步骤
    Remove {
        /// 工作流文件路径
        file: String,
        /// 步骤 ID
        id: String,
    },
    /// 查看步骤详情
    Show {
        /// 工作流文件路径
        file: String,
        /// 步骤 ID
        id: String,
    },
    /// 编辑步骤配置
    Edit {
        /// 工作流文件路径
        file: String,
        /// 步骤 ID
        id: String,
        /// 新的步骤名称
        #[arg(short = 'n', long)]
        name: Option<String>,
        /// 合并式更新配置 (JSON，只更新指定字段)
        #[arg(short = 'c', long)]
        config: Option<String>,
    },
}

#[derive(Subcommand)]
pub enum ActionCommand {
    /// 向容器步骤添加动作
    Add {
        /// 工作流文件路径
        file: String,
        /// 容器步骤 ID
        step_id: String,
        /// 动作类型 (navigate|click|fill|extract|screenshot|wait|scroll|read|write|...)
        #[arg(short = 't', long = "type")]
        action_type: String,
        /// 动作配置 JSON
        #[arg(short = 'c', long, default_value = "{}")]
        config: String,
        /// 动作 ID（默认自动生成 a<步序号>_<序号>）
        #[arg(long)]
        id: Option<String>,
    },
    /// 列出容器步骤的所有动作
    List {
        /// 工作流文件路径
        file: String,
        /// 容器步骤 ID
        step_id: String,
    },
    /// 删除容器动作
    Remove {
        /// 工作流文件路径
        file: String,
        /// 容器步骤 ID
        step_id: String,
        /// 动作 ID
        id: String,
    },
}

#[derive(Subcommand)]
pub enum LibraryCommand {
    /// 列出所有模板
    List {
        #[arg(long)]
        json: bool,
    },
    /// 查看模板详情
    Show { name: String },
    /// 从已导入的工作流创建模板
    Create {
        /// 模板名称
        name: String,
        /// 源工作流 ID
        #[arg(long)]
        from: String,
        /// 分类 (monitoring/batch/stress/general)
        #[arg(long, default_value = "general")]
        category: String,
        /// 参数声明 (逗号分隔)
        #[arg(long, default_value = "")]
        params: String,
    },
    /// 运行模板（参数化执行）
    Run {
        name: String,
        /// JSON 参数覆盖
        #[arg(long, default_value = "{}")]
        params: String,
    },
    /// 校验模板
    Validate { name: String },
    /// 为模板创建定时调度
    Schedule {
        name: String,
        /// cron 表达式 (6段: 秒 分 时 日 月 周)
        cron_expr: String,
        /// JSON 参数覆盖
        #[arg(long, default_value = "{}")]
        params: String,
    },
}

#[derive(Subcommand)]
pub enum ScheduleCommand {
    /// 列出所有调度
    List {
        /// 输出 JSON 格式
        #[arg(long)]
        json: bool,
    },
    /// 创建调度
    Create {
        workflow_id: String,
        /// cron 表达式 (如 "0 9 * * *")
        cron_expr: String,
    },
    /// 删除调度
    Delete { id: String },
}

/// 解析 --var key=value 参数
fn parse_var(s: &str) -> Result<(String, String), String> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| format!("无效格式 '{}'，应为 key=value", s))?;
    Ok((k.to_string(), v.to_string()))
}

pub async fn run_cli(cli: Cli, app: Arc<App>) -> Result<(), String> {
    match cli.command {
        Commands::List { json } => cmd_list(&app, json),
        Commands::Run { id, vars } => cmd_run(&app, &id, &vars).await,
        Commands::Status { run_id, json } => cmd_status(&app, &run_id, json),
        Commands::Export { id, output } => cmd_export(&app, &id, output.as_deref()),
        Commands::Import { file } => cmd_import(&app, &file),
        Commands::Validate { file, json } => cmd_validate(&file, json),
        Commands::Schedule(sub) => cmd_schedule(&app, sub),
        Commands::Library(sub) => cmd_library(&app, sub).await,
        Commands::Steps { json: _ } => cmd_steps(),
        Commands::New {
            name,
            output,
            description,
        } => cmd_new(&name, output.as_deref(), &description),
        Commands::Step(sub) => cmd_step(sub),
        Commands::Action(sub) => cmd_action(sub),
        Commands::Show { file } => cmd_show(&file),
        Commands::RunFile { file, vars } => cmd_run_file(&app, &file, &vars).await,
        Commands::Preview {
            run_or_action,
            step_id,
            json,
        } => cmd_preview(&run_or_action, step_id.as_deref(), json),
        Commands::Skill { file_or_id, output } => cmd_skill(&app, &file_or_id, output.as_deref()),
        Commands::Serve { bind, static_dir } => cmd_serve(app, &bind, &static_dir).await,
    }
}

fn cmd_list(app: &App, json: bool) -> Result<(), String> {
    let workflows = app
        .db
        .list_workflows()
        .map_err(|e| format!("查询失败: {e}"))?;
    if json {
        let items: Vec<serde_json::Value> = workflows
            .iter()
            .map(|w| {
                serde_json::json!({
                    "id": w.id,
                    "name": w.name,
                    "updated_at": w.updated_at,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "workflows": items,
                "count": items.len(),
            }))
            .expect("CLI JSON 序列化失败")
        );
        return Ok(());
    }
    if workflows.is_empty() {
        println!("(无工作流)");
        return Ok(());
    }
    println!("{:<38} {:<20} {:<30}", "ID", "名称", "更新时间");
    println!("{}", "-".repeat(90));
    for w in &workflows {
        println!("{:<38} {:<20} {:<30}", w.id, w.name, w.updated_at);
    }
    println!("\n共 {} 个工作流", workflows.len());
    Ok(())
}

async fn cmd_run(app: &App, workflow_id: &str, vars: &[(String, String)]) -> Result<(), String> {
    // 优先尝试通过 IPC 发送给桌面 daemon
    if crate::ipc_client::IpcClient::is_daemon_available().await {
        let vars_map: Option<std::collections::HashMap<String, String>> = if vars.is_empty() {
            None
        } else {
            Some(vars.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
        };
        return crate::ipc_client::IpcClient::run_remote(workflow_id, vars_map).await;
    }

    // 回退：本地直接执行
    let yaml = app
        .db
        .get_workflow_yaml(workflow_id)
        .map_err(|e| format!("获取工作流失败: {e}"))?
        .ok_or_else(|| "工作流不存在".to_string())?;

    let workflow = parser::parse_workflow(&yaml).map_err(|e| format!("解析失败: {e}"))?;

    let run_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let workflow_name = workflow.name.clone();
    app.db
        .create_run(&run_id, workflow_id, &workflow_name, &now)
        .map_err(|e| format!("创建运行记录失败: {e}"))?;

    // 注入 --var 变量到 context（scheduler 内部会创建 ExecutionContext）
    // 由于 scheduler 内部管理 ctx，变量注入需在调用前通过 workflow.variables 传递
    if !vars.is_empty() {
        println!(
            "注入变量: {}",
            vars.iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // 并发控制
    let _permit = app
        .run_semaphore
        .clone()
        .try_acquire_owned()
        .map_err(|_| "已达到最大并发工作流数限制，请等待其他工作流完成后再试".to_string())?;

    // 构建 RunControl（CLI 模式：无取消/暂停/断点/单步）
    let ctrl = scheduler::RunControl {
        cancel_flag: Arc::new(AtomicBool::new(false)),
        cancel_token: tokio_util::sync::CancellationToken::new(),
        pause_flag: Arc::new(AtomicBool::new(false)),
        breakpoint_flag: Arc::new(AtomicBool::new(false)),
        step_mode_flag: Arc::new(AtomicBool::new(false)),
        debug_snapshots: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    };

    if workflow.steps.is_empty() {
        app.db
            .update_run_status(&run_id, "completed", None)
            .map_err(|e| e.to_string())?;
        println!("✓ 工作流无步骤，直接完成");
        return Ok(());
    }

    let total = workflow.steps.len();
    println!("▶ {} (共 {} 步)", workflow_name, total);
    let start = std::time::Instant::now();

    // 委托 scheduler::run_workflow 执行（CLI 模式不传 app_handle → 无 Tauri 事件推送）
    let result = scheduler::run_workflow(
        &workflow,
        &run_id,
        None, // CLI 模式无 Tauri AppHandle
        &app.db,
        app.approval_store.clone(),
        "auto", // browser_channel: CLI 默认 auto
        vars,
        &ctrl,
    )
    .await;

    let elapsed = start.elapsed().as_secs_f64();

    match result {
        Ok(_state) => {
            let _ = app.db.update_run_status(&run_id, "completed", None);
            println!("\n✓ 完成 ({:.1}s)", elapsed);
            Ok(())
        }
        Err(e) => {
            let err_msg = e.to_string();
            let _ = app.db.update_run_status(&run_id, "failed", Some(&err_msg));
            eprintln!("\n✗ 失败 ({:.1}s): {}", elapsed, err_msg);
            Err(format!("工作流执行失败: {}", err_msg))
        }
    }
}

fn cmd_status(app: &App, run_id: &str, json: bool) -> Result<(), String> {
    let detail = app
        .db
        .get_run_detail(run_id)
        .map_err(|e| format!("查询失败: {e}"))?
        .ok_or_else(|| "运行记录不存在".to_string())?;

    if json {
        let steps: Vec<serde_json::Value> = detail
            .steps
            .iter()
            .map(|s| {
                serde_json::json!({
                    "step_id": s.step_id,
                    "status": s.status,
                    "output": s.output,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "run_id": detail.run.id,
                "workflow_name": detail.workflow_name,
                "status": detail.run.status,
                "started_at": detail.run.started_at,
                "finished_at": detail.run.finished_at,
                "error": detail.run.error,
                "steps": steps,
            }))
            .expect("CLI JSON 序列化失败")
        );
        return Ok(());
    }

    println!("运行 ID:   {}", detail.run.id);
    println!("工作流:    {}", detail.workflow_name);
    println!("状态:      {}", detail.run.status);
    println!("开始:      {}", detail.run.started_at);
    println!(
        "结束:      {}",
        detail.run.finished_at.as_deref().unwrap_or("-")
    );
    if let Some(ref err) = detail.run.error {
        println!("错误:      {}", err);
    }
    if !detail.steps.is_empty() {
        println!("\n步骤日志:");
        for s in &detail.steps {
            let icon = match s.status.as_str() {
                "success" => "✓",
                "error" => "✗",
                _ => " ",
            };
            println!("  {} {} - {}", icon, s.step_id, s.status);
        }
    }
    Ok(())
}

fn cmd_export(app: &App, workflow_id: &str, output: Option<&str>) -> Result<(), String> {
    let yaml = app
        .db
        .get_workflow_yaml(workflow_id)
        .map_err(|e| format!("获取失败: {e}"))?
        .ok_or_else(|| "工作流不存在".to_string())?;

    let value: serde_json::Value =
        serde_json::from_str(&yaml).map_err(|e| format!("JSON 解析失败: {e}"))?;
    let pretty = serde_json::to_string_pretty(&value).map_err(|e| format!("序列化失败: {e}"))?;

    match output {
        Some(path) => {
            std::fs::write(path, &pretty).map_err(|e| format!("写入失败: {e}"))?;
            println!("✓ 已导出: {}", path);
        }
        None => println!("{}", pretty),
    }
    Ok(())
}

fn cmd_import(app: &App, file: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("读取失败: {e}"))?;
    let value: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON 格式错误: {e}"))?;

    // 从 JSON 中读取工作流名称
    let name = value
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("未命名工作流");

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    app.db
        .create_workflow(&id, name, "", &now, &now)
        .map_err(|e| format!("创建失败: {e}"))?;
    app.db
        .save_workflow_yaml(&id, &content)
        .map_err(|e| format!("保存失败: {e}"))?;
    println!("✓ 已导入: {} (ID: {})", name, id);
    Ok(())
}

fn cmd_validate(file: &str, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file).map_err(|e| format!("读取失败: {e}"))?;
    let _: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON 格式错误: {e}"))?;
    let workflow = parser::parse_workflow(&content).map_err(|e| format!("工作流结构错误: {e}"))?;

    if json {
        let steps: Vec<serde_json::Value> = workflow
            .steps
            .iter()
            .map(|s| {
                serde_json::json!({
                    "name": s.name,
                    "type": s.step_type,
                    "next": s.next,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "valid": true,
                "name": workflow.name,
                "step_count": workflow.steps.len(),
                "steps": steps,
            }))
            .expect("CLI JSON 序列化失败")
        );
        return Ok(());
    }

    println!("✓ 校验通过");
    println!("  名称: {}", workflow.name);
    println!("  步骤数: {}", workflow.steps.len());
    for (i, s) in workflow.steps.iter().enumerate() {
        println!("  {}. {} ({})", i + 1, s.name, s.step_type);
    }
    Ok(())
}

// ─── Steps manifest ───

/// 输出所有可用步骤类型和动作（JSON），与前端 CONTAINER_DEFS 保持同步
fn cmd_steps() -> Result<(), String> {
    let manifest = build_step_manifest();
    println!(
        "{}",
        serde_json::to_string_pretty(&manifest).expect("CLI manifest 序列化失败")
    );
    Ok(())
}

fn build_step_manifest() -> serde_json::Value {
    serde_json::json!({
        "version": "6.7.0",
        "total_types": 14,
        "types": [
            // ═══ 容器节点 (isContainer: true) ═══
            {
                "type": "browser",
                "label": "浏览器",
                "isContainer": true,
                "description": "网页操作：导航、点击、输入、提取",
                "config": [
                    {"key": "browser", "type": "select", "options": ["chromium", "firefox", "webkit"], "default": "chromium"},
                    {"key": "headless", "type": "checkbox", "default": false},
                    {"key": "timeout", "type": "number", "default": 30000}
                ],
                "actions": [
                    {"type": "navigate", "label": "打开页面", "params": [{"key": "url", "type": "text"}]},
                    {"type": "click", "label": "点击元素", "params": [{"key": "selector", "type": "text"}]},
                    {"type": "input", "label": "输入文本", "params": [{"key": "selector", "type": "text"}, {"key": "value", "type": "text"}]},
                    {"type": "wait", "label": "等待", "params": [{"key": "ms", "type": "number", "default": 1000}]},
                    {"type": "screenshot", "label": "截图", "params": [{"key": "path", "type": "text"}]},
                    {"type": "evaluate", "label": "执行 JS", "params": [{"key": "script", "type": "textarea"}]},
                    {"type": "scroll", "label": "滚动页面", "params": [{"key": "x", "type": "number", "default": 0}, {"key": "y", "type": "number", "default": 500}]},
                    {"type": "extract", "label": "提取数据", "params": [{"key": "selector", "type": "text", "default": "body"}, {"key": "mode", "type": "select", "options": ["text", "html", "attr"], "default": "text"}]},
                    {"type": "get_title", "label": "获取标题", "params": []},
                    {"type": "extract_table", "label": "提取表格", "params": [{"key": "selector", "type": "text", "default": "table"}]},
                    {"type": "extract_links", "label": "提取链接", "params": [{"key": "selector", "type": "text", "default": "body"}]},
                    {"type": "select", "label": "下拉选择", "params": [{"key": "selector", "type": "text"}, {"key": "value", "type": "text"}]},
                    {"type": "check", "label": "勾选/取消", "params": [{"key": "selector", "type": "text"}, {"key": "checked", "type": "checkbox", "default": true}]},
                    {"type": "hover", "label": "鼠标悬停", "params": [{"key": "selector", "type": "text"}]},
                    {"type": "cookies", "label": "Cookie 管理", "params": [{"key": "action", "type": "select", "options": ["get", "set", "clear"], "default": "get"}, {"key": "cookies", "type": "textarea"}]},
                    {"type": "set_headers", "label": "设置请求头", "params": [{"key": "headers", "type": "textarea"}]},
                    {"type": "new_page", "label": "新建标签页", "params": [{"key": "url", "type": "text"}]},
                    {"type": "close_page", "label": "关闭标签页", "params": [{"key": "index", "type": "number"}]},
                    {"type": "switch_page", "label": "切换标签页", "params": [{"key": "index", "type": "number", "default": 0}]},
                    {"type": "pages", "label": "标签页列表", "params": []},
                    {"type": "back", "label": "后退", "params": []},
                    {"type": "forward", "label": "前进", "params": []},
                    {"type": "reload", "label": "刷新页面", "params": []},
                    {"type": "current_url", "label": "当前网址", "params": []},
                    {"type": "pdf", "label": "生成 PDF", "params": [{"key": "path", "type": "text", "default": "output.pdf"}]},
                    {"type": "wait_network_idle", "label": "等待网络空闲", "params": [{"key": "timeout", "type": "number", "default": 30000}]},
                    {"type": "wait_load_state", "label": "等待加载状态", "params": [{"key": "state", "type": "select", "options": ["load", "domcontentloaded", "networkidle"], "default": "load"}, {"key": "timeout", "type": "number", "default": 30000}]},
                    {"type": "wait_url_contains", "label": "等待 URL 变更", "params": [{"key": "substring", "type": "text"}, {"key": "timeout", "type": "number", "default": 30000}]},
                    {"type": "verify", "label": "验证健康", "params": []},
                    {"type": "download", "label": "下载文件", "params": [{"key": "save_dir", "type": "text", "default": "."}, {"key": "click_selector", "type": "text"}, {"key": "timeout", "type": "number", "default": 30000}]},
                    {"type": "upload", "label": "上传文件", "params": [{"key": "selector", "type": "text"}, {"key": "file_paths", "type": "text"}]},
                    {"type": "keyboard", "label": "键盘操作", "params": [{"key": "key", "type": "text"}, {"key": "text", "type": "text"}, {"key": "delay", "type": "number", "default": 0}]},
                    {"type": "double_click", "label": "双击元素", "params": [{"key": "selector", "type": "text"}, {"key": "timeout_ms", "type": "number", "default": 10000}]},
                    {"type": "drag_to", "label": "拖拽元素", "params": [{"key": "source", "type": "text"}, {"key": "target", "type": "text"}, {"key": "source_position", "type": "text"}, {"key": "target_position", "type": "text"}]},
                    {"type": "context_menu", "label": "右键菜单", "params": [{"key": "selector", "type": "text"}, {"key": "timeout_ms", "type": "number", "default": 10000}]},
                    {"type": "switch_frame", "label": "切换 iframe", "params": [{"key": "selector", "type": "text"}]},
                    {"type": "handle_dialog", "label": "处理弹窗", "params": [{"key": "action", "type": "select", "options": ["accept", "reject"], "default": "accept"}, {"key": "prompt_text", "type": "text"}]},
                    {"type": "scroll_to_element", "label": "滚动到元素", "params": [{"key": "selector", "type": "text"}, {"key": "behavior", "type": "select", "options": ["smooth", "instant"], "default": "smooth"}, {"key": "block", "type": "select", "options": ["center", "start", "end", "nearest"], "default": "center"}]}
                ]
            },
            {
                "type": "excel",
                "label": "Excel",
                "isContainer": true,
                "description": "Excel 操作：读写单元格、筛选、排序",
                "config": [
                    {"key": "file_path", "type": "text"},
                    {"key": "sheet", "type": "text", "default": "Sheet1"}
                ],
                "actions": [
                    {"type": "read", "label": "读取整表", "params": []},
                    {"type": "write", "label": "写入数据", "params": [{"key": "value", "type": "textarea"}]},
                    {"type": "create", "label": "创建文件", "params": [{"key": "headers", "type": "text"}]},
                    {"type": "append", "label": "追加行", "params": [{"key": "value", "type": "textarea"}]},
                    {"type": "filter", "label": "筛选", "params": [{"key": "column", "type": "text"}, {"key": "op", "type": "select", "options": ["contains", "equals", "not_equals", "gt", "gte", "lt", "lte", "is_empty", "not_empty"], "default": "contains"}, {"key": "value", "type": "text"}]},
                    {"type": "sort", "label": "排序", "params": [{"key": "column", "type": "text"}, {"key": "order", "type": "select", "options": ["asc", "desc"], "default": "asc"}]},
                    {"type": "formula", "label": "公式", "params": [{"key": "cell", "type": "text"}, {"key": "formula", "type": "text"}]}
                ]
            },
            {
                "type": "word",
                "label": "Word",
                "isContainer": true,
                "description": "Word 操作：读写、替换、合并",
                "config": [
                    {"key": "file_path", "type": "text"}
                ],
                "actions": [
                    {"type": "read", "label": "读取内容", "params": []},
                    {"type": "write", "label": "写入段落", "params": [{"key": "value", "type": "textarea"}]},
                    {"type": "replace", "label": "替换文本", "params": [{"key": "old_text", "type": "text"}, {"key": "new_text", "type": "text"}]},
                    {"type": "create", "label": "创建文档", "params": [{"key": "title", "type": "text"}]},
                    {"type": "insert_table", "label": "插入表格", "params": [{"key": "data", "type": "textarea"}]},
                    {"type": "merge", "label": "合并文档", "params": [{"key": "files", "type": "textarea"}]}
                ]
            },
            {
                "type": "file",
                "label": "文件操作",
                "isContainer": true,
                "description": "统一文件操作：读取/写入/复制/移动/删除/列表/搜索/Glob/Grep",
                "config": [],
                "actions": [
                    {"type": "read", "label": "读取文件", "params": [{"key": "path", "type": "text"}, {"key": "encoding", "type": "select", "options": ["text", "base64"], "default": "text"}]},
                    {"type": "write", "label": "写入文件", "params": [{"key": "path", "type": "text"}, {"key": "content", "type": "textarea"}]},
                    {"type": "append", "label": "追加内容", "params": [{"key": "path", "type": "text"}, {"key": "content", "type": "textarea"}]},
                    {"type": "copy", "label": "复制文件", "params": [{"key": "source", "type": "text"}, {"key": "dest", "type": "text"}]},
                    {"type": "move", "label": "移动/重命名", "params": [{"key": "source", "type": "text"}, {"key": "dest", "type": "text"}]},
                    {"type": "delete", "label": "删除文件", "params": [{"key": "path", "type": "text"}]},
                    {"type": "list", "label": "列出目录", "params": [{"key": "path", "type": "text"}, {"key": "recursive", "type": "checkbox", "default": false}]},
                    {"type": "exists", "label": "存在检查", "params": [{"key": "path", "type": "text"}]},
                    {"type": "glob", "label": "通配符匹配", "params": [{"key": "pattern", "type": "text"}, {"key": "path", "type": "text", "default": "."}]},
                    {"type": "grep", "label": "内容搜索", "params": [{"key": "pattern", "type": "text"}, {"key": "path", "type": "text", "default": "."}, {"key": "file_glob", "type": "text"}]}
                ]
            },
            {
                "type": "logic",
                "label": "条件判断",
                "isContainer": true,
                "description": "条件分支：满足/不满足走不同路径",
                "config": [
                    {"key": "condition", "type": "text"}
                ],
                "operators": [
                    {"type": "contains", "label": "包含", "hasRight": true},
                    {"type": "not_contains", "label": "不包含", "hasRight": true},
                    {"type": "equals", "label": "等于", "hasRight": true},
                    {"type": "not_equals", "label": "不等于", "hasRight": true},
                    {"type": "greater_than", "label": "大于", "hasRight": true},
                    {"type": "less_than", "label": "小于", "hasRight": true},
                    {"type": "greater_equal", "label": "大于等于", "hasRight": true},
                    {"type": "less_equal", "label": "小于等于", "hasRight": true},
                    {"type": "starts_with", "label": "开头是", "hasRight": true},
                    {"type": "ends_with", "label": "结尾是", "hasRight": true},
                    {"type": "is_empty", "label": "为空", "hasRight": false},
                    {"type": "not_empty", "label": "不为空", "hasRight": false},
                    {"type": "regex", "label": "正则匹配", "hasRight": true}
                ]
            },
            {
                "type": "cursor",
                "label": "游标迭代",
                "isContainer": true,
                "description": "逐条迭代：每次运行处理一行/一项，游标跨次保存",
                "config": [
                    {"key": "items", "type": "text"}
                ],
                "body_actions": [
                    {"type": "http", "label": "HTTP 请求", "params": [{"key": "method", "type": "select", "options": ["GET", "POST"], "default": "GET"}, {"key": "url", "type": "text"}, {"key": "body", "type": "textarea"}]},
                    {"type": "script", "label": "脚本", "params": [{"key": "script", "type": "textarea"}]},
                    {"type": "delay", "label": "延迟等待", "params": [{"key": "duration_ms", "type": "number", "default": 1000}]},
                    {"type": "notify", "label": "通知", "params": [{"key": "title", "type": "text"}, {"key": "body", "type": "textarea"}]}
                ]
            },
            {
                "type": "loop",
                "label": "批量循环",
                "isContainer": true,
                "description": "一次性遍历全部数据，适合小数据内存变换",
                "config": [
                    {"key": "items", "type": "text"}
                ],
                "body_actions": [
                    {"type": "http", "label": "HTTP 请求", "params": [{"key": "method", "type": "select", "options": ["GET", "POST"], "default": "GET"}, {"key": "url", "type": "text"}, {"key": "body", "type": "textarea"}]},
                    {"type": "script", "label": "脚本", "params": [{"key": "script", "type": "textarea"}]},
                    {"type": "delay", "label": "延迟等待", "params": [{"key": "duration_ms", "type": "number", "default": 1000}]},
                    {"type": "notify", "label": "通知", "params": [{"key": "title", "type": "text"}, {"key": "body", "type": "textarea"}]}
                ]
            },
            // ═══ 简单节点 (isContainer: false) ═══
            {
                "type": "http",
                "label": "HTTP 请求",
                "isContainer": false,
                "description": "发送 HTTP 请求：GET/POST/PUT/DELETE",
                "config": [
                    {"key": "method", "type": "select", "options": ["GET", "POST", "PUT", "DELETE"], "default": "GET"},
                    {"key": "url", "type": "text"},
                    {"key": "headers", "type": "textarea"},
                    {"key": "body", "type": "textarea"}
                ],
                "output": "{ status, body, headers }"
            },
            {
                "type": "delay",
                "label": "延迟等待",
                "isContainer": false,
                "description": "等待指定时间后继续",
                "config": [
                    {"key": "duration_ms", "type": "number", "default": 1000},
                    {"key": "max_duration_ms", "type": "number", "default": 5000}
                ],
                "output": "{ waited: ms }"
            },
            {
                "type": "notify",
                "label": "通知",
                "isContainer": false,
                "description": "发送通知：系统通知/Webhook",
                "config": [
                    {"key": "notify_type", "type": "select", "options": ["system", "webhook"], "default": "system"},
                    {"key": "title", "type": "text"},
                    {"key": "body", "type": "textarea"},
                    {"key": "url", "type": "text"}
                ],
                "output": "{ sent: true }"
            },
            {
                "type": "script",
                "label": "脚本",
                "isContainer": false,
                "description": "执行自定义脚本（Rhai）",
                "config": [
                    {"key": "script", "type": "textarea"}
                ],
                "output": "脚本返回值"
            },
            {
                "type": "clipboard",
                "label": "剪贴板",
                "isContainer": false,
                "description": "读写系统剪贴板",
                "config": [
                    {"key": "action", "type": "select", "options": ["read", "write"], "default": "read"},
                    {"key": "text", "type": "textarea"}
                ],
                "output": "剪贴板内容"
            },
            {
                "type": "approval",
                "label": "人工审批",
                "isContainer": false,
                "description": "暂停流程等待人工审核",
                "config": [
                    {"key": "title", "type": "text"},
                    {"key": "message", "type": "textarea"},
                    {"key": "options", "type": "text", "default": "同意,拒绝"},
                    {"key": "recommended", "type": "text", "default": "同意"},
                    {"key": "require_review", "type": "select", "options": ["true", "false"], "default": "true"},
                    {"key": "timeout", "type": "number", "default": 300},
                    {"key": "timeout_action", "type": "select", "options": ["recommended", "reject", "approve", "fail"], "default": "recommended"}
                ],
                "output": "{ decision, comment, item }"
            },
            {
                "type": "shell",
                "label": "Shell 命令",
                "isContainer": false,
                "description": "执行任意 Shell 命令（bash/powershell/cmd），支持 {{变量}} — 万能 OS 自动化原语",
                "config": [
                    {"key": "command", "type": "textarea"},
                    {"key": "shell", "type": "select", "options": ["auto", "bash", "powershell", "cmd"], "default": "auto"},
                    {"key": "cwd", "type": "text"},
                    {"key": "timeout_secs", "type": "number", "default": 300}
                ],
                "output": "{ stdout, stderr, exit_code }"
            }
        ]
    })
}

// ─── Schedule 管理 ───

fn cmd_schedule(app: &App, sub: ScheduleCommand) -> Result<(), String> {
    match sub {
        ScheduleCommand::List { json } => cmd_schedule_list(app, json),
        ScheduleCommand::Create {
            workflow_id,
            cron_expr,
        } => cmd_schedule_create(app, &workflow_id, &cron_expr),
        ScheduleCommand::Delete { id } => cmd_schedule_delete(app, &id),
    }
}

fn cmd_schedule_list(app: &App, json: bool) -> Result<(), String> {
    let schedules = app
        .db
        .list_schedules()
        .map_err(|e| format!("查询失败: {e}"))?;
    if json {
        let items: Vec<serde_json::Value> = schedules
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "workflow_id": s.workflow_id,
                    "workflow_name": s.workflow_name,
                    "cron_expr": s.cron_expr,
                    "enabled": s.enabled,
                    "last_run_at": s.last_run_at,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
            "results": items,
                "count": items.len(),
            }))
            .expect("CLI JSON 序列化失败")
        );
        return Ok(());
    }
    if schedules.is_empty() {
        println!("(无调度)");
        return Ok(());
    }
    println!(
        "{:<38} {:<38} {:<20} {:<8} {:<20}",
        "ID", "工作流ID", "Cron", "启用", "上次运行"
    );
    println!("{}", "-".repeat(130));
    for s in &schedules {
        let enabled = if s.enabled { "✓" } else { "✗" };
        let last = s.last_run_at.as_deref().unwrap_or("-");
        println!(
            "{:<38} {:<38} {:<20} {:<8} {:<20}",
            s.id, s.workflow_id, s.cron_expr, enabled, last
        );
    }
    println!("\n共 {} 个调度", schedules.len());
    Ok(())
}

fn cmd_schedule_create(app: &App, workflow_id: &str, cron_expr: &str) -> Result<(), String> {
    use std::str::FromStr;
    // 验证 cron 表达式
    cron::Schedule::from_str(cron_expr).map_err(|e| format!("无效的 cron 表达式: {e}"))?;
    // 验证工作流存在
    app.db
        .get_workflow_yaml(workflow_id)
        .map_err(|e| format!("查询失败: {e}"))?
        .ok_or_else(|| "工作流不存在".to_string())?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    app.db
        .create_schedule(&id, workflow_id, cron_expr, &now)
        .map_err(|e| format!("创建调度失败: {e}"))?;
    println!("✓ 调度已创建 (ID: {})", id);
    Ok(())
}

fn cmd_schedule_delete(app: &App, id: &str) -> Result<(), String> {
    app.db
        .delete_schedule(id)
        .map_err(|e| format!("删除调度失败: {e}"))?;
    println!("✓ 调度已删除");
    Ok(())
}

// ─── Library 管理 ───

const LIBRARY_DIR: &str = ".hermes/workflows";

async fn cmd_library(app: &App, sub: LibraryCommand) -> Result<(), String> {
    match sub {
        LibraryCommand::List { json } => cmd_library_list(json),
        LibraryCommand::Show { name } => cmd_library_show(&name),
        LibraryCommand::Create {
            name,
            from,
            category,
            params,
        } => cmd_library_create(app, &name, &from, &category, &params),
        LibraryCommand::Run { name, params } => cmd_library_run(app, &name, &params).await,
        LibraryCommand::Validate { name } => cmd_library_validate(&name),
        LibraryCommand::Schedule {
            name,
            cron_expr,
            params,
        } => cmd_library_schedule(app, &name, &cron_expr, &params).await,
    }
}

fn library_dir() -> std::path::PathBuf {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    home.join(LIBRARY_DIR)
}

fn load_catalog() -> Result<Vec<TemplateEntry>, String> {
    let path = library_dir().join("catalog.toml");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("读取 catalog.toml 失败: {}\n路径: {}", e, path.display()))?;
    let catalog: Catalog =
        toml::from_str(&content).map_err(|e| format!("解析 catalog.toml 失败: {}", e))?;
    Ok(catalog.templates)
}

#[derive(serde::Deserialize)]
struct Catalog {
    templates: Vec<TemplateEntry>,
}

#[derive(serde::Deserialize, Clone)]
struct TemplateEntry {
    name: String,
    version: String,
    description: String,
    params: Vec<String>,
    schedule: Option<String>,
    category: String,
    file: String,
}

fn cmd_library_list(json: bool) -> Result<(), String> {
    let templates = load_catalog()?;
    if json {
        let items: Vec<serde_json::Value> = templates
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name,
                    "version": t.version,
                    "description": t.description,
                    "params": t.params,
                    "schedule": t.schedule,
                    "category": t.category,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "templates": items,
                "count": items.len(),
            }))
            .expect("CLI JSON 序列化失败")
        );
        return Ok(());
    }
    if templates.is_empty() {
        println!("(无模板)");
        return Ok(());
    }
    println!("{:<25} {:<8} {:<12} {:<30}", "名称", "版本", "分类", "描述");
    println!("{}", "-".repeat(80));
    for t in &templates {
        let schedule = t.schedule.as_deref().unwrap_or("手动");
        println!(
            "{:<25} {:<8} {:<12} {:<30}",
            t.name, t.version, t.category, t.description
        );
        println!("  参数: {}  默认调度: {}", t.params.join(", "), schedule);
    }
    println!("\n共 {} 个模板", templates.len());
    Ok(())
}

fn cmd_library_show(name: &str) -> Result<(), String> {
    let templates = load_catalog()?;
    let entry = templates
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| format!("模板 '{}' 不存在", name))?;

    let file_path = library_dir().join(&entry.file);
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("读取模板文件失败: {}", e))?;
    let wf: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("模板 JSON 格式错误: {}", e))?;

    let params = wf.get("params").and_then(|v| v.as_object());
    let default_params: Vec<String> = params
        .map(|p| p.iter().map(|(k, v)| format!("  {} = {}", k, v)).collect())
        .unwrap_or_default();

    println!("名称:     {}", entry.name);
    println!("版本:     {}", entry.version);
    println!("分类:     {}", entry.category);
    println!("描述:     {}", entry.description);
    println!("声明参数: {}", entry.params.join(", "));
    println!(
        "默认调度: {}",
        entry.schedule.as_deref().unwrap_or("手动触发")
    );
    if !default_params.is_empty() {
        println!("\n默认参数值:");
        for p in &default_params {
            println!("{}", p);
        }
    }
    println!(
        "\n步骤数:   {}",
        wf.get("steps")
            .and_then(|s| s.as_array())
            .map(|a| a.len())
            .unwrap_or(0)
    );
    Ok(())
}

async fn cmd_library_run(app: &App, name: &str, params_json: &str) -> Result<(), String> {
    // 优先尝试通过 IPC 发送给桌面 daemon
    if crate::ipc_client::IpcClient::is_daemon_available().await {
        let params_map: Option<std::collections::HashMap<String, String>> = if params_json != "{}" {
            let overrides: serde_json::Value = serde_json::from_str(params_json)
                .map_err(|e| format!("--params JSON 解析失败: {}", e))?;
            Some(
                overrides
                    .as_object()
                    .map(|obj| {
                        obj.iter()
                            .map(|(k, v)| {
                                (
                                    k.clone(),
                                    match v {
                                        serde_json::Value::String(s) => s.clone(),
                                        other => other.to_string(),
                                    },
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            )
        } else {
            None
        };
        return crate::ipc_client::IpcClient::library_run_remote(name, params_map).await;
    }

    // 回退：本地执行
    let templates = load_catalog()?;
    let entry = templates
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| format!("模板 '{}' 不存在", name))?;

    let file_path = library_dir().join(&entry.file);
    let mut content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("读取模板文件失败: {}", e))?;

    // 解析模板 JSON
    let wf: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("模板 JSON 格式错误: {}", e))?;

    // 收集默认参数
    let mut resolved: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(params) = wf.get("params").and_then(|v| v.as_object()) {
        for (k, v) in params {
            resolved.insert(
                k.clone(),
                match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                },
            );
        }
    }

    // 覆盖用户参数
    if params_json != "{}" {
        let overrides: serde_json::Value = serde_json::from_str(params_json)
            .map_err(|e| format!("--params JSON 解析失败: {}", e))?;
        if let Some(obj) = overrides.as_object() {
            for (k, v) in obj {
                resolved.insert(
                    k.clone(),
                    match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    },
                );
            }
        }
    }

    // 替换 {{params.xxx}} 占位符
    for (key, val) in &resolved {
        let placeholder = format!("{{{{params.{}}}}}", key);
        content = content.replace(&placeholder, val);
    }

    // 校验解析后的 JSON
    let _: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("参数替换后 JSON 格式错误: {}", e))?;

    // 导入到临时工作流并运行
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let label = format!("[库] {}", entry.name);
    app.db
        .create_workflow(&id, &label, "", &now, &now)
        .map_err(|e| format!("创建失败: {e}"))?;
    app.db
        .save_workflow_yaml(&id, &content)
        .map_err(|e| format!("保存失败: {e}"))?;

    // 调用 cmd_run 逻辑（复用已有实现，通过 db 中的工作流执行）
    // 直接委托给 cmd_run
    cmd_run(app, &id, &[]).await
}

fn cmd_library_validate(name: &str) -> Result<(), String> {
    let templates = load_catalog()?;
    let entry = templates
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| format!("模板 '{}' 不存在", name))?;

    let file_path = library_dir().join(&entry.file);
    let content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("读取模板文件失败: {}", e))?;

    // 验证 JSON 结构
    let wf: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON 格式错误: {}", e))?;

    // 检查 params 声明
    let declared_params = entry
        .params
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    if let Some(params) = wf.get("params").and_then(|v| v.as_object()) {
        let actual_params: std::collections::HashSet<String> = params.keys().cloned().collect();
        let missing: Vec<_> = declared_params.difference(&actual_params).collect();
        let extra: Vec<_> = actual_params.difference(&declared_params).collect();
        if !missing.is_empty() {
            return Err(format!("模板 JSON 缺少声明的参数: {:?}", missing));
        }
        if !extra.is_empty() {
            return Err(format!("模板 JSON 包含未声明的参数: {:?}", extra));
        }
    }

    // 验证含占位符的工作流结构（占位符不影响结构校验，但类型可能变化）
    // 这里仅验证 JSON 合法性和参数完整性，完整校验用 wf-cli validate
    let steps = wf
        .get("steps")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "模板缺少 steps 数组".to_string())?;
    println!("✓ 模板校验通过");
    println!("  名称:     {}", entry.name);
    println!("  版本:     {}", entry.version);
    println!("  参数:     {}", entry.params.join(", "));
    println!("  步骤数:   {}", steps.len());
    Ok(())
}

fn cmd_library_create(
    app: &App,
    name: &str,
    workflow_id: &str,
    category: &str,
    params_decl: &str,
) -> Result<(), String> {
    // 从 DB 导出工作流 JSON
    let yaml = app
        .db
        .get_workflow_yaml(workflow_id)
        .map_err(|e| format!("获取工作流失败: {e}"))?
        .ok_or_else(|| "工作流不存在".to_string())?;

    let mut wf: serde_json::Value =
        serde_json::from_str(&yaml).map_err(|e| format!("工作流 JSON 解析失败: {e}"))?;

    let original_name = wf
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("未命名")
        .to_string();

    let declared: Vec<String> = if params_decl.is_empty() {
        Vec::new()
    } else {
        params_decl
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    };

    // 构建默认 params 值
    let default_params: serde_json::Map<String, serde_json::Value> = declared
        .iter()
        .map(|p| {
            (
                p.clone(),
                serde_json::Value::String(format!("TODO: replace with {{{{params.{}}}}}", p)),
            )
        })
        .collect();

    if let Some(obj) = wf.as_object_mut() {
        obj.insert(
            "params".to_string(),
            serde_json::Value::Object(default_params),
        );
    }

    let template_json =
        serde_json::to_string_pretty(&wf).map_err(|e| format!("序列化失败: {e}"))?;

    let category_dir = library_dir().join(category);
    std::fs::create_dir_all(&category_dir).map_err(|e| format!("创建目录失败: {e}"))?;

    let file_name = format!("{}.wf.json", name);
    let file_path = category_dir.join(&file_name);
    std::fs::write(&file_path, &template_json).map_err(|e| format!("写入模板文件失败: {e}"))?;

    // 更新 catalog.toml
    let catalog_path = library_dir().join("catalog.toml");
    let relative_file = format!("{}/{}", category, file_name);

    let entry = format!(
        "\n[[templates]]\nname = \"{}\"\nversion = \"1.0.0\"\ndescription = \"{}\"\nparams = [{}]\nschedule = \"\"\ncategory = \"{}\"\nfile = \"{}\"\n",
        name,
        original_name,
        declared.iter().map(|p| format!("\"{}\"", p)).collect::<Vec<_>>().join(", "),
        category,
        relative_file
    );

    let mut catalog_content = std::fs::read_to_string(&catalog_path).unwrap_or_default();
    catalog_content.push_str(&entry);
    std::fs::write(&catalog_path, &catalog_content)
        .map_err(|e| format!("更新 catalog.toml 失败: {e}"))?;

    println!("✓ 模板已创建: {}", name);
    println!("  源工作流: {}", original_name);
    println!("  文件:     ~/.hermes/workflows/{}", relative_file);
    println!("  分类:     {}", category);
    if !declared.is_empty() {
        println!("  参数:     {}", declared.join(", "));
        println!("\n  请编辑模板文件，将硬编码值替换为 {{{{params.xxx}}}} 占位符。");
    } else {
        println!("  (未声明参数，可手动编辑模板添加 {{{{params.xxx}}}} 占位符)");
    }
    Ok(())
}

async fn cmd_library_schedule(
    app: &App,
    name: &str,
    cron_expr: &str,
    params_json: &str,
) -> Result<(), String> {
    use std::str::FromStr;

    cron::Schedule::from_str(cron_expr).map_err(|e| format!("无效的 cron 表达式: {e}"))?;

    let templates = load_catalog()?;
    let entry = templates
        .iter()
        .find(|t| t.name == name)
        .ok_or_else(|| format!("模板 '{}' 不存在", name))?;

    let file_path = library_dir().join(&entry.file);
    let mut content =
        std::fs::read_to_string(&file_path).map_err(|e| format!("读取模板文件失败: {e}"))?;

    let wf: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("模板 JSON 格式错误: {e}"))?;

    let mut resolved: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(params) = wf.get("params").and_then(|v| v.as_object()) {
        for (k, v) in params {
            resolved.insert(
                k.clone(),
                match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                },
            );
        }
    }
    if params_json != "{}" {
        let overrides: serde_json::Value = serde_json::from_str(params_json)
            .map_err(|e| format!("--params JSON 解析失败: {e}"))?;
        if let Some(obj) = overrides.as_object() {
            for (k, v) in obj {
                resolved.insert(
                    k.clone(),
                    match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    },
                );
            }
        }
    }
    for (key, val) in &resolved {
        let placeholder = format!("{{{{params.{}}}}}", key);
        content = content.replace(&placeholder, val);
    }

    let workflow_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let label = format!("[库调度] {}", entry.name);
    app.db
        .create_workflow(&workflow_id, &label, "", &now, &now)
        .map_err(|e| format!("创建失败: {e}"))?;
    app.db
        .save_workflow_yaml(&workflow_id, &content)
        .map_err(|e| format!("保存失败: {e}"))?;

    let schedule_id = uuid::Uuid::new_v4().to_string();
    app.db
        .create_schedule(&schedule_id, &workflow_id, cron_expr, &now)
        .map_err(|e| format!("创建调度失败: {e}"))?;

    println!("✓ 调度已创建");
    println!("  模板:     {}", name);
    println!("  Cron:     {}", cron_expr);
    println!("  工作流ID: {}", workflow_id);
    println!("  调度ID:   {}", schedule_id);
    Ok(())
}
// ═══════════════════════════════════════════════════
// v8: 工作流文件编辑命令（CLI 拥有和人一样的操作能力）
// ═══════════════════════════════════════════════════

/// 读取工作流 JSON 文件
fn read_workflow_file(file: &str) -> Result<(String, serde_json::Value), String> {
    let content =
        std::fs::read_to_string(file).map_err(|e| format!("无法读取文件 '{}': {}", file, e))?;
    let wf: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("JSON 格式错误: {}", e))?;
    Ok((content, wf))
}

/// 写入工作流 JSON 文件
fn write_workflow_file(file: &str, wf: &serde_json::Value) -> Result<(), String> {
    let content = serde_json::to_string_pretty(wf).map_err(|e| format!("序列化失败: {}", e))?;
    std::fs::write(file, &content).map_err(|e| format!("无法写入文件 '{}': {}", file, e))?;
    Ok(())
}

/// 获取或创建 steps 数组
fn get_steps_mut(wf: &mut serde_json::Value) -> &mut Vec<serde_json::Value> {
    if wf.get("steps").is_none() {
        wf["steps"] = serde_json::json!([]);
    }
    wf["steps"].as_array_mut().unwrap()
}

/// 生成下一个步骤 ID (step_N, N = 当前最大 + 1)
fn next_step_id(steps: &[serde_json::Value]) -> String {
    let max_n = steps
        .iter()
        .filter_map(|s| s["id"].as_str())
        .filter_map(|id| id.strip_prefix("step_"))
        .filter_map(|n| n.parse::<usize>().ok())
        .max()
        .unwrap_or(0);
    format!("step_{}", max_n + 1)
}

fn cmd_new(name: &str, output: Option<&str>, description: &str) -> Result<(), String> {
    let default_file = format!("{}.wf.json", name);
    let file = output.unwrap_or(&default_file);
    if std::path::Path::new(file).exists() {
        return Err(format!("文件已存在: {}", file));
    }
    let wf = serde_json::json!({
        "name": name,
        "description": description,
        "params": {},
        "steps": []
    });
    let content = serde_json::to_string_pretty(&wf).map_err(|e| format!("序列化失败: {}", e))?;
    std::fs::write(file, &content).map_err(|e| format!("写入失败: {}", e))?;
    println!("✓ 已创建: {}", file);
    println!("  名称: {}", name);
    if !description.is_empty() {
        println!("  描述: {}", description);
    }
    Ok(())
}

fn cmd_step(sub: StepCommand) -> Result<(), String> {
    match sub {
        StepCommand::Add {
            file,
            step_type,
            name,
            config,
            id,
            position,
        } => {
            let (_, mut wf) = read_workflow_file(&file)?;
            let steps = get_steps_mut(&mut wf);
            let config_val: serde_json::Value = serde_json::from_str(&config)
                .map_err(|e| format!("--config JSON 解析失败: {}", e))?;
            let step_id = id.unwrap_or_else(|| next_step_id(steps));
            let mut step = serde_json::json!({
                "id": step_id,
                "type": step_type,
                "label": name,
                "config": config_val,
            });
            let container_types = [
                "browser_container",
                "excel_container",
                "word_container",
                "file_container",
                "logic_container",
            ];
            if container_types.contains(&step_type.as_str()) {
                step["actions"] = serde_json::json!([]);
            }
            if let Some(pos) = position {
                steps.insert(pos.min(steps.len()), step);
            } else {
                steps.push(step);
            }
            write_workflow_file(&file, &wf)?;
            println!(" 已添加步骤: {} ({})", name, step_type);
        }
        StepCommand::List { file } => {
            let (_, wf) = read_workflow_file(&file)?;
            let steps_arr = wf["steps"].as_array().map(|a| a.as_slice()).unwrap_or(&[]);
            if steps_arr.is_empty() {
                println!("(空)");
            } else {
                for step in steps_arr {
                    let id = step["id"].as_str().unwrap_or("?");
                    let t = step["type"].as_str().unwrap_or("?");
                    let label = step["label"].as_str().unwrap_or("");
                    let has_actions = step
                        .get("actions")
                        .and_then(|a| a.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0);
                    if has_actions > 0 {
                        println!("  {}  [{:30}]  {}  (+{} )", id, t, label, has_actions);
                    } else {
                        println!("  {}  [{:30}]  {}", id, t, label);
                    }
                }
            }
        }
        StepCommand::Remove { file, id } => {
            let (_, mut wf) = read_workflow_file(&file)?;
            let steps = get_steps_mut(&mut wf);
            let idx = steps
                .iter()
                .position(|s| s["id"].as_str() == Some(&id))
                .ok_or_else(|| format!("找不到步骤: {}", id))?;
            let removed = steps.remove(idx);
            write_workflow_file(&file, &wf)?;
            println!(
                " 已删除步骤: {} ({})",
                id,
                removed["label"].as_str().unwrap_or("")
            );
        }
        StepCommand::Show { file, id } => {
            let (_, wf) = read_workflow_file(&file)?;
            let step = wf["steps"]
                .as_array()
                .and_then(|a| a.iter().find(|s| s["id"].as_str() == Some(&id)))
                .ok_or_else(|| format!("找不到步骤: {}", id))?;
            println!("{}", serde_json::to_string_pretty(step).unwrap_or_default());
        }
        StepCommand::Edit {
            file,
            id,
            name,
            config,
        } => {
            let (_, mut wf) = read_workflow_file(&file)?;
            let steps = get_steps_mut(&mut wf);
            let step = steps
                .iter_mut()
                .find(|s| s["id"].as_str() == Some(&id))
                .ok_or_else(|| format!("找不到步骤: {}", id))?;
            if let Some(n) = name {
                step["label"] = serde_json::Value::String(n);
            }
            if let Some(c) = config {
                let update: serde_json::Value = serde_json::from_str(&c)
                    .map_err(|e| format!("--config JSON 解析失败: {}", e))?;
                if let (Some(obj), Some(update_obj)) =
                    (step["config"].as_object_mut(), update.as_object())
                {
                    for (k, v) in update_obj {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
            write_workflow_file(&file, &wf)?;
            println!(" 已更新步骤: {}", id);
        }
    }
    Ok(())
}

fn cmd_action(sub: ActionCommand) -> Result<(), String> {
    match sub {
        ActionCommand::Add {
            file,
            step_id,
            action_type,
            config,
            id,
        } => {
            let (_, mut wf) = read_workflow_file(&file)?;
            let steps = get_steps_mut(&mut wf);
            let step = steps
                .iter_mut()
                .find(|s| s["id"].as_str() == Some(&step_id))
                .ok_or_else(|| format!("找不到步骤: {}", step_id))?;
            let config_val: serde_json::Value = serde_json::from_str(&config)
                .map_err(|e| format!("--config JSON 解析失败: {}", e))?;
            let actions = match step.get_mut("actions") {
                Some(a) => a.as_array_mut().ok_or("actions 不是数组")?,
                None => {
                    step["actions"] = serde_json::json!([]);
                    step["actions"].as_array_mut().unwrap()
                }
            };
            let action_id = id.unwrap_or_else(|| {
                let n = actions.len() + 1;
                let step_num = step_id.strip_prefix("step_").unwrap_or(&step_id);
                format!("a{}_{}", step_num, n)
            });
            let action = serde_json::json!({
                "id": action_id,
                "type": action_type,
                "config": config_val,
            });
            actions.push(action);
            write_workflow_file(&file, &wf)?;
            println!(
                " 已添加动作: {} -> {} ({})",
                step_id, action_id, action_type
            );
        }
        ActionCommand::List { file, step_id } => {
            let (_, wf) = read_workflow_file(&file)?;
            let step = wf["steps"]
                .as_array()
                .and_then(|a| a.iter().find(|s| s["id"].as_str() == Some(&step_id)))
                .ok_or_else(|| format!("找不到步骤: {}", step_id))?;
            let actions = step.get("actions").and_then(|a| a.as_array());
            match actions {
                Some(a) if !a.is_empty() => {
                    println!(
                        "步骤 {} ({}) :",
                        step_id,
                        step["label"].as_str().unwrap_or("")
                    );
                    for action in a {
                        let id = action["id"].as_str().unwrap_or("?");
                        let t = action["type"].as_str().unwrap_or("?");
                        println!("  {}  [{}]", id, t);
                    }
                }
                _ => println!("()"),
            }
        }
        ActionCommand::Remove { file, step_id, id } => {
            let (_, mut wf) = read_workflow_file(&file)?;
            let steps = get_steps_mut(&mut wf);
            let step = steps
                .iter_mut()
                .find(|s| s["id"].as_str() == Some(&step_id))
                .ok_or_else(|| format!("找不到步骤: {}", step_id))?;
            let actions = step
                .get_mut("actions")
                .and_then(|a| a.as_array_mut())
                .ok_or_else(|| format!("步骤 {} 没有 actions", step_id))?;
            let idx = actions
                .iter()
                .position(|a| a["id"].as_str() == Some(&id))
                .ok_or_else(|| format!("找不到动作: {}", id))?;
            actions.remove(idx);
            write_workflow_file(&file, &wf)?;
            println!(" 已删除动作: {} 中的 {}", step_id, id);
        }
    }
    Ok(())
}

fn cmd_show(file: &str) -> Result<(), String> {
    let (content, _) = read_workflow_file(file)?;
    println!("{}", content);
    Ok(())
}

async fn cmd_run_file(app: &App, file: &str, vars: &[(String, String)]) -> Result<(), String> {
    let (content, _) = read_workflow_file(file)?;
    // 注入变量
    let mut resolved_content = content.clone();
    if !vars.is_empty() {
        for (key, val) in vars {
            let placeholder = format!("{{{{{{params.{}}}}}}}", key);
            resolved_content = resolved_content.replace(&placeholder, val);
        }
    }
    let workflow =
        parser::parse_workflow(&resolved_content).map_err(|e| format!("解析失败: {e}"))?;

    let run_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let workflow_name = workflow.name.clone();
    app.db
        .create_run(&run_id, "runfile", &workflow_name, &now)
        .map_err(|e| format!("创建运行记录失败: {e}"))?;

    let _permit = app
        .run_semaphore
        .clone()
        .try_acquire_owned()
        .map_err(|_| "已达到最大并发工作流数限制".to_string())?;

    let ctrl = scheduler::RunControl {
        cancel_flag: Arc::new(AtomicBool::new(false)),
        cancel_token: tokio_util::sync::CancellationToken::new(),
        pause_flag: Arc::new(AtomicBool::new(false)),
        breakpoint_flag: Arc::new(AtomicBool::new(false)),
        step_mode_flag: Arc::new(AtomicBool::new(false)),
        debug_snapshots: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
    };

    let total = workflow.steps.len();
    println!("  {} (共 {} 步)", workflow_name, total);
    let start = std::time::Instant::now();

    let result = scheduler::run_workflow(
        &workflow,
        &run_id,
        None,
        &app.db,
        app.approval_store.clone(),
        "auto",
        vars,
        &ctrl,
    )
    .await;

    match result {
        Ok(_) => {
            app.db
                .update_run_status(&run_id, "completed", None)
                .map_err(|e| e.to_string())?;
            println!("  完成 ({:.1}s)", start.elapsed().as_secs_f64());
        }
        Err(e) => {
            app.db
                .update_run_status(&run_id, "failed", Some(&e.to_string()))
                .map_err(|e2| e2.to_string())?;
            println!("  失败: {}", e);
        }
    }
    Ok(())
}

fn cmd_preview(run_or_action: &str, step_id: Option<&str>, json: bool) -> Result<(), String> {
    use crate::engine::preview;

    match run_or_action {
        "list" => {
            let runs = preview::list_preview_runs();
            if runs.is_empty() {
                if json {
                    println!("{{\"runs\": []}}");
                } else {
                    println!("(无预览数据)");
                }
                return Ok(());
            }
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "runs": runs,
                        "count": runs.len(),
                    }))
                    .unwrap()
                );
            } else {
                println!("{:<40} {}", "Run ID", "步骤数");
                println!("{}", "-".repeat(60));
                for run_id in &runs {
                    let steps = preview::read_trajectory(run_id).len();
                    println!("{:<40} {} 步", run_id, steps);
                }
                println!("\n共 {} 次执行", runs.len());
            }
        }
        "latest" => {
            let mut runs = preview::list_preview_runs();
            if runs.is_empty() {
                eprintln!("没有找到预览数据。先运行一个工作流。");
                return Ok(());
            }
            runs.sort();
            let run_id = runs.last().unwrap();
            print_preview(run_id, step_id, json)?;
        }
        "live" => match step_id.as_deref() {
            Some("list") | None => {
                let sessions = preview::list_live_sessions();
                if sessions.is_empty() {
                    if json {
                        println!("{{\"sessions\": []}}");
                    } else {
                        println!("(无活跃会话)");
                    }
                    return Ok(());
                }
                if json {
                    println!("{}", serde_json::to_string_pretty(&sessions).unwrap());
                } else {
                    println!(
                        "{:<40} {:<20} {:<10} {}/{}",
                        "Run ID", "工作流", "状态", "进度", "总步数"
                    );
                    println!("{}", "-".repeat(90));
                    for s in &sessions {
                        println!(
                            "{:<40} {:<20} {:<10} {}/{}",
                            s.run_id,
                            truncate(&s.workflow_name, 20),
                            s.status,
                            s.step_index,
                            s.total_steps,
                        );
                    }
                    println!("\n共 {} 个活跃会话", sessions.len());
                }
            }
            Some(run_id) => match preview::get_live_status(run_id) {
                Some(session) => {
                    if json {
                        println!("{}", serde_json::to_string_pretty(&session).unwrap());
                    } else {
                        print_live_status(&session);
                    }
                }
                None => eprintln!("会话 {} 不存在或已结束", run_id),
            },
        },
        run_id => {
            print_preview(run_id, step_id, json)?;
        }
    }
    Ok(())
}

fn print_preview(run_id: &str, step_id: Option<&str>, json: bool) -> Result<(), String> {
    use crate::engine::preview;

    let trajectory = preview::read_trajectory(run_id);
    if trajectory.is_empty() {
        eprintln!("运行 {} 没有预览数据", run_id);
        return Ok(());
    }

    if let Some(sid) = step_id {
        if let Some(preview) = trajectory.iter().find(|p| p.step_id == sid) {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!(preview)).unwrap()
                );
            } else {
                println!("步骤: {} ({})", preview.step_name, preview.step_id);
                println!("类型: {}", preview.step_type);
                println!("状态: {} ({}ms)", preview.status, preview.duration_ms);
                println!("摘要: {}", preview.summary);
                println!(
                    "详情: {}",
                    serde_json::to_string_pretty(&preview.detail).unwrap()
                );
                if let Some(ref bundle_path) = preview.bundle_path {
                    println!("\nBundle: {}", bundle_path);
                    if let Ok(entries) = std::fs::read_dir(bundle_path) {
                        println!("  文件:");
                        for entry in entries.flatten() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                            println!("    {} ({}B)", name, size);
                        }
                    }
                }
            }
        } else {
            eprintln!("步骤 {} 在运行 {} 中未找到", sid, run_id);
        }
    } else {
        if json {
            println!("{}", serde_json::to_string_pretty(&trajectory).unwrap());
        } else {
            println!("运行: {}", run_id);
            println!(
                "{:<4} {:<18} {:<12} {:<8} {}",
                "#", "步骤名称", "类型", "状态", "摘要"
            );
            println!("{}", "-".repeat(90));
            for (i, p) in trajectory.iter().enumerate() {
                let status_icon = match p.status.as_str() {
                    "completed" => "✓",
                    "failed" => "✗",
                    "skipped" => "⏭",
                    _ => "?",
                };
                println!(
                    "{:<4} {:<18} {:<12} {:<8} {}",
                    i + 1,
                    truncate(&p.step_name, 18),
                    truncate(&p.step_type, 12),
                    format!("{} {}", status_icon, p.status),
                    p.summary,
                );
            }
            println!("\n共 {} 步", trajectory.len());
        }
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}

fn print_live_status(session: &crate::engine::preview::LiveSession) {
    println!("Run ID:      {}", session.run_id);
    println!("工作流:      {}", session.workflow_name);
    println!("状态:        {}", session.status);
    println!(
        "进度:        {}/{}",
        session.step_index, session.total_steps
    );
    if let Some(ref id) = session.current_step_id {
        println!(
            "当前步骤:    {} ({})",
            session.current_step_name.as_deref().unwrap_or("?"),
            id
        );
    }
    if let Some(ref bundle) = session.latest_bundle_path {
        println!("最新 Bundle: {}", bundle);
    }
    if !session.trajectory_summary.is_empty() {
        println!("\n步骤轨迹:");
        for (i, entry) in session.trajectory_summary.iter().enumerate() {
            let icon = match entry.status.as_str() {
                "completed" => "✓",
                "failed" => "✗",
                "skipped" => "⏭",
                _ => "?",
            };
            println!("  {} {} {} — {}", i + 1, icon, entry.step_id, entry.summary);
        }
    }
}

fn cmd_skill(app: &App, file_or_id: &str, output: Option<&str>) -> Result<(), String> {
    use crate::engine::{parser, skill_generator};

    // Try as file first, then as workflow ID
    let workflow = if let Ok(content) = std::fs::read_to_string(file_or_id) {
        parser::parse_workflow(&content).map_err(|e| format!("解析失败: {}", e))?
    } else {
        let yaml = app
            .db
            .get_workflow_yaml(file_or_id)
            .map_err(|e| format!("获取工作流失败: {}", e))?
            .ok_or_else(|| format!("工作流不存在: {}", file_or_id))?;
        parser::parse_workflow(&yaml).map_err(|e| format!("解析失败: {}", e))?
    };

    let skill_md = skill_generator::generate_skill(&workflow);

    match output {
        Some(path) => {
            std::fs::write(path, &skill_md).map_err(|e| format!("写入失败: {}", e))?;
            println!("✓ SKILL.md 已生成: {}", path);
        }
        None => println!("{}", skill_md),
    }
    Ok(())
}

async fn cmd_serve(app: Arc<App>, bind: &str, static_dir: &str) -> Result<(), String> {
    use tower_http::services::ServeDir;

    let router =
        crate::server::build_router(app).fallback_service(ServeDir::new(static_dir.to_string()));

    tracing::info!("服务器启动: http://{}  (静态文件: {})", bind, static_dir);

    let listener = tokio::net::TcpListener::bind(bind)
        .await
        .map_err(|e| format!("绑定地址失败: {}", e))?;

    axum::serve(listener, router.into_make_service())
        .await
        .map_err(|e| format!("服务器错误: {}", e))
}
