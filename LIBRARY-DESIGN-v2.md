# 标准化工作流库 — 设计方案 v2

**北极星：** 工作流在 Agent + 人的参与下**制定并固化**，之后**完全独立运行**，不需要 Agent 再参与。

---

## 一、两个阶段，明确边界

```
┌────────────── 制定阶段 ──────────────┐    ┌───── 运行阶段 ─────┐
│                                      │    │                     │
│  Agent + 人 探索 → 测试 → 固化入仓    │───→│  cron / 手动触发     │
│                                      │    │  独立执行            │
│  Agent 参与                           │    │  Agent 退出          │
└──────────────────────────────────────┘    └─────────────────────┘
```

**Agent 只在左边。** 右边是一个确定性执行器，不调用任何 AI。

---

## 二、库的形态

```
~/.hermes/workflows/           # 或 wf-engine/library/
├── catalog.toml                # 索引
├── monitoring/
│   └── daily-monitor.wf.json   # 已固化的模板
├── batch/
│   └── file-process.wf.json
├── stress/
│   └── integration-smoke.wf.json
└── compose/
    └── daily-routine.wf.json   # 编排多个子工作流
```

### catalog.toml

```toml
[[templates]]
name = "daily-monitor"
version = "1.0.0"
description = "每日数据监控，支持任意 HTTP API 数据源"
params = ["source", "top_n", "output_dir"]
schedule = "0 9 * * *"       # 默认调度
```

### 模板 JSON（参数化）

```jsonc
{
  "name": "daily-monitor",
  "version": "1.0.0",
  "params": {
    "source": "https://httpbin.org/json",
    "top_n": 10,
    "output_dir": "./reports/daily"
  },
  "steps": [
    {
      "id": "fetch",
      "type": "http",
      "config": { "url": "{{params.source}}", "method": "GET" }
    }
    // ...
  ]
}
```

---

## 三、运行方式（Agent 不参与）

```bash
# 手动触发
wf-cli library run daily-monitor --params '{"source":"https://api.github.com/trending"}'

# cron 定时
wf-cli library schedule daily-monitor --cron "0 9 * * *"

# 查看所有固化的工作流
wf-cli library list --json
```

运行时链路：`cron → wf-cli library run <name> → 引擎执行 → 结果输出`。全程没有 AI 调用。

---

## 四、固化流程

```
1. Agent + 人 在编辑器中设计、测试工作流（跑通）
2. 将硬编码值替换为 {{params.xxx}}
3. wf-cli library create <name> --from <workflow-id>
4. 注册到 catalog.toml
5. wf-cli library run <name> --params '{...}'  → 验证通过
6. wf-cli library schedule <name> --cron "..."   → 投入生产
```

---

## 五、CLI 命令

```bash
# 库管理
wf-cli library list [--json]              # 列出模板
wf-cli library show <name>                # 查看详情
wf-cli library create <name> --from <id>  # 固化
wf-cli library update <name> --from <id>  # 更新到新版本

# 执行（零 Agent）
wf-cli library run <name> [--params '{}'] [--json]
wf-cli library schedule <name> [--cron "..."] [--enable|--disable]
wf-cli library validate <name>            # 校验参数完整性
```

---

## 六、第一批模板

| 模板名 | 来源 | 参数 | 默认调度 |
|--------|------|------|----------|
| `daily-monitor` | Workflow A | `source`, `top_n`, `output_dir` | 每天 9:00 |
| `file-batch-approval` | Workflow B | `data_dir`, `file_pattern`, `approval_timeout` | 手动 |
| `integration-smoke` | Workflow C | `test_dir`, `iterations` | 每次 push |

---

## 七、待定

1. **库放哪里？** wf-engine repo 内 `library/`，还是独立 repo？
2. **模板组合：** `daily-routine` 能否编排其他模板？（制定阶段定义好，非运行时动态）
