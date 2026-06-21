# Workflow Engine 开发迭代计划 v8.5 → v9.2

**制定日期：** 2026-06-21  
**当前版本：** 8.4.1  
**目标：** 替代影刀、n8n 等商业工具，成为开源可视化工作流自动化引擎首选

---

按照您 session 中的执行评估，本计划已做以下修订：

1. **v8.5 + v8.6 合并为 v8.5** — 执行增强与触发器可以并行，减少中间态
2. **AI 节点（llm_chat）提前到 v8.7** — 本质是 HTTP 薄封装，无需等到 v9.0
3. **Webhook 响应节点与 trigger_webhook 同步开发** — 触发了必须能返回结果
4. **表达式沙箱纳入 v8.5 核心交付** — 不是技术债务，是安全刚需
5. **单元测试策略前置** — v8.5 建立测试模板，每个新节点自带测试
6. **版本压缩为 5 个** — 总周期从 21 周压缩到 **13 周**

---

## 执行摘要

本计划基于对 Workflow Engine v8.4.1 的深入代码审计（34个节点、图/线性双模执行引擎、Canvas 可视化编辑器、Vue 3 + Rust + Tauri 架构），以及对影刀 RPA 和 n8n 的竞品功能分析，规划未来 6-8 个月的开发迭代路线。

**核心差距：** 与 n8n 相比缺少触发器系统、表达式引擎、400+ SaaS 集成节点、AI 节点；与影刀相比缺少元素捕获器、流程录制、手机自动化、应用市场。本计划按优先级补齐这些差距。

---

## 一、现状总览

### 1.1 已交付能力（v8.0 - v8.4）

| 维度 | 状态 | 说明 |
|------|------|------|
| 执行引擎 | ✅ 成熟 | 图模式（拓扑分层+并行）+ 线性模式，双模自适应 |
| 节点系统 | ✅ 34+ 节点 | 含 7 个容器节点（browser/excel/word/file/cursor/loop/while） |
| 节点元数据 | ✅ 100% 覆盖 | 56 个 type_def，schema-driven 配置表单 |
| Canvas 编辑器 | ✅ 可用 | 拖拽节点、端口连线、缩放平移、执行高亮 |
| 设计系统 | ✅ 落地 | DESIGN.md 暗色主题、碳灰画布、翠绿品牌色 |
| 变量系统 | ✅ 基础 | `{{nodeId.port}}` 模板引用、变量预校验 |
| 事件流 | ✅ 基础 | ExecutionEvent 5 种事件、SSE 推送 |
| 定时调度 | ✅ 可用 | Cron 表达式、调度器引擎 |
| 模板库 | ✅ 5 个 | http_fetch、web_scrape_excel、shell_script、file_processor、data_pipeline |
| 调试 API | ⚠️ 有后端缺前端 | `/api/debug/*` 断点/单步/变量查看 API 已存在，但前端无调试面板 |
| 错误处理 | ⚠️ 部分 | `on_error` 策略（fail/ignore/branch）线性模式可用，图模式未明确化 |
| CLI 工具 | ✅ 可用 | `wf-cli run-file` 支持图/线性自适应 |

### 1.2 竞品差距矩阵

| 功能 | 影刀 | n8n | 当前状态 | 优先级 |
|------|------|-----|----------|--------|
| 触发器（Webhook/定时/文件/邮件） | ✅ | ✅ | ❌ 缺触发器节点 | **P0** |
| 表达式引擎 | ❌ 简单 | ✅ JS 表达式 | ❌ 只有模板替换 | **P0** |
| 调试器（单步/断点/变量查看） | ✅ | ✅ | ⚠️ API有，前端缺 | **P0** |
| 邮件节点 | ✅ | ✅ | ❌ 无 | **P1** |
| 数据库/SQL 节点 | ✅ | ✅ | ❌ 无 | **P1** |
| AI/LLM 节点 | ✅ AI Power | ✅ 70+ AI节点 | ❌ 无 | **P1** |
| 数据转换节点（映射/筛选/聚合） | ✅ | ✅ | ❌ 无（script可替代） | **P1** |
| 流程录制 | ✅ | ❌ | ❌ 无 | **P2** |
| 元素捕获器 | ✅ | ❌ | ❌ 无 | **P2** |
| SaaS 集成节点（Slack/Notion等） | ❌ | ✅ 400+ | ❌ 无 | **P2** |
| 应用市场/模板库 | ✅ 1000+ | ✅ | ⚠️ 5个模板 | **P2** |
| 手机自动化 | ✅ | ❌ | ❌ 无 | **P3** |
| 版本控制/Git | ❌ | ✅ | ❌ 无 | **P3** |
| RBAC/审计日志 | ❌ 企业版 | ✅ 企业版 | ❌ 无 | **P3** |

---

## 二、迭代路线图（修订版：5 阶段，13 周）

