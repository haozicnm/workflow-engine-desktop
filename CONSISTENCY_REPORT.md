# Workflow Engine 前后端数据流一致性检查报告

> 检查时间：基于 v8.4.1 / v9.0 代码库
> 工作目录：`C:\Users\haozi\Dev\workflow-engine-desktop`

---

## 1. API 路由前后对照

### 前端 safeInvoke 命令清单

| 前端命令 | 后端 Tauri 命令 | 后端 HTTP 路由 | 前端 tauri.ts HTTP 映射 | 状态 | 说明 |
|---------|---------------|-------------|----------------------|------|------|
| `workflow_list` | ✅ | `GET /api/workflows` | ✅ FIXED | 正常 | |
| `workflow_create` | ✅ | `POST /api/workflows` | ✅ FIXED | 正常 | |
| `workflow_get` | ✅ | `GET /api/workflows/{id}` | ✅ DYNAMIC | 正常 | |
| `workflow_update` | ✅ | `PUT /api/workflows/{id}` | ✅ DYNAMIC | 正常 | |
| `workflow_delete` | ✅ | `DELETE /api/workflows/{id}` | ✅ DYNAMIC | 正常 | |
| `workflow_lock` | ✅ | `POST /api/workflows/{id}/lock` | ✅ DYNAMIC | 正常 | |
| `workflow_save_yaml` | ✅ | `POST /api/workflows/{id}/yaml` | ✅ DYNAMIC | 正常 | |
| `workflow_validate` | ✅ | `POST /api/workflows/validate` | ✅ FIXED | 正常 | |
| `workflow_export` | ✅ | `GET /api/workflows/{id}/export-yaml` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用，Tauri 模式可用 |
| `workflow_import` | ✅ | `POST /api/workflows/import` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用，Tauri 模式可用 |
| `workflow_auto_order` | ✅ | `POST /api/workflows/auto-order` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用，Tauri 模式可用 |
| `step_test` | ✅ | `POST /api/step-test` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用，Tauri 模式可用 |
| `run_start` | ✅ | `POST /api/runs` | ✅ FIXED | 正常 | |
| `run_cancel` | ✅ | `POST /api/runs/{run_id}/cancel` | ✅ DYNAMIC | 正常 | |
| `run_pause` | ✅ | `POST /api/runs/{run_id}/pause` | ❌ 无映射 | 🔴 **问题** | 浏览器模式调用将失败 |
| `run_resume` | ✅ | `POST /api/runs/{run_id}/resume` | ❌ 无映射 | 🔴 **问题** | 浏览器模式调用将失败 |
| `run_status` | ✅ | `GET /api/runs/{run_id}/status` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用，Tauri 模式可用 |
| `run_logs` | ✅ | `GET /api/runs/{run_id}/logs` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用，Tauri 模式可用 |
| `run_list` | ✅ | `GET /api/runs` | ✅ FIXED | 正常 | |
| `run_detail` | ✅ | `GET /api/runs/{run_id}/detail` | ✅ DYNAMIC | 正常 | |
| `run_step_logs` | ✅ | `GET /api/runs/{run_id}/step-logs` | ✅ DYNAMIC | 正常 | |
| `approval_list_pending` | ✅ | `GET /api/approvals/pending` | ✅ FIXED | 正常 | |
| `approval_response` | ✅ | `POST /api/approvals/respond` | ✅ FIXED | 正常 | |
| `schedule_list` | ✅ | `GET /api/schedules` | ✅ FIXED | 正常 | |
| `schedule_create` | ✅ | `POST /api/schedules` | ✅ FIXED | 正常 | |
| `schedule_update` | ✅ | `PUT /api/schedules/{id}` | ✅ DYNAMIC | 正常 | |
| `schedule_delete` | ✅ | `DELETE /api/schedules/{id}` | ✅ DYNAMIC | 正常 | |
| `node_list_types` | ✅ | `GET /api/nodes/types` | ✅ FIXED | 正常 | |
| `settings_get` | ✅ | `GET /api/settings` | ✅ FIXED | 正常 | |
| `settings_update` | ✅ | `PUT /api/settings` | ✅ FIXED | 正常 | |
| `check_ipc` | ✅ | `GET /api/system/check-ipc` | ✅ FIXED | 正常 | |
| `system_check_browser` | ✅ | `GET /api/system/check-browser` | ✅ FIXED | 正常 | |
| `clear_logs` | ✅ | `POST /api/system/clear-logs` | ✅ FIXED | 正常 | |
| `open_log_dir` | ✅ | `POST /api/system/open-log-dir` | ✅ FIXED | 正常 | |
| `debug_step` | ✅ | `POST /api/debug/step/{run_id}` | ✅ FIXED | 正常 | |
| `debug_continue` | ✅ | `POST /api/debug/continue/{run_id}` | ✅ FIXED | 正常 | |
| `debug_vars` | ✅ | `GET /api/debug/vars/{run_id}` | ✅ FIXED | 正常 | |
| `debug_set_breakpoint` | ✅ | `POST /api/debug/breakpoints` | ✅ FIXED | 正常 | |
| `debug_remove_breakpoint` | ✅ | `POST /api/debug/breakpoints/remove` | ✅ FIXED | 正常 | |
| `debug_get_breakpoints` | ✅ | `GET /api/debug/breakpoints/{workflow_id}` | ✅ FIXED | 正常 | |
| `template_list` | ✅ | `GET /api/templates` | ✅ FIXED | 正常 | |
| `template_import` | ✅ | `POST /api/templates/import` | ✅ FIXED | 正常 | |
| `get_trajectory` | ✅ | `GET /api/preview/trajectory/{run_id}` | ✅ DYNAMIC | 正常 | |
| `get_bundle_files` | ✅ | `GET /api/preview/bundle-files/{run_id}/{step_id}` | ✅ DYNAMIC | 正常 | |
| `read_bundle_file` | ✅ | `GET /api/preview/bundle-file/{run_id}/{step_id}/{filename}` | ✅ DYNAMIC | 正常 | |
| `browser_pick_session_start` | ✅ | `POST /api/browser/pick-start` | ✅ DYNAMIC | 正常 | |
| `browser_pick_next` | ✅ | `GET /api/browser/pick-next` | ✅ DYNAMIC | 正常 | |
| `browser_pick_session_stop` | ✅ | `POST /api/browser/pick-stop` | ✅ DYNAMIC | 正常 | |
| `browser_snapshot` | ✅ | `POST /api/browser/snapshot` | ✅ DYNAMIC | 正常 | |
| `plugin_list` | ✅ | `GET /api/plugins` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用 |
| `plugin_install` | ✅ | `POST /api/plugins/install` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用 |
| `plugin_uninstall` | ✅ | `POST /api/plugins/uninstall` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用 |
| `preview_excel` | ✅ | `POST /api/preview/excel` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用 |
| `preview_word` | ✅ | `POST /api/preview/word` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用 |
| `run_pipeline` | ✅ | `POST /api/pipeline/run` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用 |
| `web_scrape_preview` | ✅ | `POST /api/preview/web-scrape` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用 |
| `get_log_path` | ✅ | `GET /api/system/log-path` | ❌ 无映射 | ⚠️ 低风险 | 前端未调用 |

