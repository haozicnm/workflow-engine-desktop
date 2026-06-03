# Harness Doctor 评分标准 — Workflow Engine

> 版本：1.0
> 最后更新：2026-06-03

## 评分原则

- 评分基于持久的仓库产物，不是聊天指令
- 优先可执行的检查，而不是依赖纪律
- 优先项目特定规则，而不是通用建议
- 部分分给弱但真实的证据
- 没有证据时给 0 分

## 评分范围

100 分制，五个类别各 20 分。

## 1. Agent Instructions（20分）

评估仓库是否有持久的 AI 代理指令。

| 检查项 | 分值 | 说明 |
|--------|------|------|
| 指令文件存在 | 5 | AGENTS.md 存在且非空 |
| 项目概述清晰 | 3 | 包含项目描述、技术栈、架构 |
| 精确的构建/测试命令 | 4 | 包含具体的 cargo/npm 命令 |
| 架构边界文档化 | 4 | 包含目录结构、模块职责 |
| 禁止行为文档化 | 2 | 包含不要做什么的明确说明 |
| 安全/隐私说明 | 2 | 包含安全相关的注意事项 |

**得分证据**：
- ✅ AGENTS.md 存在
- ✅ 包含项目概述
- ✅ 包含构建命令
- ✅ 包含架构边界
- ✅ 包含禁止行为
- ✅ 包含安全说明

## 2. Feedback Loops（20分）

评估验证机制是否能捕获错误的代理变更。

| 检查项 | 分值 | 说明 |
|--------|------|------|
| 测试命令存在 | 4 | cargo test / npm test 可用 |
| Lint 命令存在 | 4 | cargo clippy / eslint 可用 |
| 类型检查命令存在 | 3 | vue-tsc / tsc 可用 |
| CI 工作流存在 | 5 | .github/workflows/ 存在 |
| Pre-commit 或本地验证脚本 | 2 | 脚本存在且可执行 |
| 验证说明文档化 | 2 | AGENTS.md 中有验证说明 |

**得分证据**：
- ✅ package.json 中有 test/build 脚本
- ✅ Cargo.toml 存在
- ✅ .github/workflows/ci.yml 存在
- ✅ scripts/ 目录有检查脚本
- ✅ AGENTS.md 中有验证说明

## 3. Durable Memory（20分）

评估仓库是否存储长期项目记忆。

| 检查项 | 分值 | 说明 |
|--------|------|------|
| docs/decisions 存在 | 5 | 目录存在且有模板 |
| docs/failures 存在 | 5 | 目录存在且有模板 |
| docs/conventions 存在 | 4 | 目录存在且有内容 |
| docs/domain 存在 | 3 | 目录存在且有术语表 |
| 至少一条真实决策或失败记录 | 3 | 有非模板的真实记录 |

**得分证据**：
- ✅ docs/decisions/ 存在
- ✅ docs/failures/ 存在
- ✅ docs/conventions/ 存在
- ✅ docs/domain/ 存在
- ✅ 有真实记录（0001-*.md）

## 4. Structural Safety（20分）

评估仓库是否有防止结构漂移的保障。

| 检查项 | 分值 | 说明 |
|--------|------|------|
| 结构检查脚本存在 | 5 | scripts/check_structure.py |
| 文档漂移检查存在 | 4 | scripts/check_docs_drift.py |
| 生成文件保护 | 3 | .gitignore 存在且完整 |
| 禁止路径检查 | 3 | 脚本检查临时文件 |
| 架构/依赖边界检查 | 3 | 脚本检查版本一致性 |
| CI 运行至少一项结构检查 | 2 | CI 中调用检查脚本 |

**得分证据**：
- ✅ scripts/check_structure.py 存在
- ✅ scripts/check_docs_drift.py 存在
- ✅ .gitignore 存在
- ✅ 脚本检查临时文件
- ✅ 脚本检查版本一致性

## 5. Adoption Clarity（20分）

评估新用户或代理是否能理解并采用 harness。

| 检查项 | 分值 | 说明 |
|--------|------|------|
| README 解释 harness 用途 | 4 | README.md 中有说明 |
| 快速开始存在 | 4 | 有快速开始指南 |
| 前后对比示例 | 4 | 有使用示例 |
| 采用报告模板 | 3 | 有模板文件 |
| Profile/示例存在 | 3 | examples/ 目录有内容 |
| 已知限制文档化 | 2 | AGENTS.md 中有已知问题 |

**得分证据**：
- ✅ README.md 解释项目用途
- ✅ 有快速开始指南
- ✅ examples/ 目录有示例
- ✅ AGENTS.md 有已知问题

## 等级标准

| 等级 | 分数范围 | 含义 |
|------|----------|------|
| A | 90-100 | 生产就绪基线证据 |
| B+ | 80-89 | 强基线证据 |
| B | 70-79 | 有用但不完整 |
| C | 60-69 | 基础基线证据 |
| D | 40-59 | 大部分临时 |
| F | 0-39 | 几乎无持久基线证据 |

## 使用方法

### 手动评分

1. 按照每个类别的检查项逐项评分
2. 汇总五个类别的分数
3. 根据等级标准确定等级
4. 生成报告

### 自动评分

```bash
# 运行所有检查
python scripts/check_docs_drift.py
python scripts/check_structure.py
python scripts/check_failure_memory.py
python scripts/check_decision_memory.py

# 生成报告（需要实现）
python scripts/harness_doctor.py --target .
```

## 报告格式

```text
Harness Doctor Report

Score: <score>/100
Grade: <grade>

Verdict:
<一段话解释分数含义>

Breakdown:
- Agent Instructions: <points>/20
- Feedback Loops: <points>/20
- Durable Memory: <points>/20
- Structural Safety: <points>/20
- Adoption Clarity: <points>/20

Evidence:
- <发现的证据>
- <发现的证据>
- <缺失或弱的证据>

Top Risks:
1. <最高风险>
2. <次高风险>
3. <第三风险>

Recommended Next Actions:
1. <具体改进>
2. <具体改进>
3. <具体改进>
```

## 与通用评分标准的区别

本评分标准针对 Workflow Engine 项目定制：

1. **技术栈特定**：Rust + Vue 3 + Tauri
2. **构建命令特定**：cargo + npm
3. **检查脚本特定**：项目自定义的脚本
4. **架构特定**：节点系统、工作流格式、变量系统

通用评分标准参见 harness-starter-kit 的 `docs/scoring/harness-score-rubric.md`。
