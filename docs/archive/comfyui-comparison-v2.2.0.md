# ComfyUI 最新版 vs Workflow Engine Desktop 全面对照报告

> 对照时间: 2026-05-02 | ComfyUI frontend: master 最新版 | 我们: v2.2.0

---

## 一、总体架构对照

| 维度 | ComfyUI | 我们 | 对齐度 |
|------|---------|------|:--:|
| **布局模式** | `position:absolute; pointer-events:none` overlay 覆盖 Grid 容器 | `position:fixed; inset:0` overlay 覆盖全视口 | ✅ 相同理念 |
| **Overlay 组件** | `LiteGraphCanvasSplitterOverlay` | 内联 `.ui-overlay` | ✅ 等价 |
| **Canvas 定位** | `absolute inset-0 size-full` 在 Grid cell 内 | `position:fixed; inset:0; z-index:0` 全视口 | ⚠️ 差异 |
| **Canvas 初始化** | `NaN` → `getBoundingClientRect` → DPI → `draw()` | 相同 | ✅ |
| **框架** | Vue 3 + Pinia + PrimeVue + Tailwind | Vue 3 + Pinia + Tailwind v4 | ✅ |
| **画布库** | @comfyorg/litegraph (自维护 fork) | @comfyorg/litegraph (npm) | ✅ |

**结论**: 布局理念一致（overlay + pointer-events 穿透），但实现方式略有不同。无需切换架构。

---

## 二、节点库 / 搜索机制 ⚠️ 关键差异

### ComfyUI 做法

ComfyUI **没有**永久可见的分类节点面板。节点添加走两条路径：

| 路径 | 触发方式 | 组件 |
|------|---------|------|
| **画布搜索弹窗** | 双击空白画布 | `NodeSearchboxPopover` |
| **左侧栏节点库 Tab** | 点击侧边栏图标 | `LeftSidePanel` 内嵌节点列表 |

搜索弹窗是主要方式——用户双击画布空白区 → 弹出浮动搜索框 → 输入关键词 → 点击节点名 → **直接**调用 `LiteGraph.createNode()` → `graph.add()` → 非 ghost 模式落位。

### 我们的做法

- 永久节点库 `FloatingPanel`，分类折叠列表
- 双击节点 → `LiteGraph.createNode()` → `graph.add(node, { ghost: true, dragEvent })`
- Ghost 模式：节点跟随鼠标 → 点击落位

### 差异分析

| 项目 | ComfyUI | 我们 | 建议 |
|------|---------|------|------|
| 搜索入口 | 双击画布弹出 | 侧边栏面板搜索框 | ⚠️ 应增加双击画布搜索 |
| 节点落位 | 非 ghost 模式（直接放到视图中心） | Ghost 跟随鼠标 | ✅ 我们更优 |
| 常驻面板 | 侧边栏 Tab 内（可选） | FloatingPanel（固定可见） | ✅ 均可 |

**需要对齐**: 增加双击画布空白区弹出节点搜索框（快速添加流程）。这是 ComfyUI 用户最常用的节点添加方式。

---

## 三、侧边栏 / SideToolbar ⚠️ 中等差异

### ComfyUI

```
SideToolbar (左侧图标栏)
├── 上组: ComfyMenuButton + 各 Tab 图标（node-library / model-library / workflows / assets）
├── 下组: HelpCenter / BottomPanelToggle / ShortcutsToggle / Settings
└── 点击 Tab → 左侧展开全高面板（LeftSidePanel 或 ExtensionSlot）
```

关键特性：
- 侧边栏图标点击 → 切换左侧面板 Tab
- 左侧面板是 `SplitterPanel`，可拖拽调整宽度
- 面板内容通过 `ExtensionSlot` 插件机制动态渲染
- 侧边栏有三种模式：connected / floating / small

### 我们

```
SideToolbar (左侧图标栏)
├── 上组: 📋 工作流列表 / 节点库 / 运行历史 / 定时计划
└── 下组: 控制台 / 设置
└── 点击 → 弹出独立 FloatingPanel（非侧边栏面板）
```

### 差异分析

| 项目 | ComfyUI | 我们 | 影响 |
|------|---------|------|------|
| 面板模式 | 侧边栏展开（SplitterPanel 可调宽） | 独立 FloatingPanel | 中 |
| 面板 z-index | 在 overlay splitter 内 | `<Teleport to="body">` | 低 |
| 工作流列表 | 侧边栏 Tab "workflows" | FloatingPanel | 低 |
| 节点库 | 侧边栏 Tab "node-library" | FloatingPanel | 低 |
| 插件机制 | ExtensionSlot | 无 | 低（我们不需要） |

