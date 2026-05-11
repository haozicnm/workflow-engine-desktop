// cli.rs — 命令行接口
// 用法: workflow-engine.exe --cli <command> [args]

use clap::{Parser, Subcommand};
use crate::App;
use crate::engine::{parser, executor::StepExecutor, context::ExecutionContext};
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

    let mut ctx = ExecutionContext::new(&run_id, &workflow);
    // 注入 --var 变量
    for (k, v) in vars {
        ctx.set_var(k.clone(), serde_json::Value::String(v.clone()));
    }
    if !vars.is_empty() {
        println!("注入变量: {}", vars.iter().map(|(k,v)| format!("{}={}", k, v)).collect::<Vec<_>>().join(", "));
    }
    let executor = StepExecutor::new(app.approval_store.clone(), app.db.clone());

    if workflow.steps.is_empty() {
        app.db.update_run_status(&run_id, "completed", None)
            .map_err(|e| e.to_string())?;
        println!("✓ 工作流无步骤，直接完成");
        return Ok(());
    }

    println!("▶ {} (共 {} 步)", workflow_name, workflow.steps.len());

    let mut current_id = workflow.steps[0].id.clone();
    let total = workflow.steps.len();
    let start = std::time::Instant::now();

    loop {
        let pos = workflow.steps.iter().position(|s| s.id == current_id);
        let step = match pos {
            Some(i) => &workflow.steps[i],
            None => {
                eprintln!("  ✗ 步骤 {} 不存在", current_id);
                break;
            }
        };
        let idx = pos.unwrap();
        let name = step.name.clone();

        print!("  [{}/{}] {} ... ", idx + 1, total, name);

        match executor.execute(step, &mut ctx).await {
            Ok(output) => {
                ctx.set_output(&step.id, output);
                println!("✓ ({:.1}s)", start.elapsed().as_secs_f64());
            }
            Err(e) => {
                println!("✗");
                eprintln!("      错误: {}", e);
                let _ = app.db.update_run_status(&run_id, "failed", Some(&e.to_string()));
                return Err(format!("步骤 '{}' 失败: {}", name, e));
            }
        }

        match &step.next {
            Some(next) => current_id = next.clone(),
            None => break,
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let _ = app.db.update_run_status(&run_id, "completed", None);
    println!("\n✓ 完成 ({:.1}s)", elapsed);
    Ok(())
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
    let _: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("JSON 格式错误: {e}"))?;

    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Utc::now().to_rfc3339();
    app.db.create_workflow(&id, "导入工作流", "", &now, &now)
        .map_err(|e| format!("创建失败: {e}"))?;
    app.db.save_workflow_yaml(&id, &content)
        .map_err(|e| format!("保存失败: {e}"))?;
    println!("✓ 已导入 (ID: {})", id);
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
