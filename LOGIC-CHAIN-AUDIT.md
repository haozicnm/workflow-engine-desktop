# Workflow Engine v9.0.1 逻辑链条完整性审查报告

**审查日期：** 2026-06-21  
**审查维度：** 工作流执行逻辑链条 + 变量传递路径完整性  
**审查方法：** 代码走读（executor.rs, scheduler.rs, context.rs, workflow.rs + 12 个节点文件）

---

## 一、总体结论

**存在两条独立的图执行路径，行为不一致，导致 CLI 和 GUI 的 DAG 执行结果可能不同。** 这是本报告发现的最严重问题。

| 路径 | 入口 | DAG 执行方式 | input_ports 注入 | 重试/超时 | 风险 |
|------|------|-------------|------------------|-----------|------|
| **A** | `executor.rs::run_graph` | 拓扑分层 + `run_parallel_level` | ❌ **无** | ❌ 无 | CLI 执行时 input_ports 为空 |
| **B** | `scheduler.rs::run_dag_workflow` | `DagScheduler` blockCount 调度 | ✅ **有** | ✅ 有 | GUI 执行正确 |

---

## 二、🔴 严重问题：两条图执行路径不一致

### 2.1 路径 A：`executor.rs::run_graph`（CLI 入口）

**调用链：**
```
wf-cli run-file → StepExecutor::run_workflow → run_workflow_with_events → run_graph
```

**关键代码位置：** `executor.rs:353-401`

```rust
async fn run_graph(self: &Arc<Self>, workflow: &Workflow, ctx: &mut ExecutionContext) -> Result<()> {
    self.validate_variable_references(workflow)?;
    let levels = self.topological_levels(&workflow.steps, &workflow.edges)?;
    for level in &levels {
        if level.len() == 1 {
            // 单节点层级：直接使用主 ctx
            let step = ...;
            match Arc::clone(self).execute(step, ctx).await {
                Ok(result) => ctx.set_output(&step.id, result),
                Err(e) => { /* on_error 处理 */ }
            }
        } else {
            // 并行层级：创建新 task_ctx
            self.run_parallel_level(level, workflow, ctx).await?;
        }
    }
    Ok(())
}
```

**问题分析：**

1. **单节点层级**（第 364-395 行）：直接使用主 `ctx` 执行。由于主 `ctx` 的 `step_outputs` 包含所有已执行节点输出，`resolve_var` 可以正确引用。✅ **正常**

2. **并行层级**（第 396-398 行）：调用 `run_parallel_level`，其中创建新 `task_ctx`：

```rust
// executor.rs:404-416
async fn run_parallel_level(...) {
    let mut join_set = tokio::task::JoinSet::new();
    for node_id in node_ids {
        let step = ...;
        let mut task_ctx = ExecutionContext::new(&step.id, workflow);
        task_ctx.variables = ctx.variables.clone();  // ✅ 只克隆 variables
        // ❌ 没有克隆 step_outputs
        // ❌ 没有遍历 edges 注入 input_ports
        let exec = Arc::clone(self);
        join_set.spawn(async move {
            let result = exec.execute(&step, &mut task_ctx).await;
            (step.id.clone(), result, task_ctx.variables, step.on_error.clone())
        });
    }
    // ... 合并结果
}
```

**缺失的关键行：**
```rust
// 应该添加：
task_ctx.step_outputs = ctx.step_outputs.clone();  // 传递上游输出

// 以及 input_ports 注入（见 scheduler.rs 第 912-923 行）：
for edge in &workflow.edges {
    if edge.to == *node_id {
        if let Some(upstream_output) = ctx.step_outputs.get(&edge.from) {
            task_ctx.input_ports.insert(
                edge.to_port.clone(),
                upstream_output.clone(),
            );
        }
    }
}
```