**建议**: 当前 FloatingPanel 模式已可用。如需提升可用性，可考虑将高频面板（如工作流列表、节点库）改为侧边栏面板模式——但不是必须。

---

## 四、顶部菜单栏 TopMenu

### ComfyUI

```
WorkflowTabs (tab 切换)
├── 左侧：工作流标签页 + 脏标记
├── 右侧：ActionBarButtons / TopbarBadges / CurrentUser
└── 最右：+ 新建标签

菜单栏 (TopMenuSection):
├── 独立行（非 in-tab 集成）
├── Queue 按钮（Queue / Queue Front）
└── 在 body-top Grid zone 内
```

### 我们

```
TopMenuSection + WorkflowTabs 在同一 overlay-top 行
├── 左侧：工作流名 + 节点数 + 连线数 + 脏标记
├── 中：▶运行 ⏯单步 ■停止 💾保存
├── 右：录制 拾取 导入 导出 清空
```

### 差异分析

| 项目 | ComfyUI | 我们 | 建议 |
|------|---------|------|------|
| 标签切换 | 完整 workflowStore + 持久化 | tabDataCache Map | ✅ 足够 |
| 队列控制 | Queue/QueueFront/历史 | 运行/单步/停止 | ✅ 对齐 |
| 保存按钮 | 集成在文件菜单 | 独立按钮 | ✅ |
| 新建按钮 | 标签栏最右 + 图标 | + 新建 Tab | ✅ |

**结论**: 基本对齐，无缺失。

---

## 五、右侧面板 / PropertyPanel ⚠️ 差异

### ComfyUI

```
RightSidePanel (NodePropertiesPanel)
├── 在 LiteGraphCanvasSplitterOverlay 的 right-side-panel slot
├── SplitterPanel（可拖拽宽度）
├── TabList: Info / Parameters / Settings / Errors / Subgraph
├── 多节点选中 → 批量属性编辑
└── 始终可见（选中节点时）
```

### 我们

```
PropertyPanel (FloatingPanel)
├── 选中节点时弹出
├── widget 驱动的泛型表单
├── 针脚信息展示
└── 输出预览 / 错误信息 / 执行元数据
```

### 差异分析

| 项目 | ComfyUI | 我们 | 优先级 |
|------|---------|------|:--:|
| 定位 | 右侧 SplitterPanel | FloatingPanel 浮动 | 低 |
| 多 Tab | Info / Parameters / Settings / Errors / Subgraph | 单面板平铺 | 中 |
| 多选编辑 | ✅ 支持 | ❌ 只支持单选 | 中 |
| Widget 渲染 | 自定义 Vue widget 组件 | 泛型 toRaw() 读取 | 低 |

**建议**:
1. **中优先级**: 多节点选中后批量编辑属性
2. **低优先级**: 改为右侧 SplitterPanel（可选但不紧急）

---

## 六、底部面板

### ComfyUI

```
BottomPanel (在 splitter-overlay-bottom 内)
├── SplitterPanel 垂直 splitter
├── 多个 Tab（queue history / 等）
└── z-index: 1000
```

### 我们

```
Console (在 overlay-bottom 内)
├── 执行日志行
├── 清除按钮
└── v-if="showConsole && logs.length > 0"
```

### 差异分析

| 项目 | ComfyUI | 我们 |
|------|---------|------|
| 内容 | Queue History + 多 Tab | 执行日志 |
| 交互 | 可拖拽高度 (Splitter) | 固定 140px |
| 可见性 | 始终有 toggle | v-if 有日志才显示 |

**建议**: 控制台功能足够，暂不需对齐。

---

## 七、缺失功能清单（ComfyUI 有，我们没有）

