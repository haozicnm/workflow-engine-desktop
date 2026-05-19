"""MCP Protocol — JSON-RPC 2.0 over stdio, minimal implementation.

Workflow Engine uses this to communicate with MCP node servers.
Each server reads JSON-RPC requests from stdin, writes responses to stdout.
"""

import sys
import json
import traceback


def log_stderr(msg: str):
    """Write debug log to stderr (doesn't interfere with stdout protocol)."""
    print(f"[mcp] {msg}", file=sys.stderr, flush=True)


class McpTool:
    """A tool definition exposed by this server."""

    def __init__(self, name: str, description: str, input_schema: dict):
        self.name = name
        self.description = description
        self.input_schema = input_schema

    def to_dict(self) -> dict:
        return {
            "name": self.name,
            "description": self.description,
            "inputSchema": self.input_schema,
        }

    def execute(self, arguments: dict) -> dict:
        """Override in subclass. Must return a dict (serializable to JSON)."""
        raise NotImplementedError


class McpServer:
    """Minimal MCP server — JSON-RPC 2.0 over stdio.

    Usage:
        server = McpServer("wf-http", "1.0.0")
        server.add_tool(HttpRequestTool())
        server.run()
    """

    def __init__(self, name: str, version: str):
        self.name = name
        self.version = version
        self.tools: list[McpTool] = []

    def add_tool(self, tool: McpTool):
        self.tools.append(tool)

    def run(self):
        """Main loop: read JSON-RPC from stdin, write responses to stdout."""
        log_stderr(f"Starting MCP server: {self.name} v{self.version}")
        log_stderr(f"Tools: {[t.name for t in self.tools]}")

        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue

            try:
                request = json.loads(line)
            except json.JSONDecodeError as e:
                self._send_error(None, -32700, f"Parse error: {e}")
                continue

            method = request.get("method", "")
            req_id = request.get("id")

            if method == "initialize":
                self._handle_initialize(req_id)
            elif method == "tools/list":
                self._handle_tools_list(req_id)
            elif method == "tools/call":
                self._handle_tools_call(req_id, request.get("params", {}))
            elif method == "notifications/initialized":
                pass  # no response needed
            else:
                self._send_error(req_id, -32601, f"Method not found: {method}")

        log_stderr("MCP server shutting down")

    def _send_response(self, req_id, result: dict):
        resp = {"jsonrpc": "2.0", "id": req_id, "result": result}
        sys.stdout.write(json.dumps(resp, ensure_ascii=False) + "\n")
        sys.stdout.flush()

    def _send_error(self, req_id, code: int, message: str):
        resp = {
            "jsonrpc": "2.0",
            "id": req_id,
            "error": {"code": code, "message": message},
        }
        sys.stdout.write(json.dumps(resp, ensure_ascii=False) + "\n")
        sys.stdout.flush()

    def _handle_initialize(self, req_id):
        self._send_response(
            req_id,
            {
                "protocolVersion": "2024-11-05",
                "serverInfo": {"name": self.name, "version": self.version},
                "capabilities": {"tools": {}},
            },
        )

    def _handle_tools_list(self, req_id):
        self._send_response(req_id, {"tools": [t.to_dict() for t in self.tools]})

    def _handle_tools_call(self, req_id, params: dict):
        tool_name = params.get("name", "")
        arguments = params.get("arguments", {})

        tool = next((t for t in self.tools if t.name == tool_name), None)
        if tool is None:
            self._send_error(req_id, -32602, f"Tool not found: {tool_name}")
            return

        try:
            result = tool.execute(arguments)
            # MCP expects result as {"content": [{"type": "text", "text": "..."}]}
            self._send_response(
                req_id,
                {
                    "content": [
                        {"type": "text", "text": json.dumps(result, ensure_ascii=False)}
                    ]
                },
            )
        except Exception as e:
            log_stderr(f"Tool {tool_name} failed: {traceback.format_exc()}")
            self._send_error(req_id, -32000, f"Tool execution failed: {e}")
