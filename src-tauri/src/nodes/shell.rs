// nodes/shell.rs — Shell 命令节点：执行任意 shell 命令
//
// 这是 Workflow Engine 的"万能原语"——通过 shell 命令可以操控整台电脑。
// Shell 节点本身不要求用户有编程知识：命令由 Agent 生成，用户只需描述需求。
//
// 支持：
//   - command: 要执行的命令（支持 {{变量}} 引用）
//   - shell: shell 类型（bash / powershell / cmd），默认自动检测
//   - cwd: 工作目录（可选，支持 {{变量}}）
//   - timeout_secs: 超时秒数（默认 300）
//
// 输出：
//   { stdout: "...", stderr: "...", exit_code: 0 }

use crate::engine::context::ExecutionContext;
use crate::engine::executor::StepExecutor;
use crate::engine::workflow::Step;
use crate::nodes::error_utils;
use crate::nodes::traits::NodeExecutor;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;
use tracing::{info, warn};

#[derive(Default)]
pub struct ShellNode;

#[async_trait]
impl NodeExecutor for ShellNode {
    fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
        crate::nodes::traits::NodeTypeDef {
            type_name: "shell".into(),
            version: "1.0".into(),
            display_name: "执行命令".into(),
            description: "执行 Shell 命令并返回输出".into(),
            category: "系统".into(),
            inputs: vec![],
            outputs: vec![
                crate::nodes::traits::PortDef { label: "result".into(), data_type: "any".into(), required: false },
            ],
            config_schema: serde_json::json!({ "type": "object" }),
            params: vec![],
        }
    }

    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        // 1. 提取参数（config 已经过 resolve_config 做变量替换）
        let command = step
            .config
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| error_utils::missing_parameter("command", "shell").to_error())?
            .to_string();

        // 2. 白名单检查（如配置了 shell_allowed_commands，则仅允许匹配的命令）
        if !_ctx.shell_allowed_commands.is_empty() {
            let allowed = &_ctx.shell_allowed_commands;
            let first_token = command.split_whitespace().next().unwrap_or(&command);
            let command_name = first_token.trim_matches(|c| c == '"' || c == '\'');
            // 防止 shell 元字符绕过白名单（; && || | ` $）
            if command_name.contains([';', '&', '|', '`', '$']) {
                return Err(anyhow::anyhow!(
                    "Shell 命令名包含非法元字符: '{}'",
                    command_name
                ));
            }
            // 禁止白名单中包含解释器（防止 sh -c "任意命令" 绕过）
            const FORBIDDEN_INTERPRETERS: &[&str] = &["sh", "bash", "cmd", "cmd.exe", "powershell", "powershell.exe", "zsh", "fish", "python", "python3", "perl", "ruby", "node"];
            if FORBIDDEN_INTERPRETERS.contains(&command_name.to_lowercase().as_str()) {
                return Err(anyhow::anyhow!(
                    "安全限制: 白名单模式禁止直接调用解释器 '{}'",
                    command_name
                ));
            }
            let matched = allowed.iter().any(|pattern| {
                glob::Pattern::new(pattern)
                    .map(|p| p.matches(command_name))
                    .unwrap_or(false)
            });
            if !matched {
                return Err(anyhow::anyhow!(
                    "Shell 命令 '{}' 不在白名单中。允许的命令: {:?}",
                    command_name,
                    allowed
                ));
            }
        }

        let shell = step
            .config
            .get("shell")
            .and_then(|v| v.as_str())
            .unwrap_or("auto");

        let cwd = step
            .config
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let timeout_secs = step
            .config
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(300)
            .min(3600); // 最大 1 小时，防止意外无限阻塞

        // 3. 跨平台命令适配
        let command = adapt_command(&command);

        // 4. 解析 shell
        let (shell_cmd, shell_arg) = resolve_shell(shell);

        info!(
            "Shell 执行: shell={} (resolved from '{}'), cmd=\"{}\", timeout={}s",
            shell_cmd,
            shell,
            if command.len() > 80 {
                format!("{}...", truncate_str(&command, 77))
            } else {
                command.clone()
            },
            timeout_secs
        );

        // 5. 执行命令
        let mut cmd = Command::new(&shell_cmd);
        #[cfg(target_os = "windows")]
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW — 不弹命令行窗口
        cmd.arg(&shell_arg);
        cmd.arg(&command);

        // WSL 环境下，stdin 可能会导致 powershell.exe 管道问题
        cmd.stdin(std::process::Stdio::null());

        if let Some(ref dir) = cwd {
            if !dir.is_empty() {
                cmd.current_dir(dir);
            }
        }

        let handle = tokio::task::spawn_blocking(move || cmd.output());
        let output = match tokio::time::timeout(Duration::from_secs(timeout_secs), handle).await {
            Ok(Ok(inner)) => inner
                .map_err(|e| error_utils::execution_failed("Shell 命令启动", &e.to_string()).to_error())?,
            Ok(Err(join_err)) => {
                return Err(error_utils::execution_failed("Shell 命令执行", &join_err.to_string()).to_error());
            }
            Err(_elapsed) => {
                // 超时：任务会被 drop，但 OS 进程可能仍在运行
                warn!("Shell 命令超时 ({}s)", timeout_secs);
                return Err(error_utils::execution_failed(
                    "Shell 命令",
                    &format!("执行超时 ({}s)", timeout_secs),
                ).to_error());
            }
        };

        let stdout_raw = String::from_utf8_lossy(&output.stdout);
        let stdout = stdout_raw.trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        info!(
            "Shell stdout raw (first 100 chars): {}",
            truncate_str(&stdout_raw, 100)
        );
        info!(
            "Shell stdout trimmed (first 100 chars): {}",
            truncate_str(&stdout, 100)
        );
        let exit_code = output.status.code().unwrap_or(-1);

        // 6. 记录并返回
        if exit_code != 0 {
            warn!(
                "Shell 命令非零退出: exit_code={}, stderr={}",
                exit_code,
                if stderr.len() > 200 {
                    format!("{}...", truncate_str(&stderr, 197))
                } else {
                    stderr.clone()
                }
            );
            // 非零退出码仍然返回结果，由上层 onError 策略决定是否继续
            // 已移至下方统一处理
        }

        let result = json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
        });

        info!(
            "Shell 完成: exit_code={}, stdout_len={}",
            exit_code,
            stdout.len()
        );

        if exit_code != 0 {
            // 返回错误让 scheduler 根据 onError 策略处理
            Err(error_utils::execution_failed(
                "Shell 命令",
                &format!("退出码 {}: {}", exit_code, if stderr.is_empty() {
                    "(无错误输出)".to_string()
                } else {
                    stderr.clone()
                }),
            ).to_error())
        } else {
            Ok(result)
        }
    }
}

