# Workflow Engine Desktop

基于 **Tauri 2 (Rust + WebView)** 的工作流自动化桌面应用。

混合架构：Rust 做核心引擎，Python 仅负责浏览器自动化（Playwright）。

## 架构总览

```
┌─────────────────────────────────────────────────────────────┐
│ Tauri 桌面应用 (进程)                                        │
│                                                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Vue 3 前端 (WebView)                                  │  │
│  │ 工作流画布 · 节点配置 · 执行监控 · 模板 · 设置        │  │
│  └────────────────────┬──────────────────────────────────┘  │
│                       │ Tauri invoke / events                │
│  ┌────────────────────▼──────────────────────────────────┐  │
│  │ Rust 核心 (Tauri Commands)                            │  │
│  │                                                       │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐ ┌────────────┐ │  │
│  │  │ 引擎核心 │ │ 节点系统 │ │ 数据层  │ │ 系统集成   │ │  │
│  │  └─────────┘ └─────────┘ └─────────┘ └────────────┘ │  │
│  └────────────────────┬──────────────────────────────────┘  │
│                       │ stdin/stdout JSON                    │
│  ┌────────────────────▼──────────────────────────────────┐  │
│  │ Python Sidecar (仅浏览器节点)                          │  │
│  │ playwright_driver.py                                  │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## 技术栈

| 层 | 技术 |
| ----- | ----------------------------------- |
| 桌面框架 | Tauri 2.x |
| 核心语言 | Rust (tokio 异步) |
| 前端 | Vue 3 + Vite + Pinia + TailwindCSS |
| 数据库 | SQLite (rusqlite) |
| 脚本引擎 | rhai (内置表达式求值) |
| Excel | calamine (读) + rust_xlsxwriter (写) |
| Word | docx-rs |
| HTTP | reqwest |
| 浏览器 | Playwright (Python sidecar) |

## 节点类型

| 节点 | 实现方式 | 功能 |
| ----- | -------------- | ---------------------- |
| HTTP | 纯 Rust | GET/POST/PUT/DELETE/PATCH |
| 数据处理 | 纯 Rust | 赋值/过滤/转换/格式化/JSONPath |
| 脚本 | 纯 Rust (rhai) | 表达式求值、变量计算 |
| 条件 | 纯 Rust | if/else 分支，11 种操作符 |
| 循环 | 纯 Rust | 遍历数组/重复执行 |
| While | 纯 Rust | 条件循环 |
| 并行 | 纯 Rust | 同时执行多个步骤 |
| 数据映射 | 纯 Rust | 声明式数组转换 |
| 浏览器 | Python sidecar | 导航、点击、截图、JS 执行等 16+ 动作 |
| 网页抓取 | 纯 Rust + Python | 声明式 CSS 选择器提取，分页 |
| Excel | 纯 Rust | 读/写/追加/单元格操作 |
| Word | 纯 Rust | 读取/生成/替换占位符 |
| 通知 | 纯 Rust | 系统通知/邮件/Webhook |
| 审批 | Rust + 前端 | 等待用户通过/拒绝，超时保护 |
| 键鼠操作 | 跨平台 | 模拟点击、输入、快捷键、滚动 |
| 窗口管理 | 跨平台 | 查找/激活/最小化/调整大小 |
| 子工作流 | 纯 Rust | 调用另一个工作流 |
| OCR | 纯 Rust | 图片文字识别 |
| 录制 | 跨平台 | 录制屏幕操作→生成工作流 |

## 项目结构

```
workflow-engine-desktop/
├── src-tauri/                 # Rust 后端
│   ├── src/
│   │   ├── main.rs            # Tauri 入口
│   │   ├── lib.rs             # 库导出 + App 全局状态
│   │   ├── commands/          # Tauri Commands (前端 API 层)
│   │   │   ├── workflow.rs    # 工作流 CRUD
│   │   │   ├── run.rs         # 执行控制
│   │   │   ├── schedule.rs    # 定时调度
│   │   │   ├── system.rs      # 系统操作
│   │   │   ├── template.rs    # 模板管理
│   │   │   └── pipeline.rs    # 流水线
│   │   ├── engine/            # 引擎核心
│   │   │   ├── workflow.rs    # 工作流/步骤数据结构
│   │   │   ├── scheduler.rs   # 步骤调度器（循环执行）
│   │   │   ├── state.rs       # 运行状态机
│   │   │   ├── context.rs     # 执行上下文（变量/表达式）
│   │   │   ├── executor.rs    # 步骤执行器（trait dispatch）
│   │   │   ├── parser.rs      # YAML 解析
│   │   │   ├── collect.rs     # 输出收集
│   │   │   └── recording_converter.rs  # 录制→工作流转换
│   │   ├── nodes/             # 20 种节点类型
│   │   │   ├── traits.rs      # NodeExecutor trait
│   │   │   ├── registry.rs    # 节点清单注册
│   │   │   ├── http.rs        # HTTP 请求
│   │   │   ├── data.rs        # 数据处理
│   │   │   ├── script.rs      # Rhai 脚本
│   │   │   ├── condition.rs   # 条件分支
│   │   │   ├── loop_node.rs   # 循环
│   │   │   ├── while_node.rs  # While 循环
│   │   │   ├── parallel.rs    # 并行执行
│   │   │   ├── map.rs         # 数据映射
│   │   │   ├── browser.rs     # 浏览器自动化
│   │   │   ├── web_scrape.rs  # 网页抓取
│   │   │   ├── excel.rs       # Excel
│   │   │   ├── word.rs        # Word
│   │   │   ├── notify.rs      # 通知
│   │   │   ├── approval.rs    # 审批
│   │   │   ├── mouse_keyboard.rs  # 键鼠操作
│   │   │   ├── window.rs      # 窗口管理
│   │   │   ├── sub_workflow.rs    # 子工作流
│   │   │   ├── ocr.rs         # OCR 识别
│   │   │   └── recording.rs   # 操作录制
│   │   ├── platform/          # 跨平台抽象层
│   │   │   ├── traits.rs      # InputBackend / WindowBackend / RecordingBackend
│   │   │   ├── windows.rs     # Windows 输入
│   │   │   ├── linux.rs       # Linux 输入 (enigo)
│   │   │   ├── windows_window.rs  # Windows 窗口管理 (PowerShell)
│   │   │   ├── linux_window.rs    # Linux 窗口管理 (xdotool/wmctrl)
│   │   │   └── recording.rs   # 录制后端
│   │   ├── data/              # 数据层
│   │   │   ├── db.rs          # SQLite (WAL 模式 + 连接池)
│   │   │   ├── models.rs      # 数据模型
│   │   │   ├── paths.rs       # 路径解析
│   │   │   └── config.rs      # 配置管理
│   │   └── system/            # 系统集成
│   │       ├── scheduler.rs   # 定时调度引擎
│   │       └── tray.rs        # 系统托盘
│   ├── sidecars/
│   │   ├── playwright_driver.py  # Python 浏览器驱动
│   │   └── desktop_recorder.py   # Windows 桌面录制
│   ├── tauri.conf.json
│   └── Cargo.toml
│
├── src/                       # Vue 3 前端
│   ├── package.json
│   ├── vite.config.ts
│   └── src/
│       ├── main.ts
│       ├── App.vue
│       ├── router/index.ts
│       ├── stores/            # Pinia 状态管理
│       │   ├── workflow.ts    # 统一 store
│       │   ├── workflowStore.ts
│       │   ├── editorStore.ts
│       │   └── executionStore.ts
│       ├── composables/       # 组合式函数
│       │   ├── useToast.ts    # 通知提示
│       │   ├── useUndo.ts     # 撤销重做
│       │   └── useAutoSave.ts # 自动保存
│       ├── components/        # 通用组件
│       │   ├── StepPalette.vue     # 步骤面板
│       │   ├── StepCanvas.vue      # 步骤画布
│       │   ├── YamlPanel.vue       # YAML 编辑器
│       │   ├── VariablePanel.vue   # 变量面板
│       │   ├── RecordingBar.vue    # 录制控制栏
│       │   ├── WorkflowGrid.vue    # 工作流网格
│       │   ├── TemplateSection.vue # 模板区域
│       │   ├── ScheduleSection.vue # 定时计划
│       │   ├── StatusBar.vue       # 状态栏
│       │   ├── Toast.vue           # Toast 通知
│       │   └── step-config/        # 19 种步骤配置面板
│       ├── pages/             # 页面
│       │   ├── Dashboard.vue  # 首页
│       │   ├── Editor.vue     # 工作流编辑器
│       │   ├── RunHistory.vue # 运行历史
│       │   ├── Settings.vue   # 设置
│       │   └── TestPage.vue   # 测试页
│       ├── types/workflow.ts  # TypeScript 类型定义
│       └── config/node-fields.ts  # 节点字段配置
│
├── templates/                 # 内置 YAML 模板
├── docs/                      # 文档
│   ├── cross-platform-plan-comparison.md
│   ├── cross-platform-implementation-plan.md
│   └── software-health-check.md
├── SPEC.md                    # 产品规格
├── task_plan.md               # 开发计划
├── progress.md                # 进度日志
└── README.md
```

## 数据存储

```
{app_data_dir}/
├── config.json                # 应用设置
├── workflows/{id}.yaml        # 工作流定义
├── templates/                 # 预置模板
├── data/engine.db             # SQLite 数据库
├── logs/{date}.log            # 运行日志
├── output/{run_id}/           # 执行输出
├── sidecar/                   # Python sidecar
└── cache/                     # 缓存
```

## 通信协议

### Vue ↔ Rust (Tauri invoke)

前端通过 `invoke()` 调用 Rust 命令，通过 `listen()` 监听实时事件。

核心命令：
```
workflow_list / create / update / delete / validate
run_start / pause / resume / cancel / status / logs
approval_decide / list_pending
settings_get / update
system_check_browser / install_browser
```

核心事件 (Rust → Vue 实时推送)：
```
step-update        — 步骤状态变化 (running/completed/failed)
run-update         — 工作流运行状态变化 (started/completed/failed/cancelled)
approval-required  — 需要用户审批
breakpoint-hit     — 断点命中（调试模式）
```

### Rust ↔ Python (stdin/stdout JSON)

```json
// Rust → Python
{"id":"uuid","action":"navigate","params":{"url":"https://example.com"}}

