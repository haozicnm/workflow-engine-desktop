# PropertyPanel 重构方案：复用 LiteGraph Widget 系统

> 2026-05-01 | 目标：删掉自研表单逻辑，直接读 LGraphNode.widgets 自动渲染

## 现状问题

PropertyPanel 维护了三套重复信息，新增节点要改三个地方：

| 文件 | 内容 | 行数 |
|------|------|:---:|
| `litegraph-nodes.ts` | `this.addWidget('combo', 'method', ...)` | ~950 |
| `pinTypes.ts` | NODE_REGISTRY + SELECT_FIELDS + PLACEHOLDER_MAP | ~300 |
| `PropertyPanel.vue` | 5 个 `v-else-if` 硬编码节点表单 | 200 |

拖慢节奏，维护成本高。

## LiteGraph Widget 系统已有的能力

`LGraphNode.widgets: IBaseWidget[]`，每个 widget 自带：

```
{ name, type, value, options: { values, min, max, step2, multiline... }, label?, tooltip? }
```

7 种类型：`combo` `string` `number` `toggle` `text` `slider` `button`

## 改法

PropertyPanel 从「读 FlowNode.config → 硬编码表单」改为「读 LGraphNode.widgets → 泛型渲染」：

```
widget.type = "combo"   → <select>           （options.values 驱动选项）
widget.type = "string"  → <input type="text"> （placeholder 从 name 自动生成）
widget.type = "number"  → <input type="number">（options.min/max/step2）
widget.type = "toggle"  → <input type="checkbox">
widget.type = "text"    → <textarea>          （options.multiline）
widget.type = "slider"  → <input type="range">
```

**传参变化**：PropertyPanel 接收 `LGraphNode | null`（不再是 `FlowNode | null`）

**双向同步**：widget.value 变更 → 直接写 widget.value → LiteGraph 自动 setDirtyCanvas → syncGraphToStore() 同步 Pinia

## 效果

- PropertyPanel: 694 行 → ~350 行（删 5 个硬编码 `v-else-if` 块 + SELECT_FIELDS/PLACEHOLDER_MAP）
- `pinTypes.ts`: 参数表单逻辑可删（NODE_REGISTRY 保留给 NodePalette 用图标/分类）
- 新增节点：**只改 litegraph-nodes.ts**，PropertyPanel 自动渲染
- 单测好写：泛型 widget 渲染器比硬编码表单好测得多

## 风险

- LiteGraph 节点在画布上 widget 变更不会自动触发 Pinia dirty 标记（目前靠 syncGraphToStore 手动调）
- 嵌套 widget（如 while 的 condition.check）需要特殊处理——LiteGraph 不支持嵌套 widget，这些复杂参数需要额外手段

## 扩展：pinTypes.ts 还能删什么

| 当前内容 | 重构后 |
|---------|--------|
| NODE_REGISTRY（节点图标/分类/中文名） | **保留**（NodePalette 需要） |
| SELECT_FIELDS | **删**（combo widget 自带 options.values） |
| PLACEHOLDER_MAP | **删**（widget name 自动生成 placeholder） |
| NodeDefinition.config_keys | **删**（LGraphNode.widgets 替代） |
| NodeDefinition.inputs/outputs | **删**（LGraphNode.inputs/outputs 替代） |
| pinColor / pinBadge | **保留**（针脚颜色显示仍需） |
