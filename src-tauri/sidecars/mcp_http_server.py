"""MCP HTTP Server — handles HTTP requests via Python requests or urllib.

Tool: http_request
  - method: GET/POST/PUT/DELETE
  - url: target URL
  - headers: optional JSON string of headers
  - body: optional request body
  - timeout: timeout in milliseconds
  - connect_timeout: connection timeout in milliseconds (default: same as timeout)

Returns: {status, body, headers}
"""

import json
import sys
import os

# Ensure we can import mcp_protocol from same directory
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mcp_protocol import McpServer, McpTool, log_stderr
from repl_skin import ReplSkin

skin = ReplSkin("http", version="1.0.0")


class HttpRequestTool(McpTool):
    def __init__(self):
        super().__init__(
            name="http_request",
            description="Send an HTTP request and return the response",
            input_schema={
                "type": "object",
                "properties": {
                    "method": {"type": "string", "description": "HTTP method (GET, POST, PUT, DELETE)"},
                    "url": {"type": "string", "description": "Target URL"},
                    "headers": {"type": "string", "description": "Optional JSON string of headers"},
                    "body": {"type": "string", "description": "Optional request body"},
                    "timeout": {"type": "integer", "description": "Timeout in milliseconds", "default": 10000},
                    "connect_timeout": {"type": "integer", "description": "Connection timeout in milliseconds"},
                },
                "required": ["method", "url"],
            },
        )

    def execute(self, arguments: dict) -> dict:
        method = arguments.get("method", "GET").upper()
        url = arguments["url"]
        headers_str = arguments.get("headers", "{}")
        body = arguments.get("body")
        timeout_ms = arguments.get("timeout", 10000)
        connect_timeout_ms = arguments.get("connect_timeout", timeout_ms)

        # Parse headers
        try:
            headers = json.loads(headers_str) if isinstance(headers_str, str) else headers_str
        except (json.JSONDecodeError, TypeError):
            headers = {}

        timeout = max(timeout_ms, connect_timeout_ms) / 1000.0

        # Prefer urllib3/requests, fallback to urllib
        try:
            return self._execute_requests(method, url, headers, body, timeout)
        except ImportError:
            return self._execute_urllib(method, url, headers, body, timeout)

    def _execute_requests(self, method, url, headers, body, timeout):
        import requests

        resp = requests.request(
            method=method,
            url=url,
            headers=headers,
            data=body,
            timeout=timeout,
            allow_redirects=True,
        )
        return {
            "status": resp.status_code,
            "body": resp.text,
            "headers": json.dumps(dict(resp.headers)),
        }

    def _execute_urllib(self, method, url, headers, body, timeout):
        from urllib.request import Request, urlopen
        from urllib.error import URLError, HTTPError

        req = Request(url, data=body.encode() if body else None, headers=headers, method=method)
        try:
            with urlopen(req, timeout=timeout) as resp:
                return {
                    "status": resp.status,
                    "body": resp.read().decode("utf-8", errors="replace"),
                    "headers": json.dumps(dict(resp.headers)),
                }
        except HTTPError as e:
            return {
                "status": e.code,
                "body": e.read().decode("utf-8", errors="replace"),
                "headers": json.dumps(dict(e.headers)),
            }
        except URLError as e:
            return {"status": 0, "body": "", "headers": "{}", "error": str(e)}


def main():
    server = McpServer("wf-http", "1.0.0")
    server.add_tool(HttpRequestTool())
    server.run()


if __name__ == "__main__":
    main()
