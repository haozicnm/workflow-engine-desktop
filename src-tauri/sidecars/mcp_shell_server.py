"""MCP Shell Server — Cross-platform shell command execution.

Tool: shell_exec
  - command: shell command to execute
  - shell: auto/bash/powershell/cmd (default: auto-detect)
  - cwd: working directory (optional)
  - timeout_secs: timeout in seconds (default: 300)
  - env: environment variables dict (optional)

Returns: {stdout, stderr, exit_code}
"""

import json
import sys
import os
import subprocess
import platform

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mcp_protocol import McpServer, McpTool, log_stderr


def detect_shell() -> str:
    """Auto-detect the best shell for the current OS."""
    system = platform.system()
    if system == "Windows":
        # Check for PowerShell first, fallback to cmd
        try:
            subprocess.run(["powershell.exe", "-Command", "exit 0"],
                         capture_output=True, timeout=5)
            return "powershell"
        except Exception:
            return "cmd"
    else:
        return "bash"


class ShellExecTool(McpTool):
    def __init__(self):
        super().__init__(
            name="shell_exec",
            description="Execute a shell command (cross-platform)",
            input_schema={
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "Shell command to execute"},
                    "shell": {"type": "string", "description": "auto/bash/powershell/cmd"},
                    "cwd": {"type": "string", "description": "Working directory"},
                    "timeout_secs": {"type": "integer", "default": 300},
                    "env": {"type": "object", "description": "Extra environment variables"},
                },
                "required": ["command"],
            },
        )

    def execute(self, arguments: dict) -> dict:
        command = arguments["command"]
        shell_type = arguments.get("shell", "auto")
        cwd = arguments.get("cwd") or os.getcwd()
        timeout = arguments.get("timeout_secs", 300)
        env_vars = arguments.get("env", {})
        env = os.environ.copy()
        env.update(env_vars)

        if shell_type == "auto":
            shell_type = detect_shell()

        log_stderr(f"Shell [{shell_type}] @ {cwd}: {command[:200]}")

        try:
            if shell_type == "powershell":
                proc = subprocess.run(
                    ["powershell.exe", "-NoProfile", "-Command", command],
                    capture_output=True, text=True, cwd=cwd, timeout=timeout,
                    env=env, shell=False,
                )
            elif shell_type == "cmd":
                proc = subprocess.run(
                    ["cmd.exe", "/c", command],
                    capture_output=True, text=True, cwd=cwd, timeout=timeout,
                    env=env, shell=False,
                )
            elif shell_type == "bash":
                proc = subprocess.run(
                    command,
                    capture_output=True, text=True, cwd=cwd, timeout=timeout,
                    env=env, shell=True, executable="/bin/bash",
                )
            else:
                # Generic: use system default shell
                proc = subprocess.run(
                    command,
                    capture_output=True, text=True, cwd=cwd, timeout=timeout,
                    env=env, shell=True,
                )
        except subprocess.TimeoutExpired as e:
            return {"stdout": e.stdout or "", "stderr": f"Timeout after {timeout}s", "exit_code": -1}
        except FileNotFoundError as e:
            return {"stdout": "", "stderr": f"Shell not found: {e}", "exit_code": -1}
        except Exception as e:
            return {"stdout": "", "stderr": str(e), "exit_code": -1}

        return {
            "stdout": proc.stdout.strip(),
            "stderr": proc.stderr.strip(),
            "exit_code": proc.returncode,
            "shell": shell_type,
        }


def main():
    server = McpServer("wf-shell", "1.0.0")
    server.add_tool(ShellExecTool())
    server.run()


if __name__ == "__main__":
    main()
