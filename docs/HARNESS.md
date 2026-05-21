# HARNESS.md — Workflow Engine Sidecar Development SOP

> Inspired by CLI-Anything's HARNESS.md (748 lines, single source of truth for harness development).

## Overview

This document defines the methodology for developing **MCP sidecars** — Python servers that extend Workflow Engine with specialized capabilities (Excel, Word, Browser, HTTP, Shell, etc.).

## Architecture Principle

**Use real software, don't reimplement it.**

```
✅ Call LibreOffice --headless for document conversion
✅ Call real browsers via CDP for scraping
✅ Use openpyxl for Excel, python-docx for Word
❌ Don't reimplement rendering engines in Python
❌ Don't fake output — verify with real tools
```

## Sidecar Development SOP (7 Phases)

### Phase 1: Domain Analysis
- Identify the underlying tool/library (openpyxl, python-docx, playwright, requests)
- Map core operations to MCP tool names
- Define input/output schemas

### Phase 2: Tool Design
- Each tool = one `McpTool` subclass
- `name`: lowercase_snake_case, descriptive
- `input_schema`: JSON Schema with required fields
- `execute()`: pure function, returns dict

### Phase 3: Implementation
- Import `McpServer`, `McpTool` from `mcp_protocol.py`
- Import `ReplSkin` from `repl_skin.py` for consistent logging
- Error handling: catch exceptions, return structured errors
- No external side effects beyond the tool's domain

### Phase 4: Testing
- **Unit tests**: synthetic data, no external dependencies
- **E2E tests**: invoke real tools via MCP protocol
- **No graceful degradation**: if software not installed, test FAILS (don't skip)
- Verify output structure matches expected schema

### Phase 5: Documentation
- Docstring at top of file: tool list with descriptions
- Use `skin.banner()` for startup branding
- Use `skin.info()` / `skin.error()` for structured logging

### Phase 6: Registration
- Add to `src-tauri/src/nodes/mcp_node.rs` (MCP_NODE_TYPES)
- Add to `src-tauri/src/engine/executor.rs` (register!)
- Add to `registry.json`

### Phase 7: Release
- Version bump in docstring
- Update CHANGELOG.md
- Deploy via wf-engine plugin system (.wfplug)

## Pattern: Minimal MCP Server

```python
from mcp_protocol import McpServer, McpTool, log_stderr
from repl_skin import ReplSkin

skin = ReplSkin("my_tool", version="1.0.0")

class MyTool(McpTool):
    def __init__(self):
        super().__init__(
            name="my_tool",
            description="What this tool does",
            input_schema={
                "type": "object",
                "properties": {
                    "input": {"type": "string"},
                },
                "required": ["input"],
            },
        )

    def execute(self, arguments: dict) -> dict:
        result = do_something(arguments["input"])
        return {"output": result}

def main():
    server = McpServer("wf-my-tool", "1.0.0")
    server.add_tool(MyTool())
    skin.banner(tool_count=len(server.tools))
    server.run()

if __name__ == "__main__":
    main()
```

## Pitfalls

1. **Don't print to stdout** — MCP uses stdout for JSON-RPC. Use `log_stderr()` or `skin.debug()`.
2. **Always handle missing files** — Return structured error, not Python traceback.
3. **path handling** — Use absolute paths. Don't assume CWD.
4. **Encoding** — All text must be UTF-8. Use `ensure_ascii=False` in `json.dumps`.
5. **No blocking I/O in loops** — MCP reads one line at a time. Heavy work goes in `execute()`.

## Testing Philosophy

```
# Software not installed → test FAILS (no skip)
# Test must invoke the REAL tool
# Example: excel_read must call openpyxl.load_workbook()
```

```bash
# Run a sidecar test manually
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' | python mcp_excel_server.py
echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"excel_read","arguments":{"path":"test.xlsx"}}}' | python mcp_excel_server.py
```
