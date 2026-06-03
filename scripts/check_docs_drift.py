#!/usr/bin/env python3
"""检测文档中的失效引用和断链。

扫描 AGENTS.md、README.md、docs/ 目录中的 Markdown 文件，
检查引用的文件路径是否存在。
"""

from __future__ import annotations

import re
import sys
from pathlib import Path


# 文档文件列表
DOC_FILES = [
    "AGENTS.md",
    "README.md",
]

# 忽略的引用（生成目录、依赖目录等）
IGNORED_REFERENCES = {
    "node_modules",
    "node_modules/",
    "dist",
    "dist/",
    "build",
    "build/",
    "target",
    "target/",
    ".git",
    ".git/",
    "__pycache__",
    "__pycache__/",
    ".venv",
    ".venv/",
    "venv",
    "venv/",
}

# 忽略的前缀
IGNORED_PREFIXES = (
    "http://",
    "https://",
    "mailto:",
    "#",
    "{{",
    "}}",
)

# 正则表达式
BACKTICK_RE = re.compile(r"`([^`\n]+)`")
MARKDOWN_LINK_RE = re.compile(r"(?<!!)\[[^\]\n]+\]\(([^)\n]+)\)")


def iter_existing_docs(root: Path) -> list[Path]:
    """遍历所有文档文件。"""
    docs = []
    for name in DOC_FILES:
        path = root / name
        if path.exists():
            docs.append(path)
    
    docs_dir = root / "docs"
    if docs_dir.exists():
        docs.extend(sorted(docs_dir.rglob("*.md")))
    
    return docs


def extract_references(text: str) -> set[str]:
    """提取文本中的引用路径。"""
    references = set()
    
    # 提取反引号中的引用
    for match in BACKTICK_RE.findall(text):
        references.add(match)
    
    # 提取 Markdown 链接
    for match in MARKDOWN_LINK_RE.findall(text):
        target = match.strip()
        # 移除锚点
        if "#" in target:
            target = target.split("#")[0]
        if target:
            references.add(target)
    
    return references


def is_ignored(reference: str) -> bool:
    """检查引用是否应该被忽略。"""
    ref = reference.strip()
    
    # 空引用
    if not ref:
        return True
    
    # 忽略的前缀
    if any(ref.startswith(prefix) for prefix in IGNORED_PREFIXES):
        return True
    
    # 忽略的引用
    if ref in IGNORED_REFERENCES:
        return True
    
    # 包含占位符
    if any(token in ref for token in ("...", "<", ">", "{{", "}}")):
        return True
    
    return False


def normalize_path(path: str) -> str:
    """规范化路径。"""
    path = path.strip()
    # 移除开头的 ./
    while path.startswith("./"):
        path = path[2:]
    return path


def check_reference_exists(root: Path, doc: Path, reference: str) -> bool:
    """检查引用的路径是否存在。"""
    normalized = normalize_path(reference)
    
    # 尝试相对于根目录
    if (root / normalized).exists():
        return True
    
    # 尝试相对于文档目录
    if (doc.parent / normalized).exists():
        return True
    
    return False


def check_docs(root: Path) -> int:
    """检查文档引用。"""
    missing_paths = []
    
    for doc in iter_existing_docs(root):
        try:
            text = doc.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        
        references = extract_references(text)
        
        for ref in references:
            if is_ignored(ref):
                continue
            
            # 检查是否是文件路径（包含 / 或 . 或是已知文件）
            if "/" not in ref and "." not in ref and not ref.endswith(".md"):
                continue
            
            if not check_reference_exists(root, doc, ref):
                missing_paths.append((doc, ref))
    
    # 输出结果
    if missing_paths:
        print(f"❌ 发现 {len(missing_paths)} 个失效引用：")
        for doc, ref in missing_paths:
            rel_doc = doc.relative_to(root)
            print(f"  {rel_doc}: `{ref}`")
        return 1
    else:
        print("✅ 文档引用检查通过")
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
    
    print(f"🔍 检查文档引用：{root}")
    return check_docs(root)


if __name__ == "__main__":
    sys.exit(main())
