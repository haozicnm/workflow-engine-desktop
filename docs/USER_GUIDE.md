# Workflow Engine Desktop — User Guide

> Version 8.0 | Updated: 2026-05-20

## Quick Start

1. Launch the app — welcome screen shows 5 built-in templates
2. Choose a template, fill parameters, click **Run**
3. Watch execution live with step-by-step progress
4. Results appear in Excel/Word/notifications as configured

## Node Types (14)

| Node | Description | Key Config |
|------|-------------|------------|
| **Shell** | Execute system commands | `command`, `shell` (auto/bash/powershell/cmd) |
| **HTTP** | Send API requests | `method`, `url`, `headers` |
| **Script** | Rhai calculations | `script` — access `step_X.field` for upstream data |
| **Logic** | Conditional branching | `conditionGroup` with AND/OR combinator |
| **Excel** | Read/write/create/sort Excel | `file_path` + actions |
| **Word** | Generate/merge/fill Word docs | `file_path` + actions |
| **File** | Read/write/list/copy/delete | Path-based actions |
| **Clipboard** | Read/write system clipboard | `action`: read/write |
| **Loop** | Iterate over items | `items`, `max_iterations` (default 1000) |
| **Cursor** | Process items one at a time | `items` — pauses between iterations |
| **Approval** | Human-in-the-loop | `options`, `approval_conditions`, `require_review` |
| **Notify** | System toast notifications | `title`, `body` |
| **Delay** | Pause execution | `duration_ms` |
| **Browser** | Web automation | `browser`, actions: navigate/click/input/extract |

## Variable References

```
{{step_1.stdout}}       — Shell stdout
{{step_2.body}}         — HTTP response body
{{step_3.avg}}          — Script output field
{{step_4.branch}}       — Logic result (true/false)
{{step_5.a6_1}}         — Container action output
{{params.test_dir}}     — Template parameter
```

## 5 Built-in Templates

1. **integration-smoke** — All 14 node types, end-to-end learning
2. **daily-monitor** — Multi-API aggregation → Excel/Word report
3. **file-batch-approval** — File scan → approval → archive
4. **http-approval-pipeline** — API data → quality check → human approval
5. **web-monitor-alert** — Multi-site health → trend analysis → alerts

## CLI

```bash
wf-cli run <workflow-id>
wf-cli library-run integration-smoke --param test_dir=/tmp/test
wf-cli library-list
```

Token: `~/.hermes/daemon-token` or `WF_DAEMON_TOKEN` env var.

## Platform Notes

- **Windows**: Shell commands auto-adapt (mkdir→mkdir, rm→del)
- **Linux**: Input simulation gracefully degrades if X11/Wayland unavailable
- **macOS**: Basic support via bash shell

## Troubleshooting

| Issue | Solution |
|-------|----------|
| Param not working | Check `{{params.xxx}}` match in workflow JSON |
| Step fails silently | Check RunHistory page for error details |
| Shell fails on Windows | Use `"shell": "powershell"` |
| Excel/Word missing | Verify `file_path` in container config |
