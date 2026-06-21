# Reasonix 任务：workflow-engine 推进到 v8.2

## 项目位置
C:/Users/haozi/Dev/workflow-engine-desktop/

## 当前状态
v8.0 已交付：图执行引擎 + 节点元数据 + 变量校验 + 事件流
后端：Rust (axum)
前端：Vue 3 + Vite + TailwindCSS 4 + shadcn-vue + radix-vue
桌面壳：Tauri 2.10

## 任务概览
完成两阶段工作，从 v8.0 推到 v8.2：
- 阶段E：补全所有节点 type_def（v8.1）
- 阶段F：Canvas 图编辑器（v8.2）

---

# 阶段 E：补全节点 type_def（v8.1）

## 背景
rust/src-tauri/src/nodes/traits.rs 定义了 NodeTypeDef 结构：
```rust
pub struct NodeTypeDef {
    pub type_name: String,
    pub version: String,
    pub display_name: String,
    pub description: String,
    pub category: String,
    pub inputs: Vec<PortDef>,
    pub outputs: Vec<PortDef>,
    pub config_schema: serde_json::Value,
}
```

NodeExecutor trait 的 type_def() 有默认实现返回 "unknown"。

已实现的节点（有 type_def）：condition, data(5), delay, file(6), http, loop_node, script, shell

## 需要补全的节点（22个）
为以下每个节点的 impl NodeExecutor 块添加 type_def() 方法：

