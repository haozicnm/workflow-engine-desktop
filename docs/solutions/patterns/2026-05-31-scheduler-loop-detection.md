---
date: 2026-05-31
type: pattern
tags: [scheduler, loop-detection, robustness]
reusable: true
---

# 调度器循环检测

## 问题

condition 分支或错误的 `next` 指向可能形成无限循环，导致工作流永不结束。

## 解决方案

在 scheduler.rs 主循环中添加步骤执行计数器：

```rust
const MAX_STEP_EXECUTIONS: usize = 10_000;

let mut step_execution_count: usize = 0;
loop {
    step_execution_count += 1;
    if step_execution_count > MAX_STEP_EXECUTIONS {
        // 强制失败，返回明确错误信息
        return Err(anyhow::anyhow!("检测到可能的无限循环"));
    }
    // ... 正常执行
}
```

## 设计决策

- **10000 上限**：100 步工作流 × 100 次循环 = 10000，覆盖绝大多数场景
- **不区分节点类型**：统一计数，简单可靠
- **明确错误信息**：包含当前计数和上限，便于调试

## 何时触发

1. condition 节点的 true_next/false_next 指向自己
2. 错误的 next 指向形成环路
3. 循环节点（loop/while）的退出条件永远不满足

## 相关

- `src-tauri/src/engine/scheduler.rs`
- scheduler_tests: 30/30 全通过
