---
name: workflow-engine-agent
description: "Agent 操作 Workflow Engine：发现积木、生成工作流、保存模板、执行运行。用户说需求，Agent 自动生成可执行的 YAML 工作流。"
triggers:
  - "工作流"
  - "workflow"
  - "自动化"
  - "抓取"
  - "定时任务"
  - "数据管道"
  - "帮我做一个"
  - "每天XX"
---

# Workflow Engine Agent Skill

> Agent 的操作手册：用标准化 API 发现积木、生成工作流、保存模板、执行运行。

## 核心流程

```
用户需求 → 发现积木 → 参考模板 → 生成 YAML → 用户确认 → 保存/执行
```

## 1. 发现积木

先搞清楚用户需要什么能力，再找对应的积木。

### 列出所有分类
```
GET /api/blocks/categories
→ { "categories": [{ "name": "core", "count": 6 }, ...] }
```

### 按关键词搜索积木
```
GET /api/blocks?q=web        # 按标签/名称/描述搜索
GET /api/blocks?q=excel
GET /api/blocks?q=file
GET /api/blocks?category=data  # 按分类过滤
```

### 获取积木详情（含完整 schema）
```
GET /api/blocks/shell
→ {
    "type": "shell",
    "label": "Shell",
    "category": "core",
    "desc": "执行系统命令",
    "tags": ["command", "terminal", "system"],
    "inputs": [...],
    "outputs": [
      { "name": "stdout", "data_type": "text", "desc": "标准输出" },
      { "name": "stderr", "data_type": "text", "desc": "标准错误" },
      { "name": "exit_code", "data_type": "number", "desc": "退出码" }
    ],
    "params": [
      {
        "name": "command",
        "field_type": "string",
        "required": true,
        "desc": "要执行的命令",
        "examples": [
          { "desc": "列出文件", "value": "ls -la /tmp" }
        ]
      },
      {
        "name": "timeout_secs",
        "field_type": "number",
        "required": false,
        "default": 300,
        "validation": { "min": 1, "max": 3600, "desc": "1-3600秒" }
      }
    ]
  }
```

**关键字段：**
- `params[].required` — 必填参数
- `params[].validation` — 参数边界（min/max/pattern）
- `params[].visible_when` — 条件可见性（互斥参数）
- `params[].examples` — 示例值（照着写不会错）
- `outputs` — 步骤产出（引用上游用 `{{step_xxx.field}}`）

## 2. 参考模板

先看有没有现成模板可以复用。

### 搜索模板
```
GET /api/templates?q=抓取
GET /api/templates?category=data
```

### 获取模板详情
```
GET /api/templates/http_fetch
→ { "name": "HTTP 数据抓取", "params": [...], "workflow": {...} }
```

### 从模板创建工作流
```
POST /api/templates/http_fetch/instantiate
{
  "params": { "url": "https://api.example.com/data" },
  "name": "我的数据抓取"
}
→ { "workflow_id": "xxx", "name": "我的数据抓取" }
```

## 3. 生成 YAML 工作流

如果没有现成模板，按标准格式手动生成。

### YAML 格式规范
```yaml
version: "1.0"
name: "工作流名称"
description: "做什么用的"
meta:
  author: agent
  tags: [tag1, tag2]
variables:
  my_var: "默认值"
steps:
  - id: step_1
    type: http
    name: "步骤名称"
    config:
      method: GET
      url: "{{my_var}}"
  - id: step_2
    type: shell
    name: "下一步"
    config:
      command: "echo '{{step_1.stdout}}'"
    run_condition:
      ref: step_1
      when: "true"
```

### 生成规则

1. **id 唯一** — 每个步骤的 id 必须不同，用语义化命名（fetch, parse, save）
2. **type 来自 schema** — 只用 `/api/blocks` 里注册的类型
3. **config 参数** — 只用对应积木 schema 里的 params，required 必填
4. **变量引用** — `{{step_xxx.field}}` 引用上游输出，`{{变量名}}` 引用工作流变量
5. **run_condition** — 条件执行，`ref` 指向条件步骤，`when` 为分支值
6. **validation** — 注意参数的 min/max/pattern 约束

### 常用积木组合

**HTTP 抓取：**
```yaml
- id: fetch
  type: http
  config: { method: GET, url: "https://..." }
- id: check
  type: condition
  config: { mode: expression, expression: "step_fetch.status == 200" }
  run_condition: { ref: fetch, when: "true" }
```

**Shell 命令：**
```yaml
- id: exec
  type: shell
  config: { command: "your command here", timeout_secs: 60 }
```

