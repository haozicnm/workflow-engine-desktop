# 领域术语表

> 最后更新：YYYY-MM-DD

## 核心概念

### 工作流 (Workflow)
由多个步骤组成的自动化流程。用户通过拖拽节点构建。

### 步骤 (Step)
工作流中的一个执行单元。每个步骤对应一个节点类型。

### 节点 (Node)
步骤的类型定义。定义了输入、输出、配置参数。

### 积木 (Building Block)
节点的别名。强调节点的封装性和可组合性。

### 端口 (Port)
节点的输入/输出接口。支持类型约束（string/number/boolean/array/object）。

### 变量 (Variable)
工作流中的数据载体。通过 `{{变量名}}` 语法引用。

## 执行相关

### 执行器 (Executor)
负责按顺序执行工作流中的步骤。

### 解析器 (Parser)
负责解析工作流定义，验证节点连接和参数。

### 调度器 (Scheduler)
负责管理工作流的执行时机（定时、触发、手动）。

### Sidecar
Python 子进程，用于执行浏览器自动化等任务。

## 数据相关

### 模板 (Template)
锁定的工作流，不可编辑不可删除。

### 锁定 (Lock)
标记工作流为只读状态。

### 导出 (Export)
将工作流转换为 YAML 格式。

## Agent 相关

### Harness
仓库级别的 AI 操作框架。

### ADR (Architecture Decision Record)
架构决策记录。

### Failure Record
失败记录，用于防止重复错误。

### Drift Check
漂移检查，用于检测文档/结构变更。