**影响：** 如果并行层级中的节点需要引用之前层级的节点输出（通过 `{{step_id}}` 或 `input_ports`），会找不到数据，导致：
- `resolve_var` 返回 `None` → 模板替换为 `Null` 或保留原文
- `input_ports` 为空 → 节点（如 `llm_chat`, `data_filter`）取不到上游数据

### 2.2 路径 B：`scheduler.rs::run_dag_workflow`（GUI 入口）

**调用链：**
```
Dashboard → 运行按钮 → scheduler::run_workflow → run_dag_workflow
```

**关键代码位置：** `scheduler.rs:801-1036`

```rust
async fn run_dag_workflow(...) {
    let mut dag = DagScheduler::new(&workflow.steps, &workflow.edges);
    while !dag.is_done() {
        let ready_batch = dag.pop_ready_batch();
        for node_id in &ready_batch {
            let step = ...;
            let mut task_ctx = ExecutionContext::new(run_id, workflow);
            task_ctx.variables = ctx.variables.clone();
            task_ctx.step_outputs = ctx.step_outputs.clone();  // ✅ 正确传递
            
            // ✅ 注入 input_ports
            for edge in &workflow.edges {
                if edge.to == *node_id {
                    if let Some(upstream_output) = ctx.step_outputs.get(&edge.from) {
                        task_ctx.input_ports.insert(
                            edge.to_port.clone(),
                            upstream_output.clone(),
                        );
                    }
                }
            }
            
            join_set.spawn(async move {
                execute_with_retry(&exec, &step, &mut task_ctx).await
            });
        }
        // 合并结果...
    }
}
```

**GUI 路径正确实现了：**
1. ✅ `step_outputs` 传递到并行 `task_ctx`
2. ✅ `input_ports` 从 edges 注入（按 `to_port` 映射）
3. ✅ 重试和超时（`execute_with_retry`）
4. ✅ 条件分支阻断（`DagScheduler::complete_node` 按 `from_port` 匹配）

### 2.3 对比总结

| 功能 | CLI 路径 (executor.rs) | GUI 路径 (scheduler.rs) | 严重程度 |
|------|------------------------|------------------------|----------|
| `step_outputs` 传递 | ❌ 空 | ✅ 完整克隆 | 🔴 高 |
| `input_ports` 注入 | ❌ 无 | ✅ 按 edge 注入 | 🔴 高 |
| 重试/超时 | ❌ 无 | ✅ `execute_with_retry` | 🟡 中 |
| 条件分支阻断 | ✅ 拓扑层级 | ✅ 精确 `from_port` 匹配 | 🟢 一致 |
| 并行层级 `on_error: Branch` | ⚠️ 只记录 | ✅ 调度器自然执行 | 🟡 中 |

### 2.4 修复建议

**方案一：统一入口（推荐）**

将 CLI 调用从 `executor.rs::run_graph` 改为 `scheduler.rs::run_workflow`。这样只需维护一套图执行逻辑。在 `cli.rs` 中调整调用链，传入 `approval_store`, `db`, `ctrl` 等参数。

**方案二：对齐 `run_parallel_level`**

在 `executor.rs:run_parallel_level` 中添加缺失的 `step_outputs` 克隆和 `input_ports` 注入逻辑。

**方案三：两者都做**

`executor.rs` 的 `run_graph` 对齐 `scheduler.rs` 的 `run_dag_workflow`，同时保持 CLI 可以独立运行。

---

## 三、🔴 严重问题：`run_parallel_level` 中 `on_error: Branch` 不执行分支

### 3.1 单节点层级（executor.rs）— 正确

```rust
// executor.rs:380-387
Some(ErrorStrategy::Branch { step_id }) => {
    if let Some(branch_step) = workflow.steps.iter().find(|s| s.id == *step_id) {
        let branch_result = Arc::clone(self).execute(branch_step, ctx).await?;
        ctx.set_output(&branch_step.id, branch_result);
    }
}
```

✅ **正确**：直接执行分支节点。

### 3.2 并行层级（executor.rs）— 只记录不执行

