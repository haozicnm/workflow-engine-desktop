# Changelog

## v9.0.4 (2026-07-02)

### 🏗 大框架统一 (5 项)

- **统一 DAG 执行引擎**: CLI `run-file` 改用 `scheduler::run_workflow()`，消除两套并行 DAG 引擎
- **统一 workflow_save_yaml 格式**: Tauri 命令也存 JSON，与 HTTP API 保持一致
- **定时调度走 prepare_run**: 共享并发限制/取消/暂停控制，不再绕过安全机制
- **DebugPanel 传入真正的 run_id**: 断点调试使用 `run_start` 返回的 run_id 而非 workflow_id
- **variable-update SSE 事件监听**: DebugPanel 实时接收变量更新，替代轮询方案

### ✨ 细节打磨 (7 项)

- **节点位置持久化**: Step 增加 `position` 字段，Canvas 拖拽布局保存后不丢失
- **description 触发 dirty**: WorkflowHeader 描述修改正确标记未保存状态
- **ContainerConfigPanel 支持 paramDefs**: 与 StepCard 保持一致，支持新格式参数定义
- **Edge 同步路径清理**: 移除 CanvasEditor 中冗余的 watch 写回逻辑
- **导出字段补全**: 导出时保留 `run_condition`/`body_steps`/`on_error` 等字段
- **级联删除完善**: `delete_workflow`/`delete_runs` 补删 `step_logs`/`approvals` 表
- **useCanvas 加载已保存位置**: 有位置数据的节点跳过自动布局

**Stats:** 14 files changed, +172 / -160 lines since v9.0.3

## v9.0.3 (2026-06-25)

### 🔧 Bug Fixes — 前后端对齐 + 逻辑链完整性修复 (10 项)

- **ErrorStrategy::Branch 反序列化**: 同时接受前端 `{branch:"id"}` 和后端 `{branch:{step_id:"id"}}` 格式
- **Step.on_error serde alias**: 添加 `alias="onError"`，保存-加载后不再丢失错误策略
- **RetryConfig.max serde alias**: 添加 `alias="max_retries"`，前端重试配置可正确传递到后端
- **data_set/data_get config 字段名**: 前端 `name` → `key`，与后端 DataSetNode 对齐
- **debug 断点路径**: 前端路由 `/breakpoint/set` → `/breakpoints` 匹配后端
- **Tauri 命令注册**: 补注册 `open_log_dir`/`clear_logs`/`get_log_path`
- **浏览器模式 SSE 事件**: scheduler 所有 emit 函数同时广播 SSE，run_manager 补发完成/失败事件
- **Edge/Step 字段名 roundtrip**: deserializeWorkflow 归一化 `on_error→onError`, `condition_group→conditionGroup`, `from_port→fromPort` 等
- **App.vue 文件损坏恢复**: 重构过程中文件字符被替换，恢复原始内容

**Stats:** 7 files changed, +124 / -24 lines since v9.0.2

## v9.0.2 (2026-06-24)

### 🔒 Security Fixes (P0 — 11 项)

- **Shell 白名单绕过防护**: 白名单模式禁止 `sh`/`bash`/`cmd`/`powershell`，超时后显式 kill 子进程
- **HTTP SSRF 防护**: 内网地址过滤 + 重定向限制
- **Rhai 沙箱逃逸**: 禁止 `import`/`eval`/文件系统访问
- **Secret 泄露**: 日志/错误消息自动脱敏
- **并发竞态**: 共享状态加锁保护

### 🔧 Bug Fixes (P1+P2 — 21 项)

- **Canvas**: 边删除按钮失效修复 (`pointer-events-none` → `pointer-events-auto`)
- **Canvas**: 搜索高亮 + 节点定位修复
- **状态管理**: save 非原子操作改为原子事务
- **VariablesPanel**: 集成到 Settings 页面
- **displayOptions**: 字段可见性条件计算修复
- **并行执行**: from_port 过滤 + 数据流统一
- **逻辑链**: 6 个审查报告问题修复

