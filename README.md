# Workflow Engine

**可视化工作流自动化引擎** — 独立 HTTP 服务器 + PWA 前端，支持 Windows / Linux (ARM64/x86_64)。

版本：**7.2.0**

## 架构

```
┌─────────────────────────────────────────────────┐
│  浏览器 / PWA (Vue 3 前端)                       │
│  工作流编辑器 · 步骤卡片 · 执行监控 · 模板 · 设置  │
└────────────────────┬────────────────────────────┘
                     │ HTTP REST API (axum)
┌────────────────────▼────────────────────────────┐
│  独立 HTTP 服务器 (workflow-engine)               │
│                                                  │
│  ┌──────────┐ ┌──────────┐ ┌──────────────────┐ │
│  │ 引擎核心  │ │ 节点系统  │ │ 系统集成          │ │
│  │ 解析器    │ │ 82 种节点 │ │ 定时/审批/录制    │ │
│  │ 调度器    │ │ 端口约束  │ │ 键鼠/窗口/OCR    │ │
│  └──────────┘ └──────────┘ └──────────────────┘ │
│                     │                            │
│  ┌──────────────────▼──────────────────────────┐ │
│  │ 数据层：SQLite (WAL) · Rhai 脚本 · 配置     │ │
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

82 种内置节点，支持结构化端口类型约束（`any/string/number/boolean/array/object`）。

| 类型 | 实现 | 说明 |
|---|---|---|
| HTTP | Rust | GET/POST/PUT/DELETE/PATCH |
| 数据处理 | Rust | 赋值/过滤/转换/格式化/JSONPath |
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
| 录制 | 跨平台 | 录制屏幕操作→生成工作流 |
| MCP | Rust | MCP 协议节点 |

## 快速开始

### 直接使用（推荐）

从 [GitHub Actions](https://github.com/haozicnm/workflow-engine-desktop/actions) 下载最新构建产物：

- **Windows**：下载 `workflow-engine` + `dist/`，双击 `start.bat` 启动
- **Linux ARM64**：下载 `.deb` 包，`dpkg -i workflow-engine-standalone_7.2.0_arm64.deb`
- **PWA**：启动后在浏览器打开 `http://localhost:3000`，地址栏安装为 PWA

### 从源码构建

```bash
# 前置条件
# - Rust 1.75+ (rustup)
# - Node.js 18+

# 构建前端
npm ci && npm run build

# 构建后端
cd src-tauri && cargo build --release

# 启动
STATIC_DIR=../dist ./target/release/workflow-engine
```

### CLI 模式

```bash
# 列出所有工作流
wf-cli list --json

# 执行工作流（支持变量注入）
wf-cli run <id> -v url=https://example.com -v name=test

# 查看运行状态
wf-cli status <run-id> --json

# 导入/导出
wf-cli export <id> -o workflow.json
wf-cli import workflow.json

# 定时调度
wf-cli schedule list --json
wf-cli schedule create <wid> "0 9 * * *"
```

## 运行时特性

| 特性 | 说明 |
|---|---|
| `runCondition` | 条件执行：根据逻辑节点分支决定是否运行 |
| `onError` | 错误策略：`fail`(终止) / `ignore`(跳过) / `branch`(跳转) |
| `delay` | 步骤执行前延迟（毫秒） |
| `retry` | 失败重试次数 |
| `breakpoint` | 断点调试标记 |
| `--var` | CLI 变量注入（运行时覆盖） |
| 端口约束 | 结构化输入/输出端口，必填校验 |

## 项目结构

```
workflow-engine-desktop/
├── src/                        # Vue 3 前端
│   ├── pages/                  # Dashboard · Editor · RunHistory · Settings
│   ├── components/             # StepCard · ActionRow · ParamField · StatusBar ...
│   ├── composables/            # useVariableRefs · useStepRunner · useTheme ...
│   ├── stores/                 # Pinia workflowStore
│   ├── types/                  # 类型定义 + node-registry (82 种节点)
│   └── lib/                    # shadcn-vue 工具
│
├── src-tauri/                  # Rust 后端
│   ├── src/
│   │   ├── main.rs             # HTTP 服务器入口
│   │   ├── lib.rs              # App 全局状态
│   │   ├── bin/wf-cli.rs       # CLI 二进制入口
│   │   ├── engine/             # 引擎核心 (解析/调度/执行/上下文)
│   │   ├── nodes/              # 82 种节点实现 + 端口类型系统
│   │   │   ├── registry.rs     # 节点 Schema 注册 + 端口校验
│   │   │   ├── node_registry.rs # 运行时注册表 + 端口查询 API
│   │   │   └── ...             # 各节点实现
│   │   ├── server/             # HTTP API 路由 + 管理器
│   │   ├── platform/           # 跨平台抽象 (Windows/Linux)
│   │   ├── data/               # 数据层 (SQLite/配置/路径)
│   │   └── system/             # 系统集成 (定时/托盘)
│   ├── sidecars/               # Python 浏览器驱动
│   ├── node-schema.json        # 节点元数据 (含端口定义)
│   └── Cargo.toml
│
├── templates/                  # 内置工作流模板
├── docs/                       # 项目文档
├── debian/                     # Linux deb 打包
└── .github/workflows/          # CI: ARM64 deb · Windows · Release
```

## 数据存储

```
{app_data_dir}/
├── config.json                 # 应用设置
├── workflows/                  # 工作流 JSON 定义
├── engine.db                   # SQLite 数据库
├── logs/{date}.log             # 运行日志
├── output/{run_id}/            # 执行输出
├── cursors/                    # 游标迭代持久化
└── sidecar/                    # Python sidecar
```

## API

服务器默认监听 `127.0.0.1:3000`（可通过 `BIND` 环境变量修改）。

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
| v7.0 | 独立 HTTP 服务器 + PWA + 静态文件服务 | ✅ |
| v7.1 | 端口类型系统 + 运行时节点注册表 + shadcn-vue UI 改造 | ✅ |
| v7.2 | 变量传递体验简化 | 🔄 |

## 许可证

MIT
