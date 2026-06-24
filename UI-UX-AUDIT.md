# Workflow Engine v9.0.1 前端 UI/UX 审查报告

**审查日期：** 2026-06-21  
**审查范围：** 8 个核心组件 + 6 个状态/工具文件 + 5 个页面 + 初始化配置  
**审查方法：** 代码走读 + 逻辑推演 + 规范对比（DESIGN.md）

---

## 一、总体结论

**前端存在 9 个 🔴 严重问题，影响核心功能可用性。** 主要集中在：
1. Canvas 交互缺陷（连线、删除、搜索）
2. 状态管理数据一致性（save 非原子、类型不匹配）
3. 关键组件缺失（VariablesPanel、Settings 集成配置）
4. 用户体验断层（自动布局覆盖手动位置、无 dirty 确认）

**整体 UX 评分：6.5/10**（功能框架完整，细节和交互体验需要打磨）

---

## 二、🔴 严重问题（9 个）

### 2.1 CanvasEdge.vue — 删除按钮完全失效

| 项目 | 详情 |
|------|------|
| **位置** | `CanvasEdge.vue`，删除按钮 `text` 元素 |
| **问题** | 删除按钮使用 `class="pointer-events-none"`，完全阻止了点击事件 |
| **影响** | 用户无法删除任何边，只能通过删除节点间接删除 |
| **修复** | 移除 `pointer-events-none`，改为 `pointer-events-auto` 或把事件绑在 `circle` 上 |

### 2.2 CanvasEditor.vue — 临时连线起点固定在 (0,0)

| 项目 | 详情 |
|------|------|
| **位置** | `CanvasEditor.vue`，拖拽连线逻辑 |
| **问题** | 临时连线（dragLine）的起点固定在 `(0,0)`，而非实际端口位置 |
| **影响** | 用户拖拽连线时，视觉表现完全错误，无法判断连线是否正确连接 |
| **修复** | 在 `startDragConnection` 时记录实际端口坐标，作为 `dragLine.start` 的初始值 |

### 2.3 AddStepDialog.vue — 完全没有搜索功能

| 项目 | 详情 |
|------|------|
| **位置** | `AddStepDialog.vue` 整体 |
| **问题** | 没有搜索框，节点按 MCP/非 MCP 简单分类，没有按 category（core/data/flow/ai 等）分组 |
| **影响** | 34+ 节点时用户需要滚动查找，体验极差；没有键盘导航（↑↓Enter Esc） |
| **修复** | 添加搜索框（支持名称/类型/描述模糊搜索）；按 category 分组；添加键盘导航 |

### 2.4 StepCard.vue — 缺少删除步骤操作

| 项目 | 详情 |
|------|------|
| **位置** | `StepCard.vue`，DropdownMenu |
| **问题** | DropdownMenu 中没有"删除步骤"选项；emit 定义了但 UI 未暴露 |
| **影响** | 用户无法从步骤卡片删除步骤，只能通过画布操作 |
| **修复** | 在 DropdownMenu 中添加"删除步骤"和"复制步骤"选项 |

### 2.5 VariablesPanel.vue — 组件不存在

| 项目 | 详情 |
|------|------|
| **位置** | `src/components/VariablesPanel.vue` |
| **问题** | 文件不存在，变量展示仅在 `DebugPanel.vue` 内嵌实现 |
| **影响** | 没有独立的变量面板，变量查看体验受限 |
| **修复** | 创建独立组件，支持按节点分组、JSON 树展开、搜索过滤 |

### 2.6 useCanvas.ts — autoLayout 覆盖用户手动位置

| 项目 | 详情 |
|------|------|
| **位置** | `useCanvas.ts`，deep watch on steps/edges |
| **问题** | `watch(() => steps.value, () => autoLayout(), { deep: true })` — 任何 step/edge 变化都会触发自动布局，覆盖用户手动拖拽的位置 |
| **影响** | 用户精心调整的位置在添加/删除节点后被重置，体验极差 |
| **修复** | 添加 `hasUserLayout` 标志；仅在首次加载或显式调用时触发 autoLayout |

