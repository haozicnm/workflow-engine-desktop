# Workflow-Engine 开发路线图 v8.4 → v9.0

**制定日期：** 2026-06-15
**更新日期：** 2026-06-20
**当前版本：** 8.4.0（图执行引擎 + Canvas 编辑器 + DESIGN.md 全页面对齐）
**顺序逻辑：** 引擎就绪 → 节点升级 → 前端改造 → 设计统一 → 执行增强 → 生态扩展

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

## ✅ v8.2 已交付 — 阶段 E: 节点全面升级 + 阶段 F: Canvas 编辑器

| # | 任务 | 效果 |
|---|------|------|
| E1 | 56 个 type_def 全覆盖（含 browser_container、excel、word 等复杂节点） | 100% 节点自描述 |
| F1 | Canvas 可视化画布：节点卡片 + 贝塞尔曲线连线 | DAG 可视化 |
| F2 | 拖拽连线创建 Edge + 节点自由拖拽定位 | 直觉操作 |
| F3 | 执行高亮（pending/running/success/error 四色状态） | 执行可视化 |
| F4 | 三视图切换：list / canvas / YAML | 兼容过渡 |
| F5 | 缩放平移（滚轮缩放/拖拽平移） | 操作便利 |

---

## ✅ v8.3 已交付 — DESIGN.md 设计系统落地

| # | 任务 | 效果 |
|---|------|------|
| D1 | 暗色主题重设计（碳灰 #090a0b，4 级 Surface Ladder） | 统一视觉语言 |
| D2 | 字体切换 Geist → Inter Variable (ss03) | 数字可读性 |
| D3 | 翠绿 #10b981 单一强调色策略 | 品牌识别 |
| D4 | Hairline 边框 #23252a + 无阴影扁平设计 | 桌面原生感 |
| D5 | 节点状态色对齐 DESIGN.md 色板 | 15+ 类型色彩统一 |

---

## ✅ v8.4 已交付 — Settings 重构 + 全页面 DESIGN.md 对齐

| # | 任务 | 效果 |
|---|------|------|
| S1 | Settings 重构（移除无效主题、去 shadow、行为统一） | 设置页一致性 |
| S2 | CanvasNode 状态色 Tailwind → Design tokens | 色彩体系完整 |
| S3 | 全局 Shadow 清理（AddStepDialog / ParamField / StepCard） | 纯扁平设计 |
| S4 | Surface 层级修复（StepCard output、Plugins 卡片） | 层级正确 |
| S5 | 页面统一（RunHistory / Editor / Plugins） | 全页面对齐 |

---

## ⬜ 阶段 G：执行增强

> **目标：** 图执行引擎支持错误恢复和断点调试。

| # | 任务 | 预期效果 |
|---|------|----------|
| G1 | 单步执行模式（复用现有 step_mode_flags） | 调试粒度到节点级 |
| G2 | 并行节点的错误隔离（一个节点失败不影响同层其他节点） | 部分成功 |
| G3 | 图模式的 on_error 策略（fail/ignore/branch 在图模式下行为明确化） | 错误处理一致 |
| G4 | 执行回放（记录 ExecutionEvent 序列 → 重放） | 事后分析 |

---

## ⬜ 阶段 H：生态与分发

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
v8.0  ✅ 图引擎 + 元数据 + 变量校验 + 事件流
v8.2  ✅ Canvas 图编辑器 + 56 type_def 全覆盖
v8.3  ✅ DESIGN.md 设计系统落地
v8.4  ✅ Settings 重构 + 全页面对齐（当前版本）
v8.5  ─── G 阶段完成（执行增强）
v9.0  ─── H 阶段完成（生态就绪）
```

**总估算：** G 阶段 1 周，H 阶段 1 周。总计 2 周。
