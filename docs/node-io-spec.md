# 节点输入/输出规范 (Node I/O Specification)

> **版本:** v1.1 | **最后更新:** 2026-06-03  
> **适用范围:** Workflow Engine v6.10+

本文档固化所有节点的输入（config 参数）和输出（step_outputs 结构），消除引用歧义。

---

## 核心约定

### 1. 步骤输出存储

每个步骤执行完成后，其返回值存入 `ExecutionContext.step_outputs`：

```
key: "step_{step.id}"  →  value: 步骤返回值 (serde_json::Value)
```

- 如果 step.id 是 `"2"`，则 key 为 `"step_2"`
- 如果 step.id 已是 `"step_2"`，`eval_expr` 会先 `strip_prefix("step_")` 再补，避免双重前缀（v6.10 修复）

### 2. 模板变量引用

工作流 JSON 中引用上游输出使用 `{{...}}` 语法：

| 引用方式 | 示例 | 说明 |
|----------|------|------|
| 步骤整体 | `{{step_2}}` | 步骤 2 的完整输出 |
| 容器内 action | `{{step_2.a1_2}}` | 步骤 2 中 id 为 `a1_2` 的 action 结果 |
| 简单节点字段 | `{{step_1.stdout}}` | Shell 节点的 stdout 字段 |
| 嵌套路径 | `{{step_3.results[0].step_1.body}}` | 循环结果中第一轮 step_1 的 body |

### 3. Rhai 脚本变量

Rhai 脚本中通过 `ctx.eval_expr()` 可访问：

```rhai
step_2              // 步骤 2 完整输出 (Value)
step_2.a1_1         // 点号访问嵌套字段
step_2["a1_2"]      // 索引访问（字段名含特殊字符时）
```

---

## 容器节点（Container Nodes）

**所有容器输出均为扁平结构 `{action_id: result, ...}`**，不包裹 `output_ports` 层级。

### Browser（浏览器容器）

**类型名:** `browser`

**输出结构:**

```json
{
  "a1_1": { "url": "...", "title": "...", "text_length": 1234 },
  "a1_2": "<html>...</html>",
  "a1_3": { "count": 5, "items": [...] }
}
```

**action 类型与输出:**

| action_type | 输出结构 |
|-------------|----------|
| navigate | `{ "url": "...", "title": "..." }` |
| click | `{ "selector": "...", "clicked": true }` |
| type | `{ "selector": "...", "text": "...", "typed": true }` |
| extract | `"<提取的 HTML 文本>"` (字符串) |
| extract_all | `{ "count": N, "items": [...] }` |
| screenshot | `{ "path": "/tmp/...png", "format": "png" }` |
| scroll | `{ "to": "bottom" }` |
| wait | `{ "ms": 1000 }` |
| press_key | `{ "key": "Enter" }` |
| evaluate | `"<JS 执行结果>"` |
| get_text | `{ "selector": "...", "text": "..." }` |
| get_attribute | `{ "selector": "...", "attribute": "...", "value": "..." }` |
| select_option | `{ "selector": "...", "value": "..." }` |
| check | `{ "selector": "...", "checked": true }` |
| upload_file | `{ "selector": "...", "files": [...] }` |
| alert_accept / alert_dismiss | `{ "message": "alert text" }` |
| new_tab / switch_tab / close_tab | `{ ... }` |
| reset_state | 无输出 (内部清理) |

> **注意:** 多 browser 容器共享同一浏览器实例。新容器启动时自动 `reset_state`（关闭多余 tab、清空 dialog、navigate about:blank）。

### File（文件容器）

**类型名:** `file`

**输出结构:**

```json
{
  "a2_1": { "path": "/data/input.csv", "content": "...", "size": 1024, "lines": 50 },
  "a2_2": { "path": "/data/output.json", "size": 2048 },
  "a2_3": { "path": "/data/", "entries": [...], "count": 12 }
}
```

**action 类型与输出:**

