---
date: 2026-05-30
type: pattern
tags: [rhai, script, json, escaping]
severity: medium
reusable: true
---

# Rhai 脚本中的 JSON 字符串转义

## 问题

在 Rhai `r#""#` 原始字符串中嵌套 JSON 字符串会导致引号冲突。

## 错误示例

```rust
json!({
    "script": r#"
        let json_str = '{"data": [{"id": 1, "name": "Alice"}]}';
        #{json_str: json_str}
    "#
})
```

**错误**: `Syntax error: Invalid character: '{"data": ...`

## 原因

Rhai 的 `r#""#` 原始字符串不处理转义，但 JSON 字符串中的双引号 `"` 会与 Rhai 字符串的引号冲突。

## 解决方案

### 方案 1: 用 Rhai Map 构造数据（推荐）

```rust
json!({
    "script": r#"
        let items = [
            #{id: 1, name: "Alice", age: 30},
            #{id: 2, name: "Bob", age: 25}
        ];
        #{items: items}
    "#
})
```

**优点**: 无需转义，类型安全，编译时检查。

### 方案 2: 用 `file_read` 读取 JSON 文件

```rust
json!({
    "path": "tests/fixtures/sample.json"
})
```

**优点**: 测试数据与代码分离，可复用。

### 方案 3: 用 `data_set` 注入变量

```rust
TestChain::new()
    .var("raw_data", json!([{"id": 1, "name": "Alice"}]))
    .step("parse", "json_parse", json!({
        "data": "{{vars.raw_data}}",
        "expression": "$"
    }))
```

**优点**: 变量在 TestChain 层注入，Rhai 脚本无需处理 JSON。

## 建议

- 测试中优先用 **Rhai Map** 构造数据
- 生产中用 **file_read** 或 **HTTP** 获取数据
- 避免在 Rhai 脚本中硬编码 JSON 字符串

## 相关

- 场景 2 测试：数据搬运全链路
- TestChain.var() 方法：注入工作流变量