// Python → Rust
{"id":"uuid","success":true,"data":{"title":"Example"}}
```

## 开发路线

| 阶段 | 内容 | 状态 |
| ------ | --------------------------------------------- | ----- |
| **P0** | 基础框架：Tauri 窗口 + 数据层 + 空白前端 + 4 个页面 | ✅ |
| **P1** | 引擎核心：解析器 + 调度器 + 状态机 + 基础节点（HTTP/数据/脚本/条件/循环） | ✅ |
| **P2** | 文件节点：Excel + Word 读写 | ✅ |
| P2.5 | 浏览器 + 通知 + 审批 + 并行 + 数据映射 | ✅ |
| **P3** | 前端画布：YAML 双向编辑 + 19 种步骤配置 + 数据流可视化 | ✅ |
| **P4** | 桌面集成：系统托盘 + 定时调度 + 系统通知 + 审批弹窗 | ✅ |
| **v1.1** | 网页抓取增强 + 键鼠操作 + 窗口管理 + 子工作流 + OCR | ✅ |
| **v1.2** | 录制→工作流转换引擎 + 跨平台窗口管理 | ✅ |
| **v1.3** | 错误恢复策略 + 变量实时监视 + Undo/Redo + AutoSave | 🔧 进行中 |
| **v1.4** | 模板市场 + AI 流程生成 + 元素选择器 | 📋 计划中 |
| **v1.5** | 企微/钉钉/飞书通知 + Webhook | 📋 计划中 |

## 快速开始

```bash
# 前置条件
# - Rust 1.75+ (rustup) + MSVC Build Tools
# - Node.js 18+
# - Python 3.11+ (仅浏览器节点需要)

# 安装前端依赖
cd src && npm install && cd ..

# 开发模式
cd src-tauri && cargo tauri dev

# 构建
cd src-tauri && cargo tauri build
```

## 许可证

MIT
