# Workflow Engine Desktop

基于 **Tauri 2 (Rust + WebView)** 的工作流自动化桌面应用。

混合架构：Rust 做核心引擎，Python 仅负责浏览器自动化（Playwright）。

版本：**6.6.0**

## 架构总览

```
┌─────────────────────────────────────────────────────────────┐
│ Tauri 桌面应用 (进程)                                        │
│                                                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Vue 3 前端 (WebView)                                  │  │
│  │ 工作流编辑器 · 步骤卡片 · 执行监控 · 模板 · 设置      │  │
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
| --- | --- |
| 桌面框架 | Tauri 2.x |
| 核心语言 | Rust (tokio 异步) |
| 前端 | Vue 3 + Vite + Pinia + TailwindCSS + shadcn-vue |
| 数据库 | SQLite (rusqlite, WAL 模式) |
| 脚本引擎 | Rhai (内置表达式求值) |
| Excel | calamine (读) + umya-spreadsheet (原地编辑) + rust_xlsxwriter (新建) |
| Word | docx-rs |
| HTTP | reqwest |
| 浏览器 | Playwright (Python sidecar, 39 种动作) |

## 节点类型

| 类型 | 实现 | 说明 |
| --- | --- | --- |
| HTTP | Rust | GET/POST/PUT/DELETE/PATCH |
| 数据处理 | Rust | 赋值/过滤/转换/格式化/JSONPath |
| 脚本 | Rust (Rhai) | 表达式求值、变量计算 |
| 条件判断 | Rust | conditionGroup AND/OR，11 种操作符 |
| 循环 | Rust | 遍历数组/重复执行 |
| While | Rust | 条件循环 |
| 游标迭代 | Rust | 跨次持久化迭代，每次处理一项 |
| 并行 | Rust | 同时执行多个步骤 |
| 数据映射 | Rust | 声明式数组转换 |
| 浏览器 | Python sidecar | 导航、点击、截图、JS 执行等 39 种动作 |
| 网页抓取 | Rust + Python | CSS 选择器提取，分页 |
| Excel | Rust | 读/写/追加/更新/筛选/排序（原地编辑保留格式） |
| Word | Rust | 读取/生成/替换占位符 |
| 通知 | Rust | 系统通知/邮件/Webhook |
| 审批 | Rust + 前端 | tokio channel 暂停/恢复，超时保护，SQLite 持久化 |
| 键鼠操作 | 跨平台 | 模拟点击、输入、快捷键、滚动 |
| 窗口管理 | 跨平台 | 查找/激活/最小化/调整大小 |
| 子工作流 | Rust | 调用另一个工作流 |
| OCR | Rust | 图片文字识别 |
| 录制 | 跨平台 | 录制屏幕操作→生成工作流 |

## 项目结构

```
workflow-engine-desktop/
├── src/                       # Vue 3 前端
│   ├── index.html             # Vite 入口
│   ├── package.json           # npm 依赖与脚本
│   ├── vite.config.ts         # Vite 构建配置
│   ├── tsconfig.json          # TypeScript 配置
│   ├── style.css              # Tailwind + 布局常量
│   ├── main.ts                # 应用入口
│   ├── App.vue                # 根组件 + 视图路由
│   ├── pages/
│   │   ├── Dashboard.vue      # 首页（工作流列表 + 模板 + 定时）
│   │   ├── Editor.vue         # 工作流编辑器
│   │   ├── RunHistory.vue     # 运行历史
│   │   └── Settings.vue       # 设置
│   ├── components/
│   │   ├── StepCard.vue       # 步骤卡片（Header + 配置 + 动作）
│   │   ├── ActionRow.vue      # 动作行（展开/收起 + 参数编辑）
│   │   ├── ParamField.vue     # 统一参数字段（含变量引用）
│   │   ├── LogicBranch.vue    # 条件构建器（conditionGroup）
│   │   ├── ApprovalCenter.vue # 全局审批面板
│   │   ├── ContainerConfigPanel.vue  # 容器参数面板
│   │   ├── SchedulePanel.vue  # 定时面板
│   │   ├── CodeView.vue       # JSON 代码视图
│   │   ├── StatusBar.vue      # 状态栏
│   │   ├── Toast.vue          # Toast 通知
│   │   ├── ErrorBoundary.vue  # 错误边界
│   │   ├── ActionIcon.vue     # Lucide 图标组件（~60 图标）
│   │   └── ui/                # shadcn-vue 组件库
│   ├── composables/
│   │   ├── useVariableRefs.ts       # 变量引用逻辑
│   │   ├── useStepRunner.ts         # 步骤执行控制
│   │   ├── useEditorEnhancements.ts # 编辑器增强
│   │   ├── useGlobalStatus.ts       # 全局状态
│   │   ├── useTheme.ts             # 主题切换
│   │   └── useToast.ts             # Toast 管理
│   ├── stores/
│   │   ├── index.ts           # Pinia 入口
│   │   └── workflowStore.ts   # 工作流 CRUD + normalizeSteps + deepMerge
│   ├── types/
│   │   ├── types.ts           # 核心类型 + uid/nextStepId/nextActionId
│   │   ├── node-registry.ts   # 节点注册表（48 种节点定义）
│   │   └── workflow.ts        # barrel re-export
│   ├── lib/utils.ts           # cn() 工具函数
│   └── utils/
│       ├── tauri.ts            # Tauri invoke 封装
│       └── cron.ts             # Cron 解析
│
├── src-tauri/                 # Rust 后端
│   ├── src/
│   │   ├── main.rs            # Tauri 入口（GUI/CLI 双模）
│   │   ├── lib.rs             # 库导出 + App 全局状态
│   │   ├── cli.rs             # CLI 模式（wf-cli）
│   │   ├── bin/wf-cli.rs       # CLI 二进制入口
│   │   ├── commands/          # Tauri Commands（前端 API 层）
│   │   │   ├── workflow.rs    # 工作流 CRUD
│   │   │   ├── run.rs         # 执行控制
│   │   │   ├── schedule.rs    # 定时调度
│   │   │   ├── system.rs      # 系统操作
│   │   │   ├── template.rs    # 模板管理
│   │   │   ├── browser_recording.rs  # 浏览器录制
│   │   │   ├── preview.rs     # 预览
│   │   │   └── pipeline.rs    # 流水线
│   │   ├── engine/            # 引擎核心
│   │   │   ├── workflow.rs    # 工作流/步骤数据结构
│   │   │   ├── scheduler.rs   # 顺序调度器
│   │   │   ├── state.rs       # 运行状态机
│   │   │   ├── context.rs     # 执行上下文（变量/表达式）
│   │   │   ├── executor.rs    # 步骤执行器（60+ trait dispatch）
│   │   │   ├── parser.rs      # JSON 解析与转换
│   │   │   ├── collect.rs     # 输出收集
│   │   │   ├── approval_store.rs  # 审批存储（channel 暂停/恢复）
│   │   │   └── recording_converter.rs  # 录制→工作流转换
│   │   ├── nodes/             # 30+ 种节点类型
│   │   │   ├── traits.rs      # NodeExecutor trait
│   │   │   ├── mod.rs         # 模块导出
│   │   │   ├── http.rs        # HTTP 请求
│   │   │   ├── data.rs        # 数据处理
│   │   │   ├── script.rs      # Rhai 脚本
│   │   │   ├── condition.rs   # 条件分支
│   │   │   ├── loop_node.rs   # 循环
│   │   │   ├── while_node.rs  # While 循环
│   │   │   ├── cursor.rs      # 游标迭代
│   │   │   ├── parallel.rs    # 并行执行
│   │   │   ├── map.rs         # 数据映射
│   │   │   ├── browser.rs     # 浏览器自动化
│   │   │   ├── browser_container.rs  # 浏览器容器
│   │   │   ├── web_scrape.rs  # 网页抓取
│   │   │   ├── excel.rs       # Excel 通用节点
│   │   │   ├── excel_container.rs    # Excel 容器
│   │   │   ├── word.rs        # Word
│   │   │   ├── word_container.rs     # Word 容器
│   │   │   ├── notify.rs      # 通知
│   │   │   ├── approval.rs    # 审批
│   │   │   ├── mouse_keyboard.rs  # 键鼠操作
│   │   │   ├── window.rs      # 窗口管理
│   │   │   ├── sub_workflow.rs    # 子工作流
│   │   │   ├── ocr.rs         # OCR 识别
│   │   │   ├── recording.rs   # 操作录制
│   │   │   ├── clipboard.rs   # 剪贴板
│   │   │   ├── clipboard_container.rs
│   │   │   ├── file.rs / file_save.rs  # 文件操作
│   │   │   ├── json_parse.rs  # JSON 解析
│   │   │   ├── regex.rs       # 正则
│   │   │   ├── text_template.rs  # 文本模板
│   │   │   ├── convert.rs     # 格式转换
│   │   │   ├── delay.rs       # 延迟
│   │   │   ├── array.rs       # 数组操作
│   │   │   └── print.rs       # 打印
│   │   ├── platform/          # 跨平台抽象层
│   │   │   ├── traits.rs      # InputBackend / WindowBackend
│   │   │   ├── windows.rs     # Windows 输入
│   │   │   ├── linux.rs       # Linux 输入 (enigo)
│   │   │   ├── windows_window.rs  # Windows 窗口管理
│   │   │   ├── linux_window.rs    # Linux 窗口管理
│   │   │   └── recording.rs   # 录制后端
│   │   ├── data/              # 数据层
│   │   │   ├── db.rs          # SQLite 数据库
│   │   │   ├── models.rs      # 数据模型
│   │   │   ├── paths.rs       # 路径解析
│   │   │   └── config.rs      # 配置管理
│   │   └── system/            # 系统集成
│   │       ├── scheduler.rs   # 定时调度引擎
│   │       └── tray.rs        # 系统托盘
│   ├── sidecars/
│   │   ├── playwright_driver.py  # Python 浏览器驱动（30+ handler）
│   │   └── desktop_recorder.py   # Windows 桌面录制
│   ├── tests/                 # Rust 测试
│   ├── capabilities/          # Tauri 权限
│   ├── icons/                 # 应用图标
│   ├── tauri.conf.json        # Tauri 配置
│   └── Cargo.toml             # Rust 依赖
│
├── templates/                 # 内置模板
│   ├── order-to-contracts.json
│   ├── monitor-to-report.json
│   └── data/                  # 模板数据文件
│
├── examples/                  # 示例脚本
│   ├── run_pipeline.py
│   ├── run_full_pipeline.py
│   └── create_test_data.py
│
├── scripts/
│   └── bump-version.sh        # 版本号同步脚本
│
├── docs/                      # 项目文档
│   ├── ARCHITECTURE.md        # 架构文档
│   ├── USER_GUIDE.md          # 用户指南
│   ├── HELP.md                # 帮助文档
│   ├── CODEBASE_ANALYSIS.md   # 代码库分析
│   ├── CODE_ANALYSIS.md       # 代码分析
│   ├── operation-logic-v2.md  # 执行逻辑
│   ├── 参数内嵌方案讨论.md
│   ├── 执行逻辑修复方案.md
│   └── archive/               # 历史文档归档
│
├── .github/workflows/         # GitHub Actions CI
│   ├── build-windows.yml      # Windows 构建
│   └── release.yml            # 三平台发布
│
├── .gitignore
├── eslint.config.js           # ESLint 9 flat config
└── README.md
```

## 数据存储

```
{app_data_dir}/
├── config.json                # 应用设置
├── workflows/                 # 工作流 JSON 定义
├── engine.db                  # SQLite 数据库
├── logs/{date}.log            # 运行日志
├── output/{run_id}/           # 执行输出
├── cursors/                   # 游标迭代持久化
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
approval_response / approval_list_pending
settings_get / update
system_check_browser / install_browser
```

核心事件 (Rust → Vue 实时推送)：
```
step-update        — 步骤状态变化 (running/completed/failed)
run-update         — 工作流运行状态变化 (started/completed/failed/cancelled)
```

### Rust ↔ Python (stdin/stdout JSON)

```json
// Rust → Python
{"id":"uuid","action":"navigate","params":{"url":"https://example.com"}}

