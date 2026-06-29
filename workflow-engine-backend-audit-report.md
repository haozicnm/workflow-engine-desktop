# Workflow Engine Rust 后端全面功能检查报告

> 检查时间：2025-06-25  
> 工作目录：`C:\Users\haozi\Dev\workflow-engine-desktop\src-tauri\src`  
> 项目版本：9.0.4

---

## 1. 节点实现检查（nodes/ 目录）

| 检查项 | 状态 | 说明 | 问题 |
|--------|------|------|------|
| 节点文件列表 | 通过 | 共 52 个 .rs 文件，覆盖 48+ 节点类型 | 无 |
| NodeExecutor trait 实现 | 通过 | 所有节点均实现了 `type_def()` + `execute()` | 无 |
| executor.rs 注册 | 通过 | 57 个原生节点 + MCP 动态节点在 `StepExecutor::new()` 中注册 | 无 |
| node-schema.json 定义 | 通过 | 所有 12 个关注节点均有 schema 条目 | 无 |
| trigger_cron | 通过 | 实现完整，返回 `triggered_at`/`timestamp` | 节点本身只返回元数据，实际调度由 system/scheduler.rs 负责 |
| trigger_webhook | 通过 | 实现完整，从 `ctx.variables` 读取 `_webhook_*` 注入数据 | 无 |
| trigger_file | 通过 | 实现完整，返回 `path`/`event`/`triggered_at` | 实际触发机制（轮询）与节点执行分离，见触发器系统 |
| webhook_response | 通过 | 实现完整，通过 oneshot channel 回传 HTTP 响应 | 无 |
| llm_chat | 通过 | 支持 OpenAI-compatible API，含 messages/prompt 双输入 | 无 |
| email_send | 通过 | 支持 SMTP（lettre）和 HTTP API 降级 | `#[cfg(feature = "email")]` 下 SMTP 功能需显式编译；非 email feature 下仅支持 HTTP API |
| database_query | 通过 | 支持 SQLite（内置），参数化查询（`?` 占位符） | MySQL/PostgreSQL 标记为"暂不支持"，仅返回错误 |
| json_transform | 通过 | 支持 pick/filter/sort/map/merge/keys/values 7 种操作 | 无 |
| data_filter | 通过 | 支持 12 种比较操作 + Rhai 表达式 | 无 |
| im_message | 通过 | 支持 Slack/飞书/钉钉/企业微信/Telegram | Telegram 的 bot_token:chat_id 解析逻辑有潜在边界问题（split 后未校验长度） |
| github_issue | 通过 | 支持 create_issue/create_pr，含 429 重试逻辑 | 无 |
| prompt_template | 通过 | 支持 `{{variable}}` 插值 + messages 构造 | 无 |

---

## 2. 触发器系统检查（scheduler.rs）

| 检查项 | 状态 | 说明 | 问题 |
|--------|------|------|------|
| trigger_cron 调度 | 警告 | system/scheduler.rs 每 **30 秒轮询** 一次，检查 cron 表达式是否到期 | **不是按 cron 精确触发**，最小精度 30 秒；高频 cron（如每 5 秒）会漏触发或延迟触发 |
| trigger_webhook HTTP 路由 | 通过 | `/api/webhooks/{workflow_id}` (POST) 和 `/test` (GET) 已注册 | 无 |
| trigger_file 文件监控 | 警告 | 使用 **轮询**（每 5 秒检查 mtime），非 `notify` crate | 无法检测"创建"和"删除"事件（只有 mtime 变化）；无法实时响应；大目录下性能差 |
| Webhook 响应回传 | 通过 | 使用 `tokio::sync::oneshot` channel，30 秒超时 | 无 |
| 调度计划 CRUD | 通过 | `/api/schedules` 支持 list/create/update/delete | 无 |
| cron 表达式解析 | 通过 | 支持 5 字段 -> 7 字段自动转换，使用 `cron` crate | 无 |

---

## 3. 表达式引擎检查（parser.rs + context.rs）