```typescript
// 建议修复
const hasUserLayout = ref(false)
watch(() => steps.value, () => {
  if (!hasUserLayout.value) autoLayout()
}, { deep: true })

function updateNodePosition(id: string, pos: Position) {
  hasUserLayout.value = true
  // ... 更新位置
}
```

### 2.7 workflowStore.ts — saveWorkflow 非原子操作

| 项目 | 详情 |
|------|------|
| **位置** | `workflowStore.ts`，`saveWorkflow` action |
| **问题** | 先 `workflow_create` → 再 `workflow_save_yaml` → 再 `workflow_update`，三次独立 API 调用 |
| **影响** | 中间任意一步失败会产生孤儿数据（已创建 ID 但内容未保存）；网络中断时无法回滚 |
| **修复** | 合并为单次原子操作，或添加事务回滚逻辑 |

### 2.8 types.ts — 与后端 Rust 模型严重不一致

| 后端字段 | 前端状态 | 影响 |
|----------|----------|------|
| `Step.next: Option<String>` | ❌ 缺失 | 线性链兼容字段缺失，旧格式工作流无法正确渲染 |
| `Step.retry: Option<RetryConfig>` | ❌ 缺失 | 重试配置无法在前端编辑和显示 |
| `Step.timeout: Option<u64>` | ❌ 缺失 | 超时配置无法在前端编辑和显示 |
| `Workflow.variables: Option<HashMap<String, Value>>` | ❌ 缺失 | 工作流级变量无法在前端管理 |
| `Step.trigger/schedule` | ❌ 缺失 | 触发器配置无法在前端编辑 |

**修复：** 补齐所有缺失字段，确保前后端模型一致。

### 2.9 Settings.vue — 缺少集成服务配置

| 项目 | 详情 |
|------|------|
| **位置** | `Settings.vue` 整体 |
| **问题** | 没有 LLM API Key、SMTP 配置、Webhook Token、IM Webhook URL 等集成服务配置 |
| **影响** | `llm_chat`、`email_send`、`im_message` 等节点必须将敏感信息硬编码在节点配置中，不安全且不可复用 |
| **修复** | 添加"集成服务"分组，统一管理各服务的 API Key / Token / Webhook URL |

---

## 三、🟡 重要问题（8 个）

### 3.1 CanvasNode.vue — 样式偏离 DESIGN.md 规范

| 规范项 | DESIGN.md 要求 | 当前实现 | 差距 |
|--------|---------------|----------|------|
| 边框 | 1px | `border-2` (2px) | +1px |
| 背景 | `hsl(var(--secondary))` | `bg-card` | 颜色偏离 |
| 端口半径 | 8px (r=4) | r=5 (10px) | +2px |
| 运行状态 | `border-primary` | `border-info` | 颜色错误 |
| z-index | 通过 DOM 顺序 | `<g class="z-10">` | SVG 中无效 |

### 3.2 CanvasNode.vue — 输出端口缺少 `data-port-target`

**问题：** 只有输入端口有 `data-port-target` 属性，输出端口没有。`findPortTarget` 依赖此属性进行 hit test，导致输出端口无法被正确识别为连接目标。

**修复：** 在输出端口也添加 `data-port-target` 属性。

### 3.3 DebugPanel.vue — 变量不可展开

**问题：** 变量值截断到 200 字符，没有可展开的 JSON 树组件。用户无法查看复杂对象/数组的完整内容。

**修复：** 添加 JSON 树组件（如 `vue-json-pretty` 或自实现折叠树）。

### 3.4 Editor.vue — 缺少暂停/调试按钮

