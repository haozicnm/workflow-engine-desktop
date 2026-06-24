# Workflow Engine v9.0.1 最终综合审查报告

**审查日期：** 2026-06-21  
**审查范围：** 后端 Rust 安全/并发/边界 + 前端 TS/性能/内存/可访问性 + 构建部署  
**编译状态：** ✅ 通过（7 个 unused warning）  
**测试状态：** ✅ 83/83 全部通过  

---

## 一、总体结论

本轮深度审查共发现 **5 项 🔴 严重安全漏洞**、**6 项 🔴 严重功能缺陷**、**11 项 🟡 中度问题**、**12 项 🟢 建议优化**。**合计 34 项新问题**，其中 **11 项必须立即修复**。

这些问题**均未被之前的三份报告覆盖**（AUDIT-REPORT、LOGIC-CHAIN-AUDIT、UI-UX-AUDIT），属于本轮新发现的潜在风险。

---

## 二、🔴 严重安全漏洞（5 项）

### 2.1 Shell 白名单可绕过

| 项目 | 详情 |
|------|------|
| **位置** | `nodes/shell.rs:66-91` |
| **问题** | 白名单检查仅验证命令第一个 token。若白名单包含 `sh`/`bash`/`cmd`，攻击者可通过 `sh -c "rm -rf /"` 绕过。超时后仅 drop 任务，**不 kill 子进程**。 |
| **影响** | 启用白名单后仍可执行任意系统命令 |
| **修复** | 白名单模式禁止 `sh`、`bash`、`cmd`、`powershell`；超时后显式 `kill()` 子进程 |

### 2.2 HTTP 节点无 SSRF 防护

| 项目 | 详情 |
|------|------|
| **位置** | `nodes/http.rs` |
| **问题** | 未限制访问私有地址（`10.0.0.0/8`、`192.168.0.0/16`、`127.0.0.0/8`、`localhost` 等），未禁止 `file://` 协议 |
| **影响** | 攻击者可通过工作流访问内网服务或本地文件 |
| **修复** | 请求前解析 URL，拒绝私有 IP、`localhost`、`file://`；跟踪重定向并同样校验 |

### 2.3 API 无 CORS 配置

| 项目 | 详情 |
|------|------|
| **位置** | `server/mod.rs:17` |
| **问题** | `build_router` 完全没有 CORS layer。`tower-http` 的 cors feature 已启用但未使用 |
| **影响** | Webhook 端点无法跨域调用；浏览器端开发模式下可能被同源策略阻断 |
| **修复** | 添加 `.layer(tower_http::cors::CorsLayer::permissive())` 或按配置允许特定 origin |

### 2.4 Bearer Token 时间侧信道攻击

| 项目 | 详情 |
|------|------|
| **位置** | `server/auth.rs:50` |
| **问题** | 使用普通字符串 `==` 比较 token，不是常量时间比较。攻击者可通过响应时间逐字节猜测 token |
| **影响** | 认证 token 可被暴力破解 |
| **修复** | 使用 `subtle::ConstantTimeEq` 或 `ring::constant_time` 进行常量时间比较 |

### 2.5 workflow_export 路径遍历漏洞

| 项目 | 详情 |
|------|------|
| **位置** | `server/managers/workflow_manager.rs:370` |
| **问题** | 直接使用用户传入的 `output_path` 作为文件路径，无目录限制。`../../etc/passwd` 可导致任意文件写入 |
| **影响** | 任意文件覆盖（任意文件写入） |
| **修复** | 限制导出路径必须在 `working_dir` 内；使用 `std::fs::canonicalize` 校验最终路径 |

---

## 三、🔴 严重功能缺陷（6 项）

### 3.1 `email` feature 缺少 `lettre` 依赖

| 项目 | 详情 |
|------|------|
| **位置** | `Cargo.toml:68` + `nodes/email_send.rs:80` |
| **问题** | `email = []` 空 feature，未关联 `lettre` 依赖。启用 `email` feature 时编译必然失败 |
| **修复** | `lettre = { version = "0.11", optional = true }`，并在 `email` feature 中关联 |

### 3.2 容器节点 `{{= expr}}` 被跳过

| 项目 | 详情 |
|------|------|
| **位置** | `engine/placeholder.rs:122` |
| **问题** | `is_known_variable` 不识别 `{{= expr}}` 语法，容器节点 Phase 3 占位符处理时跳过表达式求值。`{{= expr}}` 原样保留，容器反序列化时因类型不匹配失败 |
| **修复** | 在 `is_known_variable` / `scan_string` 中增加 `=` 前缀表达式识别 |

### 3.3 DAG 分支节点重复执行（executor.rs）

| 项目 | 详情 |
|------|------|
| **位置** | `executor.rs:run_graph` |
| **问题** | `on_error=Branch` 直接执行分支节点后，`run_graph` 后续仍按拓扑层级执行该节点（如果它在后续层级中），导致同一节点执行两次 |
| **修复** | 标记已手动执行的节点，后续层级跳过；或统一使用 `scheduler.rs` 的 DAG 调度器 |

