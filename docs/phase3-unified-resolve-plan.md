# Phase 3: 统一 resolve 架构 — 详细方案

## 一、问题回顾

### 1.1 当前架构的矛盾

```
executor.rs 决策点：
if is_container || resolve_self {
    config  // 跳过 resolve，原始 JSON
} else {
    ctx.resolve_config(&config)  // 全量递归解析
}
```

**跳过的原因**：容器 config 有类型化 struct 字段，全量递归解析会把模板变量变成数字/对象，导致反序列化失败。

**导致的问题**：每个容器自己实现 resolve，实现不一致：
- Excel 容器：遍历 action.config 的每个 value 调用 ctx.resolve_config
- File 容器：未知（未检查）
- Word 容器：未知（未检查）
- Browser 容器：未知（未检查）
- Loop/Cursor/While：通过 resolve_iteration_items 处理

### 1.2 实际案例

Excel write action 的 `value` 字段：
```json
{"value": "{{step_1.rows}}"}
```

如果全局 resolve：
1. `{{step_1.rows}}` → `[["1","标题A","链接A"], ...]`（数组）
2. 反序列化为 `HashMap<String, Value>` 时，value 字段是数组 ✅
3. 但如果 value 声明为 `String` 类型，反序列化会失败 ❌

这就是类型破坏问题。

## 二、解决方案：两阶段 resolve

### 2.1 核心思路

**不改变 resolve 的结果，只改变 resolve 的时机。**

```
当前：resolve → 反序列化 → 容器内部再 resolve
目标：占位符替换 → 反序列化 → 后处理替换
```

### 2.2 详细流程

```
输入: config = {
    "file_path": "/tmp/test.xlsx",
    "actions": [
        {"id": "a1", "type": "write", "config": {"value": "{{step_1.rows}}"}}
    ]
}

阶段 1: 占位符替换
    扫描所有 {{...}} 模板
    替换为 __PH_0__, __PH_1__, ...
    记录映射: {0: "step_1.rows", 1: "params.title", ...}

    结果: config = {
        "file_path": "/tmp/test.xlsx",
        "actions": [
            {"id": "a1", "type": "write", "config": {"value": "__PH_0__"}}
        ]
    }
    映射: {0: ("step_1", "rows")}

阶段 2: 反序列化
    ExcelContainerConfig = serde_json::from_value(config)
    不会类型破坏，因为 __PH_0__ 是字符串

阶段 3: 后处理替换
    遍历 ExcelContainerConfig 的所有字段
    发现 __PH_0__ → 查映射 → 替换为实际值
    实际值是数组，直接赋值（不需要反序列化）

    结果: ExcelContainerConfig {
        file_path: "/tmp/test.xlsx",
        actions: [
            ContainerAction {
                id: "a1",
                action_type: "write",
                config: {"value": [["1","标题A","链接A"], ...]}
            }
        ]
    }
```

### 2.3 技术细节

**占位符格式**：`__WF_PH_{index}__`
- 前缀 `__WF_PH_` 避免与正常内容冲突
- 后缀 `__` 闭合
- index 是数字，从 0 开始

**占位符存储**：
```rust
struct PlaceholderMap {
    map: HashMap<usize, String>,  // index → 原始表达式
}
```

**替换逻辑**：
1. 扫描：递归遍历 JSON，找到所有 `{{...}}` 字符串
2. 替换：替换为 `__WF_PH_{index}__`，记录映射
3. 反序列化：正常 serde_json::from_value
4. 后处理：递归遍历 struct，找到所有 `__WF_PH_{index}__` 字符串，替换为实际值

### 2.4 边界情况

**情况 1：字符串中混合模板和文本**
```json
{"title": "用户 {{name}} 的分数"}
```
处理：替换为 `__WF_PH_0__`，后处理时替换为实际值（字符串拼接）

**情况 2：整个值是模板**
```json
{"value": "{{step_1.rows}}"}
```
处理：替换为 `__WF_PH_0__`，后处理时替换为实际值（保持原始类型）

**情况 3：嵌套模板**
```json
{"items": "{{step_1.items}}", "filter": "{{step_2.filter}}"}
```
处理：每个模板独立替换，独立记录映射

**情况 4：模板在数组中**
```json
{"data": ["{{step_1.a}}", "{{step_1.b}}"]}
```
处理：递归扫描数组元素

**情况 5：模板在对象的 key 中**
```json
{"{{step_1.key}}": "value"}
```
处理：不支持（极少出现），忽略

