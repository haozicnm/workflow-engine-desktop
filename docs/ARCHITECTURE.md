# Workflow Engine Desktop — 架构文档

## 1. 项目概述

Workflow Engine Desktop 是一个桌面端工作流自动化引擎，用户通过可视化界面编排步骤，引擎按序或按 DAG 拓扑执行，支持浏览器操作、Excel/Word 处理、条件分支、循环、HTTP 请求等 60+ 种节点类型。

**技术栈**：
- **前端**：Vue 3 + TypeScript + Tailwind CSS + shadcn-vue
- **后端**：Rust (Tauri 2.x) + Tokio 异步运行时
- **浏览器自动化**：Playwright (Python sidecar)
- **脚本引擎**：Rhai (嵌入式脚本语言)
- **数据库**：SQLite (运行历史持久化)

---

## 2. 整体架构

```
┌─────────────────────────────────────────────────────────────┐
│                      Tauri Window                            │
│  ┌───────────────────────────────────────────────────────┐  │
│  │                   Vue 3 前端                           │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐  │  │
│  │  │ Dashboard│ │  Editor  │ │RunHistory│ │Settings │  │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └─────────┘  │  │
│  │  ┌──────────────────────────────────────────────┐     │  │
│  │  │          Pinia Stores (状态管理)               │     │  │
│  │  │  workflowStore │ editorStore │ tabStore       │     │  │
│  │  └──────────────────────────────────────────────┘     │  │
│  └────────────────────┬──────────────────────────────────┘  │
│                       │ Tauri IPC (invoke / emit / listen)   │
│  ┌────────────────────┴──────────────────────────────────┐  │
│  │                  Rust 后端                              │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐  │  │
│  │  │commands/ │ │ engine/  │ │  nodes/  │ │  data/   │  │  │
│  │  │ 命令层   │ │ 执行引擎 │ │ 节点系统 │ │ 数据持久化│  │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └──────────┘  │  │
│  │  ┌──────────┐ ┌──────────┐                            │  │
│  │  │platform/ │ │ system/  │                            │  │
│  │  │平台适配  │ │ 系统集成  │                            │  │
│  │  └──────────┘ └──────────┘                            │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              外部进程 (Sidecar)                         │  │
│  │  ┌──────────────┐  ┌──────────────┐                   │  │
│  │  │ Playwright   │  │ Python 脚本  │                   │  │
│  │  │ (浏览器自动化) │  │ (用户自定义) │                   │  │
│  │  └──────────────┘  └──────────────┘                   │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## 3. 目录结构

```
workflow-engine-desktop/
├── src/                          # 前端 Vue 3 项目
│   ├── main.ts                   # 入口
│   ├── App.vue                   # 根组件（路由 + 全局布局）
│   ├── router/index.ts           # Vue Router
│   ├── pages/                    # 页面组件
│   │   ├── Dashboard.vue         # 工作流列表首页
│   │   ├── Editor.vue            # 工作流编辑器（主页面）
│   │   ├── RunHistory.vue        # 运行历史
│   │   └── Settings.vue          # 设置页
│   ├── components/               # UI 组件
│   │   ├── StepCard.vue          # 步骤卡片（容器级）
│   │   ├── ActionRow.vue         # 动作行（内嵌参数编辑）
│   │   ├── LogicBranch.vue       # 条件分支容器
│   │   ├── ActionPanel.vue       # 动作选择面板
│   │   ├── StatusBar.vue         # 底部状态栏
│   │   ├── SchedulePanel.vue     # 定时调度面板
│   │   ├── SettingsPanel.vue     # 设置面板
│   │   └── ui/                   # shadcn-vue 基础组件
│   ├── stores/                   # Pinia 状态管理
│   │   ├── workflowStore.ts      # 工作流 CRUD + 执行状态
│   │   ├── editorStore.ts        # 编辑器 UI 状态
│   │   └── tabStore.ts           # 多标签页管理
│   ├── composables/              # 组合式函数
│   │   ├── useStepRunner.ts      # 步骤执行监听（Tauri events）
│   │   ├── useEditorEnhancements.ts
│   │   ├── useGlobalStatus.ts
│   │   └── useTheme.ts
│   ├── types/
│   │   └── workflow.ts           # 前端数据模型定义
│   └── utils/
│       ├── cron.ts               # Cron 表达式工具
│       └── tauri.ts              # Tauri API 封装
│
├── src-tauri/                    # Rust 后端
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── src/
│   │   ├── main.rs               # 程序入口
│   │   ├── lib.rs                # Tauri 插件注册
│   │   ├── commands/             # Tauri 命令（IPC 接口层）
│   │   │   ├── run.rs            # 执行工作流
│   │   │   ├── dag_run.rs        # DAG 模式执行
│   │   │   ├── workflow.rs       # 工作流 CRUD
│   │   │   ├── schedule.rs       # 定时调度
│   │   │   ├── template.rs       # 模板管理
│   │   │   ├── pipeline.rs       # 流水线
│   │   │   ├── preview.rs        # 预览
│   │   │   ├── browser_recording.rs # 录制回放
│   │   │   └── system.rs         # 系统级命令
│   │   ├── engine/               # 执行引擎核心
│   │   │   ├── workflow.rs       # 数据结构：Workflow / Step
│   │   │   ├── parser.rs         # 前端 JSON → 执行格式转换
│   │   │   ├── context.rs        # 执行上下文（变量存储 + 解析）
│   │   │   ├── executor.rs       # 步骤执行器（节点注册表）
│   │   │   ├── scheduler.rs      # 顺序调度器
│   │   │   ├── dag.rs            # DAG 数据结构 + 拓扑排序
│   │   │   ├── dag_scheduler.rs  # DAG 调度器（并行执行）
│   │   │   ├── state.rs          # 运行状态管理
│   │   │   └── collect.rs        # 数据收集
│   │   ├── nodes/                # 节点实现（60+ 种）
│   │   │   ├── traits.rs         # NodeExecutor trait
│   │   │   ├── browser.rs        # 浏览器节点（sidecar 通信）
│   │   │   ├── browser_container.rs # 浏览器容器
│   │   │   ├── excel.rs / excel_container.rs
│   │   │   ├── word.rs / word_container.rs
│   │   │   ├── condition.rs      # 条件判断
│   │   │   ├── http.rs           # HTTP 请求
│   │   │   ├── script.rs         # Rhai 脚本
│   │   │   ├── loop_node.rs      # 循环
│   │   │   ├── while_node.rs     # While 循环
│   │   │   ├── parallel.rs       # 并行执行
│   │   │   ├── data.rs           # 数据操作（set/get/merge）
│   │   │   ├── file.rs           # 文件操作
│   │   │   ├── clipboard.rs      # 剪贴板
│   │   │   ├── regex.rs          # 正则
│   │   │   ├── array.rs          # 数组操作
│   │   │   ├── convert.rs        # 类型转换
│   │   │   ├── ...               # 更多节点
│   │   │   └── registry.rs       # 节点注册表
│   │   ├── data/                 # 数据持久化
│   │   │   ├── db.rs             # SQLite 数据库
│   │   │   ├── models.rs         # 数据模型
│   │   │   ├── config.rs         # 配置管理
│   │   │   └── paths.rs          # 路径解析
│   │   ├── platform/             # 平台适配层
│   │   │   ├── windows.rs        # Windows 特有实现
│   │   │   ├── linux.rs          # Linux 特有实现
│   │   │   └── traits.rs         # 平台抽象 trait
│   │   └── system/               # 系统集成
│   │       ├── tray.rs           # 系统托盘
│   │       └── scheduler.rs      # 定时任务调度器
│   └── sidecars/
│       └── playwright_driver.py  # Playwright Python sidecar
│
├── templates/                    # 内置工作流模板
└── docs/                         # 文档
```

---

## 4. 核心数据模型

### 4.1 前端数据模型（`types/workflow.ts`）

```typescript
// 工作流
interface Workflow {
  name: string
  description?: string
  steps: Step[]
  variables?: Record<string, unknown>
}