| action_type | 输出结构 |
|-------------|----------|
| read | `{ "path": "...", "content": "...", "size": N, "lines": N }` |
| write | `{ "path": "...", "size": N }` |
| append | `{ "path": "...", "size": N }` |
| copy | `{ "from": "...", "to": "..." }` |
| move / rename | `{ "from": "...", "to": "..." }` |
| delete | `{ "path": "..." }` |
| list | `{ "path": "...", "entries": [...], "count": N }` |
| exists | `{ "path": "...", "exists": bool }` |
| glob | `{ "pattern": "...", "matches": [...], "count": N }` |
| grep | `{ "path": "...", "matches": [{line, content}], "count": N }` |

### Excel（Excel 容器）

**类型名:** `excel`

**输出结构:**

```json
{
  "a3_1": { "rows": 100, "cols": 5, "sheet": "Sheet1" },
  "a3_2": { "rows": [...], "count": 10 }
}
```

**action 类型与输出:**

| action_type | 输出结构 |
|-------------|----------|
| read | `{ "rows": [...], "count": N, "sheet": "..." }` |
| write | `{ "rows_written": N, "sheet": "..." }` |
| update | `{ "updated": N, "sheet": "..." }` |
| append | `{ "appended": N, "sheet": "..." }` |

### Word（Word 容器）

**类型名:** `word`

**输出结构:**

```json
{
  "a4_1": { "path": "/output/report.docx", "paragraphs": N },
  "a4_2": { "replacements": N }
}
```

**action 类型与输出:**

| action_type | 输出结构 |
|-------------|----------|
| create | `{ "path": "...", "paragraphs": N }` |
| open | `{ "path": "...", "paragraphs": N }` |
| replace | `{ "replacements": N }` |
| merge | `{ "merged_count": N, "output_path": "..." }` |

---

## 简单节点（Simple Nodes）

### Shell

**类型名:** `shell`

**输出:**
```json
{
  "stdout": "命令输出...",
  "stderr": "",
  "exit_code": 0
}
```

> 非零退出码时，节点返回 `Err`，触发 `onError` 策略。

### HTTP

**类型名:** `http`

**输出:**
```json
{
  "status": 200,
  "body": { "key": "value" }  /* JSON 响应自动解析；HTML/文本返回字符串 */,
  "headers": { "content-type": "application/json" }
}
```

### Script (Rhai)

**类型名:** `script`

**输出:** Rhai 表达式求值结果，类型由脚本决定。

```rhai
// 返回字符串
step_1.stdout.to_upper()

// 返回对象
#{key: "value", count: 42}

// 返回数组
[1, 2, 3]
```

**可用变量:**
- `step_N` — 步骤 N 的完整输出
- `step_N.action_id` — 容器内特定 action
- `step_N.field` — 简单节点的具体字段

### Delay

**类型名:** `delay`

**输出:**
```json
{
  "duration_ms": 3000,
  "duration_sec": 3.0
}
```

### Notify

**类型名:** `notify`

**系统通知输出:**
```json
{
  "type": "system",
  "title": "任务完成",
  "body": "数据管道执行成功",
  "sent": true
}
```

**Webhook 输出:**
```json
{
  "type": "webhook",
  "url": "https://...",
  "status_code": 200,
  "response": "..."
}
```

### Data (Set / Get / Length / Default / Merge)

**类型名:** `data_set` / `data_get` / `data_length` / `data_default` / `data_merge`

| 节点 | 输出 |
|------|------|
| data_set | `null`（副作用：写入 `ctx.variables`） |
| data_get | 变量值或 `null` |
| data_length | `N`（数字） |
| data_default | 变量值（如已存在）或默认值 |
| data_merge | `{ "target": "...", "merged_fields": [...] }` |

### Text Template

**类型名:** `text_template`

**输出:** 渲染后的字符串

### JSON Parse

**类型名:** `json_parse`

**输出:** 解析后的 JSON 对象/数组

### Array (Filter / Sort)

**类型名:** `array_filter` / `array_sort`

