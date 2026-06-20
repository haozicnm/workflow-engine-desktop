# Workflow Engine

**可视化工作流自动化引擎** — HTTP 服务器 + PWA 前端 + CLI，图执行引擎，支持 Windows / Linux (ARM64/x86_64)。

版本：**8.4.1** | FORMAT_VERSION: **2.0**

## 架构

```
┌─────────────────────────────────────────────────┐
│  浏览器 / PWA (Vue 3 前端)                       │
│  工作流编辑器 · 步骤卡片 · 执行监控 · 模板 · 设置  │
└────────────────────┬────────────────────────────┘
                     │ HTTP REST API (axum)
┌────────────────────▼────────────────────────────┐
│  HTTP 服务器 (workflow-engine v8.4.1)              │
│                                                  │
│  ┌──────────────┐ ┌──────────┐ ┌──────────────┐ │
│  │ 图执行引擎    │ │ 节点系统  │ │ 系统集成      │ │
│  │ 拓扑分层+并行 │ │ 34+ 节点 │ │ 定时/审批/    │ │
│  │ 循环检测      │ │ 端口约束  │ │ 键鼠/窗口/OCR │ │
│  │ 事件流        │ │ type_def  │ │ 变量预校验    │ │
│  └──────────────┘ └──────────┘ └──────────────┘ │
│                     │                            │
│  ┌──────────────────▼──────────────────────────┐ │
│  │ 数据层：SQLite (WAL) · Rhai · Edge 图模型   │ │
│  └─────────────────────────────────────────────┘ │
└────────────────────┬────────────────────────────┘
                     │ stdin/stdout JSON
┌────────────────────▼────────────────────────────┐
│  Python Sidecar (仅浏览器节点)                    │
│  playwright_driver.py (39 种动作)                │
└─────────────────────────────────────────────────┘
```

## 技术栈

| 层 | 技术 |
|---|---|
| 后端 | Rust (tokio 异步) + axum HTTP |
| 前端 | Vue 3 + Vite + Pinia + TailwindCSS + shadcn-vue |
| 数据库 | SQLite (rusqlite, WAL 模式) |
| 脚本引擎 | Rhai (内置表达式求值) |
| Excel | calamine (读) + umya-spreadsheet (原地编辑) + rust_xlsxwriter (新建) |
| Word | docx-rs |
| HTTP | reqwest |
| 浏览器 | Playwright (Python sidecar, 39 种动作) |

## 节点系统

34+ 内置节点，18 个核心节点已完成 `type_def()` 自描述元数据。

| 类型 | 实现 | 说明 |
|---|---|---|
| HTTP | Rust | GET/POST/PUT/DELETE/PATCH |
| 数据处理 | Rust | 赋值/读取/长度/默认值/合并 |
| 脚本 | Rust (Rhai) | 表达式求值、变量计算 |
| 条件判断 | Rust | conditionGroup AND/OR，11 种操作符 |
| 循环 | Rust | 遍历数组/重复执行/While/游标迭代 |
| 并行 | Rust | 同时执行多个步骤 |
| 数据映射 | Rust | 声明式数组转换 |
| 浏览器 | Python sidecar | 导航、点击、截图、JS 执行等 39 种动作 |
| 网页抓取 | Rust + Python | CSS 选择器提取，分页 |
| Excel | Rust | 读/写/追加/更新/筛选/排序 |
| Word | Rust | 读取/生成/替换占位符 |
| 通知 | Rust | 系统通知/邮件/Webhook |
| 审批 | Rust + 前端 | tokio channel 暂停/恢复，超时保护 |
| 键鼠操作 | 跨平台 | 模拟点击、输入、快捷键、滚动 |
| 窗口管理 | 跨平台 | 查找/激活/最小化/调整大小 |
| 子工作流 | Rust | 调用另一个工作流 |
| OCR | Rust | 图片文字识别 |
| MCP | Rust | MCP 协议节点 |

## v8.0 新特性

### 🚀 图执行引擎

- **拓扑分层执行**（Kahn 算法），自动识别并行节点
- **同层并行**（`tokio::task::JoinSet`），实测 3× 加速
- **循环依赖检测**，执行前报错
- **双模自适应**：有 `edges` 走图模式，无 `edges` 自动回退线性链
- **CLI 标记**：图模式显示 `[图引擎·并行]`

### 📋 节点类型元数据

- `NodeTypeDef` / `PortDef` / `ValidationError` 结构体
- `type_def()` / `validate_config()` 默认方法（34 节点零改动兼容）
- 18 个核心节点完成 `type_def` 实现

### 🔍 变量引用预校验

- 执行前检查 `{{nodeId.port}}` 引用合法性
- 支持 `{{params.xxx}}` / `{{__item}}` 内置变量

### 📡 执行事件流

- `ExecutionEvent` 枚举（WorkflowStarted / NodeStarted / NodeCompleted / NodeFailed / WorkflowCompleted）
- `run_workflow_with_events()` 可选 mpsc channel 参数

### 📐 数据模型扩展

- `Edge` 边类型（from/from_port/to/to_port）
- `Position` 位置类型
- `Workflow.edges` 字段
- `FORMAT_VERSION`: 1.0 → 2.0

## 快速开始

### 直接使用（推荐）

