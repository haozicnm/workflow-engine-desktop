#!/usr/bin/env python3
"""Workflow Engine Harness Doctor — 仓库就绪度诊断工具。

扫描仓库，生成 Harness Score 报告。
"""

from __future__ import annotations

import argparse
import json
import sys
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class Check:
    """检查项。"""
    label: str
    points: int
    found: bool
    evidence: str


@dataclass(frozen=True)
class Category:
    """评分类别。"""
    name: str
    checks: tuple[Check, ...]

    @property
    def score(self) -> int:
        return sum(check.points for check in self.checks if check.found)


def check_agent_instructions(root: Path) -> Category:
    """检查 Agent Instructions。"""
    checks = []
    
    # 指令文件存在（5分）
    agents_md = root / "AGENTS.md"
    found = agents_md.exists() and agents_md.stat().st_size > 100
    checks.append(Check("指令文件存在", 5, found, "AGENTS.md" if found else "AGENTS.md 不存在或过小"))
    
    # 项目概述清晰（3分）
    if found:
        content = agents_md.read_text(encoding="utf-8")
        has_overview = "## Purpose" in content or "## Project Overview" in content
        checks.append(Check("项目概述清晰", 3, has_overview, "包含项目概述" if has_overview else "缺少项目概述"))
    else:
        checks.append(Check("项目概述清晰", 3, False, "AGENTS.md 不存在"))
    
    # 精确的构建/测试命令（4分）
    if found:
        content = agents_md.read_text(encoding="utf-8")
        has_commands = "cargo" in content and "npm" in content
        checks.append(Check("精确的构建/测试命令", 4, has_commands, "包含构建命令" if has_commands else "缺少构建命令"))
    else:
        checks.append(Check("精确的构建/测试命令", 4, False, "AGENTS.md 不存在"))
    
    # 架构边界文档化（4分）
    if found:
        content = agents_md.read_text(encoding="utf-8")
        has_architecture = "## Architecture" in content or "## 目录结构" in content
        checks.append(Check("架构边界文档化", 4, has_architecture, "包含架构文档" if has_architecture else "缺少架构文档"))
    else:
        checks.append(Check("架构边界文档化", 4, False, "AGENTS.md 不存在"))
    
    # 禁止行为文档化（2分）
    if found:
        content = agents_md.read_text(encoding="utf-8")
        has_forbidden = "## Forbidden" in content or "## 禁止" in content
        checks.append(Check("禁止行为文档化", 2, has_forbidden, "包含禁止行为" if has_forbidden else "缺少禁止行为"))
    else:
        checks.append(Check("禁止行为文档化", 2, False, "AGENTS.md 不存在"))
    
    # 安全/隐私说明（2分）
    if found:
        content = agents_md.read_text(encoding="utf-8")
        has_security = "## Security" in content or "## 安全" in content
        checks.append(Check("安全/隐私说明", 2, has_security, "包含安全说明" if has_security else "缺少安全说明"))
    else:
        checks.append(Check("安全/隐私说明", 2, False, "AGENTS.md 不存在"))
    
    return Category("Agent Instructions", tuple(checks))


def check_feedback_loops(root: Path) -> Category:
    """检查 Feedback Loops。"""
    checks = []
    
    # 测试命令存在（4分）
    package_json = root / "package.json"
    cargo_toml = root / "src-tauri" / "Cargo.toml"
    has_test = False
    if package_json.exists():
        try:
            data = json.loads(package_json.read_text(encoding="utf-8"))
            has_test = "test" in data.get("scripts", {})
        except (OSError, UnicodeDecodeError, json.JSONDecodeError):
            pass
    if cargo_toml.exists():
        has_test = True  # Cargo.toml 存在就表示有 cargo test
    checks.append(Check("测试命令存在", 4, has_test, "有测试命令" if has_test else "缺少测试命令"))
    
    # Lint 命令存在（4分）
    has_lint = False
    if cargo_toml.exists():
        has_lint = True  # cargo clippy 总是可用
    checks.append(Check("Lint 命令存在", 4, has_lint, "有 cargo clippy" if has_lint else "缺少 lint 命令"))
    
    # 类型检查命令存在（3分）
    has_typecheck = False
    if package_json.exists():
        try:
            data = json.loads(package_json.read_text(encoding="utf-8"))
            has_typecheck = "vue-tsc" in data.get("scripts", {}).get("build", "")
        except (OSError, UnicodeDecodeError, json.JSONDecodeError):
            pass
    checks.append(Check("类型检查命令存在", 3, has_typecheck, "有 vue-tsc" if has_typecheck else "缺少类型检查"))
    
    # CI 工作流存在（5分）
    ci_yml = root / ".github" / "workflows" / "ci.yml"
    has_ci = ci_yml.exists()
    checks.append(Check("CI 工作流存在", 5, has_ci, "有 CI 工作流" if has_ci else "缺少 CI 工作流"))
    
    # Pre-commit 或本地验证脚本（2分）
    scripts_dir = root / "scripts"
    has_scripts = scripts_dir.exists() and any(scripts_dir.glob("check_*.py"))
    checks.append(Check("本地验证脚本", 2, has_scripts, "有检查脚本" if has_scripts else "缺少检查脚本"))
    
    # 验证说明文档化（2分）
    agents_md = root / "AGENTS.md"
    has_docs = False
    if agents_md.exists():
        content = agents_md.read_text(encoding="utf-8")
        has_docs = "## Build & Test Commands" in content or "## 验证" in content
    checks.append(Check("验证说明文档化", 2, has_docs, "有验证说明" if has_docs else "缺少验证说明"))
    
    return Category("Feedback Loops", tuple(checks))