```
v8.4.1  ━━ 当前版本
v8.5    ━━ 执行增强 + 触发器 + 表达式引擎（5 周）← 合并 v8.5+v8.6，核心交付
v8.6    ━━ 实用节点 + AI 薄封装（2 周）← 压缩，llm_chat 提前
v8.7    ━━ 前端调试器 + Canvas 增强（3 周）← 用户感知最强
v8.8    ━━ 扩展 AI + SaaS 集成（3 周）← 向量存储+IM+数据库
v9.0    ━━ 应用市场 + 模板库生态（3 周）← 生态闭环
```

**修订说明：**
- 原 v8.5（执行增强）和 v8.6（触发器+表达式）合并 → **新 v8.5**
- 原 v8.7（实用节点）压缩 → **新 v8.6**，同时引入 `llm_chat` 薄封装
- 原 v8.8（Canvas+调试器）+ v9.0（AI）拆分重组 → **新 v8.7**（前端）+ **新 v8.8**（后端扩展）
- 原 v9.1（集成）+ v9.2（市场）合并 → **新 v9.0**
- 总周期：**13 周**（原 21 周）

---

## 三、阶段详解

### ⬜ v8.5 — 执行增强 + 触发器 + 表达式引擎（5 周）

**目标：** 三大核心差距一次性补齐——图执行引擎错误处理、工作流自动触发、表达式求值。这是用户感知最强的版本，也是后续所有功能的基础。

> **策略：** 执行增强（后端）和触发器（后端+前端）是独立模块，可以并行开发。表达式引擎需要 parser.rs + context.rs + placeholder.rs 三文件协同，建议先写集成测试再动手。

---

#### 3.1 执行增强（Execution Enhancement）

| # | 任务 | 技术方案 | 验收标准 | 优先级 |
|---|------|----------|----------|--------|
| G1 | **单步执行模式** | 复用 `breakpoint` 标记 + `step_mode_flags` 状态机；`run_graph` 中检查 `ctx.step_mode` 暂停；`POST /api/debug/step/{run_id}` 推进 | 前端调试面板可点击"步过"按钮单步执行 | **P0** |
| G2 | **并行节点错误隔离** | 修改 `run_parallel_level`：取消 `cancel.cancel()` 的"一失败全取消"策略；改为仅记录失败节点，继续执行其他节点；最后汇总所有错误到 `_error.{nodeId}` | 钻石结构中一个分支失败，另一分支继续执行到汇合点 | **P0** |
| G3 | **图模式 on_error 策略** | 在 `run_graph` 中处理 `on_error`：`fail` → 停止工作流；`ignore` → 记录错误变量 `_error.{nodeId}` 继续执行；`branch` → 将后续依赖节点路由到错误处理分支 | 图模式下每个节点可独立配置 `on_error` 行为，与线性模式行为一致 | **P0** |
| G5 | **前端调试面板** | 新建 `DebugPanel.vue`：显示当前运行状态、变量快照、调用栈、断点列表；集成到 Editor 右侧面板；订阅 SSE `step-update` / `run-update` 事件 | 调试时可在面板查看所有变量值和节点执行状态 | **P0** |
| G4 | **执行回放** | 新增 `ExecutionReplayer`：将 `ExecutionEvent` 序列序列化到 SQLite `execution_events` 表；前端可按时间轴回放执行过程 | 历史运行记录可回放，Canvas 上节点状态随时间变化 | **P1** |

**关键修改文件：**
- `src-tauri/src/engine/executor.rs` — `run_parallel_level` 错误隔离（取消cancel策略）、on_error 策略
- `src-tauri/src/engine/context.rs` — 新增 `step_mode_flags`, `_error.{nodeId}` 错误变量
- `src-tauri/src/data/db/queries.rs` — 新增 `execution_events` 表及查询
- `src/components/DebugPanel.vue`（新建）— 调试器 UI（状态/变量/断点/调用栈）
- `src/components/VariablesPanel.vue`（新建）— 变量查看子面板
- `src/pages/Editor.vue` — 集成调试面板到右侧

**单元测试策略（v8.5 建立模板）：**
```rust
// 每个新节点测试模板（参考 executor.rs 现有测试）
#[tokio::test]
async fn new_node_test_name() {
    let exec = test_exec();
    let step = Step {
        id: "test".into(),
        step_type: "new_node_type".into(),
        config: json!({"param": "value"}),
        ..Default::default()
    };
    let mut ctx = ExecutionContext::new("run-1", &Workflow::default());
    let result = exec.execute(&step, &mut ctx).await;
    assert!(result.is_ok());
    // 验证具体输出
}
```
- 规则：每个新节点文件必须包含 `#[cfg(test)]` 模块，至少 2 个测试用例
- 规则：表达式引擎变更必须通过 `parser_tests.rs` 的兼容性测试（确保 `{{}}` 语法不被破坏）

---

#### 3.2 触发器系统（Trigger System）

**设计：** 新增 `trigger` 节点类型，支持多种触发方式。工作流可配置 `trigger` 节点作为入口，替代纯手动执行。`trigger_cron` 复用现有 `scheduler.rs`。

