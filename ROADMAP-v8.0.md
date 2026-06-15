# Workflow-Engine 开发路线图 v8.0 → v9.0

**制定日期：** 2026-06-15
**当前版本：** 8.0.0（图执行引擎 + 节点元数据 + 变量校验 + 事件流）
**顺序逻辑：** 引擎就绪 → 节点升级 → 前端改造 → 生态扩展

---

## ✅ v8.0 已交付

| # | 任务 | 状态 |
|---|------|------|
| 1 | 图执行引擎（拓扑分层 + 并行 + 循环检测） | ✅ |
| 2 | 节点类型元数据（NodeTypeDef / PortDef / type_def / validate_config） | ✅ |
| 3 | 18 个核心节点完成 type_def 实现 | ✅ |
| 4 | 变量引用预校验（执行前检查 {{node.port}} 合法性） | ✅ |
| 5 | ExecutionEvent 事件流（5 种事件 + 可选 mpsc channel） | ✅ |
| 6 | Edge / Position 数据模型 + Workflow.edges | ✅ |
| 7 | CLI 图/线性自适应 + `[图引擎·并行]` 标记 | ✅ |
| 8 | FORMAT_VERSION 1.0 → 2.0 | ✅ |

---

## 阶段 E：节点全面升级

> **目标：** 34 个节点全部完成 type_def + validate_config，让每个节点都有自描述能力和输入校验。

| # | 任务 | 预期效果 |
|---|------|----------|
| E1 | 剩余 16 个节点完成 type_def（approval/webhook/browser/excel/word等） | 100% 节点自描述 |
| E2 | 关键节点增加 validate_config（http url 格式、shell 命令允许列表等） | 执行前拦截错误 |
| E3 | node-schema.json 与 type_def 自动同步（CI 校验） | 双源一致 |
| E4 | `wf-cli steps` 输出包含 type_def 完整元数据 | Agent 可查询节点能力 |

---

## 阶段 F：前端图编辑器

> **目标：** Canvas 编辑器支持可视化连线（Edge），替代纯列表编辑。

| # | 任务 | 预期效果 |
|---|------|----------|
| F1 | Canvas 渲染：节点卡片 + 贝塞尔连线 | 可视化 DAG |
| F2 | 拖拽连线创建 Edge | 直觉操作 |
| F3 | 实时执行高亮（NodeStarted / NodeCompleted 用 ExecutionEvent 驱动） | 执行可视化 |
| F4 | 拓扑预览（`get_execution_plan` → 显示层级分组） | 并行结构可视化 |
| F5 | 列表/Canvas 双视图切换 | 兼容过渡 |

---

## 阶段 G：执行增强

> **目标：** 图执行引擎支持错误恢复和断点调试。

| # | 任务 | 预期效果 |
|---|------|----------|
| G1 | 单步执行模式（复用现有 step_mode_flags） | 调试粒度到节点级 |
| G2 | 并行节点的错误隔离（一个节点失败不影响同层其他节点） | 部分成功 |
| G3 | 图模式的 on_error 策略（fail/ignore/branch 在图模式下行为明确化） | 错误处理一致 |
| G4 | 执行回放（记录 ExecutionEvent 序列 → 重放） | 事后分析 |

---

## 阶段 H：生态与分发

> **目标：** 让 Agent 和外部系统更容易集成。

| # | 任务 | 预期效果 |
|---|------|----------|
| H1 | `wf-cli serve --graph` 模式（WebSocket 推送执行事件） | 前端实时更新 |
| H2 | OpenAPI/Swagger 文档自动生成（用 utoipa 从 axum 路由生成） | API 自文档化 |
| H3 | Hermes Agent skill 集成（从 workflow 生成 SKILL.md 的图模式版本） | Agent 原生支持 |
| H4 | 模板库扩展（图模式模板：并行爬虫/多源聚合/分叉汇合） | 降低上手门槛 |

---

## 里程碑

```
v8.0  ✅ 图引擎 + 元数据 + 变量校验 + 事件流（当前版本）
v8.1  ─── E 阶段完成（34 节点全自描述）
v8.2  ─── F 阶段完成（Canvas 图编辑器）
v8.5  ─── G 阶段完成（执行增强）
v9.0  ─── H 阶段完成（生态就绪）
```

**总估算：** E 阶段 1 周，F 阶段 2-3 周（前端重活），G 阶段 1 周，H 阶段 1 周。总计 5-6 周。
