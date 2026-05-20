"""MCP Samba Server — Samba 管理 (shares + users + status) + GUI 启动.

Tools:
  samba_gui    — 启动/检查 Web 管理面板, 可选自动弹浏览器窗口
  samba_status — 系统状态
  samba_shares — 共享管理 (list/create/update/delete)
  samba_users  — 用户管理 (list/create/delete/password)
"""

import json
import os
import socket
import subprocess
import sys
import webbrowser
from pathlib import Path
from configparser import ConfigParser

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mcp_protocol import McpServer, McpTool, log_stderr


# ── Config ────────────────────────────────────────────────
SMB_CONF = Path("/etc/samba/smb.conf")
WEB_PANEL_DIR = Path("/opt/samba-web-manager")
WEB_PANEL_PORT = 8080
SHARE_ROOT = Path("/data/share")


# ── Helpers ───────────────────────────────────────────────
def run_sudo(cmd: list[str], input_text: str = None) -> tuple[int, str, str]:
    """Run sudo command, return (code, stdout, stderr)."""
    full_cmd = ["sudo", "-n"] + cmd  # -n = non-interactive, fail if password needed
    try:
        result = subprocess.run(
            full_cmd,
            input=input_text,
            capture_output=True,
            text=True,
            timeout=30,
        )
        return result.returncode, result.stdout.strip(), result.stderr.strip()
    except subprocess.TimeoutExpired:
        return -1, "", "timeout"
    except FileNotFoundError:
        return -1, "", f"command not found: {cmd[0]}"


def is_port_open(port: int) -> bool:
    """Check if web panel is running on given port."""
    sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    sock.settimeout(1)
    try:
        sock.connect(("127.0.0.1", port))
        sock.close()
        return True
    except (socket.error, OSError):
        return False


def start_web_panel() -> dict:
    """Start web panel in background if not running. Returns status dict."""
    if is_port_open(WEB_PANEL_PORT):
        return {"running": True, "url": f"http://localhost:{WEB_PANEL_PORT}", "started": False}

    if not (WEB_PANEL_DIR / "server.py").exists():
        return {"running": False, "error": f"Web panel not installed at {WEB_PANEL_DIR}", "started": False}

    try:
        subprocess.Popen(
            ["python3", str(WEB_PANEL_DIR / "server.py")],
            cwd=str(WEB_PANEL_DIR),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            start_new_session=True,
        )
        # Wait briefly for startup
        import time
        for _ in range(10):
            time.sleep(0.2)
            if is_port_open(WEB_PANEL_PORT):
                return {"running": True, "url": f"http://localhost:{WEB_PANEL_PORT}", "started": True}
        return {"running": False, "error": "Web panel started but not responding", "started": True}
    except Exception as e:
        return {"running": False, "error": str(e), "started": False}


def parse_smb_conf() -> ConfigParser:
    """Parse smb.conf."""
    cp = ConfigParser()
    cp.optionxform = str
    if SMB_CONF.exists():
        with open(SMB_CONF) as f:
            cp.read_file(f)
    return cp


# ── Tool: samba_gui ───────────────────────────────────────
class SambaGuiTool(McpTool):
    def __init__(self):
        super().__init__(
            name="samba_gui",
            description="启动或检查 Samba Web 管理面板",
            input_schema={
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["open", "status"],
                        "description": "open=启动面板+可选弹窗, status=检查运行状态",
                    },
                    "open_browser": {
                        "type": "boolean",
                        "description": "open 时是否自动弹浏览器 (默认 true)",
                    },
                },
                "required": ["action"],
            },
        )

    def execute(self, arguments: dict) -> dict:
        action = arguments.get("action", "status")
        if action == "status":
            running = is_port_open(WEB_PANEL_PORT)
            return {
                "action": "status",
                "running": running,
                "url": f"http://localhost:{WEB_PANEL_PORT}" if running else None,
            }
        elif action == "open":
            result = {"action": "open", **start_web_panel()}
            # Auto-open browser (default: true)
            if arguments.get("open_browser", True) and result.get("url"):
                try:
                    webbrowser.open(result["url"])
                    result["browser_opened"] = True
                except Exception as e:
                    log_stderr(f"Failed to open browser: {e}")
                    result["browser_opened"] = False
            return result
        return {"error": f"Unknown action: {action}"}