**问题：** 工具栏只有运行（▶）和停止（■）按钮，缺少暂停（⏸）和调试（🐞）按钮。虽然后端支持单步/断点/暂停，但前端没有暴露这些操作。

**修复：** 在 `WorkflowHeader.vue` 工具栏添加暂停和调试按钮。

### 3.5 utils/tauri.ts — SSE 内存泄漏

**问题：** `_eventSource` 是全局单例，永不关闭。组件卸载时连接仍然保持，导致内存泄漏。

**修复：** 在组件 `onUnmounted` 中调用 `eventSource.close()`；或返回 unlisten 函数让调用方清理。

### 3.6 useEditorActions.ts — 撤销/重做快捷键未绑定

**问题：** `useEditorEnhancements` 实现了 `undo`/`redo`，但 `useEditorActions` 没有暴露它们，全局没有 `keydown` 监听。用户无法通过 Ctrl+Z/Y 使用撤销/重做。

**修复：** 在 `useEditorActions` 中绑定 `Ctrl+Z` / `Ctrl+Y` / `Ctrl+Shift+Z` 快捷键。

### 3.7 多处缺少 dirty 离开确认

| 场景 | 当前行为 | 期望行为 |
|------|----------|----------|
| Editor 切换工作流 | 直接切换，不提示 | 提示"有未保存的更改，是否保存？" |
| Editor 关闭窗口 | 直接关闭 | 拦截 `beforeunload`，提示保存 |
| Settings 关闭窗口 | 直接关闭 | 拦截 `beforeunload`，提示保存 |

### 3.8 Marketplace.vue — 无预览图

**问题：** `Template` 接口和 UI 中均无 `preview_image` 字段，模板卡片只有文字描述，用户无法直观预览。

**修复：** 在模板 schema 中增加 `preview_image` 字段，在卡片中渲染预览图。

---

## 四、🟢 已正确实现的亮点

| 功能 | 实现质量 | 说明 |
|------|----------|------|
| 三栏布局 | ✅ | 顶部工具栏 + 左侧步骤列表 + 主画布 + 右侧面板 |
| 视图切换 | ✅ | Visual / Canvas / Code 三种视图 |
| 实时事件监听 | ✅ | `step-update` / `run-update` SSE 事件实时更新 |
| 加载骨架屏 | ✅ | Dashboard、Marketplace 均有骨架屏 |
| 空状态处理 | ✅ | 无工作流、无步骤、无搜索结果均有空状态提示 |
| Toast 反馈 | ✅ | 保存成功/失败、运行状态等均有 Toast 提示 |
| 主题切换 | ✅ | system/light/dark 三种模式 |
| 全局错误边界 | ✅ | `ErrorBoundary.vue` 包裹全局树 |
| 操作控制台 | ✅ | 底部可折叠日志面板 |
| 拖拽导入 | ✅ | 支持拖拽 JSON/YAML 文件导入工作流 |
| 键盘快捷键 | ✅ | `Ctrl+S`, `Ctrl+Z`, `Ctrl+Y`, `Ctrl+F`, `Esc` |
| 自动保存 | ✅ | 3 秒防抖自动保存 |

---

## 五、修复优先级清单

### 🔴 立即修复（本周）

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 1 | 边删除按钮无法点击 | `CanvasEdge.vue` | 移除 `pointer-events-none` |
| 2 | 临时连线起点错误 | `CanvasEditor.vue` | 记录实际端口坐标 |
| 3 | AddStepDialog 无搜索 | `AddStepDialog.vue` | 添加搜索框 + category 分组 |
| 4 | StepCard 无删除操作 | `StepCard.vue` | DropdownMenu 添加删除/复制 |
| 5 | autoLayout 覆盖手动位置 | `useCanvas.ts` | 添加 `hasUserLayout` 标志 |
| 6 | saveWorkflow 非原子 | `workflowStore.ts` | 合并 API 调用或添加事务回滚 |
| 7 | 类型与后端不一致 | `types.ts` | 补齐 Step.next/retry/timeout 等 |
| 8 | Settings 缺少集成配置 | `Settings.vue` | 添加集成服务分组 |
| 9 | VariablesPanel 不存在 | 新建 | 创建独立变量面板组件 |