| 触发器类型 | 说明 | 实现方案 | 同步依赖 |
|-----------|------|----------|----------|
| `trigger_cron` | 定时触发 | 复用 `engine/scheduler.rs`，将调度目标从"工作流ID"改为"trigger_cron节点" | 无 |
| `trigger_webhook` | HTTP Webhook 接收 | 新增 `POST /api/webhooks/{workflow_id}` 路由，验证签名后触发 | **`webhook_response` 必须同步开发** |
| `trigger_file` | 文件/目录变化监测 | 用 `notify` crate 或 `tokio::fs` 轮询，监测文件创建/修改/删除 | 无 |
| `trigger_http_poll` | 定时轮询 HTTP API | 内部使用 cron + http 节点组合 | 无 |
| `trigger_manual` | 手动触发（默认） | 现有行为 | 无 |

> ⚠️ **关键约束：** `trigger_webhook` 和 `webhook_response` 必须同步开发。如果只有触发没有响应，外部系统调用 Webhook 后无法知道工作流执行结果——这会导致集成失败。

**新增节点：**
- `trigger_cron` — 参数：`cron_expr`, `timezone`
- `trigger_webhook` — 参数：`path`, `method`, `auth_header`（可选）
- `trigger_file` — 参数：`path`, `events`（create/modify/delete）
- `webhook_response` — 参数：`status_code`, `headers`, `body`（在 `trigger_webhook` 触发的工作流末尾使用，返回 HTTP 响应）

**新增 API：**
- `POST /api/webhooks/{id}` — 外部系统调用触发工作流
- `GET /api/webhooks/{id}/test` — 测试 Webhook 触发（返回示例 payload）
- `POST /api/webhooks/{id}/response` — 设置 Webhook 响应（由 `webhook_response` 节点内部调用）

**关键文件：**
- `src-tauri/src/nodes/trigger_cron.rs`（新建）
- `src-tauri/src/nodes/trigger_webhook.rs`（新建）
- `src-tauri/src/nodes/trigger_file.rs`（新建）
- `src-tauri/src/nodes/webhook_response.rs`（新建）← 与 trigger_webhook 同步
- `src-tauri/src/server/routes.rs` — 新增 webhook 路由
- `src-tauri/src/engine/scheduler.rs` — 扩展支持 trigger 工作流调度

---

#### 3.3 表达式引擎（Expression Engine）+ 沙箱

**设计：** 当前只有 `{{nodeId.port}}` 简单模板替换。新增 `{{= expr}}` 表达式语法，基于现有 `rhai` 依赖实现真正表达式求值。

**语法约定：**
```
向后兼容（不变）：{{step_1.body}}              → 模板变量替换
新增表达式（新）：{{= step_1.status == 200 ? step_1.body : "error"}}  → 表达式求值
新增表达式（新）：{{= step_1.items.len() > 0}}  → 布尔表达式
新增表达式（新）：{{= DateTime::now()}}         → 函数调用
```

- `{{expr}}` — 保持现有模板替换（向后兼容，不改动）
- `{{= expr}}` — 表达式求值（新语法，前缀 `=` 区分）
- 表达式内可引用 `nodeId`（自动映射到上下文变量）、`params`（工作流参数）、`__item`（循环变量）
- 支持算术、比较、逻辑、字符串、数组、自定义函数

**实现方案：** 在 `engine/parser.rs` 中新增 `resolve_expression()` 函数：
1. 扫描字符串，匹配 `{{= ... }}` 模式
2. 提取表达式内容，注入当前上下文变量到 `rhai::Engine`
3. 调用 `engine.eval_expr::<Dynamic>()` 求值
4. 将结果替换回原字符串位置

**沙箱安全（必须同期实现）：**
- 表达式求值超时：**5 秒**（防止无限循环）
- 内存限制：**10 MB**（防止内存炸弹）
- 禁用 Rhai 危险函数：`eval`, `call`, `fn`, `import`, `throw`, `type_of` 等
- 禁用文件系统访问、网络访问、环境变量读取
- 只暴露纯计算函数：数学、字符串、日期、数组操作

> ⚠️ **关键约束：** 表达式引擎上线后用户就能注入任意 Rhai 代码。沙箱不是"技术债务"，是**安全刚需**，必须在 `{{= expr}}` 语法上线前完成。

**向后兼容测试（先写测试再动手）：**
```rust
// parser_tests.rs 新增兼容性测试
#[test]
fn legacy_template_syntax_unaffected() {
    // 确保 {{nodeId}} 语法不被 {{= expr}} 破坏
    let config = json!("{{step_1.body}}");
    let resolved = resolve_config(&config, &ctx);
    assert_eq!(resolved, "expected_value"); // 与现有行为一致
}

#[test]
fn expression_syntax_basic() {
    let config = json!("{{= step_1.status + 1}}");
    let resolved = resolve_config(&config, &ctx);
    assert_eq!(resolved, "201"); // 200 + 1 = 201
}

#[test]
fn expression_sandbox_rejects_file_access() {
    let config = json!("{{= std::fs::read_file(\"/etc/passwd\")}}");
    let result = resolve_config(&config, &ctx);
    assert!(result.is_err()); // 沙箱应拒绝
}
```

