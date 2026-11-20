// safe_path.rs — 用户可控路径片段的安全校验
//
// 背景：HTTP 路由里的 `{name}` 类参数会被 axum 做 percent-decode，
// 因此 `..%2F..%2Fetc%2Fpasswd` 会以 `../../../etc/passwd` 的形式到达 handler。
// `std::path::Path::starts_with()` 是**纯词法**前缀比较，不做规范化，
// 所以 `base.join("../../../etc/passwd").starts_with(base)` 仍然为 true ——
// 这类检查在所有调用点都是无效的。正确做法是在拼接之前就把 `..`、
// 路径分隔符、盘符等成分拒掉（fail closed）。

use std::path::{Component, Path, PathBuf};

/// 判断一个路径片段是否是"安全的单层名字"。
///
/// 拒绝：空串、`.`、`..`、任何路径分隔符（`/` `\`）、盘符冒号、NUL。
pub fn is_safe_segment(s: &str) -> bool {
    !s.is_empty()
        && s != "."
        && s != ".."
        && !s.contains('/')
        && !s.contains('\\')
        && !s.contains(':')
        && !s.contains('\0')
}

/// 校验多个路径片段，全部安全才返回 true
pub fn all_safe_segments<'a, I: IntoIterator<Item = &'a str>>(segments: I) -> bool {
    segments.into_iter().all(is_safe_segment)
}

/// 从一个（可能来自客户端的）文件名中提取安全的文件名。
///
/// - 只取最后一段（丢弃 `..\..\Startup\` 之类的目录成分），
///   同时按 `/` 与 `\` 切分，保证 Windows / Linux 行为一致
/// - 只保留字母、数字、`-` `_` `.`，其余替换为 `_`
/// - 结果为空或只有点号时回退到 `fallback`
pub fn sanitize_file_name(name: &str, fallback: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let cleaned: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches('.').to_string();
    if cleaned.is_empty() {
        fallback.to_string()
    } else {
        cleaned
    }
}

/// 校验用户提供的导出/写入路径确实是"工作目录内的普通文件路径"。
///
/// 与旧的 `canonicalize().unwrap_or(raw)` 不同，这里不依赖目标文件是否存在：
/// - 任何 `..` 片段直接拒绝（这正是逃逸手段），两种分隔符都检查
/// - 相对路径按 `base` 拼接，绝对路径必须已经在 `base` 内
///
/// 通过校验后返回可安全写入的路径。
pub fn resolve_within(base: &Path, candidate: &str) -> Option<PathBuf> {
    // 显式逐段检查，不依赖平台的分隔符/组件语义
    if candidate.split(['/', '\\']).any(|seg| seg == "..") {
        return None;
    }
    let p = Path::new(candidate);
    if p.components().any(|c| matches!(c, Component::ParentDir)) {
        return None;
    }
    let absolute = if p.is_absolute() {
        p.to_path_buf()
    } else {
        base.join(p)
    };
    if absolute.starts_with(base) {
        Some(absolute)
    } else {
        None
    }
}

/// 校验一个将被拼进 SQL 的标识符（表名/列名/collection 名）。
///
/// 这些位置无法用 `?N` 参数占位（SQL 不允许绑定标识符），只能拼接，
/// 因此必须严格限制字符集：`^[A-Za-z_][A-Za-z0-9_]*$` 且长度有限。
/// 否则 `collection = "a (id); DROP TABLE runs; --"` 配合 `execute_batch`
/// 可以执行任意语句。
pub fn is_safe_sql_identifier(s: &str) -> bool {
    if s.is_empty() || s.len() > 64 {
        return false;
    }
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sql_identifier_guard() {
        assert!(is_safe_sql_identifier("default"));
        assert!(is_safe_sql_identifier("_v1"));
        assert!(is_safe_sql_identifier("a1_B2"));
        assert!(!is_safe_sql_identifier(""));
        assert!(!is_safe_sql_identifier("1abc"));
        assert!(!is_safe_sql_identifier("a-b"));
        assert!(!is_safe_sql_identifier("a.b"));
        assert!(!is_safe_sql_identifier("x (id); DROP TABLE runs; --"));
        assert!(!is_safe_sql_identifier("a b"));
        assert!(!is_safe_sql_identifier(&"a".repeat(65)));
    }

    #[test]
    fn rejects_traversal_segments() {
        assert!(!is_safe_segment(".."));
        assert!(!is_safe_segment("."));
        assert!(!is_safe_segment(""));
        assert!(!is_safe_segment("a/b"));
        assert!(!is_safe_segment("a\\b"));
        assert!(!is_safe_segment("../../../etc/passwd"));
        assert!(!is_safe_segment("C:"));
        assert!(is_safe_segment("run-123_abc"));
        assert!(is_safe_segment("trajectory.json"));
    }

    #[test]
    fn sanitize_strips_directory_components() {
        // 两种分隔符都要处理，且结果与平台无关
        assert_eq!(
            sanitize_file_name("plugin.wfplug", "plugin.wfplug"),
            "plugin.wfplug"
        );
        assert_eq!(
            sanitize_file_name("..\\..\\Startup\\x.bat", "plugin.wfplug"),
            "x.bat"
        );
        assert_eq!(
            sanitize_file_name("../../etc/passwd", "plugin.wfplug"),
            "passwd"
        );
        // 只有点号 / 空 → 回退
        assert_eq!(sanitize_file_name("...", "plugin.wfplug"), "plugin.wfplug");
        assert_eq!(sanitize_file_name("", "plugin.wfplug"), "plugin.wfplug");
        // 非法字符被替换
        assert_eq!(sanitize_file_name("a b;c", "x"), "a_b_c");
    }

    #[test]
    fn resolve_within_rejects_parent_dir_escape() {
        let base = Path::new("base");
        assert!(resolve_within(base, "out.yaml").is_some());
        assert!(resolve_within(base, "exports/a.yaml").is_some());
        // 词法上 starts_with(base)，但实际会写到 base 之外
        assert!(resolve_within(base, "base/../../Windows/evil.yaml").is_none());
        assert!(resolve_within(base, "..\\evil.yaml").is_none());
        assert!(resolve_within(base, "../evil.yaml").is_none());
    }

    #[test]
    fn resolve_within_allows_nested_relative() {
        let base = Path::new("base");
        let got = resolve_within(base, "exports/a.yaml").unwrap();
        assert!(got.starts_with(base));
    }
}
