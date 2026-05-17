# Workflow 身体化开发规划

> **北极星**：Workflow 是 Agent 的身体。Agent 制定身体（创建阶段），身体自行运作（运行阶段），Agent 不参与运行时决策。
> **当前版本**：v6.9.0 | **目标版本**：v7.0.0

---

## 身体成熟度评估

| 原则 | 现状 | 成熟度 |
|------|------|--------|
| 1. 固定形态 → 稳定接口 | 模板参数化已就绪 | 🟢 80% |
| 2. 内置反射 → 不经过大脑 | onError/retry 已实现，但设计上未强调 | 🟢 70% |
| 3. 本体感知 → 手足知道自己在哪 | 有外部查询（status/history），缺主动推送 | 🔴 30% |
| 4. 肌肉记忆 → 运行时不思考 | 原则已确立，缺乏端到端验证 | 🟡 50% |
| 5. 可组合 → 抓→提→放 | chain/merge 有，库未按小+组合设计 | 🟡 40% |
| 6. 专用化 → 不是万能原语 | Shell 是万能原语，专用模板不够 | 🟡 35% |
| 7. 有限自由度 → 只暴露高层参数 | 模板参数化有，但内部 step 可见可改 | 🟡 45% |

---

## Phase 1：本体感知 —— 让手足会说话

**核心命题**：当前的手足是哑的。运行中的工作流不知道自己在报告自己的存在——需要外部去"问"它。身体必须有**主动报告自己状态**的能力。

### 1.1 步骤级事件推送
每一步执行时主动 emit 事件：
- `step:started` — 「我开始执行 step_3 了」
- `step:completed` — 「step_3 完成了，输出是 {...}」
- `step:failed` — 「step_3 失败了，原因是 403」
- `step:skipped` — 「step_3 被跳过（条件不满足）」

**实现**：
- Rust 后端：`ExecutionContext` 新增 `event_sender: Option<mpsc::Sender<StepEvent>>`
- scheduler 在每个 step 前后发送事件
- CLI 模式：`--watch` 标志订阅事件流，实时打印进度条
- 桌面模式：WebSocket 推送到前端 StatusBar/StepCard 实时更新
- 文件模式：`~/.workflow-engine/runs/{run_id}/events.jsonl` 追加写入

### 1.2 运行心跳
长时间运行的工作流定期发送心跳：
- 每 30 秒：「我还活着，正在 step_5/12」
- CLI watch 模式显示动态进度条：`[████████░░░░░░░░] 5/12 · step_5 执行中 · 已运行 2m34s`
- 超时无心跳 → 标记为可能卡死

### 1.3 完成报告
工作流结束时主动输出结构化完成报告：
```json
{
  "workflow": "每日数据同步",
  "run_id": "r_abc123",
  "status": "completed",
  "duration_ms": 42300,
  "steps": {
    "total": 5,
    "completed": 4,
    "skipped": 1,
    "failed": 0
  },
  "errors": []
}
```
- CLI：`wf-cli run` 结束时自动打印
- Cron：完成报告作为 cron 输出交付

### 1.4 文件系统状态
`~/.workflow-engine/runs/{run_id}/` 目录结构：
```
run_id/
├── state.json        # 实时更新的运行状态
├── events.jsonl      # 事件流（追加写）
├── outputs/          # 每个 step 的输出
│   ├── step_1.json
│   ├── step_2.json
│   └── ...
└── report.json       # 完成后的汇总报告
```

---

## Phase 2：专用化 + 可组合 —— 让手足成形

**核心命题**：Shell 节点是万能原语（原材料），但身体需要的是**已塑形的手**——专用、可组合的固化工作流。

### 2.1 工作流库建设
在 `templates/` 下建立分类库：

```
templates/
├── sync/           # 数据同步类
│   ├── git-repo-sync.wf.json
│   ├── rss-feed-sync.wf.json
│   └── file-backup.wf.json
├── monitor/        # 监控类
│   ├── github-release-watch.wf.json
│   ├── website-uptime-check.wf.json
│   └── disk-space-alert.wf.json
├── report/         # 报告生成类
│   ├── daily-commit-summary.wf.json
│   └── weekly-cost-report.wf.json
├── notify/         # 通知类
│   ├── feishu-alert.wf.json
│   └── email-digest.wf.json
└── transform/      # 数据转换类
    ├── csv-to-json.wf.json
    └── markdown-to-feishu-card.wf.json
```

每个模板：
- 单一职责（做一件事）
- 2-6 个 step（不长，像手指不是手臂）
- 标准参数接口（见 Phase 3）
- 内置错误策略（Fail 除非有明确降级方案）

### 2.2 组合算子
小型工作流的组合方式：

```bash
# 管道组合：A 的输出 → B 的输入
wf-cli library run git-repo-sync --output json | wf-cli library run daily-commit-summary --input -

# 编排组合（模板中引用）
# 在 notify/feishu-alert.wf.json 中：
{
  "steps": [
    { "type": "sub_workflow", "template": "monitor/disk-space-alert", ... },
    { "type": "feishu", "message": "{{step_1.output}}", ... }
  ]
}
```