| 检查项 | 状态 | 说明 | 问题 |
|--------|------|------|------|
| {{= expr}} 语法 | 通过 | 在 `resolve_string()` 和 `interpolate()` 中均实现 | 无 |
| Rhai 沙箱配置 | 警告 | 设置了 `max_operations=100_000`、`max_string_size=1MB`、`max_array_size=10_000`、`max_map_size=10_000`，禁用 print/debug | **缺少执行时间限制（无 timeout）**；内存限制较宽松（1MB 字符串/1万数组） |
| {{}} 向后兼容 | 通过 | 支持旧语法 `{{step_1.result}}`、`{{params.x}}`、`{{vars.x}}`；无点号直接变量也兼容 | 无 |
| 变量解析优先级 | 通过 | `step_outputs` > `variables`；`params` 命名空间独立 | 无 |
| 嵌套字段访问 | 通过 | 支持 `obj.field`、`arr[0]`、`items[0][1]` | 无 |
| 自动推断依赖 | 通过 | `parser::auto_order_steps()` 根据 `{{step_xxx}}` 引用自动拓扑排序 | 无 |

---

## 4. 工作流引擎检查（executor.rs）

| 检查项 | 状态 | 说明 | 问题 |
|--------|------|------|------|
| run_graph 输入传递 | 通过 | `run_parallel_level` 中每个任务复制 `step_outputs` 和 `variables`；按 edge 注入 `input_ports` | 无 |
| run_parallel_level 错误隔离 | 警告 | 使用 `tokio::task::JoinSet`，每个节点独立任务；失败节点不影响其他节点 | 但 **分支跳转（Branch）在并行层级中直接内联执行**，可能引入额外的并发变量合并复杂度 |
| on_error 策略 | 通过 | `Fail`（默认）/ `Ignore` / `Branch` 在图模式和线性模式均生效 | 无 |
| 拓扑排序 | 通过 | Kahn 算法实现，检测循环依赖，已覆盖测试 | 无 |
| 变量引用预校验 | 通过 | `validate_variable_references()` 在图执行前检查 `{{xxx}}` 指向的节点是否存在 | 只检查节点存在性，不检查端口/字段存在性 |
| 调试断点 | 通过 | 支持 breakpoint / step_mode / pause / continue，有完整 API | 无 |
| 执行计划 | 通过 | `get_execution_plan()` 返回层级数组 | 无 |
| 容器节点占位符 | 通过 | Phase 3 两阶段 resolve：占位符扫描 -> 执行 -> 后处理替换 | 无 |

---

## 5. API 路由检查（server/routes.rs）

| 检查项 | 状态 | 说明 | 问题 |
|--------|------|------|------|
| 后端路由完整性 | 通过 | 42+ 条路由，覆盖 workflows/runs/schedules/preview/debug/plugins 等 | 无 |
| 调试 API | 通过 | `/api/debug/step`, `/continue`, `/breakpoints`, `/vars` 完整 | 无 |
| Webhook API | 通过 | `/api/webhooks/{workflow_id}` (POST), `/test` (GET) 已注册 | 无 |
| WebSocket | 通过 | `/ws/browser` WebBridge 扩展端点 | 无 |
| 事件 SSE | 通过 | `/api/events` SSE 流 | 无 |
| 前端对应性 | 警告 | 后端路由与 Tauri command 并行存在，HTTP API 是主要接口 | 部分旧 command 可能未同步更新（如 `commands/run.rs` 仍独立存在） |

---

## 6. 安全修复检查