/// UTF-8 safe string truncation at byte boundary
fn truncate_str(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

/// 解析 shell 类型 → (shell_binary, flag_for_command)
fn resolve_shell(shell: &str) -> (String, String) {
    match shell {
        "bash" | "sh" => ("bash".into(), "-c".into()),
        "powershell" | "pwsh" => ("powershell".into(), "-Command".into()),
        "cmd" => ("cmd".into(), "/C".into()),
        _ => {
            // "auto" or any other
            #[cfg(target_os = "windows")]
            {
                ("cmd".into(), "/C".into())
            }
            #[cfg(not(target_os = "windows"))]
            {
                ("bash".into(), "-c".into())
            }
        }
    }
}

/// 跨平台命令适配：将常见 Unix 命令转换为 Windows 等价命令
#[cfg(target_os = "windows")]
fn adapt_command(cmd: &str) -> String {
    let mut adapted = cmd.to_string();

    // mkdir -p → mkdir (Windows 自动创建父目录)
    if adapted.contains("mkdir -p") {
        adapted = adapted.replace("mkdir -p", "mkdir");
    }

    // rm -f / rm -rf → del /f /q 或 rmdir /s /q
    if adapted.contains("rm -rf") {
        adapted = adapted.replace("rm -rf", "rmdir /s /q");
    } else if adapted.contains("rm -f") {
        adapted = adapted.replace("rm -f", "del /f /q");
    }

    // 2>/dev/null → >NUL 2>&1
    if adapted.contains("2>/dev/null") {
        adapted = adapted.replace("2>/dev/null", ">NUL 2>&1");
    }

    // touch → type NUL >> (create empty file)
    if adapted.starts_with("touch ") {
        let path = adapted.trim_start_matches("touch ").trim();
        // Only convert simple touch; piped commands remain as-is
        if !path.contains('|') && !path.contains('&') {
            adapted = format!("type NUL > {}", path);
        }
    }

    // echo '...' → strip single quotes (cmd doesn't understand them)
    if adapted.starts_with("echo '") && adapted.ends_with('\'') {
        adapted = format!("echo {}", &adapted[6..adapted.len() - 1]);
    }

    if adapted != cmd {
        info!("Shell 命令已适配 Windows: \"{}\" → \"{}\"", cmd, adapted);
    }
    adapted
}

#[cfg(not(target_os = "windows"))]
fn adapt_command(cmd: &str) -> String {
    cmd.to_string()
}
