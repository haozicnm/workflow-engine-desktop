---
date: 2026-05-30
type: bug
tags: [condition, logic, compare, type-coercion]
severity: high
reusable: true
---

# compare_values 字符串比较 Bug + eval_condition 操作符不一致

## 问题

两个独立 bug 叠加导致多条件判断失败。

### Bug 1: compare_values 字符串比较

`normalize_condition_group` 把 `Number(10)` 转成 `"10"` (String)。然后 `compare_values` 做字符串比较：`"10" < "5"` (因为 ASCII "1" < "5")。

单条件时碰巧 OK（"90" > "60"），多条件时暴露。

### Bug 2: eval_condition 操作符不一致

前端 UI 生成 `"op": "eq"` 但 `eval_condition` 只认 `"=="` / `"equals"`。`"eq"` 落入 `_ => false`。

## 修复

### compare_values (condition.rs:282)
```rust
// 新增：字符串→数字解析层
if let (Some(l), Some(r)) = (
    left.as_str().and_then(|s| s.parse::<f64>().ok()),
    right.as_str().and_then(|s| s.parse::<f64>().ok()),
) {
    return l.partial_cmp(&r).map(|o| o as i32).unwrap_or(0);
}
```

### eval_condition (待修)
建议在 match 中添加 `"eq" => ...` 和 `"ne" => ...` 别名。

## 测试覆盖

- `scenario_3_condition_and_branch`: 两个 AND 条件（数字+字符串）
- `scenario_3_condition_or_branch`: OR 条件（数字+数字）

## 根因

`normalize_condition_group` 用 `value_to_condition_string` 把所有类型转 String，丢失了类型信息。下游 `eval_condition` 又不认 String 形式的数字。

## 相关

- 场景 3 测试：条件分支全链路
- `src-tauri/src/nodes/condition.rs`