### 关键问题

1. **`run_pause` / `run_resume` 浏览器模式缺失**：后端有 Tauri 命令和 HTTP 路由，但 `tauri.ts` 的 `FIXED_ROUTES`/`DYNAMIC_ROUTES` 中没有映射。如果前端在浏览器模式下调用这两个命令，会抛出 `Unknown command — no HTTP mapping` 错误。当前前端未调用它们，但未来扩展可能踩坑。

2. **`workflow_export` / `workflow_import` / `workflow_auto_order` 等**：Tauri 命令已注册，但前端 tauri.ts 中缺少 HTTP 回退映射。由于前端未实际调用，影响较低。

---

## 2. 事件流前后对照

### 事件订阅矩阵

| 事件名 | 前端订阅者 | 后端发射者 | 状态 | 问题 |
|-------|-----------|-----------|------|------|
| `step-update` | `useStepRunner.ts`, `Editor.vue` | `scheduler.rs` (emit_step_update) | ✅ 匹配 | 无 |
| `run-update` | `useStepRunner.ts`, `DebugPanel.vue`, `Dashboard.vue`, `Editor.vue` | `scheduler.rs`, `run.rs`, `run_manager.rs` | ✅ 匹配 | 无 |
| `breakpoint-hit` | `DebugPanel.vue` | `scheduler.rs` (check_debug_pause) | ✅ 匹配 | ⚠️ 字段不一致（见下方） |
| `workflow-changed` | `Dashboard.vue` | `commands/workflow.rs`, `server/managers/workflow_manager.rs` | ✅ 匹配 | 无 |
| `schedule-changed` | `Dashboard.vue` | `commands/schedule.rs`, `server/managers/schedule_manager.rs` | ✅ 匹配 | 无 |
| `variable-update` | ❌ **无订阅者** | `scheduler.rs` (emit_variable_snapshot) | 🔴 **断点** | 前端未监听此事件 |
| `ExecutionEvent` | ❌ 前端不可见 | `engine/executor.rs` (内部枚举) | ⚠️ 隔离 | 仅内部使用，未外化 |

