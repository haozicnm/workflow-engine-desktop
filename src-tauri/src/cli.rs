// cli.rs — 命令行接口
// 用法: workflow-engine.exe --cli <command> [args]

use clap::{Parser, Subcommand};
use crate::App;
use crate::engine::{parser, scheduler};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::collections::HashMap;

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
}

#[derive(Subcommand)]
pub enum LibraryCommand {
    /// 列出所有模板
    List {
        #[arg(long)]
        json: bool,
    },
    /// 查看模板详情
    Show {
        name: String,
    },
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
    Validate {
        name: String,
    },
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
    Delete {
        id: String,
    },
}

/// 解析 --var key=value 参数
fn parse_var(s: &str) -> Result<(String, String), String> {
    let (k, v) = s.split_once('=')
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
    }
}

fn cmd_list(app: &App, json: bool) -> Result<(), String> {
    let workflows = app.db.list_workflows().map_err(|e| format!("查询失败: {e}"))?;
    if json {
        let items: Vec<serde_json::Value> = workflows.iter().map(|w| {
            serde_json::json!({
                "id": w.id,
                "name": w.name,
                "updated_at": w.updated_at,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "workflows": items,
            "count": items.len(),
        })).unwrap());
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
    let yaml = app.db.get_workflow_yaml(workflow_id)
        .map_err(|e| format!("获取工作流失败: {e}"))?
        .ok_or_else(|| "工作流不存在".to_string())?;

    let workflow = parser::parse_workflow(&yaml)
        .map_err(|e| format!("解析失败: {e}"))?;

    let run_id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let workflow_name = workflow.name.clone();
    app.db.create_run(&run_id, workflow_id, &workflow_name, &now)
        .map_err(|e| format!("创建运行记录失败: {e}"))?;

    // 注入 --var 变量到 context（scheduler 内部会创建 ExecutionContext）
    // 由于 scheduler 内部管理 ctx，变量注入需在调用前通过 workflow.variables 传递
    if !vars.is_empty() {
        println!("注入变量: {}", vars.iter().map(|(k,v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(", "));
    }

    // 并发控制
    let _permit = app.run_semaphore.clone().try_acquire_owned()
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
        app.db.update_run_status(&run_id, "completed", None)
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
    ).await;

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
    let detail = app.db.get_run_detail(run_id)
        .map_err(|e| format!("查询失败: {e}"))?
        .ok_or_else(|| "运行记录不存在".to_string())?;

    if json {
        let steps: Vec<serde_json::Value> = detail.steps.iter().map(|s| {
            serde_json::json!({
                "step_id": s.step_id,
                "status": s.status,
                "output": s.output,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "run_id": detail.run.id,
            "workflow_name": detail.workflow_name,
            "status": detail.run.status,
            "started_at": detail.run.started_at,
            "finished_at": detail.run.finished_at,
            "error": detail.run.error,
            "steps": steps,
        })).unwrap());
        return Ok(());
    }

    println!("运行 ID:   {}", detail.run.id);
    println!("工作流:    {}", detail.workflow_name);
    println!("状态:      {}", detail.run.status);
    println!("开始:      {}", detail.run.started_at);
    println!("结束:      {}", detail.run.finished_at.as_deref().unwrap_or("-"));
    if let Some(ref err) = detail.run.error {
        println!("错误:      {}", err);
    }
    if !detail.steps.is_empty() {
        println!("\n步骤日志:");
        for s in &detail.steps {
            let icon = match s.status.as_str() {
                "success" => "✓", "error" => "✗", _ => " ",
            };
            println!("  {} {} - {}", icon, s.step_id, s.status);
        }
    }
    Ok(())
}

fn cmd_export(app: &App, workflow_id: &str, output: Option<&str>) -> Result<(), String> {
    let yaml = app.db.get_workflow_yaml(workflow_id)
        .map_err(|e| format!("获取失败: {e}"))?
        .ok_or_else(|| "工作流不存在".to_string())?;

    let value: serde_json::Value = serde_json::from_str(&yaml)
        .map_err(|e| format!("JSON 解析失败: {e}"))?;
    let pretty = serde_json::to_string_pretty(&value)
        .map_err(|e| format!("序列化失败: {e}"))?;

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
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("读取失败: {e}"))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("JSON 格式错误: {e}"))?;

    // 从 JSON 中读取工作流名称
    let name = value.get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("未命名工作流");

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    app.db.create_workflow(&id, name, "", &now, &now)
        .map_err(|e| format!("创建失败: {e}"))?;
    app.db.save_workflow_yaml(&id, &content)
        .map_err(|e| format!("保存失败: {e}"))?;
    println!("✓ 已导入: {} (ID: {})", name, id);
    Ok(())
}

