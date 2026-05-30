---
date: 2026-05-30
type: pattern
tags: [workflow, variables, yaml, config]
severity: medium
reusable: true
---

# 变量引用格式：两种等效写法

## 问题

YAML 工作流中引用上游步骤输出时，存在两种格式，文档未说明。

## 两种格式

### 格式 1: `{{step_id.field}}`
```yaml
source: "{{fetch_data.items}}"
input: "{{filter_beijing.result}}"
```
- 用双花括号包裹
- 引用 `step_id` 的 `field` 字段

### 格式 2: `output.step_id`
```yaml
source: "output.step_1"
```
- 不用花括号
- 引用整个步骤输出

## 适用场景

| 格式 | 用途 |
|------|------|
| `{{step_id.field}}` | 引用输出的某个字段（如 `.items`, `.result`, `.content`） |
| `output.step_id` | 引用整个步骤输出（如 array_filter 的 source 需要整个数组） |

## 实测结论

- `array_filter.source`: 两种格式都能工作
- `convert_to_csv.input`: 需要 `{{step_id.result}}` 格式
- `file_write.content`: 需要 `{{step_id.field}}` 格式

## 建议

统一使用 `{{step_id.field}}` 格式，更明确、更安全。

## 相关

- 场景 1 测试：HTTP → array_filter 链路
- 场景 2 测试：script → array_filter → convert_to_csv → file_write 链路
