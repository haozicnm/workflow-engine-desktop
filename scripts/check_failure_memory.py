#!/usr/bin/env python3
"""检查失败记录的完整性。

确保每个失败记录都有回归检查（测试、lint 规则、CI 检查等）。
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


# 回归检查的关键词
REGRESSION_KEYWORDS = [
    "test_",
    "test(",
    "测试",
    "lint",
    "检查",
    "CI",
    "漂移检查",
    "drift",
    "check_",
    "验证",
    "回归",
]

# 必须包含的章节
REQUIRED_SECTIONS = [
    "## What Failed",
    "## Why It Failed",
    "## Current Replacement",
    "## Agent Guidance",
]

# 回归检查章节
REGRESSION_SECTION = "## Regression Check"


def iter_failure_records(root: Path) -> list[Path]:
    """遍历所有失败记录。"""
    failures_dir = root / "docs" / "failures"
    if not failures_dir.exists():
        return []
    
    return sorted(failures_dir.glob("*.md"))


def check_failure_record(record: Path) -> list[str]:
    """检查单个失败记录。"""
    errors = []
    
    try:
        content = record.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return [f"❌ 无法读取文件：{record}"]
    
    # 检查必须包含的章节
    for section in REQUIRED_SECTIONS:
        if section not in content:
            errors.append(f"❌ 缺少章节：{section}")
    
    # 检查回归检查章节
    if REGRESSION_SECTION not in content:
        errors.append(f"⚠️ 缺少回归检查章节：{REGRESSION_SECTION}")
    else:
        # 检查回归检查内容
        regression_content = content.split(REGRESSION_SECTION)[1]
        has_regression = False
        for keyword in REGRESSION_KEYWORDS:
            if keyword.lower() in regression_content.lower():
                has_regression = True
                break
        
        if not has_regression:
            errors.append("⚠️ 回归检查章节缺少具体检查项")
    
    # 检查是否是模板文件
    if "NNNN-" in record.name:
        errors.append("ℹ️ 这是模板文件，应该重命名")
    
    return errors


def check_failure_memory(root: Path) -> int:
    """检查失败记录。"""
    records = iter_failure_records(root)
    
    if not records:
        print("⚠️ 没有找到失败记录")
        return 0
    
    print(f"🔍 检查 {len(records)} 个失败记录...")
    
    all_errors = []
    for record in records:
        errors = check_failure_record(record)
        if errors:
            rel_path = record.relative_to(root)
            print(f"\n📄 {rel_path}:")
            for error in errors:
                print(f"  {error}")
            all_errors.extend(errors)
    
    # 输出结果
    if all_errors:
        print(f"\n❌ 发现 {len(all_errors)} 个问题")
        return 1
    else:
        print("\n✅ 失败记录检查通过")
        return 0


def main():
    """主函数。"""
    if len(sys.argv) > 1:
        root = Path(sys.argv[1])
    else:
        root = Path.cwd()
    
    if not root.exists():
        print(f"❌ 目录不存在：{root}")
        return 1
    
    print(f"🔍 检查失败记录：{root}")
    return check_failure_memory(root)


if __name__ == "__main__":
    sys.exit(main())