### 事件数据结构对比

**`step-update` 事件** — 前后一致

后端发射（`scheduler.rs:1232`）：
```json
{
  "run_id": "...",
  "step_id": "...",
  "step_name": "...",
  "total_steps": 5,
  "status": "completed",
  "output": {...},
  "error": null
}
```

前端接收（`useStepRunner.ts:43`）：
```typescript
{
  run_id: string
  step_id: string
  step_name: string
  total_steps: number
  status: string
  output?: unknown
  error?: string | null
}
```
✅ 字段完全匹配。

**`run-update` 事件** — 前后一致

后端发射包含：`run_id`, `workflow_id?`, `workflow_name?`, `status`, `error?`

前端接收：`run_id`, `workflow_name?`, `status`, `error?`
✅ 字段匹配。前端 `Dashboard.vue` 未使用 `workflow_id`，不影响功能。

**`breakpoint-hit` 事件** — ⚠️ 字段不一致

后端发射两种 payload：

1. 断点命中（`scheduler.rs:368`）：
```json
{
  "run_id": "...",
  "step_id": "...",
  "step_name": "...",
  "variables": {...},
  "step_outputs": {...}
}
```
2. 单步模式（`scheduler.rs:462`）：
```json
{
  "run_id": "...",
  "step_id": "...",
  "step_name": "...",
  "reason": "step_mode",
  "variables": {...},
  "step_outputs": {...}
}
```

前端类型声明（`DebugPanel.vue:130`）：
```typescript
{
  run_id: string
  step_id: string
  reason: string        // ← 必填，但断点命中时无此字段
  variables: Record<string, unknown>
}
```

⚠️ **问题**：前端声明 `reason: string` 为必填，但普通断点命中的 payload 中没有 `reason` 字段。运行时 `event.payload.reason` 为 `undefined`，虽然不会报错，但 TypeScript 类型与实际数据不符。

**`variable-update` 事件** — 🔴 **断点**

后端 `scheduler.rs:1262` 每次步骤执行后发射 `variable-update` 事件，包含当前 `variables` 和 `step_outputs` 快照。但前端没有任何组件监听此事件，实时变量监视功能实际未生效。`DebugPanel.vue` 只能通过轮询 `debug_vars` API（每 2 秒）获取变量，而非实时推送。

---

## 3. 数据模型前后对照

### Step 结构对比

| 前端字段 (`types.ts`) | 后端字段 (`workflow.rs`) | serde 映射 | 状态 | 问题 |
|---------------------|------------------------|-----------|------|------|
| `id: string` | `id: String` | 直接 | ✅ | |
| `type: ContainerType` | `step_type: String` | `rename = "type"` | ✅ | |
| `label: string` | `name: String` | `alias = "label"` | ✅ | 前端必填，后端可选 |
| `expanded: boolean` | `expanded: Option<bool>` | `default` | ⚠️ | 前端必填 `boolean`，后端可选 `Option<bool>` |
| `actions: Action[]` | `actions: Option<Vec<Value>>` | `default` | ✅ | |
| `config: Record<string, unknown>` | `config: Value` | 直接 | ✅ | |
| `onError?: ErrorStrategy` | `on_error: Option<ErrorStrategy>` | `alias = "onError"` | ⚠️ | 序列化方向不一致（见下方） |
| `delay?: number` | `delay: Option<u64>` | `default` | ✅ | |
| `breakpoint?: boolean` | `breakpoint: bool` | `default` | ✅ | |
| `runCondition?: StepCondition` | `run_condition: Option<RunCondition>` | `alias = "runCondition"` | ✅ | |
| `condition?: string` | `condition: Option<String>` | `default` | ✅ | |
| `conditionGroup?: LogicConditionGroup` | `condition_group: Option<LogicConditionGroup>` | `alias = "conditionGroup"` | ✅ | |
| `next?: string` | `next: Option<String>` | 直接 | ✅ | |
| `retry?: { max_retries?, delay_ms? }` | `retry: Option<RetryConfig>` | `alias = "max_retries"` | ⚠️ | 序列化字段名不一致（见下方） |
| `timeout?: number` | `timeout: Option<u64>` | 直接 | ✅ | |
| ❌ **无此字段** | `body_steps: Option<Vec<Step>>` | `default` | 🔴 | 前端 Step 类型中无 `body_steps` |

