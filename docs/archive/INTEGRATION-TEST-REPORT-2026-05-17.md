# wf-engine 三个工作流全量集成测试报告

**日期：** 2026-05-17  
**版本：** v6.8.0 → v6.8.0+  
**环境：** WSL CLI 模式（无 GUI）  
**测试范围：** 14/14 节点类型，~22/68 动作

---

## 一、测试结果

| # | 工作流 | 步骤数 | 耗时 | 退出码 | 覆盖重点 |
|---|--------|--------|------|--------|----------|
| C | 嵌套容错链压力测试 | 12 | 19.1s | ✅ 0 | 全容器类型启动、嵌套容错、conditionGroup 分叉 |
| A | 每日全栈舆情监控 | 14 | 23.9s | ✅ 0 | 跨步骤变量传递、browser→file→excel→word 链路 |
| B | 文件批量处理+审批 | 12 | 3.0s | ✅ 0 | cursor 持久化、excel read/filter、approval 超时分支 |

**三战全胜。** 从首次失败的 `file 容器缺少 actions` 到最终全部通过，累计发现并修复 10 个代码 bug + 修复 4 个 workflow 设计问题。

---

## 二、Bug 清单

### 第 1 层：Parser ↔ Executor 契约断裂（最严重）

| # | 位置 | 问题 | 根因 |
|---|------|------|------|
| 1 | `parser.rs:20` | `CONTAINER_TYPES` 漏了 `"file"` | 新增容器时漏加 |
| 2 | `executor.rs` | `browser/excel/word/file` 注册名缺 `_container` 后缀 | 逐个加容器时没一起改 |
| 3 | `executor.rs:150` | `condition_group` 硬编码为 `None` | 注释误导："most Option fields are None" |

**教训：** parser 和 executor 之间没有编译期契约校验。parser 加了 `_container` 后缀后，executor 注册名必须同步更新。建议用关联常量或宏来保证一致性。

### 第 2 层：数据格式转换（中等）

| # | 位置 | 问题 |
|---|------|------|
| 4 | `parser.rs` | J SON 的 `conditionGroup`（驼峰）放在 `config` 嵌套内，serde alias 只在顶层生效 |
| 6 | `loop_node.rs` | `items` 字段是 JSON 编码字符串 `"[\"alpha\",\"beta\"]"` 而非真数组 |
| 9 | `cursor.rs` | 同上——cursor 和 loop 的 `resolve_items` 是两套独立代码 |

**教训：** 前端发送的 JSON 和后端期望的格式有差异（驼峰 vs 蛇形、字符串 vs 数组）。parser 是唯一应做格式归一化的地方，但当前 parser 只处理部分字段。

### 第 3 层：tokio 陷阱（特定场景）

| # | 位置 | 问题 |
|---|------|------|
| 5 | `browser.rs:203` | `Drop` 中用 `tokio::sync::Mutex::blocking_lock()` → 在 async 上下文中 panic |

**教训：** `blocking_lock()` 只能在非 async 线程调用。Drop 可能在 async task 结束时被触发（Arc refcount 归零），必须用 `try_lock()`。

### 第 4 层：功能缺口（用户可见）

| # | 位置 | 问题 |
|---|------|------|
| 7 | `browser_container.rs` | `wait` action 只支持 CSS selector 等待，不支持 `ms` 时间等待 |
| 8 | `loop_node.rs` | loop 节点暴露 `__item`/`__index`，但 workflow 模板用 `{{loop.current}}`/`{{loop.index}}` |
| 10 | `cursor.rs` | 同上——暴露 `__item`/`__index`，模板用 `{{cursor.current}}`/`{{cursor.index}}` |

**教训：** 内部变量名（`__item`）和用户模板语法（`loop.current`）是两套体系。用户不会知道内部命名。

---

## 三、Workflow 设计问题（非代码 Bug）

| # | 问题 | 修复 |
|---|------|------|
| — | HackerNews 被 GFW 屏蔽 | → `httpbin.org` |
| — | Rhai 脚本写成了 JS 语法 `{...}` | → `#{...}` |
| — | Workflow B 路径 `./test_data/` 不存在 | → `/tmp/wf-test-data/` |
| — | 模板引用 `step_1.action_1_1.result.files` 不存在 | → `step_1.action_1_1.matches` |