| 功能 | ComfyUI 实现 | 重要性 | 建议 |
|------|------------|:--:|------|
| **MiniMap** | `MiniMap.vue` 右下角小地图 | 🔴 高 | P1 优先实现 |
| **右键菜单** | `NodeContextMenu.vue` + `GraphCanvasMenu.vue` | 🔴 高 | P1 优先实现 |
| **画布搜索弹窗** | `NodeSearchboxPopover` 双击空白画布 | 🟡 中 | 增加快速搜索 |
| **多节点选中编辑** | 右侧面板批量属性 | 🟡 中 | 后续迭代 |
| **选择矩形** | `SelectionRectangle` 拖拽框选 | 🟢 低 | LiteGraph 内置 |
| **节点工具箱** | `SelectionToolbox` 选中后浮动工具栏 | 🟢 低 | 后续迭代 |
| **标题编辑器** | `TitleEditor` 双击标题改名 | 🟢 低 | 已有 PropertyPanel |
| **FPS 显示** | `fpsInfoLocation` 设置 | 🟢 低 | 不需要 |
| **组节点** | `GroupNode` / 子图 | 🟢 低 | 已有 sub_workflow |
| **线性模式** | LinearView / LinearArrange | 🟢 低 | 不需要 |
| **Vue Node 渲染** | `LGraphNode.vue` DOM 层渲染节点 | 🟢 低 | 我们用原生 LiteGraph |

---

## 八、Canvas 初始化检查 ✅ 已对齐

对照 `app.ts:964-975` 与我们 `initCanvasReal()`：

```typescript
// ComfyUI app.ts:964
resizeCanvas(canvas: HTMLCanvasElement) {
  const scale = Math.max(window.devicePixelRatio, 1)
  canvas.height = canvas.width = NaN           // ✅ 我们: c.width = c.height = NaN
  const { width, height } = canvas.getBoundingClientRect()  // ✅ 相同
  canvas.width = Math.round(width * scale)     // ✅ 相同
  canvas.height = Math.round(height * scale)   // ✅ 相同
  canvas.getContext('2d')?.scale(scale, scale) // ✅ 相同
  this.canvas?.draw(true, true)                // ✅ 我们: canvas.draw(true, true)
}
```

**完全对齐** ✅。

---

## 九、LiteGraph API 差异对照 ✅ 已处理

| API | 原始 LiteGraph | ComfyUI fork | 我们状态 |
|-----|:--:|:--:|:--:|
| `graph.add()` 事件 | `onAfterChange` | `on_change` | ✅ 双监听 |
| `graph._links` | `LLink[]` | `Map<LinkId, LLink>` | ✅ `[...values()]` |
| Widget 字段 | 普通对象 | ES2022 `#private` | ✅ `toRaw()` |
| `DragAndScale` 坐标 | `{x,y}` | `[x,y]` | ✅ 数组格式 |
| `canvas.resize()` | 内置 | 不用，手动画 ResizeObserver | ✅ |
| `autoresize` | `true` | 默认 `false` | ✅ 手动画 |
| `addWidget` 警告 | 无 | 需 `property` 参数 | ✅ |

---

## 十、修复优先级清单

### 🔴 P0 — 立即修复

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 1 | MiniMap 缺失 | `LiteGraphEditor.vue` | 新增 `MiniMap.vue`，参考 ComfyUI `MiniMap.vue` |
| 2 | 右键菜单缺失 | `LiteGraphEditor.vue` | 监听 canvas `contextmenu` 事件 + 弹出菜单组件 |
| 3 | 双击画布不弹搜索 | `LiteGraphEditor.vue` | `dblclick` 空白区 → 打开节点搜索弹窗 |

### 🟡 P1 — 后续迭代

| # | 问题 | 修复 |
|---|------|------|
| 4 | 多节点选中无批量编辑 | PropertyPanel 支持多选节点 |
| 5 | 侧边栏改 SplitterPanel 模式 | SideToolbar + 侧边扩展面板（可选） |
| 6 | 空画布提示文案更新 | "双击空白处搜索节点" |
| 7 | 控制台增加 Tab (日志/历史/错误) | 底部面板改 Tabs |

### 🟢 P2 — 低优先级

| # | 问题 |
|---|------|
| 8 | 节点工具箱 (SelectionToolbox) |
| 9 | 标题编辑器 (双击节点标题) |
| 10 | 选择矩形框选 |

---

## 十一、总结

| 层级 | 对齐度 | 说明 |
|------|:--:|------|
| 布局架构 | 90% | overlay + pointer-events 穿透理念一致，实现方式等价 |
| Canvas 渲染 | 98% | 初始化、DPI、ResizeObserver、网格背景完全对齐 |
| LiteGraph API | 95% | 所有 ComfyUI fork 差异已处理 |
| 节点库 | 75% | 缺双击画布搜索、缺按频率排序 |
| 属性面板 | 70% | 差多选编辑、多 Tab |
| 功能完整性 | 55% | 缺 MiniMap、右键菜单、搜索弹窗 |

**总体**: 核心画布已高度对齐 ComfyUI。P0 三缺项（MiniMap / 右键菜单 / 画布搜索）补齐后即可达到 80%+ 对齐度。