// 步骤（容器或简单步骤）
interface Step {
  id: string           // "step_a1b2c3d4e5f6" (uid('step'))
  type: ContainerType  // "browser" | "excel" | "word" | "logic" | "http" | ...
  label: string        // 显示名称
  config: Record<string, unknown>  // 步骤级配置（如 browser 的 headless）
  actions?: Action[]   // 容器内的动作列表
  expanded?: boolean   // UI 展开状态
  // 逻辑分支专用
  condition?: string
  conditionGroup?: LogicConditionGroup
  thenSteps?: Step[]
  elseSteps?: Step[]
}

// 动作（容器内的子操作）
interface Action {
  id: string           // "act_x1y2z3w4" (uid('act'))
  type: string         // "navigate" | "click" | "extract" | "read" | ...
  label: string        // "打开页面" | "点击元素" | ...
  params: Record<string, unknown>  // 动作参数
}
```

### 4.2 后端数据模型（`engine/workflow.rs`）

```rust
struct Workflow {
    name: String,
    description: Option<String>,
    steps: Vec<Step>,
    variables: Option<HashMap<String, Value>>,
}

struct Step {
    id: String,
    name: String,           // 对应前端 label
    step_type: String,      // 对应前端 type（parser 会加 _container 后缀）
    config: Value,          // 步骤配置 + actions（parser 移入）
    next: Option<String>,   // 下一步 ID（顺序链）
    actions: Option<Vec<Value>>,
    body_steps: Option<Vec<Step>>,  // 循环体
    then_steps: Option<Vec<Step>>,  // 条件为真
    else_steps: Option<Vec<Step>>,  // 条件为假
    condition: Option<String>,
    condition_group: Option<LogicConditionGroup>,
    on_error: Option<ErrorStrategy>,
    retry: Option<RetryConfig>,
    timeout: Option<u64>,
    delay: Option<u64>,
    breakpoint: bool,
}
```

### 4.3 前后端格式转换（`engine/parser.rs`）

前端 JSON → 后端 Step 的关键转换：

| 前端字段 | 后端字段 | 转换规则 |
|---------|---------|---------|
| `type: "browser"` | `step_type: "browser_container"` | 容器类型加 `_container` 后缀 |
| `type: "http"` | `step_type: "http"` | 非容器类型不变 |
| `actions` (顶层) | `config.actions` | actions 移入 config |
| `action.params` | `action.config` | params 重命名为 config |
| `label` | `name` | 字段重命名 |
| `thenSteps` / `elseSteps` | `then_steps` / `else_steps` | camelCase → snake_case |

---

## 5. 节点系统

### 5.1 NodeExecutor Trait

所有节点实现统一的 trait：

```rust
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    async fn execute(
        &self,
        step: &Step,                    // 已解析变量的步骤
        ctx: &mut ExecutionContext,      // 执行上下文
        executor: &Arc<StepExecutor>,   // 执行器引用（子步骤递归调用）
    ) -> Result<Value>;
}
```

### 5.2 节点注册表（`engine/executor.rs`）

StepExecutor 内部维护一个 `HashMap<String, Box<dyn NodeExecutor>>`，启动时注册所有节点：

```
┌─ P0 核心节点 ──────────────────┐
│  http, script, condition        │
├─ 数据处理 ─────────────────────┤
│  data_set, data_get, data_merge │
│  data_length, data_default      │
├─ 文件操作 ─────────────────────┤
│  file_read, file_write, file_list│
│  file_delete, file_exists       │
├─ Excel ────────────────────────┤
│  excel, excel_read, excel_write │
│  excel_create, excel_filter     │
│  excel_sort, excel_append       │
├─ Word ─────────────────────────┤
│  word, word_read, word_write    │
│  word_create, word_replace      │
├─ 浏览器 ───────────────────────┤
│  browser_navigate, browser_click│
│  browser_fill, browser_extract  │
│  browser_screenshot, browser_evaluate│
│  browser_scroll, browser_wait   │
│  browser_pdf, browser_container │
├─ 容器 ─────────────────────────┤
│  browser_container, excel_container│
│  word_container, logic_container│
├─ 流程控制 ─────────────────────┤
│  loop, while, parallel, map     │
├─ 工具 ─────────────────────────┤
│  regex_*, array_*, convert_*    │
│  clipboard_*, json_parse        │
│  text_template, notify, delay   │
└────────────────────────────────┘
```

### 5.3 容器节点 vs 简单节点

**简单节点**：直接执行一个操作，返回结果。
```
HTTP 节点 → 发请求 → 返回响应 JSON
```

**容器节点**：包含多个子动作（actions），在一个共享上下文中按序执行。
```
浏览器容器 → [打开页面 → 等待元素 → 点击按钮 → 提取文本] → 返回 {action_label: output}
```

容器节点的输出是一个 `HashMap<String, Value>`，key 是动作的 `label`，value 是该动作的输出。

---

## 6. 执行流程

### 6.1 完整执行链路

```
用户点击「运行」
       │
       ▼
