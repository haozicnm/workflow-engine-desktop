# Workflow Engine Code Wiki

> 自动生成于 2026-06-26 | 基于代码库 v9.0.3

---

## 目录

1. [项目概述](#1-项目概述)
2. [整体架构](#2-整体架构)
3. [技术栈](#3-技术栈)
4. [前端架构](#4-前端架构)
5. [后端架构](#5-后端架构)
6. [核心引擎](#6-核心引擎)
7. [节点系统](#7-节点系统)
8. [数据模型](#8-数据模型)
9. [API 接口](#9-api-接口)
10. [依赖关系](#10-依赖关系)
11. [项目运行方式](#11-项目运行方式)
12. [CI/CD 流水线](#12-cicd-流水线)

---

## 1. 项目概述

**Workflow Engine** 是一个可视化工作流自动化引擎，支持桌面应用、HTTP 服务器和 CLI 三种运行模式。用户可通过拖拽节点构建自动化工作流，替代商业工具（如影刀、n8n）。

### 核心特性

| 特性 | 说明 |
|------|------|
| **图执行引擎** | Kahn 拓扑分层 + 同层并行（tokio::task::JoinSet），循环依赖检测 |
| **双模自适应** | 有 `edges` 走图模式，无 `edges` 自动回退线性链 |
| **50+ 内置节点** | 覆盖数据处理、自动化、AI、系统集成等场景 |
| **Canvas 编辑器** | 节点拖拽布局 + 连线编辑 + 迷你地图 + 边删除 |
| **模板库** | 50 个预制工作流模板，5 大分类 |
| **表达式引擎** | `{{nodeId.port}}` 变量引用 + `{{= expr}}` 显式表达式（Rhai） |
| **触发器系统** | Cron 定时 / HTTP Webhook / 文件监控 |
| **调试支持** | 断点 / 单步 / 暂停 / 变量快照 |

### 版本信息

- **当前版本**: 9.0.3
- **FORMAT_VERSION**: 2.0（兼容 1.0 旧文件）

---

## 2. 整体架构

```
┌─────────────────────────────────────────────────────┐
│  桌面应用 (Tauri 2) / PWA / 浏览器                    │
│  工作流编辑器 · Canvas · 步骤卡片 · 模板库 · 设置       │
└──────────────────────┬──────────────────────────────┘
                       │ HTTP REST API (axum)
┌──────────────────────▼──────────────────────────────┐
│  HTTP 服务器 (workflow-engine v9.0.3)                 │
│                                                      │
│  ┌──────────────┐ ┌──────────┐ ┌──────────────────┐ │
│  │ 图执行引擎    │ │ 节点系统  │ │ 系统集成          │ │
│  │ 拓扑分层+并行 │ │ 50+ 节点 │ │ 定时/审批/        │ │
│  │ 循环检测      │ │ 端口约束  │ │ 键鼠/窗口/OCR     │ │
│  │ 事件流        │ │ type_def │ │ 变量预校验         │ │
│  └──────────────┘ └──────────┘ └──────────────────┘ │
│                       │                              │
│  ┌────────────────────▼────────────────────────────┐ │
│  │ 数据层：SQLite (WAL) · Rhai · Edge 图模型       │ │
│  └────────────────────────────────────────────────┘ │
└──────────────────────┬──────────────────────────────┘
                       │ stdin/stdout JSON
┌──────────────────────▼──────────────────────────────┐
│  Python Sidecar (仅浏览器节点)                        │
│  playwright_driver.py (39 种动作)                    │
└─────────────────────────────────────────────────────┘
```

### 三种运行模式

| 模式 | 入口文件 | 说明 |
|------|----------|------|
| **HTTP 服务器** | `src-tauri/src/main.rs` | 独立 HTTP 服务器，默认监听 `127.0.0.1:19529` |
| **桌面应用** | `src-tauri/src/bin/gui.rs` | Tauri 2 桌面应用，feature = "gui" |
| **CLI** | `src-tauri/src/bin/wf-cli.rs` | 命令行工具，支持 `run-file`、`export`、`import` 等 |

---

## 3. 技术栈

### 前端

| 技术 | 版本 | 用途 |
|------|------|------|
| Vue 3 | ^3.5.0 | UI 框架（Composition API + `<script setup>`） |
| Vite | ^6.0.0 | 构建工具 |
| Pinia | ^2.3.0 | 状态管理 |
| TailwindCSS | ^4.0.0 | 原子化 CSS |
| shadcn-vue (radix-vue) | ^1.9.17 | UI 组件库 |
| vue-i18n | ^9.14.5 | 国际化（中文/英文） |
| TypeScript | ~5.7.0 | 类型安全 |
| Lucide Vue Next | ^1.0.0 | 图标库 |
| @tauri-apps/api | ^2.11.1 | Tauri IPC 通信 |

### 后端

| 技术 | 版本 | 用途 |
|------|------|------|
| Rust | 2021 edition | 主语言 |
| axum | 0.8 | HTTP 框架（REST API + WebSocket + SSE） |
| tokio | 1 | 异步运行时（多线程） |
| rusqlite + r2d2 | 0.32 / 0.8 | SQLite 数据库 + 连接池 |
| rhai | 1 | 脚本引擎（表达式求值） |
| reqwest | 0.12 | HTTP 客户端 |
| serde / serde_json / serde_yaml | 1 / 1 / 0.9 | 序列化/反序列化 |
| calamine + umya-spreadsheet + rust_xlsxwriter | 0.24 / 1.2 / 0.63 | Excel 读写 |
| clap | 4 | CLI 参数解析 |
| cron | 0.13 | Cron 表达式解析 |
| tracing | 0.1 | 结构化日志 |

---

## 4. 前端架构

### 目录结构

```
src/
├── main.ts                    # 应用入口（createApp + Pinia + i18n）
├── App.vue                    # 根组件（Sidebar + Editor + 面板叠加层）
├── style.css                  # 全局样式
├── pages/                     # 页面组件
│   ├── Editor.vue             # 工作流编辑器主页面
│   ├── Dashboard.vue          # 工作流列表（侧边栏）
│   ├── Settings.vue           # 设置面板
│   ├── RunHistory.vue         # 运行历史面板
│   ├── Plugins.vue            # 插件管理面板
│   └── Marketplace.vue        # 应用市场面板
├── components/                # UI 组件
│   ├── ui/                    # shadcn-vue 基础组件（Button/Card/Dialog/...）
│   ├── CanvasEditor.vue       # Canvas 图编辑器
│   ├── CanvasNode.vue         # Canvas 节点
│   ├── CanvasEdge.vue         # Canvas 连线
│   ├── StepCard.vue           # 步骤卡片
│   ├── ActionRow.vue          # 容器动作行
│   ├── ParamField.vue         # 参数字段渲染器
│   ├── DebugPanel.vue         # 调试面板
│   ├── CodeView.vue           # 代码视图
│   ├── Minimap.vue            # 迷你地图
│   └── ...
├── composables/               # Vue 组合式函数
│   ├── useCanvas.ts           # 画布状态（布局/缩放/边拖拽）
│   ├── useEditorActions.ts    # 编辑器操作聚合器
│   ├── useEditorEnhancements.ts # 自动保存/撤销重做/搜索
│   ├── useGlobalStatus.ts     # 全局状态（运行/调度/连接）
│   ├── useNodeSchema.ts       # 节点 Schema 加载
│   ├── useRegistry.ts         # 国际化节点注册表
│   ├── useStepRunner.ts       # 工作流执行控制
│   ├── useVariableRefs.ts     # 变量引用构建
│   ├── useWorkflowValidate.ts # 数据校验
│   ├── useOpsConsole.ts       # 操作日志控制台
│   ├── useToast.ts            # Toast 通知
│   └── useTheme.ts            # 主题管理
├── stores/                    # Pinia 状态管理
│   ├── index.ts               # Store 入口
│   └── workflowStore.ts       # 核心工作流 Store
├── i18n/                      # 国际化
│   ├── index.ts
│   └── locales/
│       ├── zh-CN.ts
│       └── en-US.ts
└── lib/
    └── utils.ts               # 工具函数
```

### 核心数据流

```
App.vue (根组件)
  ├── Dashboard.vue (侧边栏工作流列表)
  │     └── workflowStore.fetchList()
  └── Editor.vue (编辑器主页面)
        ├── useEditorActions() ──── 聚合所有操作
        │     ├── workflowStore ──── Pinia 状态管理
        │     ├── useStepRunner() ── 执行控制
        │     ├── useEditorEnhancements() ── 增强功能
        │     └── useRegistry() ──── 节点注册表
        ├── CanvasEditor.vue ──── 图编辑器
        │     └── useCanvas() ──── 画布状态
        ├── StepCard.vue ──── 步骤卡片
        │     └── ParamField.vue ──── 参数渲染
        └── DebugPanel.vue ──── 调试面板
```

### 关键 Composable 说明

| Composable | 职责 |
|------------|------|
| **useCanvas** | 画布图形编辑器状态管理：节点布局（拓扑排序）、缩放/平移、边拖拽、选择 |
| **useEditorActions** | 编辑器操作聚合器：连接 store、runner、toast、enhancements 等多个 composable |
| **useEditorEnhancements** | 自动保存（dirty 后 3 秒）、撤销/重做（最多 50 快照）、步骤搜索、日志面板 |
| **useGlobalStatus** | 运行中工作流跟踪、定时调度列表（30 秒轮询）、IPC/API/WebBridge 连接状态 |
| **useNodeSchema** | 从后端 `/api/nodes/schema` 拉取节点定义，与前端 UI 覆写合并 |
| **useRegistry** | 国际化感知的节点注册表包装层，所有 UI 字符串通过 vue-i18n 解析 |
| **useStepRunner** | 工作流执行控制：启动/停止，监听后端 `step-update` 和 `run-update` 事件 |
| **useVariableRefs** | 构建可引用变量列表（`{{stepId}}` 和 `{{stepId.actionId}}` 格式） |
| **useWorkflowValidate** | 步骤规范化 + 变量引用合法性校验 |
| **useOpsConsole** | 操作日志控制台（模块级单例，最多 1000 条） |

### workflowStore 关键状态

```typescript
interface WorkflowStoreState {
  workflowList: WorkflowListItem[]  // 工作流列表
  current: Workflow | null          // 当前编辑的工作流
  dirty: boolean                    // 未保存修改标志
  runStates: Record<string, StepRunState>  // 步骤运行状态
  loading: boolean                  // 加载中
  saving: boolean                   // 保存中
  lastWarnings: string[]            // 变量引用警告
}
```

---

## 5. 后端架构

### 目录结构

```
src-tauri/src/
├── main.rs                      # HTTP 服务器入口（axum）
├── lib.rs                       # 库导出 + App 全局状态
├── bin/
│   ├── gui.rs                   # Tauri 桌面应用入口
│   └── wf-cli.rs                # CLI 入口
├── cli.rs                       # CLI 子命令定义（clap）
├── engine/                      # 引擎核心
│   ├── executor.rs              # 图/线性双模执行器
│   ├── workflow.rs              # 数据模型（Step/Edge/Workflow）
│   ├── context.rs               # 执行上下文 + Rhai 表达式求值
│   ├── parser.rs                # JSON/YAML 解析 + 版本兼容
│   ├── scheduler.rs             # 执行准备（RunPreparation）
│   ├── placeholder.rs           # 占位符替换
│   ├── validate.rs              # 变量引用校验
│   ├── approval_store.rs        # 审批存储（内存 channel）
│   ├── plugin_manager.rs        # 插件管理
│   ├── state.rs                 # 执行状态
│   ├── collect.rs               # 数据收集
│   ├── preview.rs               # 预览功能
│   ├── skill_generator.rs       # Skill 生成器
│   ├── action_def.rs            # 动作定义
│   ├── common.rs                # 通用工具
│   ├── yaml_format.rs           # YAML 格式处理
│   └── mod.rs
├── nodes/                       # 50+ 节点实现
│   ├── traits.rs                # NodeExecutor trait
│   ├── registry.rs              # 编译期 Schema 注册（node-schema.json）
│   ├── node_registry.rs         # 运行时插件注册
│   ├── mod.rs                   # 模块声明
│   └── ...                      # 各节点实现文件
├── server/                      # HTTP API 层
│   ├── routes.rs                # 路由定义
│   ├── handlers.rs              # 请求处理器
│   ├── state.rs                 # 全局状态单例（OnceLock）
│   ├── events.rs                # 事件广播
│   ├── sse.rs                   # Server-Sent Events
│   ├── auth.rs                  # 认证
│   └── managers/                # 业务管理器
│       ├── run_manager.rs       # 运行管理
│       ├── workflow_manager.rs  # 工作流管理
│       ├── schedule_manager.rs  # 调度管理
│       ├── approval_manager.rs  # 审批管理
│       ├── preview_manager.rs   # 预览管理
│       ├── compose_manager.rs   # 组合管理
│       ├── system_manager.rs    # 系统管理
│       └── template_manager.rs  # 模板管理
├── data/                        # 数据层
│   ├── db/
│   │   ├── mod.rs               # SQLite 连接池（r2d2）
│   │   ├── schema.rs            # 表结构定义
│   │   └── queries.rs           # SQL 查询
│   ├── config.rs                # 应用配置
│   ├── models.rs                # 数据模型
│   ├── paths.rs                 # 路径解析
│   └── mod.rs
├── platform/                    # 跨平台抽象
│   ├── traits.rs                # 平台 trait
│   ├── windows.rs               # Windows 实现
│   ├── windows_window.rs        # Windows 窗口操作
│   ├── linux.rs                 # Linux 实现
│   ├── linux_window.rs          # Linux 窗口操作
│   └── mod.rs
├── system/                      # 系统集成
│   ├── scheduler.rs             # 系统定时调度
│   ├── tray.rs                  # 系统托盘
│   └── mod.rs
├── commands/                    # Tauri 命令（GUI 模式）
│   ├── run.rs
│   ├── workflow.rs
│   ├── schedule.rs
│   ├── preview.rs
│   ├── pipeline.rs
│   ├── plugin.rs
│   ├── system.rs
│   └── mod.rs
├── ipc.rs                       # IPC 通信
└── ipc_client.rs                # IPC 客户端
```

### App 全局状态

```rust
pub struct App {
    pub db: Arc<Database>,                              // SQLite 数据库
    pub config: Arc<RwLock<AppConfig>>,                 // 应用配置
    pub cancel_flags: RunFlags,                         // 取消标志
    pub cancel_tokens: CancelTokens,                    // 取消令牌（结构化取消）
    pub pause_flags: RunFlags,                          // 暂停标志
    pub breakpoint_flags: RunFlags,                     // 断点标志
    pub step_mode_flags: RunFlags,                      // 单步模式标志
    pub debug_snapshots: Arc<RwLock<HashMap<...>>>,     // 调试变量快照
    pub run_semaphore: Arc<Semaphore>,                  // 并发信号量
    pub approval_store: Arc<ApprovalStore>,             // 审批存储
    pub webhook_response_channels: Arc<RwLock<...>>,    // Webhook 响应通道
}
```

### 请求处理流程

```
HTTP Request
    │
    ▼
routes.rs (路由匹配)
    │
    ▼
handlers.rs (请求解析 + 响应封装)
    │
    ▼
state::get() (获取全局 App 实例)
    │
    ▼
managers/*.rs (业务逻辑)
    │
    ▼
engine/* (核心引擎)
    │
    ▼
nodes/* (节点执行)
```

---

## 6. 核心引擎

### 6.1 执行器 (executor.rs)

执行器是整个引擎的核心，负责工作流的执行调度。

#### 关键结构体

```rust
pub struct StepExecutor {
    executors: HashMap<String, Box<dyn NodeExecutor>>,  // 节点执行器注册表
    pub approval_store: Arc<ApprovalStore>,              // 审批存储
    pub db: Arc<Database>,                               // 数据库
}
```

#### 执行模式

```rust
pub async fn run_workflow(self: &Arc<Self>, workflow: &Workflow) -> Result<ExecutionContext> {
    if !workflow.edges.is_empty() {
        self.run_graph(workflow, &mut ctx).await    // 图模式
    } else {
        self.run_linear(workflow, &mut ctx).await   // 线性模式
    }
}
```

#### 图模式执行流程

```
1. validate_variable_references()  ── 变量引用预校验
2. topological_levels()            ── Kahn 算法拓扑分层
3. for level in levels:
     if level.len() == 1:
       execute(step)               ── 单节点执行
     else:
       run_parallel_level()        ── JoinSet 并行执行
4. 应用 on_error 策略（fail/ignore/branch）
```

#### 拓扑分层算法 (Kahn)

```rust
pub fn topological_levels(&self, steps: &[Step], edges: &[Edge]) -> Result<Vec<Vec<String>>> {
    // 1. 验证 edge 引用的节点都存在
    // 2. 构建入度表和邻接表
    // 3. 入度为 0 的节点作为第一层
    // 4. 逐层移除已处理节点，更新入度
    // 5. 检测循环依赖（visited.len() != steps.len()）
}
```

#### 并行执行

```rust
async fn run_parallel_level(self: &Arc<Self>, node_ids: &[String], ...) -> Result<()> {
    let mut join_set = tokio::task::JoinSet::new();
    for node_id in node_ids {
        // 为每个节点创建独立的 ExecutionContext（克隆变量和步骤输出）
        // 注入 input_ports（按 edge 映射上游输出）
        join_set.spawn(async move {
            exec.execute(&step, &mut task_ctx).await
        });
    }
    // 收集结果，处理变量冲突和错误
}
```

#### 错误处理策略

| 策略 | 行为 |
|------|------|
| `fail` (默认) | 终止工作流执行 |
| `ignore` | 忽略错误，继续执行后续节点 |
| `branch { step_id }` | 跳转到指定错误处理节点 |

### 6.2 执行上下文 (context.rs)

```rust
pub struct ExecutionContext {
    pub run_id: String,                                  // 运行 ID
    pub variables: HashMap<String, Value>,               // 工作流变量
    pub step_outputs: HashMap<String, Value>,            // 步骤输出
    pub input_ports: HashMap<String, Value>,             // 容器输入端口
    pub sessions: HashMap<String, ContainerSession>,     // 容器会话
    pub default_timeouts: TimeoutConfig,                 // 超时配置
    pub shell_allowed_commands: Vec<String>,             // Shell 白名单
    pub sub_workflow_depth: u32,                         // 子流程嵌套深度
    pub step_mode_flag: Option<Arc<AtomicBool>>,         // 单步模式
    pub breakpoint_flag: Option<Arc<AtomicBool>>,        // 断点标志
    pub pause_flag: Option<Arc<AtomicBool>>,             // 暂停标志
}
```

#### 变量解析优先级

1. `{{params.X.Y}}` — 工作流参数
2. `{{vars.X.Y}}` — 用户变量（v7.1 新命名空间）
3. `{{step_X.Y}}` — 步骤输出（带 step_ 前缀）
4. `{{X.Y}}` — 旧语法兼容（先查 step_outputs，再查 variables）
5. `{{= expr}}` — 显式表达式求值（Rhai）

#### Rhai 表达式引擎

```rust
pub fn eval_expr(&self, expr: &str) -> Result<Value, String> {
    // 使用 thread_local 的 Rhai 引擎（沙箱化）
    // 限制：最大操作数 100,000、最大字符串 1MB、最大数组/Map 10,000
    // 禁用 print/debug 输出
}
```

### 6.3 数据模型 (workflow.rs)

```rust
pub struct Workflow {
    pub version: Option<String>,           // 格式版本
    pub name: String,                      // 工作流名称
    pub description: Option<String>,       // 描述
    pub meta: Option<WorkflowMeta>,        // 元数据
    pub steps: Vec<Step>,                  // 步骤列表
    pub variables: Option<HashMap<...>>,   // 工作流变量
    pub edges: Vec<Edge>,                  // 图的边
}

pub struct Step {
    pub id: String,                        // 步骤 ID
    pub name: String,                      // 步骤名称
    pub step_type: String,                 // 节点类型
    pub config: Value,                     // 节点配置
    pub next: Option<String>,              // 下一步（仅线性模式）
    pub retry: Option<RetryConfig>,        // 重试配置
    pub timeout: Option<u64>,              // 超时（毫秒）
    pub body_steps: Option<Vec<Step>>,     // 子步骤（循环节点）
    pub breakpoint: bool,                  // 断点标记
    pub delay: Option<u64>,                // 执行前延迟
    pub on_error: Option<ErrorStrategy>,   // 错误策略
    pub actions: Option<Vec<Value>>,       // 容器动作列表
    pub condition_group: Option<LogicConditionGroup>,  // 条件组
    pub run_condition: Option<RunCondition>,  // 条件执行
}

pub struct Edge {
    pub from: String,       // 源节点 ID
    pub from_port: String,  // 源端口标签
    pub to: String,         // 目标节点 ID
    pub to_port: String,    // 目标端口标签
}
```

---

## 7. 节点系统

### 7.1 架构设计

```
┌─────────────────────────────────────────────────────┐
│                   前端 (Vue 3)                        │
│  useNodeSchema ──→ /api/nodes/schema ──→ 动态渲染     │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│               节点元数据注册层                         │
│  registry.rs (编译期) ──→ node-schema.json            │
│  node_registry.rs (运行时) ──→ 插件节点               │
└──────────────────────┬──────────────────────────────┘
                       │
┌──────────────────────▼──────────────────────────────┐
│               节点执行层                              │
│  traits.rs ──→ NodeExecutor trait                     │
│  executor.rs ──→ StepExecutor (注册表)                │
│  各 *.rs ──→ 具体节点实现                             │
└─────────────────────────────────────────────────────┘
```

### 7.2 NodeExecutor Trait

```rust
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    // 声明式类型元数据（版本/端口/JSON Schema）
    fn type_def(&self) -> NodeTypeDef { /* 默认实现 */ }

    // 执行前配置校验
    fn validate_config(&self, config: &Value) -> Result<(), Vec<ValidationError>> { Ok(()) }

    // 必须实现：执行逻辑
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, executor: &Arc<StepExecutor>) -> Result<Value>;

    // 可选：自行解析模板变量（如 map 节点的 {{__item}}）
    fn resolve_config_self(&self) -> bool { false }
}
```

### 7.3 节点类型元数据

```rust
pub struct NodeTypeDef {
    pub type_name: String,           // 类型名称
    pub version: String,             // 版本
    pub display_name: String,        // 显示名称
    pub description: String,         // 描述
    pub category: String,            // 分类
    pub inputs: Vec<PortDef>,        // 输入端口
    pub outputs: Vec<PortDef>,       // 输出端口
    pub config_schema: Value,        // JSON Schema
    pub params: Vec<ParamDef>,       // 参数定义（支持条件显示）
}

pub struct PortDef {
    pub label: String,               // 端口标签
    pub data_type: String,           // 数据类型（any/text/number/bool/json/array）
    pub required: bool,              // 是否必填
}
```

### 7.4 节点分类

| 分类 | 节点类型 |
|------|----------|
| **流程控制** | condition, loop, while, parallel, sub_workflow, map, cursor |
| **触发器** | trigger_cron, trigger_webhook, trigger_file, webhook_response |
| **数据处理** | data_set, data_get, data_filter, json_transform, json_schema_extract, data_length, data_default, data_merge |
| **脚本表达式** | script, prompt_template |
| **AI** | llm_chat, llm_embedding, llm_agent, text_splitter, vector_store, rag_query |
| **HTTP** | http_request, webhook_response |
| **集成** | github_issue, im_message, email_send |
| **浏览器** | browser (容器，39 种动作) |
| **文件** | file_read, file_write, file_list, file_exists, file_checksum, excel, word, csv, json |
| **系统** | notify, approval, key_mouse, window, clipboard, ocr, shell, delay, print |
| **数据库** | database_query, redis, mongodb, s3 |
| **容器** | browser, excel, word, file |

### 7.5 容器节点

容器节点在一个上下文中执行多个子动作：

```rust
// 容器节点配置结构
pub struct BrowserContainerConfig {
    pub browser: String,           // 浏览器类型（chromium/firefox/webkit）
    pub headless: bool,            // 无头模式
    pub timeout: u64,              // 超时（毫秒）
    pub actions: Vec<ContainerAction>,  // 子动作列表
}
```

### 7.6 条件显示系统 (DisplayOptions)

参考 n8n 的 displayOptions 风格，支持参数级条件可见性：

```rust
pub struct DisplayOptions {
    pub show: Option<HashMap<String, Vec<ConditionValue>>>,  // AND 逻辑
    pub hide: Option<HashMap<String, Vec<ConditionValue>>>,  // OR 逻辑
}

pub enum ConditionOp {
    Eq(Value), Not(Value), Gte(f64), Lte(f64), Gt(f64), Lt(f64),
    Between { from: f64, to: f64 },
    StartsWith(String), EndsWith(String), Includes(String),
    Regex(String), Exists,
}
```

---

## 8. 数据模型

### 8.1 SQLite 数据库

```rust
pub struct Database {
    pool: Pool<SqliteConnectionManager>,  // r2d2 连接池（max 8）
}
```

- **WAL 模式**: 启用并发读取性能
- **busy_timeout**: 5000ms

### 8.2 核心数据表

| 表 | 用途 | 核心字段 |
|---|------|----------|
| **workflows** | 工作流定义 | id, name, description, yaml, enabled, locked, created_at, updated_at |
| **runs** | 运行记录 | id, workflow_id, status, current_step, started_at, finished_at, error |
| **step_runs** | 步骤运行记录 | id, run_id, step_id, status, output(JSON), error |
| **step_logs** | 步骤日志 | id, step_run_id, level, message, timestamp |
| **schedules** | 定时计划 | id, workflow_id, cron_expr, enabled, last_run_at |

### 8.3 配置模型

```rust
pub struct AppConfig {
    pub theme: String,                    // 主题
    pub language: String,                 // 语言
    pub auto_start: bool,                 // 自动启动
    pub log_level: String,                // 日志级别
    pub python_path: Option<String>,      // Python 路径
    pub working_dir: Option<String>,      // 工作目录
    pub timeouts: TimeoutConfig,          // 超时配置
    pub logging: LogConfig,               // 日志配置
    pub execution: ExecutionConfig,       // 执行配置
}

pub struct TimeoutConfig {
    pub http_request_ms: u64,             // HTTP 请求超时（30s）
    pub browser_page_ms: u64,             // 浏览器页面超时（60s）
    pub workflow_total_ms: u64,           // 工作流总超时（10min）
    pub node_exec_ms: u64,                // 节点执行超时（2min）
}

pub struct ExecutionConfig {
    pub max_concurrent_runs: u32,         // 最大并发运行数（3）
    pub default_retries: u32,             // 默认重试次数（0）
    pub retry_delay_ms: u64,              // 重试延迟（1s）
    pub shell_allowed_commands: Vec<String>,  // Shell 命令白名单
}
```

### 8.4 数据存储路径

```
{app_data_dir}/
├── config.json                 # 应用设置
├── workflows/                  # 工作流 JSON/YAML 定义
├── engine.db                   # SQLite 数据库
├── logs/{date}.log             # 运行日志（每日轮转，保留 7 天）
├── output/{run_id}/            # 执行输出
├── cursors/                    # 游标迭代持久化
└── sidecar/                    # Python sidecar
```

---

## 9. API 接口

### 9.1 服务器配置

- **默认地址**: `127.0.0.1:19529`
- **环境变量**: `BIND` (地址)、`STATIC_DIR` (静态文件目录)

### 9.2 核心端点

#### 工作流管理

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/workflows` | 工作流列表 |
| POST | `/api/workflows` | 创建工作流 |
| GET | `/api/workflows/{id}` | 获取工作流详情 |
| PUT | `/api/workflows/{id}` | 更新工作流 |
| DELETE | `/api/workflows/{id}` | 删除工作流 |
| POST | `/api/workflows/{id}/lock` | 锁定/解锁工作流 |
| POST | `/api/workflows/{id}/yaml` | 保存 YAML |
| GET | `/api/workflows/{id}/export-yaml` | 导出 YAML |
| POST | `/api/workflows/validate` | 校验工作流 |
| POST | `/api/workflows/assemble` | 组装工作流 |
| POST | `/api/workflows/auto-order` | 自动排序 |
| POST | `/api/workflows/export` | 导出工作流 |
| POST | `/api/workflows/import` | 导入工作流 |

#### 运行管理

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/runs` | 启动运行 |
| GET | `/api/runs` | 运行列表 |
| POST | `/api/runs/{run_id}/cancel` | 取消运行 |
| POST | `/api/runs/{run_id}/pause` | 暂停运行 |
| POST | `/api/runs/{run_id}/resume` | 恢复运行 |
| GET | `/api/runs/{run_id}/status` | 运行状态 |
| GET | `/api/runs/{run_id}/detail` | 运行详情 |
| GET | `/api/runs/{run_id}/logs` | 运行日志 |
| GET | `/api/runs/{run_id}/step-logs` | 步骤日志 |

#### 节点与模板

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/nodes/types` | 节点类型列表 |
| GET | `/api/nodes/schema` | 节点元数据（含端口定义） |
| GET | `/api/templates` | 模板列表 |
| GET | `/api/templates/categories` | 模板分类 |
| POST | `/api/templates/import` | 导入模板 |
| GET | `/api/templates/{name}` | 获取模板 |
| POST | `/api/templates/{name}/instantiate` | 实例化模板 |

#### 调试与预览

| 方法 | 路径 | 说明 |
|------|------|------|
| POST | `/api/debug/step/{run_id}` | 单步执行 |
| POST | `/api/debug/continue/{run_id}` | 继续执行 |
| POST | `/api/debug/breakpoints` | 设置断点 |
| GET | `/api/debug/vars/{run_id}` | 获取调试变量 |
| POST | `/api/step-test` | 单步测试 |

#### 系统

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/health` | 健康检查 |
| GET | `/api/settings` | 获取设置 |
| PUT | `/api/settings` | 更新设置 |
| GET | `/api/events` | SSE 事件流 |
| GET | `/api/sidecar/health` | Sidecar 健康检查 |

### 9.3 事件流 (SSE)

```typescript
// 前端监听事件
interface ExecutionEvent {
  type: 'step-update' | 'run-update'
  step_id?: string
  status: 'running' | 'completed' | 'failed' | 'ignored'
  output?: any
  error?: string
}
```

---

## 10. 依赖关系

### 10.1 前端依赖图

```
App.vue
  ├── Dashboard.vue
  │     └── workflowStore (fetchList, deleteWorkflow, cloneWorkflow)
  ├── Editor.vue
  │     ├── useEditorActions()
  │     │     ├── workflowStore (saveWorkflow, addStep, removeStep, ...)
  │     │     ├── useStepRunner() (runWorkflow, stopWorkflow)
  │     │     ├── useEditorEnhancements() (autoSave, undo, redo)
  │     │     ├── useGlobalStatus() (registerRun, unregisterRun)
  │     │     ├── useRegistry() (getContainerDef, newAction)
  │     │     └── useOpsConsole() (addOp)
  │     ├── CanvasEditor.vue
  │     │     └── useCanvas() (autoLayout, updateNodePosition, ...)
  │     ├── StepCard.vue
  │     │     ├── ActionRow.vue
  │     │     │     └── ParamField.vue
  │     │     └── LogicBranch.vue
  │     ├── ContainerConfigPanel.vue
  │     │     └── useRegistry()
  │     ├── DebugPanel.vue
  │     └── CodeView.vue
  ├── Settings.vue
  ├── RunHistory.vue
  ├── Plugins.vue
  ├── Marketplace.vue
  └── StatusBar.vue
```

### 10.2 后端依赖图

```
main.rs / gui.rs / wf-cli.rs
  └── App::new()
        ├── Database::open_default()
        ├── AppConfig::load_default()
        └── seed_builtin_workflows()

server::build_router(app)
  └── routes::build()
        └── handlers::*
              └── state::get() → App
                    ├── managers::*
                    │     ├── run_manager → engine::executor
                    │     ├── workflow_manager → data::db
                    │     ├── schedule_manager → engine::scheduler
                    │     └── ...
                    └── engine::*
                          ├── executor::StepExecutor
                          │     ├── nodes::* (NodeExecutor)
                          │     └── engine::context::ExecutionContext
                          ├── parser::parse_workflow()
                          └── scheduler::prepare_run()
```

### 10.3 关键 crate 依赖

| 功能域 | 主要 crate |
|--------|-----------|
| HTTP 服务器 | axum, tower-http |
| 异步运行时 | tokio (rt-multi-thread, macros, time, fs, sync, process) |
| 数据库 | rusqlite, r2d2, r2d2_sqlite |
| 脚本引擎 | rhai |
| HTTP 客户端 | reqwest (rustls-tls) |
| 序列化 | serde, serde_json, serde_yaml |
| Excel | calamine (读), umya-spreadsheet (编辑), rust_xlsxwriter (新建) |
| Word | docx-rs |
| 网页抓取 | scraper |
| 正则 | regex |
| CLI | clap |
| 日志 | tracing, tracing-subscriber, tracing-appender |
| 定时 | cron |
| UUID | uuid (v4) |
| 时间 | chrono |
| 剪贴板 | arboard |

---

## 11. 项目运行方式

### 11.1 环境要求

- **Rust**: 1.75+
- **Node.js**: 18+
- **Python**: 3.8+ (仅浏览器自动化节点需要)

### 11.2 开发模式

```bash
# 安装前端依赖
npm install

# 启动前端开发服务器
npm run dev

# 启动后端 HTTP 服务器
cd src-tauri
cargo run --bin workflow-engine

# 或启动 Tauri 桌面应用
npm run tauri dev
```

### 11.3 生产构建

```bash
# 构建前端
npm run build

# 构建桌面应用
npm run tauri build

# 或仅构建 CLI
cd src-tauri && cargo build --release --bin wf-cli
```

### 11.4 CLI 使用

```bash
# 运行工作流文件
wf-cli run-file workflow.json

# 运行已保存的工作流（支持变量注入）
wf-cli run <id> -v url=https://example.com -v name=test

# 导出/导入
wf-cli export <id> -o workflow.json
wf-cli import workflow.json

# 定时调度
wf-cli schedule list --json
wf-cli schedule create <wid> "0 9 * * *"

# 查看可用节点
wf-cli steps
```

### 11.5 测试

```bash
# 前端测试
npm run test
npm run test:watch
npm run test:coverage

# 后端测试
cd src-tauri && cargo test --lib
cd src-tauri && cargo test  # 含集成测试

# 特定测试
cd src-tauri && cargo test --lib engine::executor
cd src-tauri && cargo test topological_levels
```

### 11.6 环境变量

| 变量 | 说明 | 默认值 |
|------|------|--------|
| `BIND` | 服务器绑定地址 | `127.0.0.1:19529` |
| `STATIC_DIR` | 静态文件目录 | `dist` |
| `RUST_LOG` | 日志级别 | `info` |

---

## 12. CI/CD 流水线

### 12.1 工作流文件

| 文件 | 触发条件 | 功能 |
|------|----------|------|
| `.github/workflows/release.yml` | push tag `v*` | 构建 Windows (exe) + Linux ARM64 (deb) |
| `.github/workflows/build-standalone.yml` | push main | 构建 Windows standalone |
| `.github/workflows/build-arm64.yml` | push main | 构建 Linux ARM64 |
| `.github/workflows/ci.yml` | push/PR | 代码检查 + 测试 |

### 12.2 版本管理

- 版本号位于 `Cargo.toml` 和 `package.json`
- 使用 `scripts/bump-version.sh` 统一更新版本号
- 打 tag 触发自动构建和发布

---

## 附录

### A. 关键文件索引

| 文件 | 用途 |
|------|------|
| `src-tauri/src/engine/executor.rs` | 核心执行器：拓扑排序、并行执行、变量校验 |
| `src-tauri/src/engine/workflow.rs` | 数据模型：Step、Edge、Workflow |
| `src-tauri/src/engine/context.rs` | 执行上下文：变量解析、Rhai 表达式求值 |
| `src-tauri/src/nodes/traits.rs` | 节点 trait：NodeExecutor、NodeTypeDef、PortDef |
| `src-tauri/src/nodes/registry.rs` | 编译期 Schema 注册（node-schema.json） |
| `src-tauri/src/server/routes.rs` | HTTP 路由定义 |
| `src-tauri/src/server/handlers.rs` | 请求处理器 |
| `src-tauri/src/data/db/mod.rs` | SQLite 连接池 |
| `src-tauri/src/data/config.rs` | 应用配置 |
| `src-tauri/src/data/models.rs` | 数据模型 |
| `src-tauri/src/cli.rs` | CLI 子命令 |
| `src-tauri/node-schema.json` | 节点元数据 Schema |
| `src/stores/workflowStore.ts` | 核心 Pinia Store |
| `src/composables/useCanvas.ts` | 画布状态管理 |
| `src/composables/useEditorActions.ts` | 编辑器操作聚合器 |
| `src/composables/useNodeSchema.ts` | 节点 Schema 加载 |
| `src/pages/Editor.vue` | 编辑器主页面 |
| `src/App.vue` | 根组件 |

### B. 添加新节点类型

1. 在 `src-tauri/src/nodes/` 创建新文件
2. 实现 `NodeExecutor` trait
3. 在 `executor.rs` 的 `new()` 中用 `register!` 宏注册
4. 在 `node-schema.json` 中添加 schema 条目
5. 在 `nodes/mod.rs` 中声明模块

### C. 版本历史

| 版本 | 主要变更 |
|------|----------|
| v8.0 | 图执行引擎（拓扑分层+并行）、Edge 模型、变量预校验 |
| v8.5 | 表达式引擎（{{= expr}}）、触发器系统 |
| v8.6 | AI 节点（llm_chat、prompt_template）、实用节点 |
| v9.0 | Tauri 桌面应用、应用市场、50 模板库、GitHub/IM 集成 |
| v9.0.3 | 当前版本 |