1. **approval.rs** (ApprovalNode) — 审批节点
2. **browser_container.rs** (BrowserContainerNode) — 浏览器容器
3. **clipboard.rs** (ClipboardNode) — 剪贴板
4. **cursor.rs** (CursorNode) — 游标/指针
5. **excel.rs** (ExcelNode) — Excel 操作
6. **excel_container.rs** (ExcelContainerNode) — Excel 容器
7. **file_container.rs** (FileContainerNode) — 文件容器
8. **map.rs** (MapNode) — 映射/迭代
9. **mcp_node.rs** (McpNode) — MCP 协议
10. **mouse_keyboard.rs** (MouseKeyboardNode) — 鼠标键盘(#[cfg(feature = "gui")])
11. **ocr.rs** (OcrNode) — OCR 识别
12. **parallel.rs** (ParallelNode) — 并行执行
13. **print.rs** (PrintNode) — 打印(#[cfg(feature = "gui")])
14. **regex.rs** (RegexNode) — 正则表达式
15. **registry.rs** (RegistryNode) — Windows 注册表
16. **sub_workflow.rs** (SubWorkflowNode) — 子工作流
17. **web_scrape.rs** (WebScrapeNode) — 网页抓取
18. **webbridge.rs** (WebBridgeNode) — 浏览器桥接
19. **while_node.rs** (WhileNode) — While 循环
20. **window.rs** (WindowNode) — 窗口操作(#[cfg(feature = "gui")])
21. **word.rs** (WordNode) — Word 文档
22. **word_container.rs** (WordContainerNode) — Word 容器

## type_def 实现要求
1. 阅读每个节点的 execute() 方法了解其 config 参数
2. type_name 使用节点的小写标识符（如 "approval", "browser_container"）
3. display_name 使用中文显示名
4. category 用中文分类（如"浏览器","文件","流程控制","数据","系统","AI","Office"）
5. inputs 定义输入端口（可为空数组）
6. outputs 定义输出端口
7. config_schema 用 JSON Schema 描述 config 字段（参考 http.rs 示例）

## 参考示例（http.rs）
```rust
fn type_def(&self) -> crate::nodes::traits::NodeTypeDef {
    crate::nodes::traits::NodeTypeDef {
        type_name: "http".into(),
        version: "1.0".into(),
        display_name: "HTTP 请求".into(),
        description: "发送 HTTP 请求，支持 GET/POST/PUT/DELETE 等方法".into(),
        category: "网络".into(),
        inputs: vec![],
        outputs: vec![
            PortDef { label: "body".into(), data_type: "object".into(), required: false },
            PortDef { label: "status".into(), data_type: "number".into(), required: false },
        ],
        config_schema: serde_json::json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "url": {"type": "string", "description": "请求 URL"},
                "method": {"type": "string", "enum": ["GET", "POST", "PUT", "DELETE"]},
            }
        }),
    }
}
```

## 验收标准
1. 编译通过：`cd src-tauri && cargo check 2>&1 | tail -5`
2. 每个新加的 type_def 有 display_name、description、category 不为空
3. config_schema 与 execute() 中实际读取的 config 字段一致

---

# 阶段 F：Canvas 图编辑器（v8.2）

## 目标
在 Editor.vue 的 visual/code 双视图基础上，添加 Canvas 图编辑模式，支持可视化连线。

## 现有编辑器结构
src/pages/Editor.vue：已有 Tabs 组件切换 visual（线性列表）/ code（YAML 编辑）
visual 视图使用 StepCard 组件渲染步骤卡片，已有拖拽重排

## 需要新建的文件

### 1. src/components/CanvasEditor.vue
Core Canvas 组件：
- **渲染区域**：使用 SVG 或绝对定位 div
- **节点卡片**：从步骤列表渲染为可拖拽卡片
- **贝塞尔连线**：节点输出端口到输入端口的 SVG 曲线
- **拖拽连线**：从输出端口拖到输入端口创建 Edge
- **执行高亮**：用 ExecutionEvent 实时高亮当前执行节点
- **拓扑分层**：嵌套/并行节点用虚线框分组

Props:
- workflow: Workflow (含 steps 和 edges)
- runStates: Record<string, RunState> (执行状态)

Emits:
- add-edge(source: string, target: string)
- remove-edge(id: string)
- update-node-position(id: string, x: number, y: number)

### 2. src/composables/useCanvas.ts
Canvas 状态管理 composable：
- nodes: 节点位置 (Map<string, {x, y}>)
- edges: 连线列表
- draggingEdge: 拖拽中的临时连线
- selectedNode: 选中节点
- zoom: 缩放级别
- pan: 平移偏移

### 3. src/components/CanvasNode.vue
单个 Canvas 节点卡片：
- 显示节点类型图标 + 名称
- 输入/输出端口圆点（可连线）
- 执行状态颜色（pending=灰, running=蓝, success=绿, error=红）
- 选中高亮边框
- 拖拽移动

### 4. src/components/CanvasEdge.vue
连线组件：
- SVG path 贝塞尔曲线
- 箭头标记
- 悬停高亮 + 删除按钮
- 执行中的数据流动画（可选）

## 修改现有文件

### Editor.vue 修改
1. 在 TabsList 中添加第三个 Tab：
```vue
<TabsTrigger value="canvas">Canvas</TabsTrigger>
```
2. 添加 TabsContent：
```vue
<TabsContent value="canvas" class="flex-1 overflow-hidden mt-0 p-0">
  <CanvasEditor
    :workflow="a.workflow.value"
    :run-states="a.store.runStates"
    @add-edge="a.store.addEdge"
    @remove-edge="a.store.removeEdge"
  />
</TabsContent>
```
3. 添加 i18n 键值 `canvas`

### workflowStore.ts 修改
添加 edges 管理和操作：
```ts
edges: ref<Edge[]>()
function addEdge(source: string, target: string)
function removeEdge(id: string)
```

### i18n 添加
zh-CN.ts 和 en-US.ts 中添加 `editor.canvas: '画布' / 'Canvas'`

## 用户交互
1. **添加连线**：从输出端口（节点右侧圆点）拖拽到输入端口（节点左侧圆点）
2. **移动节点**：拖拽节点卡片到任意位置
3. **删除连线**：点击连线的 X 按钮
4. **缩放平移**：滚轮缩放，按住空白区域拖拽平移
5. **执行可视化**：运行 workflow 时 Canvas 实时高亮当前节点

## 技术约束
- 使用 Vue 3 Composition API
- SVG 用于连线渲染（性能好，缩放不变形）
- 不引入额外的 Canvas/Grah 库（如 vue-flow），手写实现以保持轻量
- 兼容现有 StepCard 的数据模型
- 贝塞尔曲线控制点：源端口左侧偏移 → 目标端口右侧偏移

## 验收标准
1. Canvas 视图可渲染步骤为卡片
2. 卡片可拖拽移动
3. 端口间可拖拽连线
4. 连线有贝塞尔曲线样式
5. 执行时高亮当前节点
6. 三个 Tab（visual/canvas/code）可自由切换
7. Canvas 上的操作同步到 workflow 数据
8. vite build 无错误

---

# 执行顺序
先完成阶段E（type_def 补全），验证编译通过后，再开始阶段F（Canvas 编辑器）。
每完成一个阶段，提交一次 commit。

# 注意事项
- 不要修改已实现 type_def 的节点
- 不要改动 Cargo.toml 中的依赖版本
- 前端代码保持 Vue 3 + TypeScript + TailwindCSS 风格
- 使用 shadcn-vue 组件库中已有的组件
- 所有新代码通过 cargo check / vue-tsc 检查