# ── Tool: samba_status ────────────────────────────────────
class SambaStatusTool(McpTool):
    def __init__(self):
        super().__init__(
            name="samba_status",
            description="获取 Samba 系统状态（服务、磁盘、统计）",
            input_schema={"type": "object", "properties": {}},
        )

    def execute(self, arguments: dict) -> dict:
        # smbd status
        rc, out, _ = run_sudo(["systemctl", "is-active", "smbd"])
        smbd = "running" if out == "active" else "stopped"

        rc, out, _ = run_sudo(["systemctl", "is-active", "nmbd"])
        nmbd = "running" if out == "active" else "stopped"

        # disk
        rc, out, _ = run_sudo(["df", "-h", str(SHARE_ROOT)])
        disk = {}
        lines = out.split("\n")
        if len(lines) >= 2:
            parts = lines[1].split()
            if len(parts) >= 6:
                disk = {
                    "filesystem": parts[0],
                    "size": parts[1],
                    "used": parts[2],
                    "available": parts[3],
                    "use_percent": parts[4],
                    "mount": parts[5],
                }

        # share count
        cp = parse_smb_conf()
        shares = [s for s in cp.sections() if s not in ("global", "homes", "printers", "print$")]

        # user count
        rc, out, _ = run_sudo(["pdbedit", "-L"])
        users = [u.split(":")[0] for u in out.split("\n") if u]

        # web panel
        gui_running = is_port_open(WEB_PANEL_PORT)

        return {
            "smbd": smbd,
            "nmbd": nmbd,
            "disk": disk,
            "share_count": len(shares),
            "user_count": len(users),
            "gui_running": gui_running,
            "gui_url": f"http://localhost:{WEB_PANEL_PORT}" if gui_running else None,
        }


# ── Tool: samba_shares ────────────────────────────────────
class SambaSharesTool(McpTool):
    def __init__(self):
        super().__init__(
            name="samba_shares",
            description="管理 Samba 共享 (list/create/update/delete)",
            input_schema={
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "create", "update", "delete"],
                    },
                    "name": {"type": "string"},
                    "path": {"type": "string"},
                    "comment": {"type": "string"},
                    "writable": {"type": "string", "enum": ["yes", "no"]},
                    "guest_ok": {"type": "string", "enum": ["yes", "no"]},
                    "valid_users": {"type": "string"},
                },
                "required": ["action"],
            },
        )

    def execute(self, arguments: dict) -> dict:
        action = arguments.get("action", "list")

        if action == "list":
            cp = parse_smb_conf()
            shares = []
            for section in cp.sections():
                if section in ("global", "homes", "printers", "print$"):
                    continue
                share = {"name": section}
                for key, value in cp.items(section):
                    share[key] = value
                shares.append(share)
            return {"action": "list", "shares": shares, "count": len(shares)}

        elif action == "create":
            name = arguments.get("name", "").strip()
            path = arguments.get("path", "").strip()
            if not name or not path:
                return {"action": "create", "error": "name and path required"}

            # Read existing config
            with open(SMB_CONF) as f:
                lines = f.readlines()

            # Build new section
            section = [f"\n[{name}]\n", f"   path = {path}\n"]
            if arguments.get("comment"):
                section.append(f"   comment = {arguments['comment']}\n")
            section.append(f"   writable = {arguments.get('writable', 'yes')}\n")
            section.append(f"   guest ok = {arguments.get('guest_ok', 'no')}\n")
            section.append("   browseable = yes\n")
            if arguments.get("valid_users"):
                section.append(f"   valid users = {arguments['valid_users']}\n")

            lines.extend(section)
            with open(SMB_CONF, "w") as f:
                f.writelines(lines)

            # Create directory
            run_sudo(["mkdir", "-p", path])
            run_sudo(["chmod", "755", path])
            run_sudo(["systemctl", "reload", "smbd"])

            return {"action": "create", "success": True, "name": name, "path": path}

        elif action == "update":
            name = arguments.get("name", "").strip()
            if not name:
                return {"action": "update", "error": "name required"}

            cp = parse_smb_conf()
            if name not in cp.sections():
                return {"action": "update", "error": f"share '{name}' not found"}

            # Read raw lines
            with open(SMB_CONF) as f:
                lines = f.readlines()

            # Find section start/end
            start = None
            for i, line in enumerate(lines):
                if line.strip() == f"[{name}]":
                    start = i
                    break
            if start is None:
                return {"action": "update", "error": "section not found"}

            end = len(lines)
            for i in range(start + 1, len(lines)):
                if lines[i].strip().startswith("["):
                    end = i
                    break

            # Rebuild section
            new_section = [f"[{name}]\n"]
            updatable = ["path", "comment", "writable", "guest_ok", "valid_users"]
            existing_keys = set()
            for line in lines[start + 1 : end]:
                stripped = line.strip()
                if "=" in stripped and not stripped.startswith(("#", ";")):
                    key = stripped.split("=")[0].strip()
                    existing_keys.add(key)
                    if key in updatable and key in arguments:
                        new_section.append(f"   {key} = {arguments[key]}\n")
                    else:
                        new_section.append(line)

            for key in updatable:
                if key in arguments and key not in existing_keys:
                    new_section.append(f"   {key} = {arguments[key]}\n")

            lines[start:end] = new_section
            with open(SMB_CONF, "w") as f:
                f.writelines(lines)

            if "path" in arguments:
                run_sudo(["mkdir", "-p", arguments["path"]])
            run_sudo(["systemctl", "reload", "smbd"])

            return {"action": "update", "success": True, "name": name}

        elif action == "delete":
            name = arguments.get("name", "").strip()
            if not name:
                return {"action": "delete", "error": "name required"}

            with open(SMB_CONF) as f:
                lines = f.readlines()

            start = None
            for i, line in enumerate(lines):
                if line.strip() == f"[{name}]":
                    start = i
                    break
            if start is None:
                return {"action": "delete", "error": f"share '{name}' not found"}

            end = len(lines)
            for i in range(start + 1, len(lines)):
                if lines[i].strip().startswith("["):
                    end = i
                    break

            del lines[start:end]
            with open(SMB_CONF, "w") as f:
                f.writelines(lines)

            run_sudo(["systemctl", "reload", "smbd"])
            return {"action": "delete", "success": True, "name": name}

        return {"error": f"Unknown action: {action}"}