┌──────────────────────────────────────────────────────┐
│  1. 前端序列化                                        │
│     workflowStore.serializeWorkflow(workflow)          │
│     → JSON string                                     │
└──────────────┬───────────────────────────────────────┘
               │ Tauri invoke("run_workflow", { json })
               ▼
┌──────────────────────────────────────────────────────┐
│  2. Parser 解析（engine/parser.rs）                    │
│     parse_workflow(json_str)                          │
│     ├─ JSON → Workflow struct                         │
│     ├─ 容器类型加 _container 后缀                      │
│     ├─ actions 移入 config                            │
│     ├─ action.params → action.config                  │
│     └─ 校验：名称非空、步骤非空、ID 唯一                │
└──────────────┬───────────────────────────────────────┘
               │ Workflow struct
               ▼
┌──────────────────────────────────────────────────────┐
│  3. 创建执行上下文（engine/context.rs）                 │
│     ExecutionContext::new(run_id, workflow)            │
│     ├─ variables: HashMap     ← 工作流变量              │
│     ├─ step_outputs: HashMap  ← 步骤输出（变量引用源）   │
│     ├─ input_ports: HashMap   ← 容器连线数据            │
│     └─ sessions: HashMap      ← 容器 session 管理      │
└──────────────┬───────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────────────┐
│  4. 调度执行                                          │
│                                                      │
│  【顺序模式】scheduler.rs                              │
│    loop {                                             │
│      1. 取当前步骤                                     │
│      2. 断点/单步检查                                   │
│      3. 步骤延迟                                       │
│      4. execute_with_retry(executor, step, ctx)       │
│      5. 成功 → set_output → 继续下一步                 │
│      6. 失败 → 按 on_error 策略处理                    │
│         ├─ Fail: 终止工作流                            │
│         ├─ Ignore: 输出 null，继续                     │
│         └─ Branch: 跳转到指定步骤                      │
│    }                                                  │
│                                                      │
│  【DAG 模式】dag_scheduler.rs                          │
│    1. 拓扑排序                                         │
│    2. 按层执行（同层可并行）                             │
│    3. 容器连线通过 input_ports 传递数据                 │
│    4. 支持条件分支（判断上游输出）                       │
└──────────────┬───────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────────────┐
│  5. 单步执行（engine/executor.rs）                      │
│     executor.execute(step, ctx)                       │
│     ├─ 变量预解析：ctx.resolve_config(&step.config)    │
│     │   递归遍历 config JSON，替换所有 {{变量引用}}     │
│     ├─ 查找节点：executors.get(&step.step_type)       │
│     └─ 调用节点：node.execute(&resolved_step, ctx)    │
└──────────────┬───────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────────────┐
│  6. 节点执行（以浏览器容器为例）                         │
│     BrowserContainerNode::execute()                   │
│     ├─ 解析 BrowserContainerConfig                    │
│     ├─ 获取 input_ports（连线数据）                     │
│     ├─ 遍历 actions，按 action_type 分发：              │
│     │   navigate → send_sidecar_action("navigate")    │
│     │   click    → send_sidecar_action("click")       │
│     │   extract  → send_sidecar_action("extract")     │
│     │              → output_ports[label] = data        │
│     └─ 返回 output_ports（HashMap<label, Value>）      │
└──────────────┬───────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────────────┐
│  7. 结果存储                                          │
│     ctx.set_output(&step_id, output)                  │
│     → step_outputs[step_id] = output                  │
│                                                      │
│     后续步骤可通过 {{step_id.action_label}} 引用       │
└──────────────┬───────────────────────────────────────┘
               │
               ▼