### 关键问题

**1. `ErrorStrategy` 序列化不一致** 🔴

后端 serialize（`workflow.rs`）使用 `#[serde(rename_all = "snake_case")]`：
- `ErrorStrategy::Fail` → `"fail"`
- `ErrorStrategy::Ignore` → `"ignore"`
- `ErrorStrategy::Branch { step_id }` → `{"branch": {"step_id": "xxx"}}`

前端期望（`types.ts`）：
```typescript
type ErrorStrategy = 'fail' | 'ignore' | { branch: string }
```

⚠️ **问题**：当前端发送 `{ branch: "step_id" }`（字符串值）时，后端自定义 deserializer 可以正确解析（`workflow.rs:38` 支持 `Value::String(s)`）。但**后端 serialize 输出的是 `{"branch": {"step_id": "..."}}`**，如果前端读取后端的 `ErrorStrategy` 数据（如从数据库加载），会得到嵌套对象，与前端类型 `{ branch: string }` 不匹配。

**2. `RetryConfig` 字段名不一致** ⚠️

```rust
struct RetryConfig {
    #[serde(alias = "max_retries")]
    pub max: u32,         // ← serialize 输出为 "max"
    pub delay_ms: u64,
}
```

前端发送 `{ max_retries: 3, delay_ms: 1000 }`，后端可以正确 deserialize（因为 `alias`）。但后端 serialize 后输出 `{ max: 3, delay_ms: 1000 }`。如果前端读取后端的 retry 数据，会得到 `max` 而非 `max_retries`。当前前端只写不读，影响不大。

**3. `body_steps` 字段缺失** 🔴

后端 `Step` 有 `body_steps: Option<Vec<Step>>` 用于容器节点的子步骤。但前端 `Step` 类型定义中完全没有此字段。前端使用 `actions: Action[]` 列表来存储容器内部操作，而非 `body_steps`。当后端 serialize 包含 `body_steps` 的工作流时，前端 `deserializeWorkflow` 不会处理此字段，可能导致容器子步骤数据丢失。

实际上，在 `executor.rs` 中，容器节点通过 `step.actions` 归一化到 `config` 中，而不是 `body_steps`。`body_steps` 可能是旧版字段或用于特定节点（如 loop/while/cursor）。需要确认前端是否通过其他方式处理 `body_steps`。

**4. `expanded` 类型不匹配** ⚠️

前端 `expanded: boolean` 是必填字段，后端 `expanded: Option<bool>` 是可选的。如果后端返回的数据中没有 `expanded`，前端会得到 `undefined` 而非 `false`。但由于 Vue 的响应式系统，实际使用时可能通过 `v-if` 或默认值处理，目前未观察到故障。

---

## 4. 变量传递路径

### 4.1 edge → input_ports 路径

```
前端 Edge ──HTTP/SQLite──► 后端 Edge { from, from_port, to, to_port }
                              │
                              ▼
                    executor.rs:run_graph()
                              │
                    ┌─────────┴─────────┐
                    │     level.len()     │
                    │        == 1         │
                    │    (单节点执行)      │
                    ▼                     ▼
            execute(step, ctx)    run_parallel_level()
            (原始 ctx)                  │
                                    创建 task_ctx
                                    注入 input_ports
                                    execute(step, task_ctx)
```

🔴 **严重问题**：`input_ports` 注入仅在**并行层级**（`run_parallel_level`，多节点同层）中执行。当 DAG 拓扑分层后某层只有**单个节点**时，代码走 `level.len() == 1` 分支，直接调用 `execute(step, ctx)`，使用**原始 `ctx`**，不会创建 `task_ctx`，因此**不会注入 `input_ports`**。

