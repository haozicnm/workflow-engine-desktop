# Workflow Engine Desktop — 开发进度报告

> 版本: v3.4.0 | 日期: 2026-05-03 | 作者: 夏若海

---

## 一、项目概述

Tauri 桌面应用，Litegraph 节点编辑器，自动化工作流引擎。对标 ComfyUI 交互模型。

**目标用户**：离线自动化、RPA 工作流编排。

**技术栈**：Tauri v2 + Vue 3 + TypeScript + Rust + SQLite + Playwright

---

## 二、版本演进

| 版本 | 日期 | 核心内容 |
|------|------|---------|
| v2.1 ~ v2.5 | 04-25~05-02 | 74节点全部落地、action参数全拆独立executor、Excel/Word全能力 |
| v3.0.0 | 05-02 | Dashboard入口、节点状态色环、输出内联预览、日志耗时 |
| v3.0.1 | 05-02 | 连线类型校验（基类onConnectOutput/onConnectInput） |
| v3.0.2 | 05-02 | 返回首页按钮、页面过渡动画 |
| v3.1.0 | 05-02 | 数据流动画（绿色粒子）+ 断点调试（右键设置/取消） |
| v3.2.0 | 05-03 | 输出徽章渲染、快捷键(Ctrl+Enter/./S/F)、弹窗定位 |
| v3.2.1 | 05-03 | 连线拒绝闪红、文档标题、版本水印 |
| v3.2.2 | 05-03 | 工作流名称可编辑、Ctrl+C/V复制粘贴 |
| v3.3.0 | 05-03 | RunHistory持久化修复、单步执行前端落地 |
| v3.4.0 | 05-03 | 审计补全：Escape三级/P/`快捷键/Debug变量面板/右键"从此运行" |

---

## 三、当前状态

### 已完成（100%）

| 分类 | 功能 | 状态 |
|------|------|------|
| **入口** | Dashboard 默认首页 | ✅ |
| **入门** | 8 个内置模板 | ✅ |
| **编辑器** | Litegraph 节点画布、MiniMap | ✅ |
| **节点库** | 74 节点全注册（50 类前端 + 38 类后端） | ✅ |
| **执行** | DAG 并行调度、取消/暂停/断点/单步 | ✅ |
| **状态** | 节点色环(蓝/绿/红)、输出徽章 | ✅ |
| **动画** | 数据流绿色粒子沿连线 | ✅ |
| **日志** | 控制台面板、每步耗时 | ✅ |
| **校验** | 连线类型校验 + 拒绝闪红 | ✅ |
| **断点** | 右键设置/取消、红点指示、breakpoint-hit事件 | ✅ |
| **调试** | 单步执行、变量查看面板 | ✅ |
| **历史** | RunHistory面板 + SQLite持久化 | ✅ |
| **快捷键** | 13 项全覆盖（运行/单步/保存/搜索/缩放/聚焦等） | ✅ |
| **Escape** | 三级：停止运行 → 返回Dashboard → 取消选择 | ✅ |
| **UI** | 工作流名称可编辑、Ctrl+C/V复制粘贴、版本水印 | ✅ |
| **文档** | 文档标题动态更新、设计文档完整 | ✅ |
| **Excel** | 读写创建筛选排序追加CSV互转 (8节点) | ✅ |
| **Word** | 读写创建替换合并 (6节点) | ✅ |
| **浏览器** | Playwright录制/拾取/9子节点 + DAG执行 | ✅ |
| **文件** | 5节点（读写追加删除移动） | ✅ |
| **数据** | JSON解析、文本模板、数组7节点、正则3、转换6 | ✅ |
| **构建** | Linux AppImage 136MB、Windows .exe（CI手动触发） | ✅ |
| **测试** | WSL2 X11 直接渲染已验证 | ✅ |

### 不做

| 功能 | 原因 |
|------|------|
| 节点推荐 | 需要用户行为数据积累 |
| 自动连线吸附 | LiteGraph架构限制，ROI低 |

### 已知限制

- Windows 构建需 GitHub Actions（分钟数已用完，6月1日重置）
- Playwright Chromium 需在线下载（墙内慢），wheels已内置
- 跨飞书App bot的@mention不生效（纯文本兜底）

---

## 四、技术架构

```
前端 (Vue 3 + TypeScript)
  ├── App.vue            — Dashboard / Editor 路由
  ├── LiteGraphEditor.vue — 画布主组件（2,300+行）
  ├── RunHistory.vue      — 运行历史面板
  ├── DebugVarPanel.vue   — 调试变量查看
  └── components/         — 侧栏/搜索/属性/节点库等 15+ 组件

后端 (Rust)
  ├── commands/           — Tauri 命令层（run/dag_run/workflow等）
  ├── engine/             — 执行引擎（DAG调度/Executor/状态机）
  ├── nodes/              — 74 节点 executor（全部无action参数）
  ├── data/               — SQLite 持久化 + Schema迁移(v3)
  └── platform/           — 录制/浏览器

数据
  ├── SQLite (WAL模式)    — 工作流/运行历史/步骤记录/调度
  └── 文件系统            — YAML工作流/输出文件
```

---

## 五、构建产物

### Linux (本地)
- ✅ `Workflow Engine_3.4.0_amd64.AppImage` — 136MB
- 路径: `src-tauri/target/release/bundle/appimage/`
- WSL2 X11 已验证可运行

### Windows (待 Actions 重置)
- 需 `workflow_dispatch` 手动触发
- 预计产物: `.exe` Full版（含wheels+playwright）

---

## 六、下一步

1. **测试**：伟哥在 Windows 上安装测试 v3.4.0
2. **反馈**：收集 bug / UI 改进意见
3. **迭代**：根据反馈调整优先级

---

## 七、提交记录（最近 10 条）

```
f68e91e chore: bump version to 3.4.0
e8de15e v3.4.0: 审计补全 — Escape三级/P/`/Debug面板/从此运行
88bc1b9 v3.3.0: RunHistory持久化修复 + 单步执行前端落地
2155b45 v3.2.2: 工作流名称可编辑 + Ctrl+C/V 复制粘贴
d3d7bd4 v3.2.1: 连线拒绝闪红 + 文档标题 + 版本水印
fbf90b6 v3.2.0: 输出徽章 + 快捷键 + 弹窗定位
d3ee753 v3.1.0: 数据流动画 + 断点调试
ffed9a2 v3.0.2: 返回首页按钮 + 页面过渡动画
6a6b08e v3.0.1: 连线类型校验
c90a618 v3.0.0: Dashboard入口 + 节点状态可视化 + 结构化日志
```
