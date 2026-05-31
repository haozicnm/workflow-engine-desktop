---
date: 2026-05-31
type: pattern
tags: [error-handling, nodes, standardization]
reusable: true
---

# 节点错误处理统一化

## 问题

各节点错误返回格式不一致，难以统一处理和展示。

## 解决方案

创建 `error_utils.rs` 统一错误处理模块：

```rust
// 标准错误格式
{
    "error": "人类可读的错误描述",
    "code": "ERROR_CODE",
    "suggestion": "修复建议"
}

// 常用错误代码
pub const MISSING_PARAM: &str = "MISSING_PARAM";
pub const INVALID_METHOD: &str = "INVALID_METHOD";
pub const NETWORK_ERROR: &str = "NETWORK_ERROR";
pub const EXECUTION_FAILED: &str = "EXECUTION_FAILED";
pub const NON_ZERO_EXIT: &str = "NON_ZERO_EXIT";
```

## 已迁移节点

- `http.rs`：缺少参数、无效方法、网络错误
- `shell.rs`：缺少命令、执行失败、非零退出码

## 迁移模式

```rust
// 旧写法
return Err(anyhow::anyhow!("missing required field: url"));

// 新写法
return Err(error_utils::missing_param("url"));
```

## 相关

- `src-tauri/src/nodes/error_utils.rs`
- `src-tauri/src/nodes/http.rs`
- `src-tauri/src/nodes/shell.rs`