┌──────────────────────────────────────────────────────┐
│  8. 前端实时反馈                                       │
│     Tauri emit("step-update", {                       │
│       step_id, status, output, error                  │
│     })                                                │
│     → useStepRunner.ts 监听事件                        │
│     → 更新 UI 状态（运行中/成功/失败）                  │
└──────────────────────────────────────────────────────┘
```

### 6.2 变量解析流程

```
模板字符串: "https://{{step_a1b2.extractText}}/page"
                │
                ▼
    ctx.resolve_config(config)  ← 递归遍历 JSON
                │
                ▼ (遇到 String 类型)
    ctx.resolve_string("https://{{step_a1b2.extractText}}/page")
                │
                ├─ 纯 {{xxx}} 模板 → ctx.resolve_var("step_a1b2.extractText")
                │       │
                │       ▼
                │   parts = ["step_a1b2", "extractText"]
                │   root_key = "step_a1b2"
                │       │
                │       ▼
                │   查找顺序：
                │     1. step_outputs["step_a1b2"]  ← 完整 key（模板引用）
                │     2. variables["step_a1b2"]      ← 工作流变量
                │     3. step_outputs["a1b2"]        ← strip step_ 前缀（Rhai 兼容）
                │       │
                │       ▼
                │   root = step_outputs["step_a1b2"] = { "extractText": "hello" }
                │   遍历 parts[1..] → current.get("extractText") → "hello"
                │       │
                │       ▼
                │   返回 Value::String("hello")
                │
                └─ 混合模板 → ctx.interpolate(s)
                        → 拼接 "https://" + resolve("step_a1b2.extractText") + "/page"
                        → "https://hello/page"