```rust
// executor.rs:435-438
ErrorStrategy::Branch { step_id } => {
    warn!("并行节点 {} 错误分支 → {}", nid, step_id);
    // 错误分支目标节点将在下轮调度中执行
}
```

❌ **严重**：只记录日志，没有实际执行分支节点。由于 `executor.rs` 的 `run_graph` 使用拓扑分层而非调度器，"下轮调度"实际上不会发生——分支节点永远不会被执行。

**影响：** 如果并行层级中的某个节点配置了 `on_error: Branch`，且执行失败，用户期望跳转到分支节点处理错误，但实际上分支节点被静默忽略，工作流继续执行或失败。

**修复：** 在 `run_parallel_level` 的 `Branch` 分支中直接执行分支节点：
```rust
ErrorStrategy::Branch { step_id } => {
    if let Some(branch_step) = workflow.steps.iter().find(|s| s.id == *step_id) {
        let branch_result = Arc::clone(self).execute(branch_step, ctx).await;
        match branch_result {
            Ok(val) => ctx.set_output(&step_id, val),
            Err(e) => hard_errors.push((step_id.clone(), e)),
        }
    }
}
```

---

## 四、🟡 中度问题：变量传递路径不一致

### 4.1 `input_ports` 读取方式不一致

| 节点 | 读取方式 | 是否按 label | 问题 |
|------|----------|-------------|------|
| `llm_chat` | `ctx.input_ports.get("messages")` / `get("prompt")` | ✅ | 正确按 label |
| `data_filter` | `ctx.input_ports.values().next()` | ❌ | 取第一个，不稳健 |
| `im_message` | `ctx.input_ports.values().next()` | ❌ | 取第一个，不稳健 |

**影响：** 如果节点有多个输入端口，`data_filter` 可能会取错数据。当前大多数节点只有一个输入端口，但这不应该是依赖假设。

**修复：** 统一为 `ctx.input_ports.get("label_name")` 按 label 获取。

### 4.2 `validate_variable_references` 的 `root_clean` 处理

```rust
// executor.rs:555
let root_clean = root.strip_prefix("step_").unwrap_or(root);
```

如果用户写 `{{step_1.result}}`：
- root = "step_1"
- root_clean = "1"

验证逻辑检查 `node_ids.contains(root)`（"step_1"）和 `node_ids.contains(root_clean)`（"1"）。如果 node_id 是 "step_1"，两者之一命中，验证通过。但如果 node_id 是 "1"，而用户写 `{{step_1.result}}`，验证失败。

**评估：** 这是一个设计选择——鼓励用户使用完整 node ID，但可能导致误报。不过不会导致运行时错误（因为 `resolve_var` 的双重查找逻辑更稳健）。

### 4.3 `resolve_var` 的双重查找逻辑

```rust
// context.rs:278-295
if let Some(step_id) = root_key.strip_prefix("step_") {
    // 1. 完整 key 查 step_outputs（如 step_1）
    if let Some(root) = self.step_outputs.get(root_key) { ... }
    // 2. strip 后的 key 查 step_outputs（如 "1"）
    if let Some(root) = self.step_outputs.get(step_id) { ... }
}
```

`step_outputs` 的 key 是 `step_id` 原始值（通过 `set_output(step_id, ...)` 存入）。如果 step_id 是 "step_1"，那么 key 就是 "step_1"。`resolve_var` 先查完整 key，再查 strip 后的 key，逻辑是安全的。

但 `run_parallel_level` 中 `task_ctx.step_outputs` 是空的，所以这部分查找在并行层级中无效。

### 4.4 并行层级变量覆盖 Race Condition

```rust
// executor.rs:421-425
Ok((nid, Ok(val), vars, _)) => {
    ctx.set_output(&nid, val);
    for (k, v) in vars {
        ctx.variables.insert(k, v);  // 直接覆盖，无版本控制
    }
}
```

如果并行节点 A 和 B 同时写入同一个变量 key（如 `data_set` 节点设置相同 key），后完成的会覆盖先完成的。这是设计上的 race condition，在当前架构下无法避免。

