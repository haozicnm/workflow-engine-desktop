#!/usr/bin/env python3
"""检测项目结构漂移。

检查关键目录和文件是否存在，防止结构变更导致的问题。
"""

from __future__ import annotations

import sys
from pathlib import Path


# 必须存在的目录
REQUIRED_DIRS = [
    "src-tauri",
    "src-tauri/src",
    "src-tauri/src/engine",
    "src-tauri/src/nodes",
    "src-tauri/src/server",
    "src-tauri/src/data",
    "src",
    "docs",
    "scripts",
    "templates",
    "examples",
]

# 必须存在的文件
REQUIRED_FILES = [
    "README.md",
    "AGENTS.md",
    "package.json",
    "src-tauri/Cargo.toml",
    "src-tauri/src/main.rs",
    "src-tauri/src/lib.rs",
    "src-tauri/node-schema.json",
]

# 禁止存在的文件/目录（临时文件、缓存等）
FORBIDDEN_PATTERNS = [
    "*.pyc",
    "__pycache__",
    ".pytest_cache",
    ".ruff_cache",
    ".mypy_cache",
    "*.swp",
    "*.swo",
    "*~",
]

# 生成目录（不应该被手动修改）
GENERATED_DIRS = [
    "node_modules",
    "dist",
    "build",
    "target",
    ".git",
]


def check_required_dirs(root: Path) -> list[str]:
    """检查必须存在的目录。"""
    errors = []
    for dir_path in REQUIRED_DIRS:
        full_path = root / dir_path
        if not full_path.exists():
            errors.append(f"❌ 缺少目录：{dir_path}")
        elif not full_path.is_dir():
            errors.append(f"❌ 不是目录：{dir_path}")
    return errors


def check_required_files(root: Path) -> list[str]:
    """检查必须存在的文件。"""
    errors = []
    for file_path in REQUIRED_FILES:
        full_path = root / file_path
        if not full_path.exists():
            errors.append(f"❌ 缺少文件：{file_path}")
        elif not full_path.is_file():
            errors.append(f"❌ 不是文件：{file_path}")
    return errors


def check_forbidden_files(root: Path) -> list[str]:
    """检查禁止存在的文件。"""
    errors = []
    for pattern in FORBIDDEN_PATTERNS:
        for path in root.rglob(pattern):
            # 跳过生成目录
            if any(part in GENERATED_DIRS for part in path.parts):
                continue
            rel_path = path.relative_to(root)
            errors.append(f"⚠️ 发现临时文件：{rel_path}")
    return errors


def check_version_consistency(root: Path) -> list[str]:
    """检查版本号一致性。"""
    import json
    
    errors = []
    versions = {}
    
    # 检查 package.json
    package_json = root / "package.json"
    if package_json.exists():
        try:
            data = json.loads(package_json.read_text(encoding="utf-8"))
            if "version" in data:
                versions["package.json"] = data["version"]
        except (OSError, UnicodeDecodeError, json.JSONDecodeError):
            pass
    
    # 检查 Cargo.toml
    cargo_toml = root / "src-tauri" / "Cargo.toml"
    if cargo_toml.exists():
        try:
            content = cargo_toml.read_text(encoding="utf-8")
            for line in content.split("\n"):
                if line.strip().startswith("version"):
                    version = line.split("=")[1].strip().strip('"')
                    versions["Cargo.toml"] = version
                    break
        except (OSError, UnicodeDecodeError):
            pass
    
    # 检查版本是否一致
    if len(set(versions.values())) > 1:
        errors.append(f"⚠️ 版本号不一致：{versions}")
    
    return errors


def check_structure(root: Path) -> int:
    """检查项目结构。"""
    all_errors = []
    
    print("🔍 检查必须存在的目录...")
    all_errors.extend(check_required_dirs(root))
    
    print("🔍 检查必须存在的文件...")
    all_errors.extend(check_required_files(root))
    
    print("🔍 检查禁止存在的文件...")
    all_errors.extend(check_forbidden_files(root))
    
    print("🔍 检查版本号一致性...")
    all_errors.extend(check_version_consistency(root))
    
    # 输出结果
    if all_errors:
        print(f"\n❌ 发现 {len(all_errors)} 个问题：")
        for error in all_errors:
            print(f"  {error}")
        return 1
    else:
        print("\n✅ 项目结构检查通过")
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
    
    print(f"🔍 检查项目结构：{root}")
    return check_structure(root)


if __name__ == "__main__":
    sys.exit(main())