fn cmd_validate(file: &str, json: bool) -> Result<(), String> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("读取失败: {e}"))?;
    let _: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("JSON 格式错误: {e}"))?;
    let workflow = parser::parse_workflow(&content)
        .map_err(|e| format!("工作流结构错误: {e}"))?;

    if json {
        let steps: Vec<serde_json::Value> = workflow.steps.iter().map(|s| {
            serde_json::json!({
                "name": s.name,
                "type": s.step_type,
                "next": s.next,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "valid": true,
            "name": workflow.name,
            "step_count": workflow.steps.len(),
            "steps": steps,
        })).unwrap());
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
    println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
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
        ScheduleCommand::Create { workflow_id, cron_expr } => cmd_schedule_create(app, &workflow_id, &cron_expr),
        ScheduleCommand::Delete { id } => cmd_schedule_delete(app, &id),
    }
}

fn cmd_schedule_list(app: &App, json: bool) -> Result<(), String> {
    let schedules = app.db.list_schedules().map_err(|e| format!("查询失败: {e}"))?;
    if json {
        let items: Vec<serde_json::Value> = schedules.iter().map(|s| {
            serde_json::json!({
                "id": s.id,
                "workflow_id": s.workflow_id,
                "workflow_name": s.workflow_name,
                "cron_expr": s.cron_expr,
                "enabled": s.enabled,
                "last_run_at": s.last_run_at,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "schedules": items,
            "count": items.len(),
        })).unwrap());
        return Ok(());
    }
    if schedules.is_empty() {
        println!("(无调度)");
        return Ok(());
    }
    println!("{:<38} {:<38} {:<20} {:<8} {:<20}", "ID", "工作流ID", "Cron", "启用", "上次运行");
    println!("{}", "-".repeat(130));
    for s in &schedules {
        let enabled = if s.enabled { "✓" } else { "✗" };
        let last = s.last_run_at.as_deref().unwrap_or("-");
        println!("{:<38} {:<38} {:<20} {:<8} {:<20}", s.id, s.workflow_id, s.cron_expr, enabled, last);
    }
    println!("\n共 {} 个调度", schedules.len());
    Ok(())
}

fn cmd_schedule_create(app: &App, workflow_id: &str, cron_expr: &str) -> Result<(), String> {
    use std::str::FromStr;
    // 验证 cron 表达式
    cron::Schedule::from_str(cron_expr)
        .map_err(|e| format!("无效的 cron 表达式: {e}"))?;
    // 验证工作流存在
    app.db.get_workflow_yaml(workflow_id)
        .map_err(|e| format!("查询失败: {e}"))?
        .ok_or_else(|| "工作流不存在".to_string())?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    app.db.create_schedule(&id, workflow_id, cron_expr, &now)
        .map_err(|e| format!("创建调度失败: {e}"))?;
    println!("✓ 调度已创建 (ID: {})", id);
    Ok(())
}

fn cmd_schedule_delete(app: &App, id: &str) -> Result<(), String> {
    app.db.delete_schedule(id)
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
        LibraryCommand::Create { name, from, category, params } => cmd_library_create(app, &name, &from, &category, &params),
        LibraryCommand::Run { name, params } => cmd_library_run(app, &name, &params).await,
        LibraryCommand::Validate { name } => cmd_library_validate(&name),
        LibraryCommand::Schedule { name, cron_expr, params } => cmd_library_schedule(app, &name, &cron_expr, &params).await,
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
    let catalog: Catalog = toml::from_str(&content)
        .map_err(|e| format!("解析 catalog.toml 失败: {}", e))?;
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
        let items: Vec<serde_json::Value> = templates.iter().map(|t| {
            serde_json::json!({
                "name": t.name,
                "version": t.version,
                "description": t.description,
                "params": t.params,
                "schedule": t.schedule,
                "category": t.category,
            })
        }).collect();
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "templates": items,
            "count": items.len(),
        })).unwrap());
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
        println!("{:<25} {:<8} {:<12} {:<30}", t.name, t.version, t.category, t.description);
        println!("  参数: {}  默认调度: {}", t.params.join(", "), schedule);
    }
    println!("\n共 {} 个模板", templates.len());
    Ok(())
}

