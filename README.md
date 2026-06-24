# Workflow Engine

**可视化工作流自动化引擎** — 桌面应用 + HTTP 服务器 + CLI，图执行引擎，支持 Windows / Linux。

版本：**9.0.0** | FORMAT_VERSION: **2.0**

## 架构

```
┌─────────────────────────────────────────────────┐
│  桌面应用 (Tauri 2) / PWA                        │
│  工作流编辑器 · Canvas · 步骤卡片 · 模板库 · 设置  │
└────────────────────┬────────────────────────────┘
                     │ HTTP REST API (axum)
┌────────────────────▼────────────────────────────┐
│  HTTP 服务器 (workflow-engine v9.0.2)               │
│                                                  │
│  ┌──────────────┐ ┌──────────┐ ┌──────────────┐ │
│  │ 图执行引擎    │ │ 节点系统  │ │ 系统集成      │ │
│  │ 拓扑分层+并行 │ │ 50+ 节点 │ │ 定时/审批/    │ │
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
| 桌面框架 | Tauri 2 (Rust) |
| 后端 | Rust (tokio 异步) + axum HTTP |
| 前端 | Vue 3 + Vite + Pinia + TailwindCSS + shadcn-vue |
| 数据库 | SQLite (rusqlite, WAL 模式) |
| 脚本引擎 | Rhai (表达式求值 + `{{= expr}}` 语法) |
| Excel | calamine (读) + umya-spreadsheet (原地编辑) + rust_xlsxwriter (新建) |
| Word | docx-rs |
| HTTP | reqwest |
| 浏览器 | Playwright (Python sidecar, 39 种动作) |

## 节点系统

50+ 内置节点，覆盖数据处理、自动化、AI、系统集成等场景。

| 类别 | 节点 | 说明 |
|---|---|---|
| **流程控制** | condition, loop, parallel, sub_workflow | 条件判断/循环/并行/子工作流 |
| **触发器** | trigger_cron, trigger_webhook, trigger_file | 定时/HTTP/文件监控触发 |
| **数据处理** | data_set, data_get, data_filter, data_map, json_transform | 赋值/读取/过滤/映射/转换 |
| **脚本表达式** | script, prompt_template | Rhai 脚本 + 提示词模板 |
| **AI** | llm_chat | LLM 对话节点 |
| **HTTP** | http_request, webhook_response | HTTP 请求 + Webhook 响应 |
| **集成** | github_issue, im_message, email_send | GitHub/IM 消息/邮件 |
| **浏览器** | browser (Python sidecar) | 导航/点击/截图/JS 执行等 39 种动作 |
| **文件** | file_read, file_write, excel, word, csv, json | 文件读写 + Office 文档 |
| **系统** | notify, approval, key_mouse, window, clipboard, ocr, database | 通知/审批/键鼠/窗口/剪贴板/OCR/数据库 |
| **数据工具** | data_length, data_default, data_merge | 长度/默认值/合并 |

所有节点支持 `type_def()` 自描述元数据 + `validate_config()` 配置校验。

## 核心特性

### 🚀 图执行引擎

- **拓扑分层执行**（Kahn 算法），自动识别并行节点
- **同层并行**（`tokio::task::JoinSet`），实测 3× 加速
- **循环依赖检测**，执行前报错
- **双模自适应**：有 `edges` 走图模式，无 `edges` 自动回退线性链

### 🎨 Canvas 编辑器

- 节点拖拽布局 + 连线编辑
- 节点复制粘贴 + 迷你地图
- 边删除 + DebugPanel 实时变量查看
- displayOptions 条件显隐（n8n 风格，12 种运算符）

### 📦 模板库 + 应用市场

- 50 个预制工作流模板
- 5 大分类：数据采集 / 自动化办公 / AI 应用 / 系统运维 / 开发工具
- 一键导入使用

### ⚡ 表达式引擎

- `{{nodeId.port}}` 变量引用
- `{{= expr}}` 显式表达式（算术/比较/逻辑/三元）
- 执行前变量预校验

### 🔌 触发器系统

- `trigger_cron`：Cron 定时触发
- `trigger_webhook`：HTTP 端点触发
- `trigger_file`：文件变更监控触发

## 快速开始

### 直接使用（推荐）

从 [GitHub Releases](https://github.com/haozicnm/workflow-engine-desktop/releases) 下载：

- **Windows**：下载 Inno Setup 安装包（`.exe`），双击安装
- **Linux**：下载 `.deb` 包，`dpkg -i workflow-engine_9.0.0_amd64.deb`

启动后自动打开桌面应用。

### CLI 模式

```bash
# 运行工作流文件
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

### 从源码构建

```bash
# 前置条件：Rust 1.75+ / Node.js 18+

# 构建前端
npm ci && npm run build

# 构建桌面应用
npm run tauri build

# 或仅构建 CLI
cd src-tauri && cargo build --release
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
| 事件流 | mpsc 实时执行状态推送 |

## 项目结构

```
workflow-engine-desktop/
├── src/                        # Vue 3 前端
│   ├── pages/                  # Dashboard · Editor · RunHistory · Settings · Marketplace
│   ├── components/             # StepCard · CanvasNode · ParamField · StatusBar ...
│   ├── composables/            # useVariableRefs · useStepRunner · useTheme ...
│   ├── stores/                 # Pinia workflowStore
│   └── lib/                    # shadcn-vue 工具
│
├── src-tauri/                  # Rust 后端
│   ├── src/
│   │   ├── main.rs             # HTTP 服务器入口
│   │   ├── lib.rs              # App 全局状态 + Tauri 集成
│   │   ├── bin/wf-cli.rs       # CLI 二进制入口
│   │   ├── engine/             # 引擎核心
│   │   │   ├── executor.rs     # 图/线性双模执行器 + 拓扑分层 + 并行
│   │   │   ├── workflow.rs     # 数据模型（Step/Edge/Position）
│   │   │   ├── parser.rs       # JSON/YAML 解析 + 版本兼容
│   │   │   └── context.rs      # 执行上下文 + 表达式求值
│   │   ├── nodes/              # 50+ 节点实现
│   │   │   ├── traits.rs       # NodeExecutor trait + NodeTypeDef/PortDef
│   │   │   └── ...             # 各节点实现
│   │   ├── server/             # HTTP API 路由 + 管理器
│   │   ├── platform/           # 跨平台抽象
│   │   ├── data/               # 数据层 (SQLite/配置/路径)
│   │   └── system/             # 系统集成 (定时/托盘)
│   ├── sidecars/               # Python 浏览器驱动
│   ├── templates/              # 50 个内置工作流模板
│   ├── node-schema.json        # 节点元数据
│   └── Cargo.toml
│
├── docs/                       # 项目文档
├── debian/                     # Linux deb 打包
└── .github/workflows/          # CI: tag push → Release
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
| v8.0-8.4 | 图执行引擎 + 节点元数据 + 变量预校验 + 事件流 + Edge 模型 | ✅ |
| v8.5-8.9 | 表达式引擎 + 触发器 + 实用节点 + Canvas 增强 + displayOptions | ✅ |
| **v9.0** | **Tauri 桌面应用 + 应用市场 + 50 模板库 + GitHub/IM 集成** | ✅ |

## 许可证

MIT