### ✨ Features (P2)

- **Canvas 数据流动画**: 运行时边高亮 + 数据流向可视化
- **动态多端口渲染**: 节点端口按连接数动态扩展
- **新增 9 个节点**: AI 扩展 + 数据存储类节点
- **trigger_cron 调度器**: 定时触发器集成
- **trigger_file 文件监控**: 文件变更触发工作流

### 🔧 Maintenance

- Clippy 4 个 warning 修复
- 新增节点单元测试
- UI/UX 审查 + 最终综合审查（5 安全 + 6 功能 + 11 中度 + 12 建议）

**Stats:** 51 files changed, +2272 / -121 lines since v9.0.1

## v9.0.0 (2026-06-21)

### 🚀 Major Features

**应用市场 + 模板库 (v9.0)**
- 新增应用市场页面，浏览和使用预制工作流模板
- 模板库扩展至 50 个模板（数据采集/自动化办公/AI应用/系统运维/开发工具）
- 模板库导航，按分类浏览

**GitHub 集成 (v8.8)**
- 新增 `github_issue` 节点：创建/查询/更新 Issue 和 PR
- 支持 GitHub API Token 认证

**IM 消息集成 (v8.8)**
- 新增 `im_message` 节点：通用多平台消息发送
- 支持 Slack、飞书、钉钉、企业微信、Telegram
- 统一 Webhook/API 发送模式

**表达式引擎 + 触发器系统 (v8.5)**
- Rhai 表达式引擎：支持算术/比较/逻辑/三元/变量访问
- `{{= expr}}` 显式表达式语法
- 三种触发器节点：`trigger_cron`（定时）、`trigger_webhook`（HTTP）、`trigger_file`（文件监控）
- `webhook_response` 节点：自定义 Webhook 响应

**Canvas 增强 (v8.7)**
- 节点复制粘贴 + 迷你地图
- 边删除 + DebugPanel 变量刷新

**displayOptions 条件显隐 (n8n-style)**
- 12 种条件运算符（Eq/Not/Gte/Lte/Gt/Lt/Between/StartsWith/EndsWith/Includes/Regex/Exists）
- show(AND)/hide(OR) 逻辑控制字段可见性
- Rust ↔ TypeScript 类型完全对齐

### 🔧 实用节点 (v8.6)

- `llm_chat`：LLM 对话节点（AI 薄封装）
- `prompt_template`：提示词模板
- `data_filter`：数据过滤
- `json_transform`：JSON 转换
- `email_send`：邮件发送
- `database_query`：数据库查询
- `clipboard_read`/`clipboard_write`：剪贴板操作
- `data_length`/`data_default`/`data_merge`：数据工具

### ⚡ 执行增强 (v8.5)

- G1: 表达式求值在执行前解析 config 中的 `{{}}` 占位符
- G2: 步骤输出自动注入上下文（`step_N.field` 访问）
- G3: 未解析变量保留原文（方便调试）
- G5: `run_condition` 条件执行

### 🐛 Bug Fixes

- 全面代码审查，修复 20 个逻辑漏洞
- 修复 `scheduler_tests` Workflow 结构体缺少新字段
- 修复 `context_tests` 未解析变量行为断言
- 修复 `integration_test` 中 "logic" → "condition" 节点类型名
- Clippy 自动修复（pattern char comparison 等）

### 📝 测试

- 79 lib tests + 35 context tests + 29 integration tests + 30 scheduler tests + 25 template tests + 8 library tests = **206 tests passing**
- `template_exec_tests` 中 2 个测试因模板文件缺失跳过（已知问题）

## v8.4.1 (2026-06-20)

### 🐛 Bug Fixes

- 修复 `ensure_defaults_fills_version` 测试断言（FORMAT_VERSION 已从 1.0 升级到 2.0，测试未同步更新）
- 修复 clippy `needless_borrows_for_generic_args` 警告 (system_manager.rs:407)
- 修复 clippy `too_many_arguments` 警告 (scheduler.rs:39, 添加 allow 属性)