fn cmd_library_show(name: &str) -> Result<(), String> {
    let templates = load_catalog()?;
    let entry = templates.iter().find(|t| t.name == name)
        .ok_or_else(|| format!("模板 '{}' 不存在", name))?;

    let file_path = library_dir().join(&entry.file);
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("读取模板文件失败: {}", e))?;
    let wf: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("模板 JSON 格式错误: {}", e))?;

    let params = wf.get("params").and_then(|v| v.as_object());
    let default_params: Vec<String> = params.map(|p| {
        p.iter().map(|(k, v)| format!("  {} = {}", k, v)).collect()
    }).unwrap_or_default();

    println!("名称:     {}", entry.name);
    println!("版本:     {}", entry.version);
    println!("分类:     {}", entry.category);
    println!("描述:     {}", entry.description);
    println!("声明参数: {}", entry.params.join(", "));
    println!("默认调度: {}", entry.schedule.as_deref().unwrap_or("手动触发"));
    if !default_params.is_empty() {
        println!("\n默认参数值:");
        for p in &default_params {
            println!("{}", p);
        }
    }
    println!("\n步骤数:   {}", wf.get("steps").and_then(|s| s.as_array()).map(|a| a.len()).unwrap_or(0));
    Ok(())
}

async fn cmd_library_run(app: &App, name: &str, params_json: &str) -> Result<(), String> {
    let templates = load_catalog()?;
    let entry = templates.iter().find(|t| t.name == name)
        .ok_or_else(|| format!("模板 '{}' 不存在", name))?;

    let file_path = library_dir().join(&entry.file);
    let mut content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("读取模板文件失败: {}", e))?;

    // 解析模板 JSON
    let wf: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("模板 JSON 格式错误: {}", e))?;

    // 收集默认参数
    let mut resolved: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(params) = wf.get("params").and_then(|v| v.as_object()) {
        for (k, v) in params {
            resolved.insert(k.clone(), match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            });
        }
    }

    // 覆盖用户参数
    if params_json != "{}" {
        let overrides: serde_json::Value = serde_json::from_str(params_json)
            .map_err(|e| format!("--params JSON 解析失败: {}", e))?;
        if let Some(obj) = overrides.as_object() {
            for (k, v) in obj {
                resolved.insert(k.clone(), match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                });
            }
        }
    }

    // 替换 {{params.xxx}} 占位符
    for (key, val) in &resolved {
        let placeholder = format!("{{{{params.{}}}}}", key);
        content = content.replace(&placeholder, val);
    }

    // 校验解析后的 JSON
    let _: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("参数替换后 JSON 格式错误: {}", e))?;

    // 导入到临时工作流并运行
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let label = format!("[库] {}", entry.name);
    app.db.create_workflow(&id, &label, "", &now, &now)
        .map_err(|e| format!("创建失败: {e}"))?;
    app.db.save_workflow_yaml(&id, &content)
        .map_err(|e| format!("保存失败: {e}"))?;

    // 调用 cmd_run 逻辑（复用已有实现，通过 db 中的工作流执行）
    // 直接委托给 cmd_run
    cmd_run(app, &id, &[]).await
}

