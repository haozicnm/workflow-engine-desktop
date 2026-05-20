# Workflow Engine — 源码全面分析报告

> 版本 6.6.0 | 分析时间：2026-05-06

## 1. 应用入口

```
main.ts → App.vue（手动管理视图，不使用 Vue Router）
```

App.vue 用 `currentView` 状态切换页面，**不使用 router/index.ts**：

```typescript
type MainView = 'welcome' | 'editor' | 'settings' | 'history'

// welcome  → Dashboard.vue   （工作流列表）
// editor   → Editor.vue      （步骤列表编辑器）
// settings → Settings.vue
// history  → RunHistory.vue
```

**LiteGraphEditor.vue 虽然在 router/index.ts 注册了路由，但 App.vue 不用 router，main.ts 也不调用 app.use(router)，所以这个页面完全不可访问，是死代码。**

---

## 2. 两条执行路径的真实使用情况

| 执行模式 | 入口命令 | 调用者 | 是否在用 |
|---------|---------|--------|---------|
| 顺序模式 | `run_start` | Dashboard.vue + Editor.vue (via useStepRunner) | ✅ 主路径 |
| DAG 模式 (FlowNode) | `run_dag_start` | 无调用者 | ❌ 死代码 |
| DAG 模式 (JSON) | `dag_run_start` | LiteGraphEditor.vue | ❌ 死代码（页面未接入） |

**结论：只有顺序模式在实际使用。DAG 模式的代码虽然完整，但没有前端入口在调用。**

---

## 3. 当前实际执行流程

```
Dashboard.vue 点击「运行」→ safeInvoke('run_start', { workflowId })
Editor.vue 点击「运行」  → useStepRunner → safeInvoke('run_start', { workflowId })
                                  │
                                  ▼
                        commands/run.rs::run_start
                                  │
                                  ▼
                        parser::parse_workflow(yaml)  ← 从 DB 读取
                                  │
                                  ▼
                        scheduler::run_workflow()     ← 顺序调度
                                  │
                                  ▼
                        executor.execute() → NodeExecutor.execute()
```

---

## 4. 在用代码清单

### 前端（Vue 3）

| 文件 | 职责 |
|------|------|
| `App.vue` | 主入口，视图切换（welcome/editor/settings/history） |
| `pages/Dashboard.vue` | 工作流列表，运行入口（调用 `run_start`） |
| `pages/Editor.vue` | 步骤列表编辑器，运行入口（调用 `run_start`） |
| `pages/RunHistory.vue` | 运行历史查看 |
| `pages/Settings.vue` | 设置页面 |
| `composables/useStepRunner.ts` | 调用 `run_start`，监听 step-update / run-update 事件 |
| `stores/workflowStore.ts` | 工作流 CRUD，序列化为 JSON 存入 DB（字段名叫 yaml 但内容是 JSON） |
| `types/workflow.ts` | 前端数据模型：Step / Action / ContainerType / ActionDef |

### 后端（Rust）

| 文件 | 职责 |
|------|------|
| `commands/run.rs::run_start` | 顺序执行入口，读 DB → parse → scheduler |
| `commands/workflow.rs` | 工作流 CRUD 命令 |
| `commands/schedule.rs` | 定时调度命令 |
| `commands/browser_recording.rs` | 浏览器录制命令 |
| `commands/system.rs` | 系统设置命令 |
| `commands/template.rs` | 模板管理命令 |
| `engine/parser.rs` | 前端 JSON → 后端 Step 转换（容器类型名透传（v8），actions 移入 config） |
| `engine/scheduler.rs` | 顺序调度器（loop 执行步骤，支持断点/暂停/取消/错误策略） |
| `engine/executor.rs` | 步骤执行器（60+ 节点注册表，resolve_config 变量替换后分发到 NodeExecutor） |
| `engine/context.rs` | 执行上下文（variables + step_outputs + resolve_var 三层查找） |
| `engine/workflow.rs` | 数据结构（Workflow / Step / ErrorStrategy / LogicConditionGroup） |
| `engine/state.rs` | 运行状态管理 |
| `nodes/traits.rs` | NodeExecutor trait 定义 |
| `nodes/*.rs` | 60+ 种节点实现 |
| `data/db.rs` | SQLite 持久化 |
| `data/config.rs` | 配置管理 |
| `platform/*.rs` | 平台适配（Windows/Linux） |
| `system/tray.rs` | 系统托盘 |
| `system/scheduler.rs` | 定时任务调度器 |

---

## 5. 未接入的代码清单（DAG 相关）

