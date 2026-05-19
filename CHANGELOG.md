# Changelog

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