**循环处理：**
```yaml
- id: loop
  type: loop
  config: { items: "{{step_fetch.body.items}}" }
  body_steps:
    - id: process
      type: shell
      config: { command: "echo {{__item}}" }
```

**文件操作：**
```yaml
- id: read
  type: file_read
  config: { path: "/path/to/file" }
- id: write
  type: file_write
  config: { path: "/path/to/output", content: "{{read.content}}" }
```

**条件分支：**
```yaml
- id: check
  type: condition
  config:
    mode: expression
    expression: "step_fetch.status == 200"
- id: success
  type: shell
  config: { command: "echo ok" }
  run_condition: { ref: check, when: "true" }
- id: fail
  type: shell
  config: { command: "echo failed" }
  run_condition: { ref: check, when: "false" }
```

## 4. 人机协作流程

### 标准交互流程

```
1. Agent 理解需求，确定需要哪些积木
2. Agent 调 /api/blocks/{type} 获取 schema
3. Agent 参考 /api/templates 看有无可复用模板
4. Agent 生成 YAML 工作流
5. Agent 把 YAML 发给用户看（飞书消息/文件）
6. 用户确认或修改
7. Agent 保存工作流：POST /api/workflows { name, yaml }
8. Agent 执行：POST /api/runs { workflow_id }
```

### 保存工作流
```
POST /api/workflows
{ "name": "工作流名称" }

POST /api/workflows/{id}/yaml
{ "yaml": "version: 1.0\nname: ..." }
```

### 执行工作流
```
POST /api/runs
{ "workflow_id": "xxx" }

GET /api/runs/{run_id}/status    # 查状态
GET /api/runs/{run_id}/logs      # 看日志
```

### 保存为模板（固化成果）
```
POST /api/workflows/{id}/save-as-template
{
  "name": "模板名称",
  "desc": "模板描述",
  "tags": ["tag1"],
  "category": "data",
  "params": [
    { "name": "url", "desc": "目标URL", "required": true }
  ]
}
```

## 5. 注意事项

### Agent 必须做的
- ✅ 生成 YAML 前先调 `/api/blocks/{type}` 确认参数 schema
- ✅ 注意 `required: true` 的参数必须提供
- ✅ 注意 `validation` 的 min/max/pattern 约束
- ✅ 注意 `visible_when` 的条件可见性（互斥参数只填一个）
- ✅ 变量引用格式正确：`{{step_id.field}}`
- ✅ 步骤 id 唯一
- ✅ 保存前先让用户确认

### Agent 不能做的
- ❌ 不要猜测参数名 — 必须从 schema 获取
- ❌ 不要忽略 required 参数 — 会导致执行失败
- ❌ 不要直接执行 — 先保存让用户确认
- ❌ 不要修改锁定的工作流 — 会返回 409

### 错误处理
- 400 Bad Request — YAML 格式错误或参数不合法，检查 schema
- 404 Not Found — 工作流/模板不存在
- 409 Conflict — 工作流已锁定，不能修改
- 500 Internal Server Error — 服务端问题，报告给用户

## 6. API 速查表

| 端点 | 方法 | 用途 |
|------|------|------|
| `/api/blocks` | GET | 列出积木（?q=xxx&category=xxx） |
| `/api/blocks/categories` | GET | 列出分类 |
| `/api/blocks/{type}` | GET | 积木详情+schema |
| `/api/nodes/schema` | GET | 完整 schema JSON |
| `/api/templates` | GET | 列出模板（?q=xxx&category=xxx） |
| `/api/templates/categories` | GET | 模板分类 |
| `/api/templates/{name}` | GET | 模板详情 |
| `/api/templates/{name}/instantiate` | POST | 从模板创建工作流 |
| `/api/templates/import` | POST | 导入模板 |
| `/api/workflows` | GET/POST | 列表/创建工作流 |
| `/api/workflows/{id}` | GET/PUT/DELETE | 查/改/删工作流 |
| `/api/workflows/{id}/yaml` | POST | 保存 YAML |
| `/api/workflows/{id}/export-yaml` | GET | 导出标准 YAML |
| `/api/workflows/{id}/lock` | POST | 锁定/解锁 |
| `/api/workflows/{id}/save-as-template` | POST | 保存为模板 |
| `/api/workflows/validate` | POST | 验证 YAML |
| `/api/runs` | POST | 执行工作流 |
| `/api/runs/{id}/status` | GET | 运行状态 |
| `/api/runs/{id}/logs` | GET | 运行日志 |
