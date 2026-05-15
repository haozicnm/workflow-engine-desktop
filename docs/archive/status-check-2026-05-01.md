# Workflow Engine Desktop — 实际状态分析

> 2026-05-01 07:30 | 全部实测，不只是看 commit 列表

## 一、硬指标（刚跑完）

```
Rust:  cargo check ✅  3.6s  |  clippy ✅  0 warn  |  test ✅  23/23
前端:  npm build ✅  2.1s  |  vue-tsc ✅  0 err   |  test ✅  32/32
调试残留: console.log 0 | dbg! 0 | eprintln! 0
```

编译没问题，测试全绿，代码卫生干净。

## 二、架构实况

### 当前组件关系

```
LiteGraphEditor.vue (976行)  ← 主编辑器，唯一路由入口
  ├── NodePalette.vue (279行)     ← 左侧节点库，拖拽创建
  ├── LiteGraph canvas            ← 画布（canvas-based）
  │   └── litegraph-nodes.ts (950行) ← 35 个 LGraphNode 子类
  └── PropertyPanel.vue (694行)   ← 右侧属性面板，按节点类型显示参数

Pinia stores:
  flowStore.ts     ← 编辑器在用（LiteGraphEditor + useAutoSave + useUndo）
  workflowStore.ts ← Dashboard + executionStore 在用 ⚠️ 未统一！
  editorStore.ts   ← 工具函数
  executionStore.ts ← 执行管理（调 workflowStore）
```

**路由**: `/editor/new` 和 `/editor/:id` → LiteGraphEditor（FlowEditor 已不路由，但文件还在）

### LiteGraph ↔ Store 双向同步

代码里有完整的 `syncGraphToStore()` / `loadFromStore()` 桥接：
- LiteGraph 拖拽/连线 → `syncGraphToStore()` → Pinia flowStore
- Pinia flowStore 变化 → `loadFromStore()` → LiteGraph graph
- ID 用 `String(node.id)` 统一（`0f4036a` 已修坐标转换）

这个桥接逻辑是完整的，验证过。

## 三、🔴 实际发现的问题

### 1. 工作树脏了 — LiteGraphEditor.vue 未提交

```diff
git status:
 M src/pages/LiteGraphEditor.vue  ← 未暂存！

改动内容：
- window.addEventListener('resize') → ResizeObserver on wrapper div
- +ref="wrapperRef" on canvas-wrapper
- +nextTick 中手动设 canvas.width/height
- 删除了 onResize() 函数
```

这是 ResizeObserver 改进，代码逻辑看起来正确，但**没有 commit**。不确定是遗忘还是开发一半。

### 2. 两套 Store 并存 — 统一未完成

若溪 4/29 报过「Store 统一，删 workflow.ts，workflowStore.ts 唯一」，但实际情况：

| Store | 在哪里用 |
|-------|---------|
| `flowStore.ts` | LiteGraphEditor, useAutoSave, useUndo, Dashboard(secondary) |
| `workflowStore.ts` | Dashboard(primary), executionStore, editorStore |

**Dashboard.vue 同时用两套 store**——第 46 行 `useWorkflowStore()` + 第 137 行 `useFlowStore()`。

两套 store 各管各的状态，互不同步。这不是「统一」，是「并存」。

### 3. 35 个 onExecute 全是空壳

litegraph-nodes.ts 里所有 35 个节点的 `onExecute()` 都是：
```typescript
onExecute(): void {
  // Execute HTTP request   ← 只有注释，没有代码
}
```

但这是**架构选择**，不是 bug —— 真实执行走 Rust Tauri backend（dag_run_start + Tauri events）。LiteGraph 只做画布渲染。如果将来要做本地预览或离线模式，需要补这些。

## 四、🟢 小问题

- **死代码**: `FlowEditor.vue`（0 处引用）、`ComfyNode.vue`（仅被 FlowEditor 引用）还在 repo 里
- **构建警告**: 609KB 单 JS chunk（非阻塞，vite 建议 code-split）
- **PropertyPanel**: 694 行功能完整，但 0 单测覆盖
- **Vite dev server**: 正跑在 :1420，可以 Windows 浏览器直连测试

## 五、总结

| 维度 | 状态 | 说明 |
|------|:----:|------|
| 编译 | ✅ | Rust + 前端 全过 |
| 测试 | ✅ | 55 个全绿（23+32） |
| 代码卫生 | ✅ | 0 调试残留，0 lint warn |
| LiteGraph 迁移 | ✅ | 画布功能完整，桥接逻辑完整 |
| 工作树 | 🔴 | 有未提交改动 |
| Store 统一 | 🟡 | 两套并存，不是申报状态 |
| 死代码清理 | 🟢 | FlowEditor 等可删 |
| 本地执行 | 🟡 | 35 个 onExecute 全空（架构选择） |

**一句话**: 代码编译干净、测试全绿、LiteGraph 画布跑通了。但工作树有脏代码没 commit，Store 名义上统一了实际上两套并存。不是危机，是没收拾干净。