def check_durable_memory(root: Path) -> Category:
    """检查 Durable Memory。"""
    checks = []
    
    # docs/decisions 存在（5分）
    decisions_dir = root / "docs" / "decisions"
    has_decisions = decisions_dir.exists() and any(decisions_dir.glob("*.md"))
    checks.append(Check("docs/decisions 存在", 5, has_decisions, "有决策记录" if has_decisions else "缺少决策记录"))
    
    # docs/failures 存在（5分）
    failures_dir = root / "docs" / "failures"
    has_failures = failures_dir.exists() and any(failures_dir.glob("*.md"))
    checks.append(Check("docs/failures 存在", 5, has_failures, "有失败记录" if has_failures else "缺少失败记录"))
    
    # docs/conventions 存在（4分）
    conventions_dir = root / "docs" / "conventions"
    has_conventions = conventions_dir.exists() and any(conventions_dir.glob("*.md"))
    checks.append(Check("docs/conventions 存在", 4, has_conventions, "有约定文档" if has_conventions else "缺少约定文档"))
    
    # docs/domain 存在（3分）
    domain_dir = root / "docs" / "domain"
    has_domain = domain_dir.exists() and any(domain_dir.glob("*.md"))
    checks.append(Check("docs/domain 存在", 3, has_domain, "有领域文档" if has_domain else "缺少领域文档"))
    
    # 至少一条真实决策或失败记录（3分）
    has_real_record = False
    if has_decisions:
        for record in decisions_dir.glob("*.md"):
            if record.name != "0000-template.md":
                has_real_record = True
                break
    if has_failures:
        for record in failures_dir.glob("*.md"):
            if record.name != "0000-template.md":
                has_real_record = True
                break
    checks.append(Check("至少一条真实记录", 3, has_real_record, "有真实记录" if has_real_record else "只有模板"))
    
    return Category("Durable Memory", tuple(checks))


def check_structural_safety(root: Path) -> Category:
    """检查 Structural Safety。"""
    checks = []
    
    # 结构检查脚本存在（5分）
    structure_script = root / "scripts" / "check_structure.py"
    has_structure = structure_script.exists()
    checks.append(Check("结构检查脚本存在", 5, has_structure, "有 check_structure.py" if has_structure else "缺少结构检查"))
    
    # 文档漂移检查存在（4分）
    drift_script = root / "scripts" / "check_docs_drift.py"
    has_drift = drift_script.exists()
    checks.append(Check("文档漂移检查存在", 4, has_drift, "有 check_docs_drift.py" if has_drift else "缺少漂移检查"))
    
    # 生成文件保护（3分）
    gitignore = root / ".gitignore"
    has_gitignore = gitignore.exists() and gitignore.stat().st_size > 100
    checks.append(Check("生成文件保护", 3, has_gitignore, "有 .gitignore" if has_gitignore else "缺少 .gitignore"))
    
    # 禁止路径检查（3分）
    if has_structure:
        content = structure_script.read_text(encoding="utf-8")
        has_forbidden = "FORBIDDEN" in content or "禁止" in content
        checks.append(Check("禁止路径检查", 3, has_forbidden, "检查临时文件" if has_forbidden else "不检查临时文件"))
    else:
        checks.append(Check("禁止路径检查", 3, False, "缺少结构检查"))
    
    # 架构/依赖边界检查（3分）
    if has_structure:
        content = structure_script.read_text(encoding="utf-8")
        has_version = "version" in content.lower() or "版本" in content
        checks.append(Check("架构/依赖边界检查", 3, has_version, "检查版本一致性" if has_version else "不检查版本"))
    else:
        checks.append(Check("架构/依赖边界检查", 3, False, "缺少结构检查"))
    
    # CI 运行至少一项结构检查（2分）
    ci_yml = root / ".github" / "workflows" / "ci.yml"
    has_ci_check = False
    if ci_yml.exists():
        content = ci_yml.read_text(encoding="utf-8")
        has_ci_check = "check_" in content or "drift" in content
    checks.append(Check("CI 运行结构检查", 2, has_ci_check, "CI 中有检查" if has_ci_check else "CI 中无检查"))
    
    return Category("Structural Safety", tuple(checks))


