# wf-engine 开发路线图 v6.9 → v7.0

**制定日期：** 2026-05-17  
**依据：** 三个全量工作流集成测试（A/B/C）发现的 10 个 bug  
**顺序逻辑：** 加固地基 → 降低翻译损耗 → 补全 CLI → 持续验证

---

## 阶段 A：加固引擎（地基）

> **目标：** 让"加新节点类型"不再重复出低级 bug。Parser ↔ Executor 的契约用编译期保证，不是靠人肉对齐。

| # | 任务 | 解决哪个 Bug | 预期效果 |
|---|------|-------------|----------|
| A1 | parser 导出 `CONTAINER_TYPES` 为 pub const，executor 用宏遍历注册 | #1, #2 | 新增容器类型时编译器自动报错，不再漏注册 |
| A2 | 抽取 `resolve_iteration_items()` 到 `engine/common.rs`，loop 和 cursor 共用 | #6, #9 | 去重，修复只在一处生效另一处遗漏 |
| A3 | executor.rs 用 `clone_fields!` 宏替换手动字段复制 | #3 | 不再出现"这个字段忘了 clone"的 bug |
| A4 | parser 增加全量 camelCase→snake_case 归一化（不只是 conditionGroup） | #4 | 前端发什么格式 parser 都能接住 |
| A5 | `build.rs` 生成测试：每个 CONTAINER_TYPES 条目在 executor 中必须有对应注册 | #1, #2 | CI 红色 = 契约断裂，零误判 |

**完成后：** 任何人（Agent 或人）新增节点类型，只需改一处 + 编译器自动校验。

---

## 阶段 B：Agent-Workflow 集成（翻译层）

> **目标：** 降低 Agent 生成 JSON → 引擎执行的"翻译损耗"。Agent 写的 JSON 不该因格式细节而失败。

| # | 任务 | 解决什么问题 | 预期效果 |
|---|------|-------------|----------|
| B1 | parser 自动识别 JS 风格 `{...}` 并转为 Rhai `#{...}` | 用户持续写错 | Agent 用 JS 语法也能跑 |
| B2 | resolve_config 增加未解析模板的友好报错（含上下文提示） | 调试困难 | `{{unknown.var}}` → "变量 unknown 不存在，可用变量: [list]" |
| B3 | 统一变量引用规范文档（`loop.*` / `cursor.*` / `step_N.*`），写入 SKILL.md | #8, #10 | Agent 生成 JSON 时自动引用正确路径 |
| B4 | parser 容错：CSV 逗号分隔的 items 自动转数组 | 前端可能以不同格式发送 | `"a,b,c"` → `["a","b","c"]` |
| B5 | 内置 workflow 模板库（对应 A/B/C 三种模式），Agent 可按需变异 | 起点太低 | 从模板改比从零写快 10x |

**完成后：** Agent 说"帮我做一个每天监控 GitHub trending 的 workflow"，生成→解析→执行的端到端成功率 > 80%。

---

## 阶段 C：CLI 能力补全（无头执行器）

> **目标：** CLI 成为一个真正可用的生产级执行器，能跑 cron 定时任务。

| # | 任务 | 解决什么问题 | 预期效果 |
|---|------|-------------|----------|
| C1 | browser sidecar 预热机制（CLI 启动时预启动） | sidecar 反复重启浪费 10s+ | 首次 browser 操作零等待 |
| C2 | CLI 模式 approval 节点行为明确化（超时→降级分支 或 报 warn 退出） | 现在静默通过，不知道走了哪个分支 | 行为可预期 |
| C3 | notifications 在 Linux 上非阻塞降级（不报 warn） | 每次运行都刷 warn | 日志干净 |
| C4 | `wf-cli run` 增加 `--json` 输出模式（所有步骤结果一行一 JSON） | 现在输出不可编程消费 | 可被 Python/shell 管道处理 |
| C5 | `wf-cli status <run_id>` 可用（当前报"运行记录不存在"） | 无法事后查看运行结果 | 事后可查 |

**完成后：** `wf-cli run <id> --json | jq` 成为标准工作方式。cron + workflow 组合稳定运行。

---

## 阶段 D：持续铺测试覆盖

> **目标：** 在写新功能之前，把现有功能的可靠性建立起来。当前 32% 动作覆盖 → 目标 80%。

| # | 任务 | 预期效果 |
|---|------|----------|
| D1 | 动作矩阵追踪（未覆盖的 46 个动作列清单） | 明确欠债规模 |
| D2 | Excel 专项：formula / pivot / chart / multi-sheet | #A/B 只测了 read/filter |
| D3 | Word 专项：template / mail-merge / table | 从未跑过 |
| D4 | Browser 专项：click/input/scroll/evaluate/extract 全套 | 只测了 navigate/screenshot/wait |
| D5 | File 专项：grep / append / delete / exists / copy/move | 只测了 glob/list/read/write |
| D6 | Logic 组合：嵌套 conditionGroup + 多级分叉 | 只测了单层 and |
| D7 | 并发安全：两个 workflow 同时跑 | 完全未测 |
| D8 | A/B/C 写入 CI test fixtures，每次 push 自动跑 | 回归保护 |

**完成后：** 80%+ 动作覆盖，CI 自动化，改代码有底气。

---

## 里程碑

```
v6.9  ─── A 阶段完成（地基加固）
v6.10 ─── B 阶段完成（Agent 集成）
v6.11 ─── C 阶段完成（CLI 生产就绪）
v7.0  ─── D 阶段完成（测试覆盖达标）
```

**总估算：** 每个阶段 1-2 周（取决于并行度），总计 1-2 个月。
