#!/usr/bin/env python3
"""检查决策记录的完整性。

确保每个决策记录都有完整的上下文、决策和后果。
"""

from __future__ import annotations

import sys
from pathlib import Path


# 必须包含的章节
REQUIRED_SECTIONS = [
    "## Context",
    "## Decision",
    "## Consequences",
]

# 可选章节
OPTIONAL_SECTIONS = [
    "## Alternatives Considered",
    "## References",
]


def iter_decision_records(root: Path) -> list[Path]:
    """遍历所有决策记录。"""
    decisions_dir = root / "docs" / "decisions"
    if not decisions_dir.exists():
        return []
    
    return sorted(decisions_dir.glob("*.md"))


def check_decision_record(record: Path) -> list[str]:
    """检查单个决策记录。"""
    errors = []
    
    try:
        content = record.read_text(encoding="utf-8")
    except (OSError, UnicodeDecodeError):
        return [f"❌ 无法读取文件：{record}"]
    
    # 检查必须包含的章节
    for section in REQUIRED_SECTIONS:
        if section not in content:
            errors.append(f"❌ 缺少章节：{section}")
    
    # 检查可选章节（仅警告）
    for section in OPTIONAL_SECTIONS:
        if section not in content:
            errors.append(f"⚠️ 建议添加章节：{section}")
    
    # 检查状态字段
    if "> 状态：" not in content:
        errors.append("⚠️ 缺少状态字段")
    
    # 检查日期字段
    if "> 日期：" not in content:
        errors.append("⚠️ 缺少日期字段")
    
    # 检查是否是模板文件
    if "NNNN-" in record.name:
        errors.append("ℹ️ 这是模板文件，应该重命名")
    
    return errors


def check_decision_memory(root: Path) -> int:
    """检查决策记录。"""
    records = iter_decision_records(root)
    
    if not records:
        print("⚠️ 没有找到决策记录")
        return 0
    
    print(f"🔍 检查 {len(records)} 个决策记录...")
    
    all_errors = []
    for record in records:
        errors = check_decision_record(record)
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
        print("\n✅ 决策记录检查通过")
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
    
    print(f"🔍 检查决策记录：{root}")
    return check_decision_memory(root)


if __name__ == "__main__":
    sys.exit(main())