**关键文件：**
- `src-tauri/src/engine/parser.rs` — 扩展 `resolve_expression()`，区分 `{{}}` 和 `{{=}}`
- `src-tauri/src/engine/context.rs` — 将变量注入 Rhai engine（建立 `to_rhai_scope()`）
- `src-tauri/src/engine/placeholder.rs` — 确保与表达式引擎不冲突（placeholder 用于容器节点，表达式用于普通节点）
- `src-tauri/tests/parser_tests.rs` — 新增兼容性测试 + 沙箱测试

---

### ⬜ v8.6 — 实用节点 + AI 薄封装（2 周）

**目标：** 补齐日常自动化中最常用的节点类型，同时引入 `llm_chat` 薄封装——本质上是 HTTP 节点 + OpenAI-compatible 格式，无需等到 v9.0。压缩到 2 周，只保留 P0 节点。

> **策略：** 原 17 个节点压缩为 5+1 个核心节点。其余节点（P1/P2）延后到 v8.8 或后续版本按需添加。

#### 3.4 新增节点列表（P0 必做）

| 节点 | 类型 | 说明 | 依赖 | 测试 |
|------|------|------|------|------|
| `email_send` | 核心 | 发送邮件（SMTP） | `lettre` crate | ✅ 至少 2 个测试 |
| `database_query` | 核心 | 执行 SQL 查询（SQLite/MySQL/PostgreSQL） | `sqlx` 或 `rusqlite`（已有） | ✅ 至少 2 个测试 |
| `json_transform` | 数据 | JSON 数据映射/筛选/重排（用 Rhai 表达式） | 纯 Rust | ✅ 至少 2 个测试 |
| `data_filter` | 数据 | 数组过滤（条件表达式，复用 v8.5 表达式引擎） | Rhai | ✅ 至少 2 个测试 |
| `llm_chat` | AI | 调用 LLM API（OpenAI/Claude/DeepSeek/Kimi 等） | `reqwest`（已有） | ✅ 至少 2 个测试 |
| `prompt_template` | AI | Prompt 模板（变量插值，为 llm_chat 准备输入） | 纯 Rust | ✅ 至少 2 个测试 |

**`llm_chat` 薄封装设计：**

```rust
// llm_chat.rs — 本质：HTTP POST + OpenAI-compatible JSON 格式
impl NodeExecutor for LlmChatNode {
    async fn execute(&self, step: &Step, ctx: &mut ExecutionContext, _exec: &Arc<StepExecutor>) -> Result<Value> {
        let config = step.config;
        let api_key = resolve_config(config.get("api_key")); // 从环境变量或 config 读取
        let model = config.get("model").as_str().unwrap_or("gpt-4o");
        let messages = config.get("messages").clone(); // 数组格式 [{role, content}, ...]
        let temperature = config.get("temperature").as_f64().unwrap_or(0.7);
        let max_tokens = config.get("max_tokens").as_u64().unwrap_or(1024);
        
        // 构造 OpenAI-compatible 请求体
        let body = json!({
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        });
        
        // 调用 HTTP API（复用 reqwest）
        let client = reqwest::Client::new();
        let resp = client.post(config.get("api_url").as_str().unwrap_or("https://api.openai.com/v1/chat/completions"))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send().await?;
        
        let result: Value = resp.json().await?;
        // 返回 { content, usage, model } 结构化输出
        Ok(json!({
            "content": result["choices"][0]["message"]["content"],
            "usage": result["usage"],
            "model": model,
        }))
    }
}
```

**为什么 llm_chat 可以提前：**
- 不涉及向量存储、RAG 等复杂功能——纯 HTTP 调用
- 不引入新依赖——`reqwest`, `serde_json` 已存在
- 测试简单——mock HTTP server 即可
- 用户价值高——立即能接入 AI 能力

**v8.6 延后到 v8.8 的节点（P1/P2）：**
- `email_read`, `csv_transform`, `data_merge`, `data_aggregate`, `datetime`, `hash`, `base64`
- `jwt`, `html_to_text`, `markdown_to_html`, `pdf_extract`, `notify`, `system_info`
- `string_template`（可用 `script` 节点替代）

**关键文件：**
- `src-tauri/src/nodes/email_send.rs`（新建）
- `src-tauri/src/nodes/database_query.rs`（新建）
- `src-tauri/src/nodes/json_transform.rs`（新建）
- `src-tauri/src/nodes/data_filter.rs`（新建）
- `src-tauri/src/nodes/llm_chat.rs`（新建）← 薄封装，提前引入
- `src-tauri/src/nodes/prompt_template.rs`（新建）
- `node-schema.json` — 新增 AI 分类扩展（`llm_chat`, `prompt_template`）

---

### ⬜ v8.7 — 前端调试器 + Canvas 增强（3 周）

**目标：** 用户感知最强的版本。让 Canvas 编辑器成为真正的可视化开发环境，调试器补齐最后一块前端体验短板。