| 文件 | 说明 |
|------|------|
| `pages/LiteGraphEditor.vue` | Canvas 画布编辑器，未接入 App.vue |
| `router/index.ts` | Vue Router 配置，App.vue 不用 |
| `stores/flowStore.ts` | LiteGraphEditor 的 store，未接入 |
| `stores/editorStore.ts` | 编辑器 UI 状态，可能未使用 |
| `stores/tabStore.ts` | 多标签页管理，可能未使用 |
| `commands/run.rs::run_dag_start` | DAG 执行入口（FlowNode/FlowEdge），无调用者 |
| `commands/dag_run.rs` | DAG 执行入口（JSON 格式），只有 LiteGraphEditor 调用 |
| `engine/dag.rs` | DAG 数据结构 + Kahn 拓扑排序 + 并行组识别 |
| `engine/dag_scheduler.rs` | DAG 调度器（并行执行、条件路由、容器 session） |
| `commands/pipeline.rs` | 流水线命令 |
| `commands/preview.rs` | 预览命令 |

---

## 6. 数据流详解

### 6.1 工作流存储格式

前端 `workflowStore.ts` 的 `serializeWorkflow()` 把 Workflow 对象序列化为 JSON 字符串，存入 DB 的 `yaml` 字段（字段名是 yaml 但内容是 JSON）：

```
前端 Workflow 对象
    → JSON.stringify()
    → 存入 DB（字段名: yaml，内容: JSON 字符串）
    → 读取时 JSON.parse() 还原
```

### 6.2 前后端格式转换（parser.rs）

```
前端 JSON:
{
  "type": "browser",           ← 容器类型
  "label": "浏览器操作",
  "actions": [
    { "id": "act_123", "type": "navigate", "label": "打开页面", "params": { "url": "..." } }
  ]
}

         ↓ parser::convert_step()

后端 Step:
{
  "step_type": "browser_container",    ← 类型名透传（v8）
  "name": "浏览器操作",                 ← label → name
  "config": {
    "actions": [                        ← actions 移入 config
      { "id": "act_123", "type": "navigate", "label": "打开页面", "config": { "url": "..." } }
    ]                                   ← params → config
  }
}
```

### 6.3 变量引用流程

```
模板字符串: "{{step_a1b2c3.extractText}}"
                │
                ▼
    executor.execute() 内部调用 ctx.resolve_config(&step.config)
                │
                ▼ (递归遍历 JSON，遇到 String 调用 resolve_string)
    ctx.resolve_string("{{step_a1b2c3.extractText}}")
                │
                ├─ 纯 {{xxx}} → ctx.resolve_var("step_a1b2c3.extractText")
                │       │
                │       ▼
                │   parts = ["step_a1b2c3", "extractText"]
                │   root_key = "step_a1b2c3"
                │       │
                │       ▼ 三层查找：
                │   1. step_outputs["step_a1b2c3"]  ← 完整 key（模板引用）
                │   2. variables["step_a1b2c3"]      ← 工作流变量
                │   3. step_outputs["a1b2c3"]        ← strip step_ 前缀（Rhai 兼容）
                │       │
                │       ▼
                │   root = { "extractText": "hello" }
                │   current.get("extractText") → "hello"
                │
                └─ 混合模板 → interpolate(s) 拼接
```

### 6.4 容器执行模型

```
浏览器容器 (BrowserContainerNode)
    │
    ├─ config.actions = [
    │    { type: "navigate",  label: "打开页面", config: { url: "..." } },
    │    { type: "click",     label: "登录按钮", config: { selector: "#login" } },
    │    { type: "extract",   label: "提取标题", config: { selector: "h1" } },
    │  ]
    │
    ├─ 遍历 actions，按 type 分发到 send_sidecar_action()
    │
    ├─ navigate/click/input → 操作类，不产出数据
    ├─ extract/screenshot/evaluate → 查询类，产出数据到 output_ports[label]
    │
    └─ 返回 output_ports = { "提取标题": "Welcome" }
         → 存入 ctx.step_outputs["step_browser1"]
```

---

## 7. 关键设计决策

| 决策 | 选择 | 原因 |
|------|------|------|
| 前后端通信 | Tauri IPC (invoke/emit) | 零网络开销，类型安全 |
| 浏览器自动化 | Playwright Python sidecar | 跨浏览器，Rust 生态无更好选择 |
| 脚本引擎 | Rhai | 嵌入式，沙箱化，Rust 原生 |
| 步骤 ID | `uid('step')` → 12 位随机 hex | 短且唯一，前端生成 |
| 容器输出 key | action.label | 用户可读，前端直接显示 |
| 变量引用格式 | `{{stepId.actionLabel}}` | step_outputs 用 id 定位，.actionLabel 取子字段 |
| 存储格式 | JSON（字段名叫 yaml） | 历史遗留，字段名未改 |
| 错误策略 | Fail / Ignore / Branch | 灵活的错误恢复 |
| 视图切换 | 手动 currentView 状态 | 不依赖 Vue Router |