### 📝 文档

- 修正 README / AGENTS.md 版本号为 8.4.1
- 更新 ROADMAP-v8.0 反映 8.2/8.3/8.4 已完成阶段
- 统一 ARCHITECTURE.md 节点数为 34（56 type_def）
- 归档旧文件至 docs/archive/，删除重复 LIBRARY-DESIGN.md

## v8.4.0 (2026-06-16)

### 🎨 全页面 DESIGN.md 对齐（第二轮）

- **Settings 重构**: 移除无效 Light/System 主题选择（纯暗色）、去 box-shadow、Browser Node 信息重组、Auto-start 统一保存行为
- **节点组件色彩对齐**: CanvasNode 状态色 Tailwind → Design tokens（info/success/danger）、port dots hairline-strong
- **Shadow 清理**: AddStepDialog、ParamField 变量下拉移除 shadow-xl/lg，全面无阴影
- **Surface 层级修复**: StepCard output 区、Plugins 插件卡片 bg-background→bg-muted
- **新增 Token**: `--color-info` `#57c1ff`、`--color-hairline-strong` `#34343a`
- **页面统一**: RunHistory（标题/hover/bg）、Editor（drag handle）、Plugins（卡片层级）

## v8.3.0 (2026-06-16)

### 🎨 DESIGN.md 设计系统全面落地

- **暗色主题重设计**: 碳灰画布 `#090a0b`（比 Linear 更深）+ 翠绿品牌色 `#10b981`（单一强调色策略）
- **4 级 Surface Ladder**: `#111214` → `#16171a` → `#1a1c1f`，无阴影纯扁平设计
- **字体切换**: Geist → Inter Variable（含 ss03 风格化数字），Google Fonts CDN 加载
- **Hairline 边框**: `#23252a` 极细分割线，替代 GitHub 风格粗边框
- **圆角规范**: 按钮 8px（`0.5rem`）/ 卡片 12px（`0.75rem`），统一克制
- **节点状态色**: running（sky-400）、success（emerald-600）、error（rose-600），选中/端口 hover → 翠绿
- **容器颜色同步**: 15+ 节点类型色彩全部对齐 DESIGN.md 色板
- **全局清理**: 消除所有硬编码蓝/绿/红/紫/灰，统一 CSS 变量驱动

### 📦 Windows 安装包

- **Inno Setup 安装程序**: 双击安装到 Program Files，开始菜单快捷方式，标准卸载
- **CI 集成**: Release workflow 自动构建 `WorkflowEngine-vX.X.X-windows-x64-setup.exe`
- **双语安装向导**: 中文 + English

### 📝 设计资产

- 新增 `DESIGN.md`: 完整设计令牌文档（颜色/字体/圆角/间距/组件规范），供 AI agent 参考
- 参考源: Cursor（单橙策略）、Linear（最黑底哲学）、Raycast（桌面原生感）

## v8.2.0 (2026-06-16)

### 🎨 Canvas 图编辑器

- **可视化画布**: 新增 Canvas 编辑模式，SVG 渲染节点卡片 + 贝塞尔曲线连线
- **拖拽连线**: 从节点输出端口拖拽到输入端口创建 Edge，直觉操作
- **节点拖拽**: 卡片可自由拖动定位，实时更新位置
- **执行高亮**: 运行时当前节点高亮（pending/灰色、running/蓝色、success/绿色、error/红色）
- **三视图切换**: visual（列表）/ canvas（画布）/ code（YAML）Tab 自由切换
- **缩放平移**: 滚轮缩放、空白区域拖拽平移画布
- **状态管理**: useCanvas composable（10 个方法：节点位置/连线管理/缩放/拖拽）

### 📦 节点元数据全覆盖

- **22 个节点完成 type_def**: approval, browser_container, clipboard, cursor, excel, excel_container, file_container, map, mcp_node, mouse_keyboard, ocr, parallel, print, regex, registry, sub_workflow, web_scrape, webbridge, while_node, window, word, word_container
- **总计 56 个 type_def** 实现（含多子类型节点），100% 节点自描述

