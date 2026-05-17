---
name: workflow-engine-cli
description: "Control Workflow Engine via wf-cli — list, run, schedule workflows. Agent-driven workflow creation and deployment."
version: 3.0.0
author: Hermes Agent
---

# Workflow Engine CLI — Agent Guide

Control [Workflow Engine](https://github.com/haozicnm/workflow-engine-desktop) (RPA/automation desktop app, v6.8.0+) via `wf-cli` or `workflow-engine --cli`. All commands support `--json` for machine-readable output.

## Install

```bash
# Full desktop app build (root directory):
cd workflow-engine-desktop
npm install && npm run build && npx tauri build

# CLI-only build:
cd src-tauri && cargo build --bin wf-cli --release
```

## Available Step Types (14 types, 68 actions)

### Container nodes (isContainer:true)
| Type | Label | Config | Actions |
|------|-------|--------|---------|
| `browser` | 浏览器 | `browser`, `headless`, `timeout` | 38 actions |
| `excel` | Excel | `file_path`, `sheet` | 7 actions |
| `word` | Word | `file_path` | 6 actions |
| `file` | 文件操作 | — | 10 actions |
| `logic` | 条件判断 | `condition` | 13 operators |
| `cursor` | 游标迭代 | `items` | 4 body actions |
| `loop` | 批量循环 | `items` | 4 body actions |

### Simple nodes
| Type | Label | Key config |
|------|-------|------------|
| `http` | HTTP 请求 | `method`, `url`, `headers` |
| `delay` | 延迟等待 | `duration_ms` |
| `notify` | 通知 | `notify_type`, `title`, `body` |
| `script` | 脚本 | `script` (Rhai) |
| `clipboard` | 剪贴板 | `action` |
| `approval` | 人工审批 | `title`, `timeout` |
| `shell` | Shell 命令 | `command`, `shell`, `cwd`, `timeout_secs` |

## Commands

| Command | Purpose | Key Flags |
|---------|---------|-----------|
| `list` | List workflows | `--json` |
| `run <id>` | Execute workflow | `-v key=value` |
| `status <run_id>` | Check run status | `--json` |
| `export <id>` | Export as JSON | `-o file.json` |
| `import <file>` | Import from JSON | — |
| `validate <file>` | Validate JSON | `--json` |
| `schedule list/create/delete` | Cron schedules | `--json` |
| `steps` | List step types+actions | `--json` (default) |

> `steps` is the authoritative source of truth. Use `wf-cli steps --json` to query available step types instead of hardcoding.

### Cron format
Uses 6-field format: `"sec min hour dom month dow"`

```bash
wf-cli schedule create <id> "0 0 18 * * *"   # daily 18:00
wf-cli schedule create <id> "0 0 9 * * 1-5"  # weekdays 09:00
```

⚠️ 5-field format → "Invalid cron expression" error.

## Agent Workflow Creation Pattern

```bash
# 0. Query available step types
terminal("wf-cli steps --json")

# 1. Generate workflow JSON → file
write_file("/tmp/workflow.json", "{...}")

# 2. Validate
terminal("wf-cli validate /tmp/workflow.json --json")

# 3. Import
terminal("wf-cli import /tmp/workflow.json")

# 4. Test run
terminal("wf-cli run <id> --var url=https://example.com")

# 5. Schedule (optional)
terminal('wf-cli schedule create <id> "0 0 18 * * *"')
```

### JSON authoring rules
- **Step IDs**: `step_1`, `step_2`, ... (sequential)
- **Action IDs**: `action_1_1`, `action_1_2`, ... (`action_{stepNum}_{actionIndex}`)
- **Variable refs**: `{{stepId.actionId}}` or `{{stepId.actionId.field}}`
- **Browser**: set `"headless": true` for CLI mode
- **Condition branching**: `runCondition: {"ref": "step_logic", "when": "true"}`
- **Logic nodes**: `conditionGroup` with `combinator: "and"/"or"` and `conditions` array
- **Error handling**: `"onError": "ignore"` / `"fail"` / `{"branch": "step_id"}`
- **Retry**: `"retry": {"max": 3, "delay_ms": 1000}`
- **Step chain**: set `"next": "step_3"` to override sequential order

## Pitfalls
1. wf-cli needs same SQLite DB as GUI — same machine required
2. Browser steps need Playwright + Chromium installed
3. `run` is synchronous — blocks until complete
4. `--var` values are always JSON strings
5. Cron is 6-field: `"sec min hour dom month dow"` — NOT 5-field
6. Import reads name from JSON — ensure meaningful `name` field
7. Only 14 valid step types — no legacy/`_container` suffix
8. Exit codes: 0=success, 1=runtime error, 2=CLI argument error