> **策略：** 这部分纯前端工作，与后端无关。可以与 v8.8（后端扩展）并行开发。

#### 3.5 前端调试器（Frontend Debugger）

| 功能 | 说明 | 实现 | 优先级 |
|------|------|------|--------|
| **变量面板** | 实时显示当前作用域所有变量 | 右侧面板 `VariablesPanel.vue`，订阅 SSE 事件更新，按节点分组 | **P0** |
| **断点管理** | 在 Canvas 节点上点击设置/取消断点 | 复用 `step.breakpoint`，CanvasNode 上显示红点标记；右键菜单切换 | **P0** |
| **调用栈** | 显示当前执行路径 | 列表展示已执行节点序列，点击跳转到节点 | **P0** |
| **单步控制** | 步过/步入/继续/停止按钮 | 工具栏按钮组，调用 `POST /api/debug/step/{run_id}` 等 API | **P0** |
| **日志过滤** | 按节点/级别/关键字过滤执行日志 | 复用现有日志面板，增加过滤条件（复选框：节点ID、级别、时间范围） | **P1** |
| **数据流预览** | 鼠标悬停端口显示数据快照 | CanvasEdge 上 tooltip 显示最近传递的数据（JSON 预览，截断 > 200 字符） | **P1** |

#### 3.6 Canvas 编辑器增强

| 功能 | 说明 | 实现 | 优先级 |
|------|------|------|--------|
| **多端口支持** | 节点支持多个输入/输出端口（不只是 in/out） | 扩展 `CanvasNode.vue` 动态渲染 ports，根据 `node-schema.json` 的 `inputs`/`outputs` 数组 | **P0** |
| **端口类型约束** | 按 data_type 限制可连线（如 number 不能连 string） | 修改 `findPortTarget`，检查两端 `data_type` 兼容性，不兼容时显示红色提示 | **P0** |
| **边删除** | 点击选中边，按 Delete 删除 | 增加 `CanvasEdge` 选中状态（stroke 加粗），监听 Delete 键 | **P0** |
| **节点复制粘贴** | Ctrl+C / Ctrl+V 复制节点 | 剪贴板存储节点 JSON（含 actions），粘贴时生成新 ID | **P1** |
| **画布对齐** | 拖拽节点时显示对齐辅助线 | 吸附到最近节点的水平/垂直线（CSS 伪元素实现） | **P1** |
| **迷你地图** | 左下角显示整个画布缩略图 | 新建 `Minimap.vue`，SVG 缩略渲染，支持点击跳转 | **P1** |
| **搜索定位** | 在画布中搜索节点并自动定位 | 复用现有搜索框，增加 Canvas 模式，命中节点高亮并平移画布 | **P1** |
| **执行动画** | 数据流沿边移动的动画效果 | SVG `stroke-dasharray` 动画，运行中的边显示流动效果 | **P2** |
| **自动布局算法** | 支持多种布局：层次、力导向、网格 | 扩展 `autoLayout`，可选算法（默认层次，可选力导向） | **P2** |
| **节点分组** | 框选多个节点分组为子图 | 新增 `Group` 概念，类似 n8n 的 sticky notes，可折叠/展开 | **P2** |

**关键文件：**
- `src/components/CanvasNode.vue` — 多端口动态渲染、断点红点标记
- `src/components/CanvasEdge.vue` — 选中状态、删除、数据流动画
- `src/components/DebugPanel.vue` — 调试面板（状态/单步控制/调用栈）
- `src/components/VariablesPanel.vue` — 变量面板（按节点分组折叠）
- `src/components/Minimap.vue`（新建）— 迷你地图
- `src/composables/useCanvas.ts` — 自动布局扩展、节点分组、复制粘贴
- `src/pages/Editor.vue` — 集成调试面板到右侧抽屉

---

### ⬜ v8.8 — 扩展 AI + SaaS 集成（3 周）

**目标：** 在 `llm_chat` 薄封装基础上，补齐向量存储、RAG 等高级 AI 能力；同时扩展 IM 集成和数据库，从"单机工具"进化为"连接中枢"。

> **策略：** 与 v8.7（前端）并行开发。后端扩展 AI 和集成节点，不依赖前端改动。

#### 3.7 AI 扩展节点

| 节点 | 说明 | 参数 | 依赖 |
|------|------|------|------|
| `llm_embedding` | 生成文本嵌入向量 | `model`, `api_key`, `text` | `reqwest` |
| `llm_agent` | 构建简单 AI Agent（工具调用循环） | `system_prompt`, `tools`, `max_iterations` | `reqwest` + 循环逻辑 |
| `vector_store` | 向量存储操作（add/search/delete） | `action`, `collection`, `vectors`, `top_k` | 本地 SQLite 扩展 或 Qdrant 客户端 |
| `rag_query` | RAG 检索增强生成 | `query`, `vector_store`, `llm_config`, `top_k` | 复用 `llm_chat` + `vector_store` |
| `text_splitter` | 文本分割（用于 RAG 文档处理） | `chunk_size`, `chunk_overlap`, `separator` | 纯 Rust |
| `json_schema_extract` | 从 LLM 输出中提取结构化 JSON | `schema`, `text` | 纯 Rust |

