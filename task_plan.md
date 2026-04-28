# Workflow Engine Desktop — 开发计划

> 创建于 2026-04-27 | 更新于 2026-04-28（代码审计）

## 目标

将 Workflow Engine Desktop 从「能用」提升到「好用」「易用」，对标影刀RPA的产品体验。

**v2.0 重大升级**：引入 ComfyUI 风格的可视化节点编辑器，实现零代码拖拽连线的工作流构建体验。

**⚡ 离线优先**：软件需在断网条件下正常工作。所有功能默认依赖本地资源，联网功能作为可选增强。

## 版本路线

### v1.x — 线性步骤引擎

---

### v1.2 — 录制生成流程 ✅ 开发完成，待构建

| Phase | 任务 | 状态 |
|-------|------|------|
| 1.1 | 录制→YAML 转换引擎 (`recording_converter.rs`) | ✅ 完成 |
| 1.2 | 桌面录制器 (`desktop_recorder.py`) | ✅ 完成 |
| 1.3 | 录制节点升级 (v2，双通道+全局状态) | ✅ 完成 |
| 1.4 | 前端录制控制栏 (`RecordingBar.vue`) | ✅ 完成 |
| 1.5 | API命令 + 编辑器集成 | ✅ 完成 |
| 1.6 | Rust 编译验证 (`cargo check` 0 err) | ✅ 完成 |
| 1.7 | 前端编译验证 (`npm run build` 无错) | ✅ 完成 |
| **1.8** | **Windows 构建 + 真机测试** | ⏳ 待用户回家 |

### v1.3 — 错误恢复 + 体验打磨 ✅ 完成

| Phase | 任务 | 状态 |
|-------|------|------|
| 2.1 | 错误恢复策略（ignore/skip/branch） | ✅ 完成 |
| 2.2 | 审批超时后自动 fallback 逻辑 | ✅ 完成 |
| 2.3 | 变量实时监视窗（执行时侧栏刷新） | ✅ 完成 |
| 2.4 | 步骤搜索/过滤（Ctrl+F） | ✅ 完成 |
| 2.5 | 并发工作流限制（Semaphore） | ✅ 完成 |

### v1.4 — 模板库 + 流程生成

| Phase | 任务 | 状态 |
|-------|------|------|
| 3.1 | 内置模板扩充到 20+ | ⬚ 未开始 |
| 3.2 | 模板导入/导出 (.workflow.yaml) | ⬚ 未开始 |
| 3.4 | 元素选择器辅助（鼠标悬停高亮） | ⬚ 未开始 |
| 3.5 | 执行历史对比 | ⬚ 未开始 |

### v1.5 — 通知渠道

| Phase | 任务 | 状态 |
|-------|------|------|
| 4.1 | Webhook 通知 | ⬚ 未开始 |
| 4.2 | 企业微信/钉钉/飞书 通知 | ⬚ 未开始 |
| 4.3 | 邮件通知 (SMTP) | ⬚ 未开始 |

## 当前阻塞

- **v1.2 需要在 Windows 上做 `cargo tauri build` 出安装包**，代码已就绪
- 桌面录制功能仅在 Windows 上有效（依赖 user32.dll 全局钩子）

---

### v2.0 — ComfyUI 风格编辑器改造 🚀

> 设计文档：[docs/comfyui-style-design-vision.md](docs/comfyui-style-design-vision.md)
> 核心理念：工作流 = 数据流。节点 = 数据加工站，零代码拖拽连线。
> 技术栈：Vue Flow + Rust/Tauri（DAG 执行引擎）
> 代码路径：前端 `src/components/flow/`，后端 `src-tauri/src/engine/dag*.rs`

#### P1: 基础节点编辑器 ✅ 完成

| 文件 | 说明 |
|------|------|
| `components/flow/pinTypes.ts` (353行) | 针脚类型系统（8色）、节点注册表（19个节点）、辅助函数 |
| `components/flow/ComfyNode.vue` | 自定义 Vue Flow 节点：状态灯、针脚、参数区 |
| `components/flow/NodePalette.vue` | 左侧节点库面板：搜索、分类折叠、拖拽 |
| `components/flow/PropertyPanel.vue` | 右侧属性面板：参数表单、输出预览 |
| `stores/flowStore.ts` (190行) | Pinia store：节点/连线/状态/dirty 标记 |
| `pages/FlowEditor.vue` (584行) | 三栏布局 + 跑马灯连线动画 + 悬停数据预览 + Tauri events |
| 路由 | `/editor-flow/new`、`/editor-flow/:id` |