### 2.3 每个模板自带「使用说明」
模板 JSON 中包含 `meta` 字段：
```json
{
  "meta": {
    "name": "GitHub 仓库同步",
    "description": "克隆或拉取指定 GitHub 仓库到本地",
    "category": "sync",
    "params": {
      "repo_url": "仓库地址（必填）",
      "branch": "分支名（默认 main）"
    },
    "cron_example": "0 0 6 * * *",
    "expected_duration": "30s - 2min"
  }
}
```

---

## Phase 3：有限自由度 —— 手和手腕的边界

**核心命题**：手有 27 个自由度但意识只控制"抓"。工作流模板应该只暴露关键参数，内部 step 不可见不可改。

### 3.1 模板实例化 vs 编辑
两个完全不同的模式：

| | 模板编辑（造手） | 模板实例化（用手） |
|---|---|---|
| 谁操作 | Agent + 人（制定阶段） | 人或 cron（运行阶段） |
| 看到什么 | 完整 step 结构 | 只有参数表 |
| 能改什么 | 一切 | 只改参数值 |
| 入口 | Editor 页面 | TemplatePreview / CLI |

### 3.2 TemplatePreview 加固
现有 TemplatePreview 只做了基础的参数检测。加固：
- 参数从 `meta.params` 读取（标准化，不靠正则扫描）
- 参数类型校验（string/number/boolean/select/upload）
- 必填标记，默认值展示
- 「填入参数并运行」按钮从 TemplatePreview 直达执行
- 执行后不留在编辑器——用户不"拥有"这个工作流实例，它只是一个被手执行的动作

### 3.3 库运行模式（只读）
从工作流库运行时：
- 不创建编辑器副本
- 不进入「我的工作流」列表
- 参数 + 运行，一次性
- 执行历史保存在 `runs/` 下，关联模板名而非工作流 ID
- CLI：`wf-cli library run <template> --param key=value`
- 桌面：TemplatePreview → 填参数 → 点运行 → 看结果

### 3.4 内部 step 的可见性控制
- 从库运行时：不展示内部 step，只展示进度（「正在执行 GitHub 仓库同步 · 步骤 3/5」）
- 高级用户可加 `--verbose` 展开查看内部 step
- 编辑模式（造手）：当然全部可见

---

## Phase 4：肌肉记忆验证 —— 证明身体可以独立行走

**核心命题**：端到端验证「运行阶段零 Agent」。找出所有隐藏的 Agent 依赖并消除。

### 4.1 长时间运行测试
选取 3 个模板，每个跑 50+ 次：
- `git-repo-sync`：每小时一次，持续 3 天
- `website-uptime-check`：每 10 分钟一次，持续 1 天
- `daily-commit-summary`：每天一次，持续 7 天

验证：
- 不需要 Agent 干预
- 错误正确记录和处理（不卡死、不静默失败）
- 资源不泄漏（文件句柄、内存、进程）

### 4.2 错误场景测试
人为制造故障：
- 网络断开时执行 HTTP 请求
- 文件不存在时执行文件读
- 并发执行超过上限
- 磁盘满时写入

验证：
- 内置反射（onError/retry）正确处理
- 错误信息清晰，包含足够上下文
- 不会产生脏状态影响后续运行

### 4.3 Agent 完全剥离
关闭 Agent 接入，只靠 cron + CLI：
```bash
# 只有 cron + wf-cli，没有 Agent 进程
$ crontab -l
0 */6 * * * wf-cli library run monitor/website-uptime-check
0 9 * * * wf-cli library run report/daily-commit-summary
```

验证：工作流独立运行 7 天，产出一致、无中断。

---

## 里程碑

| 版本 | 阶段 | 核心交付 |
|------|------|---------|
| v6.9.1 | Phase 1 本体感知 | 步骤事件推送 + CLI --watch + 文件状态 + 完成报告 |
| v6.9.2 | Phase 1 收尾 | 心跳 + 进度条 + WebSocket 前端实时更新 |
| v6.10.0 | Phase 2 专用化 | 工作流库 10+ 模板 + 组合算子 + meta 字段 |
| v6.10.1 | Phase 3 有限自由度 | 库运行只读模式 + 参数校验 + TemplatePreview 加固 |
| v6.11.0 | Phase 3 收尾 | 内部 step 可见性控制 + verbose 模式 |
| v7.0.0 | Phase 4 肌肉记忆 | 50+ 次长跑测试 + Agent 剥离验证 + 生产就绪 |

---

## 反模式（不要做的事）

1. **不要给 Agent 开运行时后门**：所有决策在固化时做出，运行时零思考
2. **不要把大工作流当手**：6 个 step 以上的工作流是手臂不是手指——拆成小+组合
3. **不要让手足依赖大脑**：错误处理、重试是手足的反射，不经过外部编排
4. **不要把内部 step 暴露给使用者**：「用手的人」不该看到手指的关节
5. **不要在工作流运行时修改它**：身体在执行时不可重新配置——这是「运行阶段零 Agent」的硬约束