**技术方案：**
- 使用 `reqwest` 调用各 LLM 提供商的 REST API（OpenAI-compatible 格式）
- 向量存储：优先本地 SQLite 向量扩展（无外部依赖），可选 Qdrant 客户端
- 不引入 LangChain 等重依赖，保持轻量
- `llm_agent` 节点：内部循环调用 `llm_chat` + 工具解析，不引入新依赖

**新增 API：**
- `GET /api/llm/models` — 列出支持的模型列表（硬编码常用模型 + 自定义）
- `POST /api/llm/test` — 测试 LLM 连接（验证 API Key 可用）

#### 3.8 SaaS 集成节点

| 节点 | 类型 | 说明 | 实现方案 |
|------|------|------|----------|
| `slack_message` | 集成 | 发送 Slack 消息 | HTTP POST（Slack Webhook API） |
| `feishu_message` | 集成 | 发送飞书消息/卡片 | HTTP POST（飞书 Webhook API） |
| `dingtalk_message` | 集成 | 发送钉钉消息 | HTTP POST（钉钉 Webhook API） |
| `wecom_message` | 集成 | 发送企业微信消息 | HTTP POST（企业微信 Webhook API） |
| `telegram_message` | 集成 | 发送 Telegram 消息 | HTTP POST（Telegram Bot API） |
| `redis` | 数据 | Redis 读写操作 | `redis` crate |
| `mongodb` | 数据 | MongoDB 查询/插入/更新 | `mongodb` crate |
| `github_issue` | 集成 | 创建 GitHub Issue/PR | HTTP POST（GitHub REST API） |
| `s3` | 数据 | AWS S3 / MinIO / 阿里云 OSS | `aws-sdk-s3` 或 `rust-s3` crate |
| `kafka` | 数据 | Kafka 生产消息 | `rdkafka` crate（可选） |
| `rabbitmq` | 数据 | RabbitMQ 发送消息 | `lapin` crate（可选） |

> **降低维护成本的策略：** 所有 IM 集成节点（Slack/飞书/钉钉/企业微信/Telegram）统一用 HTTP 节点实现，不引入专用 crate。配置在 Settings 中统一管理 Webhook URL / Token。

**配置管理：**
- Settings 新增"集成服务"页签，统一配置各服务的 API Key / Token / Webhook URL
- 节点执行时从全局配置读取，不在节点参数中重复输入

**关键文件：**
- `src-tauri/src/nodes/llm_embedding.rs`（新建）
- `src-tauri/src/nodes/llm_agent.rs`（新建）
- `src-tauri/src/nodes/vector_store.rs`（新建）
- `src-tauri/src/nodes/rag_query.rs`（新建）
- `src-tauri/src/nodes/text_splitter.rs`（新建）
- `src-tauri/src/nodes/json_schema_extract.rs`（新建）
- `src-tauri/src/nodes/integrations/`（新建目录）— slack_message, feishu_message, dingtalk_message, wecom_message, telegram_message, redis, mongodb, github_issue, s3
- `src-tauri/src/data/config.rs` — 扩展集成服务配置结构
- `src/pages/Settings.vue` — 新增集成服务配置页

---

### ⬜ v9.0 — 应用市场 + 模板库生态（3 周）

**目标：** 降低上手门槛，建立社区生态。对标影刀应用市场、n8n 工作流模板。

> **策略：** 模板内容制作可以与前端开发并行。技术实现完成后，用 1 周集中制作模板内容。

#### 3.9 应用市场（Marketplace）

**设计：**
- 前端新增 `Marketplace.vue` 页面，从 Dashboard 导航进入
- 模板分类：数据采集、数据处理、自动化办公、AI 应用、电商运营、开发运维
- 每个模板包含：预览图、描述、参数说明、安装按钮、标签
- 模板来源：
  - 内置（随安装包分发，离线可用）
  - 在线（从 GitHub 仓库/CDN 拉取，可更新）
  - 社区（用户提交，审核后上架，未来扩展）

**新增功能：**
- 模板导入：一键导入模板为工作流（自动创建并打开 Editor）
- 模板导出：将工作流导出为模板 JSON（参数化，替换硬编码为 `{{params.xxx}}`）
- 模板评分/评论（v9.1 未来扩展）
- 模板搜索：按名称、标签、分类搜索

#### 3.10 模板库扩展（目标 50+）

| 类别 | 模板数量 | 示例 |
|------|----------|------|
| 数据采集 | 8 | 每日新闻聚合、竞品价格监控、社交媒体数据采集、RSS 订阅抓取、网站变更检测 |
| 数据处理 | 8 | CSV 清洗转换、Excel 报表生成、数据去重合并、JSON 数据转换、数据库同步 |
| 自动化办公 | 8 | 自动邮件发送、会议纪要生成、文件批量处理、定时备份、报告自动生成 |
| AI 应用 | 8 | 智能客服回复、文档摘要生成、RAG 知识库问答、代码审查助手、邮件自动分类 |
| 电商运营 | 6 | 订单自动处理、库存监控、评价自动回复、价格监控、物流跟踪 |
| 开发运维 | 6 | 日志监控报警、定时备份、API 健康检查、自动部署通知、GitHub PR 提醒 |
| 触发器示例 | 6 | Webhook 接收处理、文件变化自动处理、定时数据抓取、Slack 命令响应 |

