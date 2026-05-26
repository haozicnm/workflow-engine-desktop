# Changelog

## v7.1.1 (2026-05-26) — PWA Brand + Windows Launcher

### PWA 品牌升级
- **新图标**: 四节点流转 + 蓝紫渐变设计（SVG + 192/512 PNG）
- **Manifest 增强**: lang→zh-CN, categories, #0d1117 暗色主题
- **Service Worker**: Workbox precache + API NetworkFirst 策略

### Windows 一键启动
- **start.bat**: 双击自动启动服务 → 打开浏览器 → 提示安装 PWA
- **发布包**: `workflow-engine-v7.1.1-windows-x64.zip` (6 MB)

### PWA 布局修复
- `min-h-dvh` → `h-dvh`（修复 PWA standalone 模式滚动问题）
- `.app-shell` `overflow-y:auto` → `overflow:hidden`

### Taste 补漏
- Dashboard/Plugins/ParamField 残留 emoji 清理
- i18n 补充 `nav.plugins` key

### 模板修复
- `integration-smoke`: 移除已废弃的 clipboard 节点

## v7.1.0 (2026-05-24) — MCP Expansion + Namespace Isolation

### Rhai Context Namespace Isolation
- **`__vars__` object**: User variables now injected into `__vars__` Rhai map instead of flat scope. Field name collisions between step outputs and user variables eliminated at the architecture level
- **resolve_var layers**: `step_X.field` → step_outputs; `vars.xxx` → variables; bare `xxx` → step_outputs → variables fallback (backward compatible)
- **Iteration variables** (`__item`, `__index`, `__index1`, `loop`) kept at top-level scope (lifecycle limited to one iteration)

### MCP Node Expansion
- **6 Python sidecar servers**: `mcp_http_server`, `mcp_json_server`, `mcp_script_server`, `mcp_excel_server`, `mcp_word_server`, `mcp_web_scrape_server`, `mcp_shell_server`
- **13 MCP node types** registered alongside existing nodes (dual-track architecture)
- **`mcp_node.rs`** wrapper: creates MCP executor with fallback to native node on failure

### Feature Flags
- **`gui` feature**: Gates desktop-only nodes (mouse_keyboard, window, recording, print) and Tauri build
- **`cli` feature**: Default feature, enables CLI binary (`wf-cli`)
- **axum + tower-http**: HTTP server dependencies added for headless mode

### Template Updates
- **web-monitor-alert**: File-based init (no shell dependency), trend comparison with `json_parse`
- **file-batch-approval**: Cross-platform `work_dir` paths, file-based file preparation
- **daily-monitor**: Removed shell init step, direct HTTP start

### CSP Fix
- **Vue runtime-only build**: `vite.config.ts` aliases `vue` → `vue.runtime.esm-bundler.js` to avoid `unsafe-eval` CSP violation

### Audit Fixes (2026-05-23)
- **IPC token auth re-enabled**: was disabled during debugging, now enforced
- **i18n**: `containerConfig.noParams` key added (EN/ZH), ContainerConfigPanel hardcoded Chinese replaced
- **ParamField**: `hint` field now rendered below parameter labels

### Testing
- 46 lib tests + 26 integration tests passing
- 8 template library validation tests

---

## v7.0.0 (2026-05-19) — First Production Release

### Security
- **CSP**: Enabled Content Security Policy (was `null`)
- **IPC**: Enforced token authentication for WebSocket connections
- **Vite**: Fixed path traversal vulnerability in `/api/templates` dev server
- **Validation**: Workflow semantic validation on save (refs, required fields, variable format)
- **Resource limits**: Loop max 1000 iterations, global 30min execution timeout

### Reliability
- **Container output normalization**: Excel/Word actions collect errors instead of aborting; all containers inject `_container_type` and `_step_name` metadata
- **Error handling**: 16 `console.error` calls upgraded to user-visible toast notifications
- **Shell cross-platform**: Windows `cmd /c` as default, automatic Unix→Windows command translation (`mkdir -p`, `rm -rf`, `2>/dev/null`, `touch`)
- **Input simulation**: Linux enigo init degraded gracefully (panic→fallback error)
- **Regex**: Pattern compilation cached via `LazyLock` (was per-call)

### Templates
- **Parameterization**: All 5 built-in templates use `{{params.xxx}}` instead of hardcoded values
- **File paths**: Excel/Word steps now specify output file paths
- **Trend comparison**: Template 5 (web-monitor-alert) now implements actual historical trend analysis
- **Timeout consistency**: Approval timeout defaults fixed (ms→seconds)

### Architecture (UI-TARS learnings)
- **ActionDef strong typing**: 35 actions across 5 containers with structured `ParamDef` (replaces `Record<string, unknown>`)
- **Parser chain**: `ContainerParser` / `IterationParser` / `SimpleStepParser` replace if/else chain
- **TS auto-generation**: `cargo run --bin gen_action_ts` generates `src/types/action-metadata.ts`

### Testing
- 46 unit tests (40 existing + 6 new)
- 26 integration tests (6 new core chain tests)
- CI pipeline: `cargo test` + `cargo clippy` + `vue-tsc --noEmit`
- 0 compiler warnings (Rust + TypeScript)

### Internationalization
- All Rust backend error messages use English (was mixed Chinese/English)
- `newWorkflow()` returns empty name (was hardcoded Chinese)

---

## v6.9.0 — Pre-release

- 14 node types (shell, http, script, logic, excel, word, file, clipboard, loop, cursor, approval, notify, delay, browser)
- 5 built-in teaching templates
- SQLite persistence + IPC WebSocket server
- Tauri v2 desktop application