### 3.4 Rhai 沙箱不完整

| 项目 | 详情 |
|------|------|
| **位置** | `engine/context.rs:eval_expr` |
| **问题** | 未显式禁用 `print`、`debug`、`type_of` 等函数；未设置 `tokio::time::timeout` 超时；仅依赖 `max_operations` 操作数限制 |
| **修复** | 注册空实现覆盖危险函数；`tokio::time::timeout` 包裹 `eval_expr` 调用（5 秒） |

### 3.5 SQL 注入风险

| 项目 | 详情 |
|------|------|
| **位置** | `nodes/database_query.rs:70` |
| **问题** | `trimmed_sql.starts_with("SELECT")` 防护可被 `SELECT * FROM t; DROP TABLE t;` 绕过。`allow_write` 开启后 `sql` 参数完全由用户控制 |
| **修复** | 使用 `sqlparser` 做 AST 解析，只允许白名单语句；禁止多语句 |

### 3.6 MCP 节点任意代码执行

| 项目 | 详情 |
|------|------|
| **位置** | `nodes/mcp_node.rs:resolve_path` |
| **问题** | 从 `MCP_SERVERS_DIR` 环境变量、插件目录、当前 exe 目录加载 Python 脚本并执行。若攻击者能写入这些目录，可执行任意 Python 代码 |
| **修复** | 对 `MCP_SERVERS_DIR` 和插件目录做权限校验；对加载的脚本做签名验证 |

---

## 四、🟡 中度问题（11 项）

### 4.1 后端

| # | 问题 | 位置 | 修复建议 |
|---|------|------|----------|
| 1 | 循环引用检测不完整 | `executor.rs:validate_variable_references` | 线性模式下也检测 `next` 指针和 `on_error` Branch 循环 |
| 2 | `MAX_STEP_EXECUTIONS` 不合理 | `scheduler.rs:20`（10,000） | 按模式分别设置上限；DAG 按节点数×10；线性允许用户配置 |
| 3 | JSON 输入无大小限制 | `parser.rs:parse_workflow` | 输入字符串 > 50MB 拒绝；使用流式解析 |
| 4 | `cancel_token` 清理不完整 | `run_manager.rs:140-145` | 使用 `scopeguard` 或定期扫描清理过期 run_id |
| 5 | LiveSession 不自动清理 | `preview.rs` | 启动时扫描并清理过久的 `running` session 文件 |
| 6 | 浏览器实例未关闭 | `browser_container.rs` | 容器节点返回前发送 `close_page`/`close_tab` 命令 |
| 7 | API 错误响应格式不一致 | `server/routes.rs` | 统一为 `{"success": bool, "data": T, "error": Option<String>}` |
| 8 | Webhook 端点缺乏防护 | `handlers.rs:405` | 添加 `X-Webhook-Signature` HMAC 签名验证和 rate limiting |
| 9 | 并行变量合并 race condition | `scheduler.rs:946` | 定义冲突策略：默认报错（而非静默覆盖） |

### 4.2 前端

| # | 问题 | 位置 | 修复建议 |
|---|------|------|----------|
| 10 | `matchMedia` 监听器泄漏 | `useTheme.ts:35` | 保存引用，cleanup 中 `removeEventListener` |
| 11 | `useStepRunner` 响应式滥用 | `useStepRunner.ts:82` | 避免 `runStates` 整体替换，使用 `reactive` 直接赋值 |

---

## 五、🟢 建议优化（12 项，前 6 项较重要）

### 5.1 前端（6 项）

| # | 问题 | 位置 | 修复建议 |
|---|------|------|----------|
| 1 | 缺少全局错误处理 | `main.ts` | 添加 `app.config.errorHandler` + `window.onunhandledrejection` |
| 2 | `ErrorBoundary` 不捕获异步错误 | `ErrorBoundary.vue` | 添加 `window.onerror` 代理或使用 `vue-promised` |
| 3 | `undo/redo` 栈不完整 | `useEditorEnhancements.ts:53` | 序列化整个 `Workflow`（steps + edges + variables + name） |
| 4 | `onSaveAs` 类型 hack | `useEditorActions.ts:94` | `Step.id` 改为 `string \| undefined` 类型 |
| 5 | `ApprovalCenter` 模块级初始化 | `ApprovalCenter.vue:138` | 将 `init()` 移入 `onMounted` |
| 6 | `edgeLines` computed 未优化 | `CanvasEditor.vue:214` | 使用 `shallowRef` + 增量更新 |
| 7 | `dirty` 字符串比较不可靠 | `workflowStore.ts:129` | 使用 `structuredClone` + `deepEqual` |
| 8 | 文件导入无大小限制 | `workflowStore.ts:186` | `file.size > 5MB` 拒绝提示 |
| 9 | Canvas 完全不可访问 | `CanvasNode.vue` | 添加 `aria-label` 和 `role` |
| 10 | i18n 缺少 v9.0 新增节点翻译 | `zh-CN.ts` / `en-US.ts` | 补充 `llm_chat`、`email_send` 等翻译 |
| 11 | `offline.html` 缺失 | `vite.config.ts:70` | 创建 `public/offline.html` 或移除配置 |
| 12 | 缺少死代码检测 | — | 添加 `knip` 检查未使用的导出和文件 |

