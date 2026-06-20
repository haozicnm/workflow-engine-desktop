# AGENTS.md — Workflow Engine v8.4.1

## Purpose

Workflow Engine 是一个可视化工作流自动化引擎，由 Rust 后端 + Vue 3 前端组成。
v8.0 新增**图执行引擎**（拓扑分层+并行执行），同时保持线性链完全向后兼容。
目标：让用户通过拖拽节点构建自动化工作流，替代商业工具（影刀、n8n）。

## Project Overview

- **后端**：Rust (tokio async) + axum HTTP 服务器
- **前端**：Vue 3 + Vite + Pinia + TailwindCSS + shadcn-vue
- **数据库**：SQLite (rusqlite, WAL 模式)
- **脚本引擎**：Rhai (内置表达式求值)
- **浏览器自动化**：Playwright Python sidecar (39 种动作)
- **当前版本**：8.4.1 | FORMAT_VERSION: 2.0

## Build & Test Commands

### 后端 (src-tauri/)

```bash
# 检查编译
cd src-tauri && cargo check --workspace

# 运行测试
cd src-tauri && cargo test --lib

# 运行全部测试（含集成测试）
cargo test

# 构建 CLI
cargo build --bin wf-cli

# 构建 release
cargo build --release
```

### 前端

```bash
npm install
npm run dev      # 开发服务器
npm run build    # 生产构建
```

## Architecture

### 执行模型

v8.0 支持**双模执行**：

1. **图模式**（`Workflow.edges` 非空）：
   - `topological_levels()` — Kahn 算法拓扑分层
   - 同层节点 `JoinSet` 并行执行
   - 执行前 `validate_variable_references()` 检查 `{{node.port}}` 引用合法性
   - `validate_variable_references` → `topological_levels` → `run_parallel_level`

2. **线性模式**（`edges` 为空，向后兼容）：
   - 按 `steps` 数组顺序逐个执行
   - 保留原有容器节点/占位符/condition_group 逻辑

### 数据模型

```rust
// 步骤（线性链的基本单元）
pub struct Step {
    pub id: String,
    pub step_type: String,  // "http" | "script" | "condition" | ...
    pub config: Value,      // 节点配置（JSON）
    pub next: Option<String>,
    pub retry: Option<RetryConfig>,
    pub timeout: Option<u64>,
    pub breakpoint: bool,
    pub on_error: Option<ErrorStrategy>,
    pub condition_group: Option<LogicConditionGroup>,
    pub run_condition: Option<RunCondition>,
    // ...
}

// 边（v8 新增：图的显式连接）
pub struct Edge {
    pub from: String,       // 源节点 ID
    pub from_port: String,  // 源端口标签
    pub to: String,         // 目标节点 ID
    pub to_port: String,    // 目标端口标签
}

// 工作流
pub struct Workflow {
    pub steps: Vec<Step>,
    pub edges: Vec<Edge>,   // v8 新增
    pub variables: Option<HashMap<String, Value>>,
    // ...
}
```

## Key Files

| 文件 | 用途 |
|------|------|
| `src-tauri/src/engine/executor.rs` | **核心执行器**：`run_workflow()` 入口，`topological_levels()` 拓扑排序，`run_parallel_level()` 并行执行，`validate_variable_references()` 变量校验 |
| `src-tauri/src/engine/workflow.rs` | 数据模型：`Step`, `Edge`, `Position`, `Workflow` |
| `src-tauri/src/nodes/traits.rs` | **节点 trait**：`NodeExecutor` (execute/type_def/validate_config)，`NodeTypeDef`, `PortDef` |
| `src-tauri/src/nodes/` | 34+ 节点实现（http/script/condition/data/loop/shell/delay/file/...） |
| `src-tauri/src/cli.rs` | CLI 实现：`wf-cli run-file` 自动识别图/线性模式 |
| `src-tauri/node-schema.json` | 节点元数据 schema |
| `src-tauri/Cargo.toml` | Rust 依赖 + 版本号 |
| `src/` | Vue 3 前端 |

## Common Tasks

### 添加新节点类型

1. 在 `src-tauri/src/nodes/` 创建新文件（参考 `http.rs` 或 `delay.rs`）
2. 实现 `NodeExecutor` trait：
   ```rust
   #[async_trait]
   impl NodeExecutor for MyNode {
       fn type_def(&self) -> NodeTypeDef { /* 元数据 */ }
       async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, executor: &Arc<StepExecutor>) -> Result<Value> {
           // 实现
       }
   }
   ```
3. 在 `executor.rs` 的 `new()` 中用 `register!` 宏注册
4. 在 `node-schema.json` 中添加 schema 条目

### 运行图工作流

```bash
# 创建带 edges 的 workflow JSON
cat > my_graph.json << 'EOF'
{
  "name": "并行示例",
  "steps": [
    {"id": "a", "type": "data_set", "config": {"key": "x", "value": "1"}},
    {"id": "b", "type": "data_set", "config": {"key": "y", "value": "2"}},
    {"id": "c", "type": "data_set", "config": {"key": "z", "value": "3"}}
  ],
  "edges": [
    {"from": "a", "from_port": "out", "to": "b", "to_port": "in"},
    {"from": "a", "from_port": "out", "to": "c", "to_port": "in"}
  ]
}
EOF

# 运行（自动走图引擎）
wf-cli run-file my_graph.json
# 输出：完成 (0.0s) [图引擎·并行]
```

### 调试技巧

```bash
# 查看执行计划（不执行）
cd src-tauri && cargo test --lib engine::executor::tests::get_execution_plan -- --nocapture

# 运行特定测试
cargo test --lib engine::executor

# 检查拓扑分层
cargo test topological_levels
```

## v8.0 新增 API

### StepExecutor

```rust
// 执行工作流（自动选择图/线性模式）
 executor.run_workflow(&workflow).await?;

// 执行工作流 + 事件流
executor.run_workflow_with_events(&workflow, Some(event_tx)).await?;

// 获取执行计划（不执行）
let plan = executor.get_execution_plan(&workflow)?;
// plan: [["a"], ["b", "c"], ["d"]]

// 拓扑分层
executor.topological_levels(&steps, &edges)?;

// 变量引用预校验
executor.validate_variable_references(&workflow)?;
```

### NodeExecutor trait

```rust
#[async_trait]
pub trait NodeExecutor: Send + Sync {
    // v8 新增：类型元数据
    fn type_def(&self) -> NodeTypeDef { /* 默认实现 */ }

    // v8 新增：配置预校验
    fn validate_config(&self, config: &Value) -> Result<(), Vec<ValidationError>> { Ok(()) }

    // 必须实现：执行
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, executor: &Arc<StepExecutor>) -> Result<Value>;

    // 可选：自行解析模板变量
    fn resolve_config_self(&self) -> bool { false }
}
```

## 向后兼容约定

- **34 个现有节点**：无需任何改动即可编译通过
- **线性链工作流**：`edges` 为空时自动回退线性执行
- **FORMAT_VERSION**：2.0 格式兼容 1.0 旧文件
- **破坏性变更**：无

## CI

- **Release**：`push tag v*` 触发，构建 Windows (exe) + Linux ARM64 (deb)
- **Build**：`push main` 触发，构建 Windows standalone
- **CI 工作流**：`.github/workflows/release.yml`, `build-standalone.yml`, `build-arm64.yml`