**影响**：
- 如果一个节点在串行链中（每层只有一个节点），即使有 `edges` 连接，该节点也无法通过 `input_ports` 获取上游数据
- 节点只能通过 `ctx.step_outputs` 和模板变量 `{{step_id}}` 来访问上游输出
- 如果节点实现（尤其是 v8 新增节点）期望从 `input_ports` 读取数据，在串行链中会失败

**建议**：在 `run_graph` 的单节点分支中，也应该注入 `input_ports`：

```rust
// 当前代码（有问题的）
if level.len() == 1 {
    let step = ...;
    match Arc::clone(self).execute(step, ctx).await { ... }  // ← ctx 没有 input_ports
} else {
    self.run_parallel_level(level, workflow, ctx).await?;      // ← 正确注入 input_ports
}
```

### 4.2 表达式引擎 `{{= expr}}` 路径

| 步骤 | 位置 | 状态 | 说明 |
|------|------|------|------|
| 模板解析 | `context.rs:resolve_string()` | ✅ | 识别 `{{= ...}}` 语法 |
| 表达式求值 | `context.rs:eval_expr()` | ✅ | 使用 Rhai 引擎 |
| 变量注入 | `eval_expr()` scope | ✅ | `__vars__`, `step_*`, `__item`/`__index`/`__index1`/`loop` |
| 沙箱限制 | `eval_expr()` | ✅ | 100_000 ops, 1MB string, 10_000 array/map |
| 返回值 | `rhai_to_json()` | ✅ | INT/Bool/f64/String/Array/Map → serde_json::Value |

表达式求值路径完整，无断点。`{{= expr}}` 在 `resolve_string()` 和 `interpolate()` 两处都支持。但需要注意：

⚠️ `eval_expr` 中注入的变量是 `step_{stem}`（如 `step_1`），这与前端模板中常用的 `{{step_1.body}}` 一致。但 `eval_expr` 中不会注入 `input_ports` 的数据，表达式中无法直接引用 `input_ports.xxx`。

---

## 5. node-schema.json 对照

### 后端注册节点 vs Schema 定义

后端 `executor.rs` 注册（含 v9.0 新增）共约 **57 个类型**，node-schema.json 定义 **48 个类型**。

#### Schema 中缺少的节点（后端已注册，但 schema 未定义）

| 节点类型 | 后端注册位置 | 状态 | 影响 |
|---------|------------|------|------|
| `llm_embedding` | `executor.rs:163` | 🔴 缺失 | 前端无法识别此节点类型，UI 不显示 |
| `llm_agent` | `executor.rs:164` | 🔴 缺失 | 前端无法识别此节点类型，UI 不显示 |
| `text_splitter` | `executor.rs:165` | 🔴 缺失 | 前端无法识别此节点类型，UI 不显示 |
| `json_schema_extract` | `executor.rs:166` | 🔴 缺失 | 前端无法识别此节点类型，UI 不显示 |
| `vector_store` | `executor.rs:167` | 🔴 缺失 | 前端无法识别此节点类型，UI 不显示 |
| `rag_query` | `executor.rs:168` | 🔴 缺失 | 前端无法识别此节点类型，UI 不显示 |
| `redis` | `executor.rs:169` | 🔴 缺失 | 前端无法识别此节点类型，UI 不显示 |
| `mongodb` | `executor.rs:170` | 🔴 缺失 | 前端无法识别此节点类型，UI 不显示 |
| `s3` | `executor.rs:171` | 🔴 缺失 | 前端无法识别此节点类型，UI 不显示 |

**共 9 个 v9.0 新增节点在 node-schema.json 中缺失定义。**

前端 `types.ts` 中的 `ContainerType` 联合类型已经包含了这些节点名（如 `mcp_script` 等），但 `node-schema.json` 是**唯一真相来源**，前端通过 `useNodeSchema` 或 `useRegistry` 读取的节点列表将不包含这 9 个节点。用户无法在画布编辑器中看到和使用它们。

#### container_types 对比

- `node-schema.json` 的 `container_types`: `['browser', 'excel', 'word', 'file', 'cursor', 'loop', 'while']`
- `executor.rs` 中 `register_containers!` 注册: `browser`, `excel`, `word`, `file`
- `loop`, `while`, `cursor` 通过 `register!` 单独注册，但 `registry.is_container()` 会返回 `true`