def check_adoption_clarity(root: Path) -> Category:
    """检查 Adoption Clarity。"""
    checks = []
    
    # README 解释 harness 用途（4分）
    readme = root / "README.md"
    has_readme = readme.exists() and readme.stat().st_size > 500
    checks.append(Check("README 存在", 4, has_readme, "有 README" if has_readme else "缺少 README"))
    
    # 快速开始存在（4分）
    if has_readme:
        content = readme.read_text(encoding="utf-8")
        has_quickstart = "快速开始" in content or "Quick Start" in content
        checks.append(Check("快速开始存在", 4, has_quickstart, "有快速开始" if has_quickstart else "缺少快速开始"))
    else:
        checks.append(Check("快速开始存在", 4, False, "缺少 README"))
    
    # 前后对比示例（4分）
    examples_dir = root / "examples"
    has_examples = examples_dir.exists() and any(examples_dir.glob("*"))
    checks.append(Check("示例存在", 4, has_examples, "有示例" if has_examples else "缺少示例"))
    
    # 采用报告模板（3分）
    has_template = False
    docs_dir = root / "docs"
    if docs_dir.exists():
        for template in docs_dir.rglob("*template*"):
            has_template = True
            break
    checks.append(Check("模板存在", 3, has_template, "有模板" if has_template else "缺少模板"))
    
    # Profile/示例存在（3分）
    checks.append(Check("示例存在", 3, has_examples, "有示例" if has_examples else "缺少示例"))
    
    # 已知限制文档化（2分）
    agents_md = root / "AGENTS.md"
    has_limitations = False
    if agents_md.exists():
        content = agents_md.read_text(encoding="utf-8")
        has_limitations = "Known Issues" in content or "已知问题" in content
    checks.append(Check("已知限制文档化", 2, has_limitations, "有已知问题" if has_limitations else "缺少已知问题"))
    
    return Category("Adoption Clarity", tuple(checks))


def generate_report(root: Path) -> str:
    """生成 Harness Doctor 报告。"""
    categories = [
        check_agent_instructions(root),
        check_feedback_loops(root),
        check_durable_memory(root),
        check_structural_safety(root),
        check_adoption_clarity(root),
    ]
    
    total_score = sum(cat.score for cat in categories)
    
    # 确定等级
    if total_score >= 90:
        grade = "A / 生产就绪"
    elif total_score >= 80:
        grade = "B+ / 强基线"
    elif total_score >= 70:
        grade = "B / 有用但不完整"
    elif total_score >= 60:
        grade = "C / 基础"
    elif total_score >= 40:
        grade = "D / 大部分临时"
    else:
        grade = "F / 几乎无持久基线"
    
    # 生成报告
    lines = []
    lines.append("=" * 60)
    lines.append("Harness Doctor Report — Workflow Engine")
    lines.append("=" * 60)
    lines.append("")
    lines.append(f"Score: {total_score}/100")
    lines.append(f"Grade: {grade}")
    lines.append("")
    lines.append("Breakdown:")
    for cat in categories:
        lines.append(f"- {cat.name}: {cat.score}/20")
    lines.append("")
    lines.append("Evidence:")
    for cat in categories:
        for check in cat.checks:
            status = "✅" if check.found else "❌"
            lines.append(f"  {status} {check.label}: {check.evidence}")
    lines.append("")
    lines.append("=" * 60)
    
    return "\n".join(lines)


def main():
    """主函数。"""
    parser = argparse.ArgumentParser(
        description="Workflow Engine Harness Doctor — 仓库就绪度诊断工具"
    )
    parser.add_argument(
        "--target",
        default=".",
        help="仓库根目录（默认当前目录）"
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="输出 JSON 格式"
    )
    
    args = parser.parse_args()
    root = Path(args.target).resolve()
    
    if not root.exists():
        print(f"❌ 目录不存在：{root}")
        return 1
    
    if args.json:
        categories = [
            check_agent_instructions(root),
            check_feedback_loops(root),
            check_durable_memory(root),
            check_structural_safety(root),
            check_adoption_clarity(root),
        ]
        total_score = sum(cat.score for cat in categories)
        result = {
            "score": total_score,
            "categories": [
                {
                    "name": cat.name,
                    "score": cat.score,
                    "checks": [
                        {
                            "label": check.label,
                            "points": check.points,
                            "found": check.found,
                            "evidence": check.evidence,
                        }
                        for check in cat.checks
                    ]
                }
                for cat in categories
            ]
        }
        print(json.dumps(result, indent=2, ensure_ascii=False))
    else:
        print(generate_report(root))
    
    return 0


if __name__ == "__main__":
    sys.exit(main())