# ── Tool: samba_users ─────────────────────────────────────
class SambaUsersTool(McpTool):
    def __init__(self):
        super().__init__(
            name="samba_users",
            description="管理 Samba 用户 (list/create/delete/password)",
            input_schema={
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["list", "create", "delete", "password"],
                    },
                    "username": {"type": "string"},
                    "password": {"type": "string"},
                    "full_name": {"type": "string"},
                },
                "required": ["action"],
            },
        )

    def execute(self, arguments: dict) -> dict:
        action = arguments.get("action", "list")

        if action == "list":
            rc, out, err = run_sudo(["pdbedit", "-L"])
            if rc != 0:
                return {"action": "list", "error": err}
            users = []
            for line in out.split("\n"):
                if not line:
                    continue
                parts = line.split(":")
                users.append({
                    "username": parts[0],
                    "full_name": parts[2] if len(parts) > 2 else "",
                })
            return {"action": "list", "users": users, "count": len(users)}

        elif action == "create":
            username = arguments.get("username", "").strip()
            password = arguments.get("password", "").strip()
            if not username or not password:
                return {"action": "create", "error": "username and password required"}
            if len(password) < 4:
                return {"action": "create", "error": "password too short (min 4)"}

            # Check if Samba user exists
            rc, out, _ = run_sudo(["pdbedit", "-L"])
            if username in [l.split(":")[0] for l in out.split("\n") if l]:
                return {"action": "create", "error": f"user '{username}' already exists"}

            # Create system user if needed
            rc, _, _ = run_sudo(["id", username])
            if rc != 0:
                cmd = ["useradd", "-M", "-s", "/sbin/nologin"]
                if arguments.get("full_name"):
                    cmd.extend(["-c", arguments["full_name"]])
                cmd.append(username)
                rc, _, err = run_sudo(cmd)
                if rc != 0:
                    return {"action": "create", "error": f"system user creation failed: {err}"}

            # Set Samba password
            rc, _, err = run_sudo(
                ["smbpasswd", "-a", "-s", username],
                input_text=f"{password}\n{password}\n",
            )
            if rc != 0:
                return {"action": "create", "error": f"smbpasswd failed: {err}"}

            return {"action": "create", "success": True, "username": username}

        elif action == "delete":
            username = arguments.get("username", "").strip()
            if not username:
                return {"action": "delete", "error": "username required"}

            rc, _, err = run_sudo(["smbpasswd", "-x", username])
            if rc != 0:
                return {"action": "delete", "error": err}

            return {"action": "delete", "success": True, "username": username}

        elif action == "password":
            username = arguments.get("username", "").strip()
            password = arguments.get("password", "").strip()
            if not username or not password:
                return {"action": "password", "error": "username and password required"}
            if len(password) < 4:
                return {"action": "password", "error": "password too short (min 4)"}

            rc, _, err = run_sudo(
                ["smbpasswd", "-a", "-s", username],
                input_text=f"{password}\n{password}\n",
            )
            if rc != 0:
                return {"action": "password", "error": err}

            return {"action": "password", "success": True, "username": username}

        return {"error": f"Unknown action: {action}"}


