# v4.0 容器数据模型

## Container Node Schema

```json
{
  "id": "n1",
  "type": "browser_container",
  "label": "浏览器操作",
  "config": {
    "browser": "chromium"
  },
  "actions": [
    { "id": "a1", "type": "navigate", "label": "导航", "config": { "url": "https://..." } },
    { "id": "a2", "type": "wait",    "label": "等待", "config": { "selector": "#form" } },
    { "id": "a3", "type": "input",   "label": "输入", "config": { "selector": "#name" } },
    { "id": "a4", "type": "click",   "label": "点击", "config": { "selector": "#submit" } },
    { "id": "a5", "type": "extract", "label": "提取", "config": { "selector": ".result" } }
  ]
}
```

## 端口推论规则

| Action type | 输入口 | 输出口 | 说明 |
|------------|--------|--------|------|
| navigate | - | - | URL 固定写死 |
| wait | - | - | 无数据进出 |
| click | - | - | 无数据进出 |
| scroll | - | - | 无数据进出 |
| **input/fill** | **`值`** (string) | - | 填入值来自其他容器 |
| **extract** | selector (string) | **`数据`** (array/object) | 提取结果输出 |
| **screenshot** | - | **`截图`** (image) | 截图输出 |
| **evaluate** | - | **`结果`** (any) | JS 返回值输出 |
| **get_title** | - | **`标题`** (string) | 页面标题输出 |
| **pdf** | - | **`PDF`** (file) | PDF 输出 |

**规则**: 消费外部数据的 action → 1 输入口，产生数据的 action → 1 输出口。端口名 = action.label 或 action.id。

## UI 交互流

```
Canvas 上:
┌──────────────┐
│ 浏览器       │
│ [导航: xxx]  │  ← action 层叠显示在节点内
│ [等待: xx]   │
│ [输入: xx]   │
│ [提取: xx]   │
│  [+]          │  ← 点击弹出动作菜单
│ ○ 输入1      │  ← 动态端口 (input action 产生)
│ ○ 提取数据  │  ← 动态端口 (extract action 产生)
└──────────────┘
```

## DAG 执行流

```
Excel容器(读A1) ──[单元格值]──→ Browser容器(输入.值) ──[提取数据]──→ Word容器(填入)
```

执行顺序按连线拓扑排序，数据通过端口名映射传递。
