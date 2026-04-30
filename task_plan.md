# Workflow Engine Desktop 2.0 — 开发计划

> 创建于 2026-04-27 | 更新于 2026-04-28

## 目标

Workflow Engine Desktop 2.0 — ComfyUI 风格的可视化 DAG 节点编辑器，零代码拖拽连线工作流构建。

**⚡ 离线优先**：所有功能默认依赖本地资源，联网功能作为可选增强。

## 版本路线

### ✅ v1.x — 线性步骤引擎（全部完成）

P0→P4 全部交付，含 19 种节点类型、录制生成引擎、Playwright 浏览器自动化、定时调度、桌面托盘。

### ✅ v2.0 — ComfyUI 风格 DAG 编辑器（已完成）

| Phase | 内容 | 状态 |
|-------|------|------|
| P1 | 基础节点编辑器：pinTypes + ComfyNode + NodePalette + PropertyPanel + FlowEditor | ✅ |
| P2 | DAG 执行引擎：拓扑排序 + 并行组 + 断点/单步 + 重试/超时 | ✅ |
| P3 | 高级节点：19 种节点类型注册 | ✅ |
| P4 | 增强：Undo/Redo + AutoSave + 悬停预览 + 跑马灯动画 | ✅ |
| 健检 | 代码审计 + Bug 修复 + 双端构建 | ✅ |
| 清理 | 删除 v1.x 弃用代码（Editor.vue、step-config/ 等 27 文件）| ✅ |

**技术栈**：Vue 3 + Vue Flow + Pinia + Tauri 2 + Rust DAG Engine

### 📋 v2.1 — 后续规划

| 任务 | 状态 |
|------|------|
| Windows 构建 + 真机测试 | ⏳ 待用户 |
| 模板市场 + 导入/导出 | ⬚ 未开始 |
| 企微/钉钉/飞书通知 + Webhook | ⬚ 未开始 |

## 构建状态

| 端 | 命令 | 结果 |
|------|------|------|
| Rust | `cargo check` | ✅ 0 errors |
| 前端 | `npm run build` | ✅ 0 errors |
| Rust 单测 | `cargo test` | ✅ 21/21 pass |
| 前端单测 | `npm test` | ✅ 32/32 pass |
| Windows CI | `.github/workflows/build-windows.yml` | 🔧 已修复，待验证 |

## 当前进展（截至 2026-04-30）

| 角色 | 完成项 | 进行中 |
|------|--------|--------|
| 若溪 | Store 统一 + DAG 接入 + 表达式引擎 + 32 单测 | M2 前端节点 |
| 若海 | CI 修复 ×3 + DAG 调度 | M1 Windows 构建验证 |

## 当前阻塞

- Windows CI 构建结果待确认（已 push，等 GitHub Actions 跑完）
- Windows 真机冒烟测试待伟哥执行

## 技术债务

| 项 | 说明 | 优先级 |
|----|------|--------|
| AI 节点无独立 Rust 实现 | 通过 script 节点间接执行 | 中 |
| RHAI thread_local | tokio work-stealing 下可能跨线程 | 低 |
| Step O(n) 查找 | `scheduler.rs` 线性查找 | 中 |