⚠️ 代码组织上，`register_containers!` 宏和 `register!` 宏混用，但 `is_container` 判断仅依赖 `node-schema.json` 的 `container_types`。这本身不会导致功能错误，但增加了维护复杂度。如果 `node-schema.json` 的 `container_types` 与 `executor.rs` 的注册逻辑不同步，会导致容器路径判断错误。

---

## 6. 调试 API 前后对照

### DebugPanel.vue 调用 API

| 前端 API | 后端 Tauri 命令 | 后端 HTTP 路由 | 状态 | 问题 |
|---------|---------------|-------------|------|------|
| `debug_step` | ✅ | `POST /api/debug/step/{run_id}` | ✅ 正常 | |
| `debug_continue` | ✅ | `POST /api/debug/continue/{run_id}` | ✅ 正常 | |
| `debug_vars` | ✅ | `GET /api/debug/vars/{run_id}` | ✅ 正常 | |
| `run_cancel` | ✅ | `POST /api/runs/{run_id}/cancel` | ✅ 正常 | |

### 调试事件对比

| 事件 | 后端发射 | 前端接收 | 状态 | 问题 |
|------|---------|---------|------|------|
| `breakpoint-hit` | `scheduler.rs` | `DebugPanel.vue` | ✅ 匹配 | ⚠️ 字段不一致（见 §2） |
| `run-update` | `scheduler.rs`, `run_manager.rs` | `DebugPanel.vue` | ✅ 匹配 | 无 |
| `variable-update` | `scheduler.rs` | ❌ 无监听 | 🔴 断点 | 实时变量推送未生效 |

### 调试快照数据结构

后端 `debug_vars` 返回：
```json
{
  "variables": { ... },
  "step_outputs": { ... }
}
```

前端 `DebugPanel.vue:105` 处理：
```typescript
const vars = await safeInvoke('debug_vars', { runId: props.workflowId })
variables.value = {
  ...(v.variables || {}),
  ...(v.step_outputs || {}),
}
```
✅ 展平逻辑正确。

---

## 总结：问题清单与优先级

| 优先级 | 问题 | 影响 | 建议修复 |
|--------|------|------|----------|
| 🔴 **高** | `input_ports` 在单节点层级不注入 | DAG 串行链中节点无法通过 `input_ports` 读取上游数据 | 在 `run_graph` 的单节点分支中注入 `input_ports` |
| 🔴 **高** | 9 个 v9.0 节点未在 `node-schema.json` 中定义 | 前端画布不显示这些节点，用户无法使用 | 在 `node-schema.json` 中添加缺失节点定义 |
| 🔴 **高** | `ErrorStrategy` 序列化方向不一致 | 后端输出 `{"branch": {"step_id": "..."}}`，前端期望 `{"branch": "..."}` | 修改后端 serialize 逻辑或前端类型定义 |
| 🔴 **高** | `body_steps` 字段前端缺失 | 容器子步骤数据可能丢失 | 前端 `Step` 类型添加 `body_steps?: Step[]`，或确认是否已废弃 |
| 🟡 **中** | `variable-update` 事件前端无监听 | 实时变量监视功能失效，只能靠轮询 | 在 `DebugPanel.vue` 或 `useStepRunner.ts` 中监听 `variable-update` |
| 🟡 **中** | `breakpoint-hit` 事件字段不一致 | `reason` 字段在普通断点命中时缺失 | 后端统一添加 `reason: "breakpoint"` 或前端改为可选类型 |
| 🟡 **中** | `run_pause` / `run_resume` 浏览器模式不可用 | 前端 tauri.ts 缺少 HTTP 映射 | 在 `FIXED_ROUTES` 或 `DYNAMIC_ROUTES` 中添加映射 |
| 🟢 **低** | `RetryConfig` 字段名 `max` vs `max_retries` | 前端读取后端数据时字段名不一致 | 将后端字段改为 `max_retries`，或前端兼容读取 |
| 🟢 **低** | `expanded` 类型 `boolean` vs `Option<bool>` | 可能得到 `undefined` | 前端添加默认值处理 |
| 🟢 **低** | 多个 API 有 Tauri 命令但无 HTTP 映射 | 仅在浏览器模式不可用 | 按需补充 HTTP 映射（目前前端未调用，可延后） |

---

*报告生成完毕。*
