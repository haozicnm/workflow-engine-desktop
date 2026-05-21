"""MCP JSON Parse Server — parses JSON strings into structured data.

Tool: json_parse
  - data: JSON string to parse
  
Returns: parsed JSON object
"""

import json
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mcp_protocol import McpServer, McpTool, log_stderr
from repl_skin import ReplSkin

skin = ReplSkin("json", version="1.0.0")


class JsonParseTool(McpTool):
    def __init__(self):
        super().__init__(
            name="json_parse",
            description="Parse a JSON string into a structured object",
            input_schema={
                "type": "object",
                "properties": {
                    "data": {"type": "string", "description": "JSON string to parse"},
                },
                "required": ["data"],
            },
        )

    def execute(self, arguments: dict):
        data = arguments["data"]
        # If data is already a dict/list (from engine), return as-is
        if isinstance(data, (dict, list)):
            return data
        # Parse string
        parsed = json.loads(data) if isinstance(data, str) else data
        return parsed


def main():
    server = McpServer("wf-json-parse", "1.0.0")
    server.add_tool(JsonParseTool())
    server.run()


if __name__ == "__main__":
    main()