### 🟡 短期修复（2 周内）

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 10 | CanvasNode 样式偏离规范 | `CanvasNode.vue` | 按 DESIGN.md 调整边框/背景/端口/颜色 |
| 11 | 输出端口缺 data-port-target | `CanvasNode.vue` | 添加属性 |
| 12 | DebugPanel 变量不可展开 | `DebugPanel.vue` | 添加 JSON 树组件 |
| 13 | 缺少暂停/调试按钮 | `WorkflowHeader.vue` | 添加按钮 |
| 14 | SSE 内存泄漏 | `utils/tauri.ts` | 组件卸载时关闭 EventSource |
| 15 | 撤销快捷键未绑定 | `useEditorActions.ts` | 绑定 Ctrl+Z/Y |
| 16 | dirty 离开确认 | `Editor.vue`, `Settings.vue` | 添加 `beforeunload` 拦截 |
| 17 | Marketplace 无预览图 | `Marketplace.vue`, `types.ts` | 添加 preview_image 字段和渲染 |

### 🟢 建议优化

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 18 | 缺少 CSP | `index.html` | 添加 `<meta http-equiv="Content-Security-Policy">` |
| 19 | 字体依赖 Google CDN | `index.html` | 转为本地字体或添加 fallback |
| 20 | 全局错误处理 | `main.ts` | 添加 `window.onerror` / `unhandledrejection` |
| 21 | 虚拟渲染 | `useCanvas.ts` | 节点 > 100 时启用 |
| 22 | removeEdge 不实际工作 | `useCanvas.ts` | 修复数组修改逻辑 |
| 23 | 拓扑布局遗漏孤立节点 | `useCanvas.ts` | 处理无边节点 |

---

## 六、前后端对接问题

### 6.1 事件流对接

| 前端 | 后端 | 状态 |
|------|------|------|
| `step-update` | `ExecutionEvent::StepUpdate` | ✅ 正常对接 |
| `run-update` | `ExecutionEvent::RunUpdate` | ✅ 正常对接 |
| `variable-snapshot` | `ExecutionEvent::VariableSnapshot` | ✅ 正常对接 |
| `debug-snapshot` | 后端通过 `update_debug_snapshot` 写入 | ⚠️ 前端未显示快照历史 |
| `debug-update` | 后端有 `emit_debug_update` | ⚠️ 前端未处理 |

### 6.2 API 对接

| 前端调用 | 后端路由 | 状态 |
|----------|----------|------|
| `workflow_list` | `GET /api/workflows` | ✅ |
| `workflow_create` | `POST /api/workflows` | ✅ |
| `workflow_save_yaml` | `POST /api/workflows/{id}/yaml` | ✅ |
| `workflow_update` | `PUT /api/workflows/{id}` | ✅ |
| `workflow_run` | `POST /api/workflows/{id}/run` | ✅ |
| `workflow_stop` | `POST /api/workflows/{id}/stop` | ✅ |
| `debug_step` | `POST /api/debug/step/{run_id}` | ⚠️ 前端有按钮但可能未绑定 |
| `debug_continue` | `POST /api/debug/continue/{run_id}` | ⚠️ 同上 |
| `webhook_test` | `GET /api/webhooks/{id}/test` | ❌ 前端无调用 |
| `llm_models` | `GET /api/llm/models` | ❌ 前端无调用 |
| `llm_test` | `POST /api/llm/test` | ❌ 前端无调用 |

---

*审查方法：代码走读（8 个组件 + 6 个状态文件 + 5 个页面 + 2 个初始化文件）+ 规范对比（DESIGN.md）+ 前后端 API 对照*
