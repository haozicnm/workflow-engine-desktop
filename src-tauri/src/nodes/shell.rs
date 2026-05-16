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

use async_trait::async_trait;
use crate::engine::workflow::Step;
use crate::engine::context::ExecutionContext;
use crate::nodes::traits::NodeExecutor;
use crate::engine::executor::StepExecutor;
use std::sync::Arc;
use std::process::Command;
use anyhow::{Result, anyhow};
use serde_json::json;
use tracing::{info, warn};

#[derive(Default)]
pub struct ShellNode;

#[async_trait]
impl NodeExecutor for ShellNode {
    async fn execute(
        &self,
        step: &Step,
        _ctx: &mut ExecutionContext,
        _executor: &Arc<StepExecutor>,
    ) -> Result<serde_json::Value> {
        // 1. 提取参数（config 已经过 resolve_config 做变量替换）
        let command = step.config
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Shell 节点缺少 command 参数"))?
            .to_string();

        let shell = step.config
            .get("shell")
            .and_then(|v| v.as_str())
            .unwrap_or("auto");

        let cwd = step.config
            .get("cwd")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let timeout_secs = step.config
            .get("timeout_secs")
            .and_then(|v| v.as_u64())
            .unwrap_or(300);

        // 2. 解析 shell
        let (shell_cmd, shell_arg) = resolve_shell(shell);

        info!(
            "Shell 执行: shell={}, cmd=\"{}\", timeout={}s",
            shell_cmd,
            if command.len() > 80 { format!("{}...", &command[..77]) } else { command.clone() },
            timeout_secs
        );

        // 3. 执行命令
        let mut cmd = Command::new(&shell_cmd);
        cmd.arg(&shell_arg);
        cmd.arg(&command);

        // WSL 环境下，stdin 可能会导致 powershell.exe 管道问题
        cmd.stdin(std::process::Stdio::null());

        if let Some(ref dir) = cwd {
            if !dir.is_empty() {
                cmd.current_dir(dir);
            }
        }

        let output = tokio::task::spawn_blocking(move || {
            cmd.output()
        }).await
            .map_err(|e| anyhow!("Shell 命令执行失败: {}", e))?
            .map_err(|e| anyhow!("Shell 命令启动失败: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let exit_code = output.status.code().unwrap_or(-1);

        // 4. 记录并返回
        if exit_code != 0 {
            warn!(
                "Shell 命令非零退出: exit_code={}, stderr={}",
                exit_code,
                if stderr.len() > 200 { format!("{}...", &stderr[..197]) } else { stderr.clone() }
            );
            // 非零退出码仍然返回结果，由上层 onError 策略决定是否继续
        }

        let result = json!({
            "stdout": stdout,
            "stderr": stderr,
            "exit_code": exit_code,
        });

        info!(
            "Shell 完成: exit_code={}, stdout_len={}",
            exit_code, stdout.len()
        );

        if exit_code != 0 {
            // 返回错误让 scheduler 根据 onError 策略处理
            Err(anyhow!(
                "Shell 命令退出码 {}: {}",
                exit_code,
                if stderr.is_empty() { "(无错误输出)" } else { &stderr }
            ))
        } else {
            Ok(result)
        }
    }
}

/// 解析 shell 类型 → (shell_binary, flag_for_command)
fn resolve_shell(shell: &str) -> (String, String) {
    match shell {
        "bash" | "sh" => ("bash".into(), "-c".into()),
        "powershell" | "pwsh" => ("powershell".into(), "-Command".into()),
        "cmd" => ("cmd".into(), "/C".into()),
        "auto" | _ => {
            #[cfg(target_os = "windows")]
            { ("powershell".into(), "-Command".into()) }
            #[cfg(not(target_os = "windows"))]
            { ("bash".into(), "-c".into()) }
        }
    }
}
