# DAG 执行引擎 — ComfyUI 对照审计

> 2026-06-22 | 对照 ComfyUI execution.py + comfy_execution/graph.py 逐环节比对

## 总览：执行管线对照

| # | 阶段 | ComfyUI | 我们 | 状态 |
|---|------|---------|------|------|
| 1 | 输入解析 | Prompt JSON → DynamicPrompt | JSON/YAML → parse_workflow | ✅ 完整 |
| 2 | 环检测 | DFS visiting 栈 + Kahn 逆 | DFS visiting 栈 | ✅ 完整 |
| 3 | 拓扑排序 | 增量 blockCount | 增量 blockCount | ✅ 完整 |
| 4 | 就绪调度 | ux_friendly_pick_node | FIFO VecDeque | 🟡 可优化 |
| 5 | 条件分支 | lazy eval + ExecutionBlocker | fromPort 匹配 + propagate_blocked | ✅ 完整 |
| 6 | 数据传递 | edge → cache → input_slots | **step_outputs + 模板字符串** | 🟡 路径不同 |
| 7 | 输入端口注入 | get_input_data() 自动注入 | **input_ports 未被调度器填充** | 🔴 缺失 |
| 8 | 并行执行 | asyncio + externalBlock | tokio::JoinSet | ✅ 完整 |
| 9 | 输出缓存 | HierarchicalCache + LRU | 无缓存（每次重跑） | 🟡 可优化 |
| 10 | 增量执行 | IS_CHANGED + 缓存命中跳过 | 无 | ⚪ 未实现 |
| 11 | 类型校验 | isValidConnection() | 无 | ⚪ 未实现 |
| 12 | 子图展开 | DynamicPrompt.add_ephemeral_node | 无 | ⚪ 未实现 |
| 13 | 错误策略 | 级联传播 | Fail/Ignore/Branch | ✅ 完整 |
| 14 | 调试支持 | 无内置 | 断点/单步/暂停 | ✅ 超越 |
| 15 | 运行时死锁 | Kahn 逆（剩余=环） | 无就绪+未完成=死锁 | ✅ 完整 |

## 🔴 必须修复（影响正确性）

### Gap 1: input_ports 未被 DAG 调度器注入

**问题**：42 个节点实现读取 `ctx.input_ports`（browser_container、excel_container、word_container、llm_chat、json_transform 等），但 `run_dag_workflow` 只设置 `step_outputs` 和 `variables`，不填充 `input_ports`。

**ComfyUI 做法**：`get_input_data()` 遍历节点的所有输入端口，如果是 link（`[source_node_id, output_slot_index]`），从 cache 读取上游输出，注入到节点的输入参数中。

**我们的现状**：
```rust
// scheduler.rs run_dag_workflow 中
let mut task_ctx = ExecutionContext::new(run_id, workflow);
task_ctx.variables = ctx.variables.clone();
task_ctx.step_outputs = ctx.step_outputs.clone();
// ❌ task_ctx.input_ports 为空！
```

**修复方案**：在 spawn 并行任务前，根据 edges 构建 input_ports：
```rust
// 从 edges 找到指向当前节点的所有入边
for edge in &workflow.edges {
    if edge.to == node_id {
        // 从 step_outputs 读取上游输出
        if let Some(upstream_output) = ctx.step_outputs.get(&edge.from) {
            // 根据 to_port 注入到 input_ports
            task_ctx.input_ports.insert(edge.to_port.clone(), upstream_output.clone());
        }
    }
}
```

**影响**：浏览器容器、Excel 容器、Word 容器等通过连线传入的数据在 DAG 模式下不工作。

### Gap 2: DAG 模式下变量引用路径不同

**问题**：ComfyUI 的数据传递是 **edge → cache → input_slots**（端口驱动），我们用的是 **step_outputs + 模板字符串**（引用驱动）。

**ComfyUI**：节点 A 的 output slot 0 → edge → 节点 B 的 input slot "data"。数据自动流入，节点直接读取。

**我们**：节点 A 输出 → `ctx.step_outputs["a"]` → 节点 B 用 `{{a.field}}` 模板引用。这是 pull 模式（下游主动拉取），不是 push 模式（上游主动推送）。

**评估**：两种模式都能工作，但 pull 模式在 DAG 并行时有隐含问题——如果 B 和 C 并行执行，B 引用 `{{a.field}}` 没问题（a 已完成），但如果 B 引用 `{{c.field}}`（c 还在并行执行中），会拿到 null。

**当前状态**：由于 blockCount 保证上游完成后下游才执行，pull 模式在串行/同层并行下是安全的。但跨层并行引用不安全。

**建议**：保持 pull 模式（模板字符串），但在 DAG 调度器中同时注入 input_ports 作为冗余数据通道。

## 🟡 可优化（不影响正确性，提升体验）

### Gap 3: 无缓存 / 增量执行

**ComfyUI**：`HierarchicalCache` + `IS_CHANGED` 方法。节点输入不变时跳过执行，直接用缓存输出。

**我们**：每次运行全量执行所有节点。

**评估**：对工作流自动化场景，缓存优先级低。用户通常期望每次运行都执行完整流程。但如果未来做"编辑后重跑"功能，缓存有价值。

### Gap 4: 无 UX 调度优化

**ComfyUI**：`ux_friendly_pick_node()` 优先选择输出节点、异步节点、距离输出最近的节点。

**我们**：FIFO VecDeque，先入先出。

**评估**：对串行执行无影响。对并行执行，UX 优化让用户更快看到输出节点的结果。低优先级。

### Gap 5: 无类型校验

**ComfyUI**：`isValidConnection()` 检查端口类型兼容性，`*` 通配符，逗号分隔多类型。

**我们**：连线不做类型检查，任意端口都可以连。

**评估**：对自动化工作流，类型错误通常在运行时报错。前端加类型校验可以提前拦截，但需要先定义节点端口类型系统（node-schema.json 已有 `inputs`/`outputs` 但未用于连线校验）。

## ⚪ 未实现（未来可选）

### Gap 6: 无子图展开

ComfyUI 支持节点在运行时返回新图，动态扩展 DAG。我们没有这个需求。

### Gap 7: 无 Lazy Evaluation

ComfyUI 的 `check_lazy_status()` 让节点声明哪些输入真正需要，不需要的分支不执行。我们的条件分支通过 `fromPort` 过滤实现类似效果，但粒度更粗。

## 修复优先级

| # | Gap | 影响 | 工作量 | 建议 |
|---|-----|------|--------|------|
| 1 | **input_ports 注入** | 🔴 容器节点数据断流 | 🟡 中 | **立即修** |
| 2 | 变量引用路径 | 🟡 跨层并行可能 null | 🟢 低 | 保持现状+文档说明 |
| 3 | 缓存/增量执行 | ⚪ 无 | 🔴 高 | Phase 4+ |
| 4 | UX 调度优化 | ⚪ 无 | 🟢 低 | 有空再做 |
| 5 | 类型校验 | ⚪ 无 | 🟡 中 | Phase 3 |
| 6 | 子图展开 | ⚪ 无 | 🔴 高 | 不做 |
| 7 | Lazy eval | ⚪ 无 | 🟡 中 | 不做（fromPort 已覆盖） |