### 🔧 修复

- 修复 StepCard.vue header div 未闭合导致 vite build 失败
- 修复 Editor.vue 重复 import
- 修复 workflowStore.ts 未使用的 Edge import

## v8.0.0 (2026-06-15)

### 🚀 图执行引擎

- **拓扑分层执行**: 基于 Kahn 算法的拓扑排序，自动识别并行节点
- **同层并行**: 使用 `tokio::task::JoinSet`，独立节点自动并行执行（实测 3× 加速）
- **循环检测**: 执行前检测循环依赖，即时报错而非死锁
- **双模自适应**: 有 `edges` 字段走图模式，无 `edges` 自动回退线性链（完全向后兼容）
- **CLI 集成**: `wf-cli run-file` 自动识别图/线性模式，图模式显示 `[图引擎·并行]`

### 📋 节点类型元数据

- **NodeTypeDef**: 每个节点现在有 `type_name`、`version`、`display_name`、`description`、`category` 元数据
- **PortDef**: 输入/输出端口定义（label + data_type + required）
- **validate_config()**: 执行前配置校验钩子（默认通过，节点可按需覆盖）
- **18 个核心节点** 已完成 type_def 实现: http, script, condition, data_set/get/length/default/merge, file_read/write/list/delete/exists/append, shell, delay, loop

### 🔍 变量引用预校验

- 执行前检查所有 `{{nodeId.port}}` 模板引用是否指向存在的节点
- 支持 `{{params.xxx}}` 和 `{{__item}}` 内置变量豁免
- 错误信息明确指向具体步骤和引用

### 📡 执行事件流

- `ExecutionEvent` 枚举: WorkflowStarted / NodeStarted / NodeCompleted / NodeFailed / WorkflowCompleted
- `run_workflow_with_events()`: 可选 `mpsc::Sender` 参数，实时推送执行状态
- 用于 WebSocket/SSE 前端实时更新

### 📐 数据模型扩展

- **Edge**: 新增图边类型（from/from_port/to/to_port），支持显式端口连接
- **Position**: Canvas 节点位置（x/y）
- **Workflow.edges**: 工作流级边集合
- **FORMAT_VERSION**: `1.0` → `2.0`

### ⚠️ 破坏性变更

- **无** — 34 个节点零改动编译通过，线性链完全兼容


## v7.7.0 (2026-06-03)

### ✨ New Features

- **积木自描述标准化 (Step 1)**
  - node-schema.json: 36 节点全部添加 `tags`，12 参数添加 `validation`（min/max/pattern），6 参数添加 `visible_when`（条件可见性），12 参数添加 `examples`
  - 新增结构体：`ParamValidation`、`VisibleWhen`、`ParamExample`
  - 新增 API：`GET /api/blocks?q=xxx` 按标签搜索，`GET /api/blocks/categories` 分类列表
  - `blocks_list` 返回 `tags`，`blocks_get` 返回完整自描述（含 validation/visible_when/examples）

- **YAML 标准化格式 (Step 2)**
  - `workflow.rs`: 新增 `FORMAT_VERSION="1.0"`、`WorkflowMeta`（author/tags/created_at）、`Workflow.version`
  - `yaml_format.rs`: 标准 YAML 导出器（干净可读格式，带注释头），版本兼容性检查，`ensure_defaults()`
  - `parser.rs`: 解析时自动检查版本兼容性（高版本拒绝，低版本兼容）
  - 新增 API：`GET /api/workflows/{id}/export-yaml` 导出标准 YAML

- **模板库增强 + 锁定强制 (Step 3)**
  - 锁定强制：`workflow_update` 和 `workflow_save_yaml` 新增 locked 检查（409 Conflict），三个写操作全部受保护
  - `TemplateMeta` 新增 `category` 字段（data/automation/file/general）
  - 新增 API：`GET /api/templates?q=xxx&category=data` 搜索+分类过滤，`GET /api/templates/categories`，`POST /api/templates/import`，`POST /api/workflows/{id}/save-as-template`
  - 5 个种子模板：http_fetch、web_scrape_excel、shell_script、file_processor、data_pipeline

