"""Unified REPL Skin — consistent terminal UI for all MCP sidecars.

Inspired by CLI-Anything's repl_skin.py (567 lines).

Usage:
    from repl_skin import ReplSkin
    skin = ReplSkin("excel", version="1.0.0")
    skin.success("Read 42 rows")
    skin.error("File not found")
    skin.info("Processing sheet 'Sales'...")
    skin.warning("Empty file, using defaults")
"""

import sys
from typing import Optional


class ReplSkin:
    """Shared terminal UI for all MCP sidecars — consistent output for humans and agents."""

    # ANSI terminal colors
    GREEN = "\033[32m"
    RED = "\033[31m"
    YELLOW = "\033[33m"
    BLUE = "\033[34m"
    CYAN = "\033[36m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    RESET = "\033[0m"

    # Per-sidecar accent colors
    ACCENTS = {
        "script": CYAN,
        "shell": BLUE,
        "http": GREEN,
        "excel": GREEN,
        "word": BLUE,
        "browser": CYAN,
        "json": YELLOW,
        "web_scrape": BLUE,
        "playwright": CYAN,
        "desktop_recorder": YELLOW,
    }

    def __init__(self, name: str, version: str = "1.0.0"):
        self.name = name
        self.version = version
        self.accent = self.ACCENTS.get(name, self.CYAN)

    def _log(self, msg: str) -> None:
        """Write to stderr (MCP protocol uses stdout for JSON-RPC)."""
        print(msg, file=sys.stderr, flush=True)

    # ── Status messages ──

    def success(self, msg: str) -> None:
        self._log(f"{self.GREEN}✓{self.RESET} {msg}")

    def error(self, msg: str) -> None:
        self._log(f"{self.RED}✗{self.RESET} {msg}")

    def warning(self, msg: str) -> None:
        self._log(f"{self.YELLOW}⚠{self.RESET} {msg}")

    def info(self, msg: str) -> None:
        self._log(f"{self.BLUE}●{self.RESET} {msg}")

    def debug(self, msg: str) -> None:
        self._log(f"{self.DIM}{msg}{self.RESET}")

    # ── Structured output ──

    def banner(self, tool_count: Optional[int] = None) -> None:
        """Print startup banner."""
        self._log(f"{self.accent}{self.BOLD}{self.name} v{self.version}{self.RESET}")
        if tool_count:
            self._log(f"{self.DIM}  {tool_count} tools registered{self.RESET}")

    def tool_call(self, tool_name: str, args_summary: str = "") -> None:
        """Log a tool invocation."""
        detail = f" — {args_summary}" if args_summary else ""
        self._log(f"{self.accent}▶ {tool_name}{self.RESET}{self.DIM}{detail}{self.RESET}")

    def result_summary(self, summary: str) -> None:
        """Log the result of a tool call."""
        self._log(f"{self.DIM}  → {summary}{self.RESET}")

    def json_result(self, data: dict) -> str:
        """Pretty-print JSON result (for stdout). Returns the JSON string."""
        import json
        return json.dumps(data, indent=2, ensure_ascii=False)