fn cmd_library_validate(name: &str) -> Result<(), String> {
    let templates = load_catalog()?;
    let entry = templates.iter().find(|t| t.name == name)
        .ok_or_else(|| format!("模板 '{}' 不存在", name))?;

    let file_path = library_dir().join(&entry.file);
    let content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("读取模板文件失败: {}", e))?;

    // 验证 JSON 结构
    let wf: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("JSON 格式错误: {}", e))?;

    // 检查 params 声明
    let declared_params = entry.params.iter().cloned().collect::<std::collections::HashSet<_>>();
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
    let steps = wf.get("steps").and_then(|v| v.as_array())
        .ok_or_else(|| "模板缺少 steps 数组".to_string())?;
    println!("✓ 模板校验通过");
    println!("  名称:     {}", entry.name);
    println!("  版本:     {}", entry.version);
    println!("  参数:     {}", entry.params.join(", "));
    println!("  步骤数:   {}", steps.len());
    Ok(())
}

fn cmd_library_create(app: &App, name: &str, workflow_id: &str, category: &str, params_decl: &str) -> Result<(), String> {
    // 从 DB 导出工作流 JSON
    let yaml = app.db.get_workflow_yaml(workflow_id)
        .map_err(|e| format!("获取工作流失败: {e}"))?
        .ok_or_else(|| "工作流不存在".to_string())?;

    let mut wf: serde_json::Value = serde_json::from_str(&yaml)
        .map_err(|e| format!("工作流 JSON 解析失败: {e}"))?;

    let original_name = wf.get("name").and_then(|v| v.as_str()).unwrap_or("未命名").to_string();

    let declared: Vec<String> = if params_decl.is_empty() {
        Vec::new()
    } else {
        params_decl.split(',').map(|s| s.trim().to_string()).collect()
    };

    // 构建默认 params 值
    let default_params: serde_json::Map<String, serde_json::Value> = declared.iter()
        .map(|p| (p.clone(), serde_json::Value::String(format!("TODO: replace with {{{{params.{}}}}}", p))))
        .collect();

    if let Some(obj) = wf.as_object_mut() {
        obj.insert("params".to_string(), serde_json::Value::Object(default_params));
    }

    let template_json = serde_json::to_string_pretty(&wf)
        .map_err(|e| format!("序列化失败: {e}"))?;

    let category_dir = library_dir().join(category);
    std::fs::create_dir_all(&category_dir)
        .map_err(|e| format!("创建目录失败: {e}"))?;

    let file_name = format!("{}.wf.json", name);
    let file_path = category_dir.join(&file_name);
    std::fs::write(&file_path, &template_json)
        .map_err(|e| format!("写入模板文件失败: {e}"))?;

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

async fn cmd_library_schedule(app: &App, name: &str, cron_expr: &str, params_json: &str) -> Result<(), String> {
    use std::str::FromStr;

    cron::Schedule::from_str(cron_expr)
        .map_err(|e| format!("无效的 cron 表达式: {e}"))?;

    let templates = load_catalog()?;
    let entry = templates.iter().find(|t| t.name == name)
        .ok_or_else(|| format!("模板 '{}' 不存在", name))?;

    let file_path = library_dir().join(&entry.file);
    let mut content = std::fs::read_to_string(&file_path)
        .map_err(|e| format!("读取模板文件失败: {e}"))?;

    let wf: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("模板 JSON 格式错误: {e}"))?;

    let mut resolved: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(params) = wf.get("params").and_then(|v| v.as_object()) {
        for (k, v) in params {
            resolved.insert(k.clone(), match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            });
        }
    }
    if params_json != "{}" {
        let overrides: serde_json::Value = serde_json::from_str(params_json)
            .map_err(|e| format!("--params JSON 解析失败: {e}"))?;
        if let Some(obj) = overrides.as_object() {
            for (k, v) in obj {
                resolved.insert(k.clone(), match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                });
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
    app.db.create_workflow(&workflow_id, &label, "", &now, &now)
        .map_err(|e| format!("创建失败: {e}"))?;
    app.db.save_workflow_yaml(&workflow_id, &content)
        .map_err(|e| format!("保存失败: {e}"))?;

    let schedule_id = uuid::Uuid::new_v4().to_string();
    app.db.create_schedule(&schedule_id, &workflow_id, cron_expr, &now)
        .map_err(|e| format!("创建调度失败: {e}"))?;

    println!("✓ 调度已创建");
    println!("  模板:     {}", name);
    println!("  Cron:     {}", cron_expr);
    println!("  工作流ID: {}", workflow_id);
    println!("  调度ID:   {}", schedule_id);
    Ok(())
}
