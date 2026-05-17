# 标准化工作流库 — 设计方案

**核心理念：** 把 Agent 操作标准化、固化、可复用。不追求更大的模型或更多的算力，而是追求更少的重复探索。

---

## 一、现状 vs 目标

```
现状（Agent 每次重新探索）：
  用户 "每天监控 GitHub trending"
    → Agent 理解需求（不确定）
    → Agent 凭空生成 JSON（格式错误率 1/3）
    → 反复重试
    → 终于跑通（但下次还要重来）

目标（标准化工作流库）：
  用户 "每天监控 GitHub trending"
    → Agent 匹配模板: daily-monitor
    → wf-cli library run daily-monitor --params '{"source":"github","top_n":10}'
    → 一次跑通
```

**节省的不是算力，是探索成本。** 每个已验证的工作流都是一份固化下来的"经验"。

---

## 二、标准工作流库架构

### 2.1 库的形态

```
~/.hermes/workflows/          # 或独立 repo
├── catalog.toml               # 模板索引（名称、描述、参数、依赖）
├── monitoring/                # 监控类
│   ├── daily-monitor.wf.json   # A 模式：舆情/数据监控
│   └── uptime-check.wf.json    # 健康检查
├── batch/                     # 批处理类
│   ├── file-process.wf.json    # B 模式：文件批处理+审批
│   └── data-migrate.wf.json    # 数据迁移
├── stress/                    # 测试类
│   └── integration-smoke.wf.json  # C 模式：集成冒烟
├── compose/                   # 组合类
│   ├── daily-routine.wf.json   # 编排多个子工作流
│   └── incident-response.wf.json
└── ...
```

### 2.2 模板定义

```jsonc
// daily-monitor.wf.json — 不是完整 workflow，是可参数化模板
{
  "name": "Daily Monitor",
  "version": "1.0.0",
  "params": {
    "source": {
      "type": "string",
      "default": "httpbin",
      "description": "监控数据源"
    },
    "top_n": {
      "type": "number",
      "default": 10,
      "description": "取 top N 条"
    },
    "output_dir": {
      "type": "path",
      "default": "./reports/daily",
      "description": "输出目录"
    }
  },
  // 工作流中使用 {{params.source}} 引用参数
  "steps": [
    {
      "id": "fetch",
      "type": "http",
      "config": {
        "url": "{{params.source}}",
        "method": "GET"
      }
    },
    // ... 后续步骤引用 {{params.*}}
  ]
}
```

### 2.3 CLI 命令

```bash
# 库管理
wf-cli library list                      # 列出所有模板
wf-cli library search "监控"             # 搜索
wf-cli library show daily-monitor        # 查看详情（参数说明）

# 执行
wf-cli library run daily-monitor \
  --params '{"source":"https://api.github.com/trending","top_n":5}' \
  --json

# 固化（从已验证工作流创建模板）
wf-cli library create daily-monitor \
  --from-workflow a90a7319-... \
  --params source,top_n,output_dir

# Agent 调用示例
wf-cli library run daily-monitor \
  --params '{"source":"https://hn.algolia.com/api/v1/search?query=ai"}'
```

---

## 三、Agent 调用模式

### 3.1 确定性命令（非自然语言探索）

Agent 不再写 JSON。Agent 做三件事：
1. 匹配模板
2. 填充参数
3. 执行

```
Agent 内部逻辑：
  if 任务 ∈ "定期监控" × "网络数据源":
      template = "daily-monitor"
      params = extract_params(user_query)  // {"source": "...", "top_n": 10}
      run("wf-cli library run {template} --params '{json}'")
```

### 3.2 模板匹配策略（避免 AI 猜）

不用 NLP 语义匹配，用**标签+规则**：

```toml
# catalog.toml
[[templates]]
name = "daily-monitor"
tags = ["monitoring", "http", "scheduled", "report"]
pattern = "监控|monitor|舆情|趋势|每日|daily"
params = ["source", "top_n", "output_dir"]
```

Agent 匹配：用户输入含"监控" → 命中 daily-monitor。

### 3.3 模板变体

同个模板支持不同的 connector：

```
daily-monitor + source=github    → GitHub trending 监控
daily-monitor + source=hackernews → HN 热门监控
daily-monitor + source=rss        → RSS 源监控
```

**模板不变，参数换。**

---

## 四、从 A/B/C 到第一批标准模板

### 4.1 立即可固化

| 来源 | 模板名 | 参数 | 用途 |
|------|--------|------|------|
| Workflow A | `daily-monitor` | `source`, `top_n`, `output_dir` | 舆情/数据监控 |
| Workflow B | `file-batch-approval` | `data_dir`, `file_pattern`, `approval_timeout` | 文件批处理+审批 |
| Workflow C | `integration-smoke` | `test_dir`, `iterations` | 集成冒烟测试 |

### 4.2 参数化改造

把 A/B/C 中的硬编码路径和 URL 抽成参数：

```
./test_data/         → {{params.data_dir}}
https://httpbin.org/ → {{params.source}}
{{loop.current}}     → 保持（内部变量不变）
```

### 4.3 固化流程

```bash
# Step 1: 跑通（已完成 ✅）
wf-cli run <id>

# Step 2: 参数化
编辑 workflow JSON，把硬编码值替换为 {{params.xxx}}

# Step 3: 固化入库
wf-cli library create daily-monitor --from <id>

# Step 4: 验证
wf-cli library run daily-monitor --params '{...}'
```

---

## 五、人与 Agent 的分工

```
人的工作（不可替代）：
  - 发现模式：从需求中识别可标准化的流程
  - 设计模板：定义参数接口、错误处理策略
  - 审查质量：确保模板可靠
  - 版本管理：模板升级、兼容性

Agent 的工作（效率工具）：
  - 生成变体：基于模板快速创建新场景
  - 填充参数：从用户描述中提取参数
  - 执行监控：定时跑、收集结果、告警
  - 反馈改进：发现模板不够用时向人报告
```

**工作流库是人与 Agent 共享的语言。** 人定义"动词"（模板），Agent 填入"宾语"（参数）。

---

## 六、里程碑规划

```
v6.9   ─── A 阶段（引擎加固，保证库的执行基础可靠）
            ↓
v6.10  ─── 库基础设施（library create/list/run/show/search）
            catalog.toml 模板索引
            {{params.xxx}} 参数解析
            ↓
v6.11  ─── 第一批模板（A/B/C 参数化 + 入库）
            Agent 调用演示（Hermes 用 wf-cli library run）
            ↓
v7.0  ─── 模板丰富 + CI 质量保证
```

---

## 七、关键设计决策（待讨论）

1. **库的存放位置：** wf-engine repo 内（`library/`）还是独立 repo？独立 repo 方便 Agent 直接 clone 使用。

2. **模板发现机制：** Agent 是通过 `wf-cli library search` 还是直接读 `catalog.toml`？建议提供 JSON API：`wf-cli library list --json`。

3. **cron 集成：** `wf-cli library run` 是否直接支持 `--schedule "0 9 * * *"`？还是由 Hermes cron 来调度？

4. **模板组合：** 能否在模板中引用其他模板？（如 `daily-routine` 编排 `daily-monitor` + `file-batch-approval`）

5. **Agent 失败处理：** Agent 调用模板失败时的标准响应格式？