| 检查项 | 状态 | 说明 | 问题 |
|--------|------|------|------|
| HTTP SSRF 防护 | 通过 | 拒绝 `file://`、`data:` 协议；拒绝 localhost/127.0.0.1/::1/10.x/192.168.x/172.16-31.x/169.254.169.254 | 防护基于字符串前缀匹配，**未使用 url crate 正规解析**，可能通过 `http://evil@127.0.0.1` 或 DNS 重绑定绕过 |
| shell.rs 白名单 | 通过 | 禁用 `sh`, `bash`, `cmd`, `powershell`, `python`, `node` 等解释器；检查元字符 `; & | \ ` `$` | 无 |
| auth.rs 常量时间比较 | 通过 | `constant_time_eq()` 实现正确，长度不等时仍执行 fold | 无 |
| workflow_export 路径限制 | 通过 | 使用 `canonicalize()` + `starts_with(cwd)` 限制导出路径在工作目录内 | 若 `current_dir()` 获取失败会回退到默认路径，但回退路径未加限制 |
| database_query 参数化查询 | 通过 | SQLite 使用 `?` 占位符 + `rusqlite::types::ToSql` 绑定 | 无 |
| SQL 注入防护 | 通过 | 默认只允许 SELECT/WITH；`allow_write=true` 时仍禁止 DROP/ALTER/TRUNCATE | 无 |
| 输入大小限制 | 通过 | parser 限制 50MB；HTTP 响应限制 100MB | 无 |

---

## 7. 测试覆盖检查

| 检查项 | 状态 | 说明 | 问题 |
|--------|------|------|------|
| executor.rs 测试 | 警告 | 有拓扑排序、图执行、变量解析、条件节点等测试（约 20 个） | **缺少 on_error Branch 策略的图模式测试**；缺少并行层级错误隔离测试 |
| scheduler.rs 测试 | 警告 | 有 DAG 调度器单元测试（4 个：线性链、并行就绪、条件分支过滤、钻石结构） | 缺少 `run_dag_workflow` 的集成测试；缺少错误恢复策略测试 |
| parser.rs 测试 | 通过 | 有版本兼容、ID 唯一性、循环检测、变量解析、条件引用等测试（15+ 个） | 无 |
| context.rs 测试 | 警告 | 无独立测试文件，依赖 parser/executor 的间接测试 | 缺少 `eval_expr` 和 `resolve_var` 的独立单元测试 |
| 节点单元测试 | 失败 | **无任何节点有独立单元测试** | 52 个节点文件均未发现 `#[cfg(test)]` 模块；这是最大的测试缺口 |
| 安全相关测试 | 失败 | 缺少 SSRF、shell 白名单、auth 中间件、路径遍历的专项测试 | 无 |
| 触发器测试 | 失败 | 缺少 cron 调度、webhook 触发、文件监控的自动化测试 | 无 |

---

## 总体评估

| 维度 | 评分 | 总结 |
|------|------|------|
| 功能完整性 | 4/5 | 所有节点和核心功能均已实现，但 trigger_file 未使用 notify crate，trigger_cron 精度不足 |
| 代码质量 | 4/5 | 结构清晰、trait 抽象良好、错误处理到位，但部分边界情况（SSRF 解析、Telegram token 解析）可加强 |
| 测试覆盖 | 2/5 | **严重短板**：52 个节点无任何单元测试；缺少安全、触发器、错误恢复策略的集成测试 |
| 安全性 | 4/5 | SSRF、shell、auth、路径遍历、SQL 注入均有防护，但 SSRF 基于字符串匹配存在绕过可能 |
| 文档/Schema | 5/5 | node-schema.json 是单一真相来源，所有节点有 type_def 自描述，前后端共享 |

---

## 关键问题清单（按优先级排序）

1. **高优先级 - 测试缺失**：52 个节点零单元测试，建议至少为核心节点（http/shell/database/llm/email）添加测试。
2. **中优先级 - trigger_cron 精度**：30 秒轮询无法满足高频 cron 需求，建议改用事件驱动或更小轮询间隔。
3. **中优先级 - trigger_file 实时性**：轮询方案无法检测创建/删除，建议引入 `notify` crate 或 `tokio::fs` 的异步事件。
4. **中优先级 - SSRF 防护强度**：基于字符串前缀的防护可被 `http://evil@127.0.0.1` 绕过，建议使用 `url` crate 解析后检查 host。
5. **中优先级 - Rhai 超时缺失**：`eval_expr` 无执行时间上限，复杂表达式可能阻塞工作流。
6. **低优先级 - database_query 数据库支持**：MySQL/PostgreSQL 标记为"暂不支持"，仅实现 SQLite。
7. **低优先级 - workflow_export 路径**：`current_dir()` 回退路径未加限制（极低风险）。
