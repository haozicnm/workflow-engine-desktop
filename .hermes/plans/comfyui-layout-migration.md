# ComfyUI 布局对齐改造方案

> 2026-05-02 | 分析依据: Comfy-Org/ComfyUI_frontend main 分支

## 现状 vs 目标

```
【我们现在】                          【ComfyUI 目标】
┌────────────────────────────┐       ┌──┬──────────────────────┬──┐
│  工具栏 (横排按钮)          │       │侧│  顶部菜单栏            │  │
├──────┬──────────────┬──────┤       │边│  (面包屑+运行按钮)      │右│
│节点库│  Canvas 画布  │属性  │  →    │栏│──────────────────────│侧│
│      │              │面板  │       │图│                      │面│
│      │              │      │       │标│    Canvas 画布        │板│
│      │              │      │       │列│                      │  │
│      │              │      │       │  │                      │  │
├──────┴──────────────┴──────┤       │  ├──────────────────────┤  │
│  底部栏 (日志/状态)         │       │  │  底部面板 (日志/控制台) │  │
└────────────────────────────┘       └──┴──────────────────────┴──┘
```

## ComfyUI 布局核心要素

### 1. 顶级 CSS Grid 容器
```
grid-template-columns: auto 1fr auto
grid-template-rows: auto 1fr auto
```
- 左列: 侧边图标栏 (48px)
- 中列: 画布 (1fr)
- 右列: 属性面板 (可折叠)
- 顶行: 菜单栏
- 底行: 底部面板

### 2. 侧边栏 (SideToolbar)
垂直图标列，分上下两组:
- **上组**: 品牌 Logo → 各功能 Tab 图标 → 模板按钮
- **下组**: 帮助 → 底部面板切换 → 快捷键 → 设置

### 3. 面板系统
- **左侧面板**: 节点库 (从现在的顶部附着改为侧边弹出)
- **右侧面板**: 属性面板 + 预览面板 (现有，保留)
- **底部面板**: 日志/控制台/执行历史
- 全部用 Splitter 可拖拽调整大小

### 4. 菜单系统
- 侧边栏 Comfy Logo → 多层弹出菜单 (设置、主题、模板、帮助)
- 顶部栏: 运行/停止按钮 + 工作流名称面包屑
- 画布右下角浮动: 缩放控件 + 适应画布 + 小地图

---

## 改造步骤

### 第一步: 布局外壳 (LayoutDefault)
- 新建 `LayoutDefault.vue` 替代现在的 `App.vue` 中的裸 `<RouterView>`
- CSS Grid 三列两行
- 注入点: `#app-body-top` `#app-body-left` `#app-body-right` `#app-body-bottom`
- `<RouterView>` 包含 `<KeepAlive>` 放在中央画布区域

### 第二步: 侧边图标栏 (SideToolbar)
- 新建 `SideToolbar.vue` — 48px 宽垂直图标列
- 顶部组图标: 节点库 / 模板 / 工作流列表
- 底部组图标: 日志 / 设置 / 帮助
- 点击图标 → 切换对应面板开启/关闭

### 第三步: 顶部菜单栏 (TopMenuSection)
- 从现有 toolbar 中提取: 运行/停止按钮、工作流名称
- 移到顶部栏 `#app-body-top`
- 删除原有 toolbar

### 第四步: 画布容器 + Splitter 面板
- 画布区域用 PrimeVue Splitter 或简易 flex 分栏
- 左面板: 节点库 (可折叠)
- 中面板: Canvas
- 右面板: 属性 + 预览 (上下分)
- 底部面板: 日志/控制台

### 第五步: 菜单/设置重组
- Comfy Logo → 弹出菜单: 设置/主题/帮助/关于
- 右下角浮动按钮: 缩放 / 适应画布

---

## 改动清单

| 文件 | 操作 | 说明 |
|------|------|------|
| `App.vue` | 重写 | CSS Grid 外壳 + 注入容器 |
| `SideToolbar.vue` | 新建 | 侧边图标栏 |
| `TopMenuSection.vue` | 新建 | 顶部运行栏 |
| `LiteGraphEditor.vue` | 重写 | 去掉旧 toolbar/footer，只留 canvas+palette+panels |
| `NodePalette.vue` | 改造 | 从顶部拖放面板改为侧边面板 |
| `PropertyPanel.vue` | 保留 | 基本不用改 |
| `PreviewPanel.vue` | 保留 | 基本不用改 |

## 不引入新依赖
- ComfyUI 用 PrimeVue Splitter，我们不用装——用 CSS flex + 手写拖拽分隔线即可
- 其余全部用现有技术栈 (Vue 3 + Tailwind CSS + LiteGraph)

## 工作量预估
- 布局外壳: 1 个文件新建 + 1 个重写
- 侧边栏: 1 个文件新建
- 顶部栏: 1 个文件新建  
- 画布改造: 去掉旧 toolbar/footer 代码
- 总计约 4-5 个文件的增删改