## 三、实现计划

### 3.1 新增模块

**文件**：`src-tauri/src/engine/placeholder.rs`

```rust
/// 占位符映射
pub struct PlaceholderMap {
    map: HashMap<usize, String>,
    next_index: usize,
}

impl PlaceholderMap {
    /// 扫描 JSON，替换 {{...}} 为占位符
    pub fn scan_and_replace(&mut self, config: &mut Value) -> Result<()>;

    /// 后处理：替换占位符为实际值
    pub fn resolve_placeholders<T: serde::Serialize + serde::Deserialize>(
        &self,
        value: &mut T,
        ctx: &ExecutionContext,
    ) -> Result<()>;

    /// 生成占位符字符串
    fn make_placeholder(&self, index: usize) -> String {
        format!("__WF_PH_{}__", index)
    }

    /// 检查字符串是否是占位符
    fn is_placeholder(s: &str) -> Option<usize> {
        if s.starts_with("__WF_PH_") && s.ends_with("__") {
            let inner = &s[8..s.len()-2];
            inner.parse().ok()
        } else {
            None
        }
    }
}
```

### 3.2 修改 executor.rs

```rust
// 当前：
let resolved_config = if is_container || resolve_self {
    config  // 跳过 resolve
} else {
    ctx.resolve_config(&step.config)
};

// 修改后：
let (resolved_config, placeholders) = if is_container {
    let mut config = step.config.clone();
    // 合并 actions
    if let Some(actions) = &step.actions { ... }
    // 阶段 1: 占位符替换
    let mut ph = PlaceholderMap::new();
    ph.scan_and_replace(&mut config)?;
    (config, Some(ph))
} else if resolve_self {
    (step.config.clone(), None)
} else {
    (ctx.resolve_config(&step.config), None)
};

// 反序列化
let mut typed_config = serde_json::from_value(resolved_config)?;

// 阶段 3: 后处理替换
if let Some(ph) = placeholders {
    ph.resolve_placeholders(&mut typed_config, ctx)?;
}
```

### 3.3 修改容器节点

移除容器内部的 resolve 实现：

```rust
// excel_container.rs 当前：
for action in &mut config.actions {
    for (_, v) in action.config.iter_mut() {
        *v = ctx.resolve_config(v);
    }
}

// 修改后：删除这段代码（executor 层已处理）
```

同样修改：
- file_container.rs
- word_container.rs
- browser.rs
- loop_node.rs / cursor.rs / while_node.rs

## 四、风险评估

### 4.1 低风险
- 占位符格式 `__WF_PH_{n}__` 极少与正常内容冲突
- 只改变 resolve 时机，不改变 resolve 结果

### 4.2 中风险
- 后处理替换需要递归遍历 struct，性能可能下降
- 缓解：只遍历字符串字段，其他类型跳过

### 4.3 高风险
- 容器内部 resolve 可能有特殊逻辑（如 loop 的 __item 注入）
- 缓解：先实现基础版本，特殊逻辑保留

## 五、实施步骤

### Step 1: 实现 PlaceholderMap（2小时）
- 创建 placeholder.rs
- 实现 scan_and_replace
- 实现 resolve_placeholders
- 单元测试

### Step 2: 修改 executor.rs（1小时）
- 集成 PlaceholderMap
- 处理容器和非容器分支

### Step 3: 修改容器节点（2小时）
- 移除 Excel 容器内部 resolve
- 移除 File 容器内部 resolve
- 移除 Word 容器内部 resolve
- 保留 Loop/Cursor 的特殊逻辑（__item 注入）

### Step 4: 测试验证（1小时）
- 运行现有测试（13 个变量流转测试）
- 运行全量测试
- 手动测试 ithome-news-excel 工作流

### Step 5: push（30分钟）
- commit
- push to main

## 六、预期效果

### 6.1 代码简化
- 移除各容器内部的 resolve 实现（约 100 行）
- 统一到 executor 层（约 50 行）

### 6.2 行为一致
- 所有容器节点的变量引用行为一致
- 新增容器不需要自己实现 resolve

### 6.3 测试覆盖
- 现有 13 个测试验证行为不变
- 新增测试验证容器变量引用

## 七、回退方案

如果出现问题，可以快速回退：
1. git revert 最后的 commit
2. 恢复容器内部 resolve 实现
3. 重新运行测试

回退时间：< 5 分钟