---

## 六、修复优先级总表

### 立即修复（本周，11 项）

| 优先级 | 问题 | 类型 | 文件 |
|--------|------|------|------|
| 🔴 P0 | Shell 白名单可绕过 | 安全 | `nodes/shell.rs` |
| 🔴 P0 | HTTP 无 SSRF 防护 | 安全 | `nodes/http.rs` |
| 🔴 P0 | API 无 CORS 配置 | 安全 | `server/mod.rs` |
| 🔴 P0 | Bearer Token 时间侧信道 | 安全 | `server/auth.rs` |
| 🔴 P0 | workflow_export 路径遍历 | 安全 | `server/managers/workflow_manager.rs` |
| 🔴 P0 | `email` feature 缺少 `lettre` | 编译 | `Cargo.toml` |
| 🔴 P0 | 容器节点 `{{= expr}}` 被跳过 | 功能 | `engine/placeholder.rs` |
| 🔴 P0 | DAG 分支节点重复执行 | 功能 | `executor.rs` |
| 🔴 P0 | Rhai 沙箱不完整 | 安全 | `engine/context.rs` |
| 🔴 P0 | SQL 注入风险 | 安全 | `nodes/database_query.rs` |
| 🔴 P0 | MCP 任意代码执行 | 安全 | `nodes/mcp_node.rs` |

### 短期修复（2 周内，9 项）

| 优先级 | 问题 | 类型 | 文件 |
|--------|------|------|------|
| 🟡 P1 | `matchMedia` 监听器泄漏 | 内存 | `useTheme.ts` |
| 🟡 P1 | `useStepRunner` 响应式滥用 | 性能 | `useStepRunner.ts` |
| 🟡 P1 | 循环引用检测不完整 | 功能 | `executor.rs` |
| 🟡 P1 | JSON 输入无大小限制 | 安全 | `parser.rs` |
| 🟡 P1 | `cancel_token` 清理不完整 | 内存 | `run_manager.rs` |
| 🟡 P1 | Webhook 端点缺乏防护 | 安全 | `handlers.rs` |
| 🟡 P1 | 并行变量合并 race condition | 功能 | `scheduler.rs` |
| 🟡 P1 | 缺少全局错误处理 | 功能 | `main.ts` |
| 🟡 P1 | `undo/redo` 栈不完整 | 功能 | `useEditorEnhancements.ts` |

### 建议优化（后续迭代，12 项）

| 优先级 | 问题 | 类型 | 文件 |
|--------|------|------|------|
| 🟢 P2 | LiveSession 自动清理 | 资源 | `preview.rs` |
| 🟢 P2 | 浏览器实例未关闭 | 资源 | `browser_container.rs` |
| 🟢 P2 | API 错误格式统一 | 规范 | `server/routes.rs` |
| 🟢 P2 | `ErrorBoundary` 异步错误 | 功能 | `ErrorBoundary.vue` |
| 🟢 P2 | `onSaveAs` 类型安全 | TS | `useEditorActions.ts` |
| 🟢 P2 | `ApprovalCenter` 初始化 | 内存 | `ApprovalCenter.vue` |
| 🟢 P2 | `edgeLines` 性能优化 | 性能 | `CanvasEditor.vue` |
| 🟢 P2 | `dirty` 检测不可靠 | 功能 | `workflowStore.ts` |
| 🟢 P2 | 文件导入大小限制 | 安全 | `workflowStore.ts` |
| 🟢 P2 | Canvas 可访问性 | A11y | `CanvasNode.vue` |
| 🟢 P2 | i18n 缺失翻译 | i18n | `zh-CN.ts` / `en-US.ts` |
| 🟢 P2 | `offline.html` 缺失 | 构建 | `vite.config.ts` |

---

## 七、安全加固检查清单

```
□ Shell 白名单：禁止 sh/bash/cmd/powershell 等解释器
□ HTTP SSRF：拒绝 10/172/192/127/localhost/file://
□ CORS：添加 CorsLayer（至少允许特定 origin）
□ Auth：使用 ConstantTimeEq 比较 token
□ 路径遍历：限制导出路径在 working_dir 内
□ SQL 注入：使用 AST 解析替代 starts_with 检查
□ MCP 安全：校验目录权限和脚本签名
□ Rhai 沙箱：注册空实现覆盖 print/debug/type_of/eval/call/fn/import
□ Webhook 防护：添加 HMAC 签名验证和 rate limiting
□ JSON 大小限制：输入 > 50MB 拒绝
```

---

*审查方法：编译检查 + 83 个单元测试 + Clippy + 代码走读（executor.rs 974 行 + scheduler.rs 1494 行 + context.rs 418 行 + 12 个节点文件 + 8 个前端组件 + 6 个状态文件 + 5 个页面 + 2 个初始化文件）+ 安全分析 + 性能分析 + 可访问性审查*