```

### 6.3 容器执行模型

```
浏览器容器 (BrowserContainerNode)
    │
    ├─ config.actions = [
    │    { type: "navigate",  label: "打开页面", config: { url: "..." } },
    │    { type: "wait",      label: "等待加载", config: { selector: "#app" } },
    │    { type: "click",     label: "登录按钮", config: { selector: "#login" } },
    │    { type: "extract",   label: "提取标题", config: { selector: "h1" } },
    │  ]
    │
    ├─ 执行顺序：按 actions 数组顺序依次执行
    │
    ├─ input_ports: { "登录按钮_in": "admin" }  ← DAG 连线传入
    │
    ├─ 输出：output_ports = {
    │    "提取标题": "Welcome to Dashboard"       ← extract 类型产出数据
    │  }
    │
    └─ 最终存入 ctx.step_outputs["step_browser1"] = output_ports
```

**关键规则**：
- `navigate` / `click` / `input` / `wait` 等操作类动作 → 不产出数据到 output_ports
- `extract` / `screenshot` / `evaluate` / `get_title` 等查询类动作 → 产出数据到 output_ports
- `input` / `fill` 动作优先从 input_ports 获取值，其次从 config.value

---

## 7. 条件分支执行

### 7.1 条件组（LogicConditionGroup）

```json
{
  "combinator": "and",
  "conditions": [
    { "id": "c1", "left": "{{step_a.extract}}", "op": "contains", "right": "异常" },
    { "id": "c2", "left": "{{step_b.count}}", "op": "gt", "right": "0" }
  ]
}
```

**运算符**：`eq` `neq` `gt` `gte` `lt` `lte` `contains` `not_contains` `is_empty` `is_not_empty` `starts_with` `ends_with` `regex`

### 7.2 执行逻辑

```
condition_group 评估
    │
    ├─ 遍历 conditions，逐个 eval_condition
    │   left/right 先经 resolve_var 解析变量
    │
    ├─ combinator = "and" → 全部为真 → then_steps
    │                       任一为假 → else_steps
    │
    └─ combinator = "or"  → 任一为真 → then_steps
                           全部为假 → else_steps
```

---

## 8. DAG 执行模式

当工作流包含多个步骤且有连线关系时，使用 DAG 模式：

```
┌─────────┐     ┌─────────┐     ┌─────────┐
│ HTTP 请求 │────▶│ 数据处理 │────▶│ 浏览器操作│
└─────────┘     └─────────┘     └─────────┘
                      │
                      ▼
                ┌─────────┐
                │ Excel 写入│
                └─────────┘