| 节点 | 输出 |
|------|------|
| array_filter | `[...]` 过滤后的数组 |
| array_sort | `[...]` 排序后的数组 |

---

## 流程控制节点（Flow Control Nodes）

### Condition（逻辑判断）

**类型名:** `condition`

**输出（conditionGroup / actions 格式）:**
```json
{
  "branch": "true",
  "value": { "...": "..." },
  "result": true
}
```

**输出（旧 left/op 格式，兼容）:**
- 通过: 返回 `left` 值
- 不通过: 返回 `null`

### Loop（循环，全遍历）

**类型名:** `loop`

**输出:**
```json
{
  "count": 3,
  "results": [
    { "step_5": {"stdout": "item1", "exit_code": 0} },
    { "step_5": {"stdout": "item2", "exit_code": 0} },
    { "step_5": {"stdout": "item3", "exit_code": 0} }
  ]
}
```

- 可选 `collect` 后处理（summarize/flatten 等）
- 可选 `table` 后处理（转 markdown/CSV 表格）
- 迭代变量: `{{__item}}`, `{{__index}}`, `{{__index1}}`, `{{loop.current}}`, `{{loop.index}}`

### While（条件循环）

**类型名:** `while`

**输出:**
```json
{
  "count": 2,
  "stopped_at": 2,
  "results": [...]
}
```

- `stopped_at` 标识条件不再满足时的轮次索引
- 支持 `max_iterations` 安全上限（默认 1000）
- 条件操作符: `not_empty`, `empty`, `eq`, `ne`, `gt`, `gte`, `lt`, `lte`, `in`, `contains`, `not_contains`

### Cursor（游标迭代）

**类型名:** `cursor`

**输出（未完成）:**
```json
{
  "done": false,
  "item": { "...": "当前数据项" },
  "index": 3,
  "total": 10,
  "remaining": 6
}
```

**输出（已完成）:**
```json
{
  "done": true,
  "total": 10
}
```

> 游标状态持久化到 `~/.workflow-engine/cursors/{step_id}.json`，跨次执行保持位置。

### Approval（人工审批）

**类型名:** `approval`

**输出:**
```json
{
  "decision": "同意",
  "comment": "没问题",
  "item": { "...": "审批上下文数据" },
  "auto": false
}
```

- `auto: true` 表示超时自动决策
- 超时动作: `recommended` / `approve` / `reject`

---

## 引用速查表

### 容器内 action 结果

```
{{step_2.a1_1}}           → action a1_1 的输出
{{step_2.a2_3.entries}}   → action a2_3 的 entries 字段
{{step_2.a1_1.title}}     → action a1_1 的 title 字段
```

### 简单节点字段

```
{{step_1.stdout}}          → Shell 的标准输出
{{step_3.body}}            → HTTP 响应体
{{step_5.status}}          → HTTP 状态码
```

### 循环结果

```
{{step_4.results}}                  → 所有迭代结果数组
{{step_4.results[0].step_5.stdout}} → 第一轮中 step_5 的 stdout
{{step_4.count}}                    → 迭代次数
```

### 条件分支

```
{{step_3.branch}}  → "true" 或 "false"
{{step_3.value}}   → output_template 渲染结果或原始值
```

### 审批结果

```
{{step_6.decision}}  → 用户选择或超时默认值
{{step_6.auto}}      → 是否超时自动决策
```

---

## 常见错误

| 错误 | 原因 | 解决 |
|------|------|------|
| `{{step_2.output_ports.a1_1}}` 取到 null | 容器输出是扁平的，没有 `output_ports` 包裹 | 改用 `{{step_2.a1_1}}` |
| Rhai 中 `step_1.stdout` 未定义 | 旧版双重前缀 bug，变量名是 `step_step_1` | 已修复 (v6.10)，升级即可 |
| 容器 action 配置中 `{{step_3}}` 解析为 Object | executor 全局 resolve_config 类型破坏 | 已修复，容器自行解析模板不经过全局 |
