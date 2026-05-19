"""MCP Script Server — Python replaces Rhai for workflow scripts.

Tool: execute
  - code: Python code string to execute
  - context: optional dict of workflow variables injected as Python locals

Returns: the value of 'result' variable, or the last expression value.
"""

import json
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mcp_protocol import McpServer, McpTool, log_stderr


class ExecuteTool(McpTool):
    def __init__(self):
        super().__init__(
            name="execute",
            description="Execute Python code with workflow context variables",
            input_schema={
                "type": "object",
                "properties": {
                    "script": {"type": "string", "description": "Python code to execute"},
                },
                "required": ["script"],
            },
        )

    def execute(self, arguments: dict) -> dict:
        code = arguments["script"]

        # Build safe locals: only allow Python builtins (no os, sys, subprocess)
        safe_builtins = {
            "True": True, "False": False, "None": None,
            "int": int, "float": float, "str": str, "bool": bool,
            "list": list, "dict": dict, "tuple": tuple, "set": set,
            "len": len, "range": range, "enumerate": enumerate,
            "zip": zip, "map": map, "filter": filter,
            "max": max, "min": min, "sum": sum, "abs": abs, "round": round,
            "sorted": sorted, "reversed": reversed,
            "print": lambda *a, **kw: log_stderr(" ".join(str(x) for x in a)),
            "json": json,
            "__builtins__": {
                "True": True, "False": False, "None": None,
                "int": int, "float": float, "str": str, "bool": bool,
                "list": list, "dict": dict, "tuple": tuple, "set": set,
                "len": len, "range": range, "enumerate": enumerate,
                "zip": zip, "map": map, "filter": filter,
                "max": max, "min": min, "sum": sum, "abs": abs,
                "sorted": sorted, "reversed": reversed,
                "round": round, "type": type, "isinstance": isinstance,
                "print": lambda *a, **kw: log_stderr(" ".join(str(x) for x in a)),
                "Exception": Exception, "ValueError": ValueError,
                "TypeError": TypeError, "KeyError": KeyError,
            },
        }

        locals_vars = {}
        try:
            exec(code, {"__builtins__": safe_builtins["__builtins__"]}, locals_vars)
        except Exception as e:
            log_stderr(f"Script error: {e}")
            return {"error": str(e), "result": None}

        # Return the 'result' variable if set, otherwise return all local vars
        if "result" in locals_vars:
            return {"result": locals_vars["result"], "locals": locals_vars}
        # If no explicit result, try to return the last dict/object created
        result = locals_vars
        return {"result": result, "locals": locals_vars}


def main():
    server = McpServer("wf-script", "1.0.0")
    server.add_tool(ExecuteTool())
    server.run()


if __name__ == "__main__":
    main()