- **Agent 集成 (Step 4)**
  - `workflow-engine-agent` Skill：发现积木→参考模板→生成 YAML→用户确认→保存执行
  - `skill_generator.rs` v2：利用 schema 生成富 Skill 文档（输出端口、必需参数、验证约束、变量引用速查表）

- **浏览器节点 action_definitions**
  - browser 节点新增 `action_definitions`（18 个操作：navigate/snapshot/click/fill/evaluate/screenshot/save_as_pdf/mouse_click/key_type/send_keys/find_tab/list_tabs/close_tab/close_session/network/upload/download/cdp）
  - 每个 action 有：params（含 required/type/validation）+ output fields
  - 新增结构体：`ActionDefinition`、`ActionParamDef`、`ActionOutputDef`
  - `blocks_get` API 返回 `action_definitions`

### 📦 Version Bumps

- tauri.conf.json: 7.6.0 → 7.7.0
- Cargo.toml: 7.6.0 → 7.7.0
- node-schema.json: 8.2 → 8.3
- package.json: 7.5.0 → 7.7.0
- node-io-spec.md: v1.0 → v1.1

## v7.6.0 (2026-06-01)

### ⚠️ Breaking Changes

- **节点精简：78 → 36（-54%）** — 删除 42 个冗余节点，功能由 shell/script 替代
- 删除的节点类型：file_delete/append/mkdir/copy/move/glob、data_length/default/merge、json_parse、text_template、6 个类型转换、7 个数组操作、regex_replace、notify、3 个幽灵节点、10 个 MCP 重复节点
- 现有工作流如使用已删除节点，需改用 shell 节点等价命令

### ✨ New Features

- **积木自描述系统** — 36 个节点全部有 params schema，UI 自动渲染配置表单，Agent 读 schema 即可搭工作流
- **积木发现 API** — `GET /api/blocks` 列出所有积木，`GET /api/blocks/{type}` 查看详情
- **工作流组装 API** — `POST /api/workflows/assemble` Agent 提交 YAML 验证+保存
- **模板库** — 5 个预制模板（http-to-file、excel-pipeline、web-scrape、file-batch、shell-pipeline）
- **模板 API** — `GET /api/templates` / `POST /api/templates/{name}/instantiate`
- **组合助手** — `POST /api/compose/chain` 自动串联积木
- **前端自动渲染** — 配置表单根据 params schema field_type 自动选择组件

### 🔧 Improvements

- Shell timeout_secs 现在真正执行超时（`tokio::time::timeout`，上限 3600s）
- regex_extract + regex_match 合并为统一 regex 节点（mode 参数）
- WebBridge 成为唯一浏览器控制路径（移除 Playwright sidecar 和 Kimi Browser）
- Release 构建隐藏 Windows 控制台窗口

### 🐛 Bug Fixes

- Shell timeout_secs 参数未生效（子进程永不被 kill）
- 配置文件非原子写入（改为 write_to_temp → fsync → rename）
- 前端 vue-tsc 编译错误（ContainerType 缺少 logic）
- 端口绑定重试 3 次（处理僵尸 socket 延迟释放）
- 跨平台 Python 检测（Windows: python→python3→py）

### 🧹 Cleanup

- 移除 42 个冗余节点（MCP 重复、文件操作、数组操作、类型转换等）
- 清理后端/前端死代码引用（~300 行）
- 前端控制台移除 Playwright 残留
- 移除 browser_channel、browser_executable_path 配置项

### 📦 Dependencies

- 无变更

---

## v7.5.1 (2026-05-31)

- 修复版本号硬编码（7.3.0 → 7.5.0）
- CI clippy 修复（explicit_auto_deref）

## v7.5.0 (2026-05-30)

- WebBridge Chrome 扩展集成
- 浏览器自动化容器节点
- 工作流执行引擎优化
