# 执行上下文命名空间隔离 — 实施方案

> **For Hermes:** Use subagent-driven-development skill to implement this plan task-by-task.

**Goal:** 消除 Rhai 作用域中步骤输出与用户变量同名字段的遮蔽 bug，从根本上杜绝字段碰撞。

**Architecture:** 将当前扁平作用域改为层级化：用户变量从顶层移入 `__vars__` 命名空间，步骤输出稳定在 `step_*` 前缀下。`resolve_var` 兼容旧语法 `{{my_var}}` 和 `{{step_3.body}}`，同时新增 `{{vars.my_var}}` 明确引用。

**Tech Stack:** Rust (Rhai engine, serde_json), 现有模板 JSON

**North Star:** 工作流固化后 Agent 退出独立运行。此方案消除运行时不确定性（变量遮蔽），直接服务北极星目标。

---

## 根因分析

### 当前架构（context.rs 91-101 行）

```rust
let mut scope = rhai::Scope::new();
for (k, v) in &self.variables {          // ← 用户变量全摊在顶层
    scope.push(k.clone(), dynamic);
}
for (k, v) in &self.step_outputs {       // ← 步骤输出也摊在顶层
    scope.push(format!("step_{}", stem), dynamic);
}
```

### 碰撞场景

用户设变量 `body = "hello"`，HTTP 返回 `{"body": {"name": "alice"}}`。Rhai 作用域中 `body` 存在两个冲突定义：
1. 用户变量 `body` → `"hello"`（层级更高，直接 push）
2. 步骤输出的字段 `step_3.body` → `{"name": "alice"}`（在 map 内部）

**结论：这不是"字段名叫 body 才会 bug"，是任何同名字段都会碰撞。用户变量叫 `status`、`name`、`id` 都会撞。**

### 目标架构

```
scope {
    __vars__ → {           ← 用户变量包在子对象里
        body → "hello"
        status → "ok"
    }
    step_1 → {...}         ← 步骤输出保持 step_* 前缀
    step_2 → {...}
    step_3 → {
        body: {...}        ← 不再被顶层 body 遮蔽
        status: 200
    }
}
```

---

## 实施阶段

### Phase 1: 引擎层改造（context.rs）

#### Task 1.1: 修改 `eval_expr` 作用域注入

**Objective:** 变量注入 `__vars__` 子命名空间，不再直接 push 到顶层

**Files:**
- Modify: `src-tauri/src/engine/context.rs:90-101`

**Step 1: 修改变量注入逻辑**

将第 92-94 行：
```rust
for (k, v) in &self.variables {
    let dynamic = json_to_rhai(v);
    scope.push(k.clone(), dynamic);
}
```

改为：
```rust
// 变量注入 __vars__ 子命名空间，避免与步骤输出字段碰撞
let mut vars_map = rhai::Map::new();
for (k, v) in &self.variables {
    vars_map.insert(k.clone().into(), json_to_rhai(v));
}
scope.push("__vars__", rhai::Dynamic::from(vars_map));
```

**Step 2: 编译验证**

```bash
cd /mnt/c/Users/haozi/Dev/workflow-engine-desktop/src-tauri
cargo build 2>&1 | tail -5
```

**Step 3: 验证现有模板兼容性**

```bash
# 先在 WSL 端测试（需要先用 PowerShell 同步修改后的模板）
```

#### Task 1.2: 修改 `resolve_var` 新增 `vars.xxx` 路径支持

**Objective:** `{{vars.my_var}}` 能正确解析，旧语法 `{{my_var}}` 通过兼容逻辑查 `__vars__`

**Files:**
- Modify: `src-tauri/src/engine/context.rs:162-194`

**Step 1: 在 resolve_var 开头添加 vars 路径处理**

在第 163 行 `let parts: Vec<&str> = key.split('.').collect();` 之后，`let root_key = parts[0];` 之前，插入：

```rust
// vars.xxx 命名空间：明确引用用户变量
if parts[0] == "vars" && parts.len() >= 2 {
    if let Some(root) = self.variables.get(parts[1]) {
        let mut current = root;
        for part in &parts[2..] {
            if let Some(v) = current.get(*part) {
                current = v;
            } else {
                return None;
            }
        }
        return Some(current);
    }
}
```

**Step 2: 变量回退查找逻辑**

在第 183 行 `step_outputs.get(root_key)` 之后，`or_else(|| self.variables.get(root_key))` 保持不变——旧语法 `{{my_var}}` 依然能直接从 variables 查到。变量从顶层 scope 移除只影响 Rhai eval，不影响 `resolve_var` 的直接查找。

**Step 3: 编译验证**

```bash
cd /mnt/c/Users/haozi/Dev/workflow-engine-desktop/src-tauri
cargo build 2>&1 | tail -5
```

#### Task 1.3: 添加 Rhai 快捷访问语法糖（可选，降低迁移成本）

**Objective:** Rhai 脚本中 `my_var` 自动映射到 `__vars__.my_var`，无需改脚本

**Files:**
- Modify: `src-tauri/src/engine/context.rs:90-101`

```rust
// 注入后，为常用变量添加快捷引用
for (k, v) in &self.variables {
    if !scope.contains(k.as_str()) {
        scope.push(k.clone(), json_to_rhai(v));
    }
}
```