**关键文件：**
- `src/pages/Marketplace.vue`（新建）— 应用市场页面
- `src/components/TemplateCard.vue`（新建）— 模板卡片组件
- `library/` — 扩展模板库（YAML 格式，参数化）
- `src-tauri/src/server/handlers.rs` — 新增模板 CRUD API（导入/导出/列表）
- `src-tauri/src/engine/template_manager.rs` — 模板管理逻辑

---
---

## 四、技术债务与基础设施

### 4.1 持续维护项

| 任务 | 频率 | 说明 |
|------|------|------|
| 依赖升级 | 每月 | `cargo update`, `npm update`，关注安全漏洞 |
| Clippy 清理 | 每次 PR | 保持零 warning |
| 测试覆盖率 | 每次版本 | 目标 Rust 80%+，前端 60%+ |
| 文档同步 | 每次版本 | AGENTS.md, README.md, CHANGELOG.md 同步更新 |
| 性能基准 | 每季度 | 图执行引擎性能基准（并行加速比、大工作流内存占用） |

### 4.2 架构优化项

| 任务 | 优先级 | 说明 |
|------|--------|------|
| 节点热插拔 | P2 | 运行时动态加载/卸载节点（无需重启） |
| 插件系统完善 | P2 | 当前 plugin_manager.rs 有基础，需完善前端插件 UI |
| 分布式执行 | P3 | 多实例工作流分发（Worker 模式） |
| 数据持久化优化 | P3 | 大数据量工作流变量使用磁盘缓存（避免内存溢出） |
| ~~表达式沙箱~~ | ✅ | **已纳入 v8.5 核心交付**，不再是技术债务 |
| 测试覆盖率基线 | P1 | 每个新节点自带测试，v8.5 建立测试模板（见 3.1 单元测试策略） |

---

## 五、里程碑与验收标准（修订版）

| 版本 | 日期 | 周期 | 核心交付 | 验收标准 |
|------|------|------|----------|----------|
| **v8.5** | 2026-07-05 | 5 周 | 执行增强 + 触发器 + 表达式引擎 | ① 并行节点错误隔离可运行（钻石结构一个失败不影响其他） ② 图模式 `on_error` 策略与线性模式行为一致 ③ 前端调试面板可查看变量和断点 ④ Webhook 可外部触发工作流并返回响应 ⑤ Cron 定时触发器可用 ⑥ 文件变化监测触发器可用 ⑦ `{{= expr}}` 表达式语法生效且沙箱安全 ⑧ `{{}}` 旧语法完全向后兼容 ⑨ 每个新节点自带测试 |
| **v8.6** | 2026-07-19 | 2 周 | 实用节点 + AI 薄封装 | ① `email_send` 可发 SMTP 邮件 ② `database_query` 可执行 SQL ③ `json_transform` 可用 ④ `data_filter` 可用 ⑤ `llm_chat` 可调通至少 3 家 LLM API ⑥ `prompt_template` 可用 |
| **v8.7** | 2026-08-09 | 3 周 | 前端调试器 + Canvas 增强 | ① 前端可设置断点并单步执行（步过/继续/停止） ② Canvas 多端口连线可用，端口类型约束生效 ③ 边可点击选中并删除 ④ 节点可复制粘贴 ⑤ 迷你地图/搜索定位/画布对齐可用 ⑥ 数据流预览 tooltip 可用 |
| **v8.8** | 2026-08-30 | 3 周 | 扩展 AI + SaaS 集成 | ① `llm_embedding` / `vector_store` / `rag_query` 可用 ② `text_splitter` / `json_schema_extract` 可用 ③ 至少 5 个 IM 集成节点可用（Slack/飞书/钉钉/企业微信/Telegram） ④ `redis` / `mongodb` 节点可用 ⑤ `s3` / `github_issue` 节点可用 |
| **v9.0** | 2026-09-20 | 3 周 | 应用市场 + 模板库生态 | ① 应用市场页面可用 ② 模板数量达到 50+ ③ 一键导入/导出模板可用 ④ 模板分类浏览+搜索可用 ⑤ 在线模板可从 GitHub/CDN 拉取 |

---

## 六、风险与应对（修订版）

