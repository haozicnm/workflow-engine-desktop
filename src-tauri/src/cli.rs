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
    List,
    /// 运行工作流
    Run { id: String },
    /// 查看运行状态
    Status { run_id: String },
    /// 导出工作流 (JSON)
    Export {
        id: String,
        #[arg(short = 'o', long)]
        output: Option<String>,
    },
    /// 导入工作流
    Import { file: String },
    /// 校验工作流文件
    Validate { file: String },
}

pub async fn run_cli(cli: Cli, app: Arc<App>) -> Result<(), String> {
    match cli.command {
        Commands::List => cmd_list(&app),
        Commands::Run { id } => cmd_run(&app, &id).await,
        Commands::Status { run_id } => cmd_status(&app, &run_id),
        Commands::Export { id, output } => cmd_export(&app, &id, output.as_deref()),
        Commands::Import { file } => cmd_import(&app, &file),
        Commands::Validate { file } => cmd_validate(&file),
    }
}

fn cmd_list(app: &App) -> Result<(), String> {
    let workflows = app.db.list_workflows().map_err(|e| format!("查询失败: {e}"))?;
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

async fn cmd_run(app: &App, workflow_id: &str) -> Result<(), String> {
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
        let name = step.label.clone();

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

fn cmd_status(app: &App, run_id: &str) -> Result<(), String> {
    let detail = app.db.get_run_detail(run_id)
        .map_err(|e| format!("查询失败: {e}"))?
        .ok_or_else(|| "运行记录不存在".to_string())?;

    println!("运行 ID:   {}", detail.run_id);
    println!("工作流:    {}", detail.workflow_name);
    println!("状态:      {}", detail.status);
    println!("开始:      {}", detail.started_at.as_deref().unwrap_or("-"));
    println!("结束:      {}", detail.finished_at.as_deref().unwrap_or("-"));
    if let Some(ref err) = detail.error {
        println!("错误:      {}", err);
    }
    if !detail.steps.is_empty() {
        println!("\n步骤日志:");
        for s in &detail.steps {
            let icon = match s.status.as_str() {
                "success" => "✓", "error" => "✗", _ => " ",
            };
            println!("  {} {} - {}", icon, s.step_name, s.status);
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

fn cmd_validate(file: &str) -> Result<(), String> {
    let content = std::fs::read_to_string(file)
        .map_err(|e| format!("读取失败: {e}"))?;
    let _: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("JSON 格式错误: {e}"))?;
    let workflow = parser::parse_workflow(&content)
        .map_err(|e| format!("工作流结构错误: {e}"))?;

    println!("✓ 校验通过");
    println!("  名称: {}", workflow.name);
    println!("  步骤数: {}", workflow.steps.len());
    for (i, s) in workflow.steps.iter().enumerate() {
        println!("  {}. {} ({})", i + 1, s.label, s.step_type);
    }
    Ok(())
}