**评估：** 对于 DAG 执行来说，同一层级的节点不应该互相依赖，也不应该写入相同的变量 key。用户可以通过设计工作流避免。但如果用户不小心设计了冲突的工作流，结果是不确定的。

**建议：** 增加警告——如果并行层级中的节点写入相同的变量 key，发出警告日志。

---

## 五、🟡 中度问题：Edge 数据流未通过 `from_port` 过滤

### 5.1 `scheduler.rs` 的 `input_ports` 注入逻辑

```rust
// scheduler.rs:914-922
for edge in &workflow.edges {
    if edge.to == *node_id {
        if let Some(upstream_output) = ctx.step_outputs.get(&edge.from) {
            task_ctx.input_ports.insert(
                edge.to_port.clone(),
                upstream_output.clone(),  // 注入整个上游输出，不区分 from_port
            );
        }
    }
}
```

**问题：** 注入的是整个上游输出 `upstream_output.clone()`，而不是 `upstream_output` 中按 `from_port` 过滤的部分。

如果上游节点有多个输出端口（如 `condition` 节点有 `true`/`false` 两个端口），`input_ports` 中注入的是整个输出对象，而不是特定分支的数据。

**期望行为：** 如果 `edge.from_port = "true"`，应该只注入 `output.true` 的数据（或根据节点类型定义处理）。

**当前行为：** 注入整个 `output` 对象，节点通过 `input_ports.get("in")` 获取整个对象。

**评估：** 对于当前大多数节点（只有一个输出端口），这不是问题。但如果节点有多个输出端口（如 `condition` 的 `true`/`false`），下游节点无法区分数据来源。

**修复建议：** 在 `input_ports` 注入时按 `from_port` 过滤：
```rust
let upstream_data = if edge.from_port.is_empty() || edge.from_port == "out" {
    upstream_output.clone()
} else {
    upstream_output.get(&edge.from_port).cloned().unwrap_or(upstream_output.clone())
};
task_ctx.input_ports.insert(edge.to_port.clone(), upstream_data);
```

---

## 六、🟢 已正确实现的逻辑链条

### 6.1 变量解析链

```
{{step_1.result}}  →  resolve_string  →  resolve_var("step_1.result")
  →  step_outputs.get("step_1") →  get_field(output, "result") →  返回 value
```

✅ 正确实现，支持多级路径（`step_1.body.items[0]`）。

### 6.2 表达式引擎链

```
{{= step_1.status == 200 ? step_1.body : "error"}}  →  resolve_string
  →  检测到 {{= 前缀  →  eval_expr(expr)  →  Rhai Engine 求值
  →  注入 variables + step_outputs 到 scope  →  返回结果
```

✅ 正确实现，沙箱安全（max_operations: 100,000，max_string_size: 1MB）。

### 6.3 DAG 调度器条件分支

```
condition 节点输出 {"branch": "true"}  →  DagScheduler::complete_node
  →  from_port == "true" 的边激活  →  from_port == "false" 的边被阻断
  →  被阻断的下游节点 blockCount 不递减  →  永远不会进入 ready 队列
```

✅ 正确实现，支持复杂条件分支阻断。

### 6.4 错误变量传递

```
节点执行失败  →  ctx.set_var("_error.{node_id}", error_message)
  →  下游节点可通过 {{_error.node_id}} 引用
```

✅ 正确实现，线性模式和单节点图模式均支持。

---

## 七、关键使用场景端到端验证

### 场景 1：线性链（data_set → data_get → script）

```
A[data_set key=x value=1] → B[data_get key=x] → C[script {{B}}]
```

**执行路径：** 线性模式（edges 为空）→ `run_linear` → 顺序执行。

**验证：** ✅ 正确。A 设置 `variables["x"] = 1`，B 读取 `variables["x"]`，C 通过 `{{B}}` 引用 B 的 `step_outputs`。