从 [GitHub Releases](https://github.com/haozicnm/workflow-engine-desktop/releases) 下载：

- **Windows**：下载 Inno Setup 安装包（`.exe`），双击安装
- **Linux ARM64**：下载 `.deb` 包，`dpkg -i workflow-engine-standalone_8.0.0_arm64.deb`
- **PWA**：启动后在浏览器打开 `http://localhost:19529`，地址栏安装为 PWA

### 从源码构建

```bash
# 前置条件：Rust 1.75+ / Node.js 18+

npm ci && npm run build
cd src-tauri && cargo build --release
STATIC_DIR=../dist ./target/release/workflow-engine
```

### CLI 模式

```bash
# 运行工作流文件（自动识别图/线性模式）
wf-cli run-file workflow.json

# 运行工作流（支持变量注入）
wf-cli run <id> -v url=https://example.com -v name=test

# 导出/导入
wf-cli export <id> -o workflow.json
wf-cli import workflow.json

# 定时调度
wf-cli schedule list --json
wf-cli schedule create <wid> "0 9 * * *"
```

### 图模式工作流示例

```json
{
  "name": "钻石结构",
  "steps": [
    {"id": "start", "type": "data_set", "config": {"key": "msg", "value": "hello"}},
    {"id": "branch_a", "type": "data_set", "config": {"key": "a", "value": "A"}},
    {"id": "branch_b", "type": "data_set", "config": {"key": "b", "value": "B"}},
    {"id": "merge", "type": "data_set", "config": {"key": "result", "value": "done"}}
  ],
  "edges": [
    {"from": "start", "from_port": "out", "to": "branch_a", "to_port": "in"},
    {"from": "start", "from_port": "out", "to": "branch_b", "to_port": "in"},
    {"from": "branch_a", "from_port": "out", "to": "merge", "to_port": "in"},
    {"from": "branch_b", "from_port": "out", "to": "merge", "to_port": "in"}
  ]
}
```

## 运行时特性

| 特性 | 说明 |
|---|---|
| `runCondition` | 条件执行：根据逻辑节点分支决定是否运行 |
| `onError` | 错误策略：`fail` / `ignore` / `branch` |
| `delay` | 步骤执行前延迟（毫秒） |
| `retry` | 失败重试次数 |
| `breakpoint` | 断点调试标记 |
| `--var` | CLI 变量注入（运行时覆盖） |
| 端口约束 | 结构化输入/输出端口，必填校验 |
| 变量预校验 | 执行前检查模板引用合法性 |
| 事件流 | 可选的 mpsc 实时执行状态推送 |

## 项目结构

```
workflow-engine-desktop/
├── src/                        # Vue 3 前端
│   ├── pages/                  # Dashboard · Editor · RunHistory · Settings
│   ├── components/             # StepCard · ActionRow · ParamField · StatusBar ...
│   ├── composables/            # useVariableRefs · useStepRunner · useTheme ...
│   ├── stores/                 # Pinia workflowStore
│   └── lib/                    # shadcn-vue 工具
│
├── src-tauri/                  # Rust 后端
│   ├── src/
│   │   ├── main.rs             # HTTP 服务器入口
│   │   ├── lib.rs              # App 全局状态
│   │   ├── bin/wf-cli.rs       # CLI 二进制入口
│   │   ├── engine/             # 引擎核心
│   │   │   ├── executor.rs     # 图/线性双模执行器 + 拓扑分层 + 并行
│   │   │   ├── workflow.rs     # 数据模型（Step/Edge/Position）
│   │   │   ├── parser.rs       # JSON/YAML 解析 + 版本兼容
│   │   │   └── context.rs      # 执行上下文
│   │   ├── nodes/              # 节点实现
│   │   │   ├── traits.rs       # NodeExecutor trait + NodeTypeDef/PortDef
│   │   │   └── ...             # 34+ 节点实现
│   │   ├── server/             # HTTP API 路由 + 管理器
│   │   ├── platform/           # 跨平台抽象
│   │   ├── data/               # 数据层 (SQLite/配置/路径)
│   │   └── system/             # 系统集成 (定时/托盘)
│   ├── sidecars/               # Python 浏览器驱动
│   ├── node-schema.json        # 节点元数据
│   └── Cargo.toml
│
├── templates/                  # 内置工作流模板
├── docs/                       # 项目文档
├── debian/                     # Linux deb 打包
└── .github/workflows/          # CI: Release · Build
```

## 数据存储

```
{app_data_dir}/
├── config.json                 # 应用设置
├── workflows/                  # 工作流 JSON/YAML 定义
├── engine.db                   # SQLite 数据库
├── logs/{date}.log             # 运行日志
├── output/{run_id}/            # 执行输出
├── cursors/                    # 游标迭代持久化
└── sidecar/                    # Python sidecar
```

## API

服务器默认监听 `127.0.0.1:19529`（可通过 `BIND` / `PORT` 环境变量修改）。

核心端点：
- `GET /api/workflows` — 工作流列表
- `POST /api/workflows` — 创建工作流
- `POST /api/runs` — 执行工作流
- `GET /api/nodes/schema` — 节点元数据（含端口定义）
- `GET /api/health` — 健康检查

## 开发路线

| 阶段 | 内容 | 状态 |
|---|---|---|
| P0-P3 | 基础框架→引擎→文件节点→浏览器/审批/并行 | ✅ |
| v5-v6 | 步骤编辑器+条件执行+游标迭代+审批重构 | ✅ |
| v7.0-7.9 | 独立 HTTP 服务器 + PWA + 端口系统 + 节点注册表 + UI 改造 | ✅ |
| **v8.0** | **图执行引擎 + 节点元数据 + 变量预校验 + 事件流 + Edge 模型** | ✅ |

## 许可证

MIT
