# Changelog

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