**节点注册表（19 种）：**

| 类别 | 节点 |
|------|------|
| 📥 数据源 | HTTP 请求、文件操作、剪贴板 |
| 🔧 处理 | JSON 解析、正则处理、数组操作、类型转换、文本拼接、子流程 |
| 🤖 AI | LLM 调用、翻译、摘要、分类、情感分析、实体提取 |
| 📤 输出 | 保存文件、控制台打印 |

#### P2: DAG 执行引擎 ✅ 完成

| 文件 | 说明 |
|------|------|
| `engine/dag.rs` (611行) | FlowNode/FlowEdge 结构、Kahn 拓扑排序、环检测、并行组识别 |
| `engine/dag_scheduler.rs` (645行) | 按拓扑序执行、并行组并发（tokio::spawn）、重试+超时、断点+单步 |
| `commands/dag_run.rs` | `dag_run_start` / `dag_run_cancel` Tauri 命令 |

**DAG 引擎特性：**
- 🟢 拓扑排序（Kahn 算法）+ 环检测
- 🟢 层级并行组（同层级无依赖节点并发）
- 🟢 错误策略：Fail / Ignore / Branch
- 🟢 重试 + 超时（Step.retry / Step.timeout）
- 🟢 断点 + 单步调试
- 🟢 变量快照实时推送
- 🟢 `RunControl` 统一控制结构

#### P3: 高级节点 ✅ 完成

| 节点 | 前端注册 | Rust 实现 | 说明 |
|------|---------|----------|------|
| 文件操作 | ✅ `file` | ✅ `nodes/file.rs` | 读取本地文件 |
| 剪贴板 | ✅ `clipboard` | ✅ `nodes/clipboard.rs` | 读写系统剪贴板 |
| 正则处理 | ✅ `regex` | ✅ `nodes/regex.rs` | 正则提取/替换 |
| 数组操作 | ✅ `array` | ✅ `nodes/array.rs` | 过滤/映射/排序/去重 |
| 类型转换 | ✅ `convert` | ✅ `nodes/convert.rs` | string↔number↔bool↔JSON |
| 控制台打印 | ✅ `print` | ✅ `nodes/print.rs` | 调试输出 |
| AI 调用 | ✅ `call_llm` | 通过 script 节点 | 调用 LLM |
| AI 翻译 | ✅ `translate` | 通过 script 节点 | 多语言翻译 |
| AI 摘要 | ✅ `summarize` | 通过 script 节点 | 文本摘要 |
| AI 分类 | ✅ `classify` | 通过 script 节点 | 文本分类 |
| AI 情感 | ✅ `sentiment` | 通过 script 节点 | 情感分析 |
| AI 实体 | ✅ `extract_entities` | 通过 script 节点 | 命名实体提取 |

> ⚠️ AI 节点前端已注册，但后端通过 script 节点间接执行（依赖 API key 配置），
> 没有独立的 Rust node executor。如需离线 AI 能力，后续可集成本地模型。

#### P4: 增强 ✅ 部分完成

| 功能 | 状态 | 说明 |
|------|------|------|
| 子流程 (`sub_workflow`) | ✅ 完成 | 前后端均已实现（`nodes/sub_workflow.rs`） |
| 悬停数据预览 | ✅ 完成 | FlowEditor 已集成 |
| 跑马灯连线动画 | ✅ 完成 | 执行时连线绿色流动动画 |
| 自动布局 | ✅ 完成 | Vue Flow fitView |
| 工作流模板市场 | ⬚ 未开始 | v1.4 范围 |
| 工作流导入/导出 | ⬚ 未开始 | v1.4 范围 |
| Standalone 桌面应用 | ⬚ 未开始 | 脱离浏览器，用 Tauri webview 运行 |

## 构建状态

| 端 | 命令 | 结果 |
|------|------|------|
| Rust | `cargo check` | ✅ 0 errors |
| 前端 | `npm run build` | ✅ 0 errors |

## 技术债务

| 项 | 说明 | 优先级 |
|----|------|--------|
| AI 节点无独立 Rust 实现 | 通过 script 节点间接执行，依赖联网 API | 中 |
| RHAI thread_local | tokio work-stealing 下可能跨线程 | 低 |
| Step O(n) 查找 | `scheduler.rs` 线性查找 | 中 |
| 全局 SIDECAR 单例 | 浏览器录制进程全局共享 | 低 |

## 错误日志

_（暂无）_
