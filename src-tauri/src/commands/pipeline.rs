// commands/pipeline.rs — 自动化管道执行命令
use tauri::State;
use crate::App;
use std::process::Command as StdCommand;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;

#[tauri::command]
pub async fn run_pipeline(
    _app: State<'_, App>,
    excel_path: Option<String>,
    template_path: Option<String>,
    output_path: Option<String>,
    use_browser: Option<bool>,
) -> Result<serde_json::Value, String> {
    let base = crate::data::paths::resolve_data_dir().join("examples");

    let script_path = base.join("run_full_pipeline.py");

    // 如果脚本不存在，从项目目录复制
    if !script_path.exists() {
        // 尝试从项目目录找到 examples 文件夹
        let project_examples = PathBuf::from("examples");
        if project_examples.exists() {
            std::fs::create_dir_all(&base).map_err(|e| e.to_string())?;
            // 复制必要的文件
            for file in &["run_full_pipeline.py", "test_data.xlsx", "report_template.docx"] {
                let src = project_examples.join(file);
                let dst = base.join(file);
                if src.exists() {
                    std::fs::copy(&src, &dst).map_err(|e| format!("复制 {} 失败: {}", file, e))?;
                }
            }
        }
    }

    let script = if script_path.exists() {
        script_path.to_string_lossy().to_string()
    } else {
        return Err(format!("管道脚本不存在: {:?}", script_path));
    };

    // 构建命令参数
    let mut args = vec![script];
    if let Some(ep) = excel_path {
        args.push("--excel".into());
        args.push(ep);
    }
    if let Some(tp) = template_path {
        args.push("--template".into());
        args.push(tp);
    }
    if let Some(op) = output_path {
        args.push("--output".into());
        args.push(op);
    }
    if use_browser.unwrap_or(false) {
        args.push("--browser".into());
        args.push("--headless".into());
    }

    // 执行 Python 管道
    let mut cmd = StdCommand::new("python");
    #[cfg(target_os = "windows")]
    cmd.creation_flags(0x08000000);
    let output = cmd
        .args(&args)
        .output()
        .map_err(|e| format!("执行 Python 失败: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        return Err(format!("管道执行失败:\n{}\n{}", stdout, stderr));
    }

    // 解析结果 JSON
    for line in stdout.lines() {
        if let Some(json_str) = line.strip_prefix("__RESULT_JSON__:") {
            if let Ok(result) = serde_json::from_str::<serde_json::Value>(json_str) {
                return Ok(result);
            }
        }
    }

    // 如果没有找到 JSON，返回 stdout
    Ok(serde_json::json!({
        "success": true,
        "output": stdout,
    }))
}