# ── Tool: samba_service ────────────────────────────────────
class SambaServiceTool(McpTool):
    def __init__(self):
        super().__init__(
            name="samba_service",
            description="控制 Samba 服务 (start/stop/restart/status)",
            input_schema={
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["start", "stop", "restart", "status"],
                        "description": "start=启动, stop=停止, restart=重启, status=状态",
                    },
                },
                "required": ["action"],
            },
        )

    def execute(self, arguments: dict) -> dict:
        action = arguments.get("action", "status")

        if action == "start":
            rc1, out1, err1 = run_sudo(["systemctl", "start", "smbd"])
            rc2, out2, err2 = run_sudo(["systemctl", "start", "nmbd"])
            # Re-check status
            rc_s, smbd_status, _ = run_sudo(["systemctl", "is-active", "smbd"])
            rc_n, nmbd_status, _ = run_sudo(["systemctl", "is-active", "nmbd"])
            return {
                "action": "start",
                "success": rc1 == 0 and rc2 == 0,
                "smbd": smbd_status,
                "nmbd": nmbd_status,
                "error": (err1 + "; " + err2).strip("; "),
            }

        elif action == "stop":
            rc1, _, err1 = run_sudo(["systemctl", "stop", "smbd"])
            rc2, _, err2 = run_sudo(["systemctl", "stop", "nmbd"])
            rc_s, smbd_status, _ = run_sudo(["systemctl", "is-active", "smbd"])
            rc_n, nmbd_status, _ = run_sudo(["systemctl", "is-active", "nmbd"])
            return {
                "action": "stop",
                "success": rc1 == 0 and rc2 == 0,
                "smbd": smbd_status,
                "nmbd": nmbd_status,
                "error": (err1 + "; " + err2).strip("; "),
            }

        elif action == "restart":
            rc1, _, err1 = run_sudo(["systemctl", "restart", "smbd"])
            rc2, _, err2 = run_sudo(["systemctl", "restart", "nmbd"])
            rc_s, smbd_status, _ = run_sudo(["systemctl", "is-active", "smbd"])
            rc_n, nmbd_status, _ = run_sudo(["systemctl", "is-active", "nmbd"])
            return {
                "action": "restart",
                "success": rc1 == 0 and rc2 == 0,
                "smbd": smbd_status,
                "nmbd": nmbd_status,
                "error": (err1 + "; " + err2).strip("; "),
            }

        elif action == "status":
            rc_s, smbd, _ = run_sudo(["systemctl", "is-active", "smbd"])
            rc_n, nmbd, _ = run_sudo(["systemctl", "is-active", "nmbd"])
            rc_e, enabled, _ = run_sudo(["systemctl", "is-enabled", "smbd"])
            # uptime
            rc_t, uptime, _ = run_sudo(["systemctl", "show", "smbd", "-P", "ActiveEnterTimestamp"])
            return {
                "action": "status",
                "smbd": "running" if smbd == "active" else "stopped",
                "nmbd": "running" if nmbd == "active" else "stopped",
                "enabled": enabled.strip() if rc_e == 0 else "unknown",
                "uptime": uptime.strip() if rc_t == 0 else None,
            }

        return {"error": f"Unknown action: {action}"}


# ── Main ──────────────────────────────────────────────────
def main():
    server = McpServer("wf-samba", "1.0.0")
    server.add_tool(SambaGuiTool())
    server.add_tool(SambaStatusTool())
    server.add_tool(SambaServiceTool())
    server.add_tool(SambaSharesTool())
    server.add_tool(SambaUsersTool())
    server.run()


if __name__ == "__main__":
    main()