```

**拓扑排序** → 确定执行顺序 → 同层节点可并行（`tokio::join!`）

**数据传递**：通过 `FlowEdge` 定义连线，`inject_input_ports()` 在执行前将上游输出注入到下游节点的 `input_ports`。

---

## 9. 前端架构

### 9.1 页面路由

```
/             → Dashboard.vue    (工作流列表)
/editor/:id   → Editor.vue       (工作流编辑器)
/history      → RunHistory.vue   (运行历史)
/settings     → Settings.vue     (设置)
```

### 9.2 Editor 页面结构

```
┌──────────────────────────────────────────────┐
│  顶部工具栏（运行/停止/保存/导入导出/设置）     │
├──────────┬───────────────────────────────────┤
│          │                                    │
│  步骤列表 │        步骤详情区域                  │
│  (左侧)  │                                    │
│          │  ┌─ StepCard ──────────────────┐   │
│  拖拽排序 │  │  步骤标题 + 配置              │   │
│          │  │  ┌─ ActionRow ────────────┐ │   │
│          │  │  │  动作图标 + 标签         │ │   │
│          │  │  │  变量引用名 (step.action)│ │   │
│          │  │  │  ▶ 点击展开参数编辑      │ │   │
│          │  │  │    ├ selector: #btn     │ │   │
│          │  │  │    └ value: {{ref}}     │ │   │
│          │  │  └────────────────────────┘ │   │
│          │  │  [+ 添加动作]               │   │
│          │  └─────────────────────────────┘   │
│          │                                    │
├──────────┴───────────────────────────────────┤
│  StatusBar（运行状态 / 定时任务 / 版本）        │
└──────────────────────────────────────────────┘
```

### 9.3 状态管理

**workflowStore** — 核心数据 store：
- `current`: 当前编辑的工作流
- `workflows`: 工作流列表
- `runStates`: 步骤执行状态映射
- CRUD: addStep, removeStep, addAction, removeAction, renameStep, renameAction
- 序列化：serializeWorkflow / deserializeWorkflow

**editorStore** — 编辑器 UI 状态：
- 选中步骤、展开状态、拖拽状态等

---

## 10. Sidecar 通信（浏览器自动化）

```
┌──────────────┐     stdio JSON      ┌──────────────────┐
│  Rust 后端    │ ◄──────────────────► │  Python Sidecar   │
│  (Tauri)     │     stdin/stdout     │  (Playwright)     │
│              │                      │                    │
│  BrowserSide-│  send_sidecar_action │  playwright_       │
│  car         │ ──────────────────► │  driver.py         │
│              │                      │                    │
│              │  { action: "navigate"│  解析 JSON 命令     │
│              │    url: "https://.." │  执行浏览器操作     │
│              │  }                   │  返回 JSON 结果     │
│              │ ◄────────────────── │                    │
└──────────────┘                      └──────────────────┘
```

**支持的操作**：navigate, click, fill, extract, screenshot, evaluate, scroll, wait, select, hover, pdf

**进程管理**：
- `BrowserSidecar` 启动 Python 子进程
- `Drop` trait 实现 `start_kill()`，确保应用退出时清理子进程
- 支持多浏览器通道（chromium / firefox / webkit）

---

## 11. 数据持久化

### 11.1 SQLite 数据库（`data/db.rs`）

存储内容：
- 工作流定义（JSON）
- 运行历史（每次执行的记录）
- 步骤运行记录（每步的输入/输出/耗时/状态）
- 定时调度配置

### 11.2 文件系统

- 工作流导出/导入：JSON 文件
- 模板：内置模板存储在 `templates/` 目录
- 截图/下载：浏览器操作产出的文件

---

## 12. 关键设计决策

| 决策 | 选择 | 原因 |
|------|------|------|
| 前后端通信 | Tauri IPC | 零网络开销，类型安全 |
| 浏览器自动化 | Playwright sidecar | 跨浏览器支持，Rust 生态无更好选择 |
| 脚本引擎 | Rhai | 嵌入式，沙箱化，Rust 原生 |
| 变量引用格式 | `{{stepId.actionLabel}}` | 前端友好显示 + 后端 id 定位 |
| 步骤 ID 生成 | `uid('step')` → 12 位随机 hex | 短且唯一，前端生成 |
| 容器输出 key | action.label | 用户可读，前端直接显示 |
| 错误策略 | Fail/Ignore/Branch 三级 | 灵活的错误恢复 |
| 平台适配 | trait 抽象 | Windows/Linux 共用核心逻辑 |
