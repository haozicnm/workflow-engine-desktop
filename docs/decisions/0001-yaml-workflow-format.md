# 0001-选择 YAML 作为标准工作流格式

> 状态：accepted
> 日期：2026-06-03
> 决策者：若海 + 伟哥

## Context

需要选择一种格式来存储和导出工作流定义。候选格式有 JSON、YAML、TOML。

工作流定义需要：
1. 人可读（方便调试和手动编辑）
2. Agent 可生成（方便 AI 代理生成）
3. 支持注释（方便文档说明）
4. 支持复杂数据结构（嵌套对象、数组）

## Decision

选择 **YAML v1.0** 作为标准工作流格式。

在 `workflow.rs` 中定义 `FORMAT_VERSION="1.0"`，在 `yaml_format.rs` 中实现导出器。

## Consequences

### 正面
- 人可读性好，比 JSON 更易读
- 支持注释，可以添加文档说明
- Agent 生成 YAML 比 JSON 更容易（更少的引号和括号）
- 与现有配置文件格式一致

### 负面
- YAML 解析比 JSON 更复杂（需要处理缩进、引用等）
- 需要额外的解析库（serde_yaml）
- 某些边缘情况需要特殊处理（如特殊字符）

### 后续工作
- 实现 YAML 导出器（yaml_format.rs）
- 实现版本兼容性检查
- 更新 Skill 文档指导 Agent 生成 YAML

## Alternatives Considered

### JSON
- 优点：解析简单，格式标准
- 缺点：不支持注释，人可读性差

### TOML
- 优点：人可读性好，支持注释
- 缺点：不适合复杂嵌套结构

## References

- `src-tauri/src/engine/workflow.rs` — FORMAT_VERSION 定义
- `src-tauri/src/engine/yaml_format.rs` — YAML 导出器
- `docs/workflow-engine-agent-skill.md` — Agent 集成文档
