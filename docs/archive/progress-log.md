# Workflow Engine Desktop — 进度日志

> 负责人：伟哥（审批/测试）、若溪（前端/督促）、若海（后端/全栈）、小艺（技术指导）
> 最后更新：2026-05-04 23:00 | **当前版本：v4.1.0**

---

## 版本路线总览

| 版本 | 日期 | 里程碑 | 状态 |
|------|------|--------|:--:|
| v2.0–v2.5 | 04/29–05/02 | VueFlow→LiteGraph 迁移、DAG 引擎、浏览器自动化、离线打包 | ✅ |
| v3.0–v3.4 | 05/02–05/03 | Dashboard 入口、ComfyUI 对齐、单步调试、Windows NSIS | ✅ |
| **v4.0** | 05/04 | 容器化重构：4 容器替代 70+ 原子节点 | ✅ |
| **v4.1 P0** | 05/04 | 4 容器后端全实现 + 23/23 集成测试 | ✅ |
| **v4.1 P1** | 05/04 | 控制台内嵌面板 + 容器 Session 管理 | ✅ |
| **v4.1 P2** | 05/04 | 日志持久化 + RunHistory 接入 + 性能监控 | ✅ |
| **v4.1 P3** | — | 待定 | ⬚ |

---

## v4.0 — 容器化重构 (2026-05-04)

**决策（伟哥命名）**：4 容器 (Browser/Excel/Word/逻辑判断) 替代 70+ 原子节点。一容器一 session，容器内多 action，连线=端口→端口数据流。

| 改动 | 详情 |
|------|------|
| 前端 | `litegraph-nodes.ts` 1494→295 行 (-1200)，旧类物理删除 |
| 后端 | Excel (calamine+xlsxwriter)、Word (OOXML+zlib)、Browser (Playwright sidecar)、逻辑判断 (11 action) |
| UI | Dashboard v4.0：WorkFlow logo + 搜索 + 新建 + 纯卡片列表（运行/编辑/导出/删除） |
| 模板 | 4 个示例工作流通过 `seed_builtin_workflows()` 写入 DB，纯容器格式 |
| 打包 | `Workflow Engine_4.0.0_x64-setup.exe` ✅ |

---

## v4.1 P0 — 容器后端验证 (2026-05-04)

**测试结果：23/23 通过**

| 容器 | 依赖 | Action 数 | 测试 |
|------|------|:--:|:--:|
| Excel | calamine + xlsxwriter | 7 | ✅ |
| Word | zip (手写 OOXML) | 6 | ✅ |
| Browser | Python sidecar + Playwright | 9 | ✅ |
| 逻辑判断 | serde_json | 11 | ✅ |

**修复 Bug：**
- `condition.rs` actions 路径返回值错误：裸值 → `{branch, value}` JSON
- 逻辑判断容器别名注册（向后兼容）
- 废弃 data 节点测试改为 `ctx.set_var()` 基础操作

---

## v4.1 P1 — 控制台 + Session 管理 (2026-05-04)

| 功能 | 实现 |
|------|------|
| **控制台内嵌面板** | 右下角浮动 (`right:12px, bottom:12px`)，半透明 `rgba(0d1117,0.92)` + blur，可折叠，日志计数徽章，`` ` `` 快捷键 |
| **容器 Session** | `ExecutionContext.sessions: HashMap<node_id, ContainerSession>`，`open_session()` 幂等，`close_session()` 状态切换 |
| **DAG 集成** | `dag_scheduler` 执行前自动 `open_session`，执行后自动 `close_session` |

---

## v4.1 P2 — 日志持久化 + 历史查看 (2026-05-04)

| 层 | 改动 | 文件 |
|----|------|------|
| DB | `StepLogEntry` 模型 + `insert_step_log` / `get_step_logs` 方法 | `models.rs`, `db.rs` |
| 引擎 | `dag_scheduler` 步骤开始/成功/失败自动写 DB 日志 | `dag_scheduler.rs` |
| 命令 | 新增 `run_step_logs` Tauri 命令 | `run.rs`, `main.rs` |
| 前端 | RunHistory 接入 App.vue 第四视图，Dashboard 加「📋 历史」按钮 | `App.vue`, `Dashboard.vue` |
| 日志查看 | 展开运行详情→「📋步骤 / 📟日志」双 Tab，按颜色显示日志级别 | `RunHistory.vue` |

---

## 当前版本信息

| 项目 | 值 |
|------|-----|
| 版本号 | **4.1.0** |
| 安装包 | `Workflow Engine_4.1.0_x64-setup.exe` |
| Rust 后端 | 编译通过，4 warnings（cosmetic） |
| 前端 | vite build 通过，76 modules，559KB bundle |
| 测试 | 23/23 集成测试通过 |

---

## v4.1 架构速查

```
v4.1 容器架构
┌─────────────────────────────────────────────┐
│ 📦 Browser Container (9 actions)            │
│ 📦 Excel Container  (7 actions)             │
│ 📦 Word Container   (6 actions)             │
│ 🔀 逻辑判断 Container (11 actions)           │
├─────────────────────────────────────────────┤
│ DAG 调度器（Kahn 拓扑排序 + 并行组）         │
│ ExecutionContext（session 管理 + 变量解析）   │
│ SQLite（step_logs 持久化 + RunHistory 面板） │
└─────────────────────────────────────────────┘
```

---

## 下一阶段 (P3 候选)

| 候选 | 说明 |
|------|------|
| 错误重试可视化 | `step_logs` 记录每次重试，前端展示重试链 |
| 执行性能面板 | per-step 耗时图表、瓶颈分析 |
| 工作流导入/导出 | JSON 文件导入导出（跨设备迁移） |
| 工作流商店 | 社区模板共享（需后端服务） |

---

*此日志手动更新，记录里程碑进度。*