此改动在 `__vars__` 注入之后执行，只 push 未被步骤输出占用的变量名到顶层。如果 `body` 被变量和步骤输出同时使用，顶层优先保留步骤输出（因为先 push 的变量被后来检查到 `contains` 后跳过）。**行为变更为：步骤输出优先于变量**，需要评估影响。

**替代方案（更安全）：** 不加语法糖，要求 Rhai 脚本中显式写 `__vars__.my_var`。

---

### Phase 2: 模板迁移

#### Task 2.1: 扫描所有模板中的变量引用

**Objective:** 找出所有需要更新的引用

```bash
cd /mnt/c/Users/haozi/Dev/workflow-engine-desktop
grep -r '{{' library/ --include="*.json" | grep -v '{{step_' | grep -v '{{__item'
```

预期：大部分是 `{{step_N.xxx}}` 引用（无需改动），少数是 `{{params.xxx}}`（已在 resolve_var 特殊处理）。

#### Task 2.2: 更新模板中的独立变量引用

将 `{{my_var}}` 改为 `{{vars.my_var}}`（如果有的话）。当前 5 个模板主要用 `{{step_N.xxx}}` 和 `{{params.xxx}}`，可能零改动。

#### Task 2.3: 更新 script 节点中的 Rhai 表达式

搜索所有 `script` 节点中包含独立变量名（非 `step_` 前缀）的表达式：

```bash
grep -r '"script"' library/ --include="*.json" -A1
```

将脚本中的 `my_var` 改为 `__vars__.my_var`。

---

### Phase 3: 测试验证

#### Task 3.1: 运行单元测试

```bash
cd /mnt/c/Users/haozi/Dev/workflow-engine-desktop/src-tauri
cargo test 2>&1 | tail -20
```

#### Task 3.2: 运行模板库回归测试

```bash
# Windows PowerShell（先杀 daemon）
powershell.exe -Command "taskkill /f /im wf-cli.exe 2>&1 | Out-Null"

# 遍历 5 个模板
for wf in integration-smoke daily-monitor file-batch-approval http-approval-pipeline data-pipeline; do
    echo "=== $wf ==="
    powershell.exe -Command "C:\Users\haozi\Dev\workflow-engine-desktop\src-tauri\target\debug\wf-cli.exe library run '$wf' 2>&1"
done
```

#### Task 3.3: 碰撞场景专项测试

手动创建测试工作流：

```json
{
  "steps": [
    {"id": "step_1", "step_type": "data_set", "config": {"key": "body", "value": "this_should_NOT_appear"}},
    {"id": "step_2", "step_type": "data_set", "config": {"key": "status", "value": "wrong_status"}},
    {"id": "step_3", "step_type": "http", "config": {"method": "GET", "url": "https://httpbin.org/json"}},
    {"id": "step_4", "step_type": "script", "config": {"script": "let result = step_3.body;\nresult"}},
    {"id": "step_5", "step_type": "script", "config": {"script": "let s = step_3.status;\ns"}}
  ]
}
```

验证：
- `step_4` 应该输出 HTTP 响应的 `body` 字段（JSON 对象），而非变量 `"this_should_NOT_appear"`
- `step_5` 应该输出 HTTP 响应的 `status` 字段（200），而非变量 `"wrong_status"`
- 旧语法 `{{body}}` 在 `resolve_var` 中依然查 variables（向后兼容），但在 Rhai 中 `body` 不再遮蔽

---

### Phase 4: 清理和文档

#### Task 4.1: 提交代码

```bash
git add src-tauri/src/engine/context.rs
git commit -m "fix: isolate variables from step outputs in context namespace

- Variables injected under __vars__ instead of flat scope
- resolve_var now supports vars.xxx path syntax
- Prevents field name collisions (body/status/name) in Rhai expressions
- Backward compatible: {{my_var}} still resolves via resolve_var direct lookup"
```

#### Task 4.2: Push

```bash
git push
```

---

## 破坏性评估

| 影响范围 | 变更内容 | 用户感知 |
|---------|---------|---------|
| **Rhai 脚本** | 独立变量名 `my_var` → `__vars__.my_var` | ⚠️ 需更新脚本模板 |
| **`{{ }}` 模板** | 新增 `{{vars.xxx}}` 语法，旧语法兼容 | ✅ 无感知 |
| **`data_set` 节点** | 不变 | ✅ 无感知 |
| **`resolve_config`** | 不变 | ✅ 无感知 |
| **`resolve_var`** | 新增 `vars.` 路径支持 | ✅ 向下兼容 |
| **步骤输出引用** | `{{step_3.body}}` 不变 | ✅ 无感知 |

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| 现有工作流中 Rhai 脚本引用独立变量 | Phase 2 扫描 + 自动更新 |
| `resolve_var` 双查逻辑引入性能开销 | 可忽略（已是 HashMap 查找） |
| 引擎改动影响容器节点 | 容器节点不直接调用 `eval_expr`，只在子步骤执行时使用 |

## 原则

- **治本不治标**：从架构层解决，不针对单个字段打补丁
- **最小破坏性**：`{{step_3.body}}` 语法完全不变
- **显式优于隐式**：新写法 `{{vars.my_var}}` 比 `{{my_var}}` 更清晰表达意图
- **北极星对齐**：消除运行时不确定性，工作流固化后更可靠
