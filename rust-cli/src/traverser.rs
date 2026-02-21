use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// 默认排除目录
const DEFAULT_EXCLUDE: &[&str] = &[
    "node_modules",
    "dist",
    "build",
    ".git",
    "vendor",
    "__pycache__",
    "target",
    ".codemap",
];

/// 语言与扩展名映射
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    TypeScript,
    JavaScript,
    Python,
    Go,
    Rust,
    Java,
    C,
    Cpp,
}

impl Language {
    pub fn as_str(self) -> &'static str {
        match self {
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Python => "python",
            Language::Go => "go",
            Language::Rust => "rust",
            Language::Java => "java",
            Language::C => "c",
            Language::Cpp => "cpp",
        }
    }
}

/// 根据文件扩展名检测语言
pub fn detect_language(path: &Path) -> Option<Language> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    match ext.as_str() {
        "ts" | "tsx" => Some(Language::TypeScript),
        "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
        "py" => Some(Language::Python),
        "go" => Some(Language::Go),
        "rs" => Some(Language::Rust),
        "java" => Some(Language::Java),
        "c" | "h" => Some(Language::C),
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => Some(Language::Cpp),
        _ => None,
    }
}

/// 检查文件列表中是否包含 C++ 源文件（用于 .h 重分类）
pub fn has_cpp_source_files(files: &[PathBuf]) -> bool {
    files.iter().any(|f| {
        f.extension()
            .and_then(|e| e.to_str())
            .map(|e| matches!(e.to_lowercase().as_str(), "cpp" | "cc" | "cxx" | "hpp" | "hh"))
            .unwrap_or(false)
    })
}

/// 当项目包含 C++ 源文件时，将 .h 文件重分类为 C++
pub fn effective_language(path: &Path, base: Language, project_has_cpp: bool) -> Language {
    if base == Language::C
        && project_has_cpp
        && path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase() == "h")
            .unwrap_or(false)
    {
        Language::Cpp
    } else {
        base
    }
}

/// 遍历目录，返回所有支持语言的源文件路径
pub fn traverse_files(root_dir: &Path, extra_exclude: &[String]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let walker = WalkBuilder::new(root_dir)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .build();

    for entry in walker.flatten() {
        let path = entry.path().to_path_buf();

        if !path.is_file() {
            continue;
        }

        // 检查是否在默认排除目录中
        if is_excluded(&path, root_dir, extra_exclude) {
            continue;
        }

        // 只保留支持语言的文件
        if detect_language(&path).is_some() {
            files.push(path);
        }
    }

    files.sort();
    files
}

fn is_excluded(path: &Path, root: &Path, extra_exclude: &[String]) -> bool {
    let rel = match path.strip_prefix(root) {
        Ok(r) => r,
        Err(_) => return false,
    };

    // 检查路径各组件是否命中默认排除列表
    for component in rel.components() {
        let name = component.as_os_str().to_string_lossy();
        if DEFAULT_EXCLUDE.contains(&name.as_ref()) {
            return true;
        }
        // 检查额外排除模式（简单前缀/名称匹配）
        for pattern in extra_exclude {
            if name.as_ref() == pattern.as_str() {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_detect_language_typescript() {
        assert_eq!(detect_language(Path::new("foo.ts")), Some(Language::TypeScript));
        assert_eq!(detect_language(Path::new("foo.tsx")), Some(Language::TypeScript));
    }

    #[test]
    fn test_detect_language_javascript() {
        assert_eq!(detect_language(Path::new("foo.js")), Some(Language::JavaScript));
        assert_eq!(detect_language(Path::new("foo.jsx")), Some(Language::JavaScript));
        assert_eq!(detect_language(Path::new("foo.mjs")), Some(Language::JavaScript));
    }

    #[test]
    fn test_detect_language_others() {
        assert_eq!(detect_language(Path::new("foo.py")), Some(Language::Python));
        assert_eq!(detect_language(Path::new("foo.go")), Some(Language::Go));
        assert_eq!(detect_language(Path::new("foo.rs")), Some(Language::Rust));
        assert_eq!(detect_language(Path::new("foo.java")), Some(Language::Java));
        assert_eq!(detect_language(Path::new("foo.c")), Some(Language::C));
        assert_eq!(detect_language(Path::new("foo.cpp")), Some(Language::Cpp));
    }

    #[test]
    fn test_detect_language_unknown() {
        assert_eq!(detect_language(Path::new("foo.txt")), None);
        assert_eq!(detect_language(Path::new("foo.json")), None);
    }

    #[test]
    fn test_h_reclassification() {
        let path = Path::new("foo.h");
        assert_eq!(effective_language(path, Language::C, false), Language::C);
        assert_eq!(effective_language(path, Language::C, true), Language::Cpp);
    }

    #[test]
    fn test_has_cpp_source_files() {
        let files: Vec<PathBuf> = vec![
            PathBuf::from("a.ts"),
            PathBuf::from("b.cpp"),
        ];
        assert!(has_cpp_source_files(&files));

        let no_cpp: Vec<PathBuf> = vec![PathBuf::from("a.c"), PathBuf::from("b.h")];
        assert!(!has_cpp_source_files(&no_cpp));
    }
}