| 风险 | 影响 | 应对策略 |
|------|------|----------|
| 触发器系统与现有调度器冲突 | 高 | 先设计统一触发器抽象层（`TriggerEngine` trait），再逐步替换；`trigger_cron` 复用现有 scheduler 逻辑 |
| 表达式引擎安全（Rhai 沙箱逃逸） | **高** | **必须在 `{{= expr}}` 上线前完成**：超时 5s + 内存限制 10MB + 禁用危险函数 + 禁止文件/网络/环境访问 |
| 表达式向后兼容 | 高 | 上线前跑 `parser_tests.rs` 兼容性测试套件；`{{}}` 和 `{{=}}` 语法严格区分，不互相干扰 |
| `llm_chat` 薄封装提前引入的稳定性 | 低 | 本质是 HTTP 节点封装，测试简单（mock HTTP server）；失败 gracefully（返回 error 节点） |
| 前端 Canvas 性能下降（节点 > 100） | 中 | 节点数量超过 100 时启用虚拟渲染，限制实时动画；提供"简化模式"切换 |
| 集成节点维护成本膨胀 | 中 | IM 节点统一用 HTTP 实现，不引入专用 crate；配置集中管理，降低单节点复杂度 |
| 版本跨度大导致兼容性问题 | 低 | 每个版本保持 FORMAT_VERSION 兼容；表达式引擎和触发器系统不破坏现有工作流 |
| 资源压缩后的质量下降 | 中 | 2 周 v8.6 只保留 P0 节点；延后节点明确标注到 v8.8 或后续；模板内容制作可与前端开发并行 |

---

## 七、资源估算（修订版）

| 版本 | 开发周期 | 人力（全职） | 主要工作量 |
|------|----------|-------------|------------|
| v8.5 | 5 周 | 1-2 人 | 后端 executor.rs + context.rs 改造（2 周）+ 3 个触发器节点（1.5 周）+ 表达式引擎+沙箱（1.5 周）+ 前端调试面板（1 周，与后端并行） |
| v8.6 | 2 周 | 1 人 | 5 个实用节点 + 1 个 AI 节点（Rust + schema + 测试） |
| v8.7 | 3 周 | 1 人 | 纯前端：Canvas 增强 + 调试器完善 |
| v8.8 | 3 周 | 1-2 人 | 6 个 AI 节点 + 8 个集成节点（Rust + 配置管理） |
| v9.0 | 3 周 | 1 人 | 应用市场前端 + 模板内容制作（可并行） |
| **总计** | **13 周** | **约 3.5 人月** | 原 21 周压缩到 13 周（-38%） |

**修订说明：**
- v8.5 合并后工作量可部分并行（后端执行增强与触发器独立；前端调试面板与后端并行）
- v8.6 压缩到 2 周，只保留 P0 节点（`email_send`, `database_query`, `json_transform`, `data_filter`, `llm_chat`）
- v8.7 纯前端，与 v8.8（后端）可并行开发，但实际串行更稳妥
- 模板内容制作（50+ 模板）可以与前端开发并行，不额外增加人月

---

## 八、附录：节点全景图（v9.0 目标，修订版）

```
核心 (core)           : http, script, condition, shell, delay, approval
数据 (data)           : data_set, data_get, web_scrape, regex, json_transform, data_filter,
                        datetime, hash, base64  ← P1 节点延后，按需添加
文件 (file)           : file_read, file_write, file_list, file_exists, file_checksum, file (container)
系统 (system)         : clipboard_read, clipboard_write, print, mouse_keyboard, window, ocr,
                        notify, system_info  ← P2 节点延后
流程控制 (flow)       : loop, while, cursor, parallel, map, sub_workflow
浏览器 (browser)      : browser (container) — 39 种动作
办公 (office)         : excel (container), word (container) — 含 MCP 扩展
桌面 (desktop)        : mouse_keyboard, window, ocr, print
AI (ai)               : llm_chat, prompt_template, llm_embedding, llm_agent, vector_store,
                        rag_query, text_splitter, json_schema_extract
触发器 (trigger)      : trigger_cron, trigger_webhook, trigger_file, trigger_http_poll, trigger_manual
邮件 (email)          : email_send, email_read  ← v8.8 引入
数据库 (db)           : database_query, redis, mongodb  ← redis/mongodb v8.8 引入
消息 (messaging)      : slack_message, feishu_message, dingtalk_message, wecom_message, telegram_message
开发工具 (dev)        : github_issue, webhook_response, s3
消息队列 (mq)         : kafka, rabbitmq  ← 可选，延后
MCP 扩展 (mcp)        : mcp_excel_csv, mcp_word_write, mcp_word_create, mcp_word_replace, mcp_word_merge
─────────────────────────────────────────────────────────────
v8.4 目标：39 个节点
v8.5 目标：39 + 4 触发器 + 表达式引擎 = 43 类型
v8.6 目标：43 + 6 节点 = 49 类型
v8.7 目标：49（纯前端，无新增节点）
v8.8 目标：49 + 13 节点 = 62 类型
v9.0 目标：62（纯前端+模板，无新增节点）
─────────────────────────────────────────────────────────────
最终目标：62+ 节点（压缩后从原 80+ 调整为 62+，更务实）
```

---

*本计划已根据 session 执行评估完成修订：v8.5+v8.6 合并、AI 提前、Webhook 响应同步、表达式沙箱纳入核心交付、测试策略前置、总周期从 21 周压缩到 13 周。可直接转化为开发任务执行。*