---

## 四、系统性风险

### 🔴 高风险

1. **Parser-Executor 契约无校验**
   - 当前：parser 和 executor 各自维护类型列表，无交叉验证
   - 风险：新增容器类型时必重复出错
   - 建议：在 parser 中导出 `CONTAINER_TYPES` 常量，executor 编译期引用

2. **重复代码（loop vs cursor）**
   - `resolve_items` 函数在两个文件中各有一套
   - 当前已同步修复（JSON 解析 + 变量注册），但未来分叉风险高
   - 建议：抽取公共 `resolve_iteration_items()` 到 `engine/` 或 `nodes/common.rs`

3. **格式归一化不完整**
   - parser 只处理了 `conditionGroup` → `condition_group` 的转换
   - 其他前端发送的 camelCase 字段（如果有）不会被处理
   - 建议：parser 统一做一次全量 camelCase→snake_case 转换

### 🟡 中风险

4. **变量引用链路不透明**
   - `{{loop.current}}` 的解析路径：`resolve_config` → `resolve_string` → `resolve_var("loop")` → 找 `variables["loop"]` → 钻取 `.current`
   - 需要 5 步才能追踪，出错时完全不可调试
   - 建议：为关键变量引用增加 debug 日志

5. **Browser sidecar 稳定性**
   - 每次运行都有 2-3 次 "ping 失败，重新启动" 警告
   - 虽然最终成功，但浪费 10+ 秒启动时间
   - 建议：增加 sidecar 预热机制，或在 CLI 模式跳过不必要的 browser 操作

6. **CLI 模式下的 approval 节点**
   - Workflow B 的 approval 节点在 3 秒内通过了——可能是立即超时或空操作
   - 未验证 approval 的实际分支逻辑（CLI 无法交互）
   - 建议：CLI 模式中 approval 应明确报 warning 而不是静默通过

### 🟢 低风险

7. **Notifications 仅支持 Windows**——CLI 模式中无害（非阻塞 warning）
8. **测试数据路径硬编码**——`/tmp/wf-test-data/` 在重启后丢失

---

## 五、优化建议（按优先级）

### 优先级 1：安全加固

```
1. parser 导出 CONTAINER_TYPES，executor 编译期引用
   → const ALL_CONTAINER_TYPES: &[&str] = parser::CONTAINER_TYPES;
   → 宏宏生成 executor 注册代码

2. 抽取公共 resolve_iteration_items() 到 engine/common.rs
   → loop_node 和 cursor 共用同一实现

3. executor.rs 用 clone_fields! 宏减少手动字段复制错误
   → 所有 Step → resolved_step 的字段搬运用宏保证不漏
```

### 优先级 2：用户体验

```
4. 统一变量引用规范
   → loop:   {{loop.current}} {{loop.index}} {{loop.index1}}
   → cursor: {{cursor.current}} {{cursor.index}} {{cursor.total}}
   → 文档写入 SKILL.md 和前端提示

5. browser wait action 文档更新
   → 说明 ms 模式（时间等待）和 selector 模式（元素等待）的区别
```

### 优先级 3：工程效率

```
6. 编译期测试
   → 每个 container_type 在 executor 中必须有一一对应的注册
   → 用 build.rs 生成测试代码自动验证

7. 集成测试套件
   → 将 A/B/C 三个工作流写入测试用例
   → CI 每次 push 自动跑
```

---

## 六、统计

| 指标 | 数值 |
|------|------|
| 代码 bug 发现 | **10 个** |
| workflow 设计问题 | 4 个 |
| Rust 文件修改 | 6 个（parser, executor, browser, browser_container, loop_node, cursor） |
| Git commits | 3 个（648adf1, 7408bf7, e3d4159） |
| 编译次数 | 10 次 |
| 运行次数 | 14 次 |
| 总耗时 | ~2 小时（含分析+修复+重跑） |
| 覆盖节点类型 | **14/14（100%）** ✅ |
| 覆盖动作数 | ~22/68（32%）⚠️ |

**结论：** 核心引擎（parser + executor + context 变量传递）经过三轮压力测试证明稳定。主要风险集中在 Parser-Executor 契约对齐和变量引用一致性的维护上。建议优先实施编译期契约校验和代码去重。