### 场景 2：并行分支（HTTP 并行调用）

```
A[http] → B[http] → D[script {{B}} {{C}}]
A[http] → C[http] →
```

**执行路径：** 图模式 → `run_graph` → 拓扑分层 `[A], [B, C], [D]`。

**验证：** ⚠️ **部分正确**。D 在单节点层级中使用主 `ctx`，可以引用 B 和 C 的 `step_outputs`。但如果 D 在并行层级中，会失败。

### 场景 3：LLM 对话链（prompt_template → llm_chat）

```
A[prompt_template] → B[llm_chat]
```

**执行路径：** 图模式 → 取决于调用入口。

**GUI 验证：** ✅ 正确。`run_dag_workflow` 将 A 的输出注入到 B 的 `input_ports["prompt"]`，B 优先从 `input_ports` 读取 messages。

**CLI 验证：** ❌ **失败**。`run_graph` 不注入 `input_ports`，B 的 `input_ports` 为空，回退到 `config` 中的 messages/prompt。如果 config 中没有配置，B 报错"没有消息可发送"。

### 场景 4：数据过滤链（data_set → data_filter）

```
A[data_set key=items value=[1,2,3,4]] → B[data_filter op="gt" value=2]
```

**GUI 验证：** ✅ 正确。`run_dag_workflow` 将 A 的输出注入到 B 的 `input_ports["data"]`。

**CLI 验证：** ❌ **失败**。`run_graph` 不注入 `input_ports`，B 的 `data_filter` 使用 `values().next()` 取空，回退到 `variables["__item"]`，如果没有 loop 上下文则报错"输入必须是数组"。

### 场景 5：错误处理（on_error: Branch）

```
A[http] → B[email_send]（on_error: Branch → C）
```

**线性模式验证：** ✅ 正确。A 失败后直接执行 C。

**图模式单节点层级验证：** ✅ 正确。A 失败后直接执行 C。

**图模式并行层级验证：** ❌ **失败**。A 失败后只记录日志，C 永远不会执行。

---

## 八、修复优先级清单

### 🔴 立即修复（本周）

| # | 问题 | 文件 | 行号 | 修复 |
|---|------|------|------|------|
| 1 | `run_parallel_level` 缺少 `step_outputs` 传递 | `executor.rs` | 409 | `task_ctx.step_outputs = ctx.step_outputs.clone();` |
| 2 | `run_parallel_level` 缺少 `input_ports` 注入 | `executor.rs` | 411 后 | 遍历 edges 注入（参考 scheduler.rs:912-923） |
| 3 | 并行层级 `on_error: Branch` 不执行 | `executor.rs` | 435-438 | 直接执行分支节点 |

### 🟡 短期修复（2 周内）

| # | 问题 | 文件 | 修复 |
|---|------|------|------|
| 4 | `data_filter` 使用 `values().next()` | `data_filter.rs` | 58 | 改为 `input_ports.get("data")` |
| 5 | `im_message` 使用 `values().next()` | `im_message.rs` | 150 | 改为 `input_ports.get("text")` |
| 6 | `input_ports` 注入未按 `from_port` 过滤 | `scheduler.rs` | 916 | 按 `from_port` 提取子数据 |
| 7 | 并行层级变量覆盖无警告 | `executor.rs` | 423-424 | 添加 key 冲突警告日志 |

### 🟢 建议优化

| # | 问题 | 说明 |
|---|------|------|
| 8 | 统一图执行入口 | CLI 和 GUI 统一走 `scheduler.rs::run_workflow` |
| 9 | `executor.rs::run_graph` 废弃 | 移除重复代码，避免维护两套逻辑 |
| 10 | `validate_variable_references` 警告 | 如果用户用 `{{step_1.result}}` 但 node_id 是 "1"，给出友好提示 |

---

*审查方法：代码走读（executor.rs 974 行 + scheduler.rs 1494 行 + context.rs 418 行 + 12 个节点文件）+ 5 个端到端场景推演*
