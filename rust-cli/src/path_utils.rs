use std::path::Path;

/// 去掉路径的文件扩展名（posix 风格字符串）
pub fn strip_extension(path: &str) -> String {
    if let Some(dot) = path.rfind('.') {
        let slash = path.rfind('/').map(|i| i + 1).unwrap_or(0);
        if dot > slash {
            return path[..dot].to_string();
        }
    }
    path.to_string()
}

/// 获取 posix 路径的目录部分
pub fn posix_dirname(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) if i > 0 => &path[..i],
        Some(_) => "/",
        None => ".",
    }
}

/// 简单的 posix 路径规范化（处理 `.` 和 `..`，不访问文件系统）
pub fn posix_normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

/// 将 Path 规范化为 posix 风格字符串（解析 `..` 和 `.`，统一为 `/` 分隔符）
pub fn normalize_path(path: &Path) -> String {
    let raw = path.to_string_lossy().replace('\\', "/");
    posix_normalize(&raw)
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_extension() {
        assert_eq!(strip_extension("/foo/bar.ts"), "/foo/bar");
        assert_eq!(strip_extension("/foo/bar"), "/foo/bar");
        assert_eq!(strip_extension("main.rs"), "main");
    }

    #[test]
    fn test_posix_dirname() {
        assert_eq!(posix_dirname("src/auth/login.ts"), "src/auth");
        assert_eq!(posix_dirname("main.ts"), ".");
        assert_eq!(posix_dirname("/root/file.ts"), "/root");
    }

    #[test]
    fn test_posix_normalize() {
        assert_eq!(posix_normalize("src/auth/../utils/helper"), "src/utils/helper");
        assert_eq!(posix_normalize("src/./auth/login"), "src/auth/login");
        assert_eq!(posix_normalize("a/b/c"), "a/b/c");
    }

    #[test]
    fn test_normalize_path() {
        let p = std::path::Path::new("src/auth/../utils/helper");
        assert_eq!(normalize_path(p), "src/utils/helper");
    }
}