// Python → Rust
{"id":"uuid","success":true,"data":{"title":"Example"},"events":[...]}
```

## CLI 模式

同二进制支持 GUI 和 CLI 双模（通过 `--cli` 切换）：

```powershell
# 列出所有工作流
workflow-engine.exe --cli list --json

# 执行工作流（支持变量注入）
workflow-engine.exe --cli run <id> -v url=https://example.com

# 查看运行状态
workflow-engine.exe --cli status <run-id> --json

# 导入/导出
workflow-engine.exe --cli export <id> -o workflow.json
workflow-engine.exe --cli import workflow.json

# 定时调度
workflow-engine.exe --cli schedule list --json
workflow-engine.exe --cli schedule create <wid> "0 9 * * *"
```

详细用法：[docs/CLI.md](./docs/CLI.md)

## 开发路线

| 阶段 | 内容 | 状态 |
| --- | --- | --- |
| P0 | 基础框架：Tauri 窗口 + 数据层 + 4 页面 | ✅ |
| P1 | 引擎核心：解析器 + 调度器 + 基础节点 | ✅ |
| P2 | 文件节点：Excel + Word 读写 | ✅ |
| P2.5 | 浏览器 + 通知 + 审批 + 并行 + 数据映射 | ✅ |
| P3 | 前端编辑器：步骤卡片 + 参数编辑 + 变量引用 | ✅ |
| P4 | 桌面集成：系统托盘 + 定时调度 + CLI 模式 | ✅ |
| v5.0 | 步骤编辑器重写 + 条件执行 + 迭代节点 | ✅ |
| v6.0 | 游标迭代 + 审批重构（channel）+ 容器统一 | ✅ |
| v6.5 | CLI 双模 + 布局常量 + Lucide 图标迁移 | ✅ |
| v6.6 | 仓库整理 + 文档更新 + GitHub 迁移 | ✅ |

## 快速开始

```bash
# 前置条件
# - Rust 1.75+ (rustup)
# - Node.js 18+
# - Python 3.11+ (仅浏览器节点需要)

# 安装前端依赖
cd src && npm install

# 开发模式（从项目根目录执行）
npx --prefix src tauri dev

# 构建（Windows 需在 PowerShell 中执行）
npx --prefix src tauri build
```

## 许可证

MIT
