use crate::graph::{ClassInfo, FunctionInfo, ImportInfo, TypeInfo};
use crate::traverser::Language;
use std::path::Path;
use tree_sitter::Tree;

// ── LanguageAdapter trait ─────────────────────────────────────────────────────

/// 语言适配器 trait（已被 languages/mod.rs 的 LanguageAdapter 取代，仅测试使用）
#[allow(dead_code)]
pub trait LanguageAdapter: Send + Sync {
    fn language(&self) -> tree_sitter::Language;
    fn extract_functions(&self, tree: &Tree, source: &[u8]) -> Vec<FunctionInfo>;
    fn extract_imports(&self, tree: &Tree, source: &[u8]) -> Vec<ImportInfo>;
    fn extract_exports(&self, tree: &Tree, source: &[u8]) -> Vec<String>;
    fn extract_classes(&self, tree: &Tree, source: &[u8]) -> Vec<ClassInfo>;
    fn extract_types(&self, tree: &Tree, source: &[u8]) -> Vec<TypeInfo>;
}

// ── ParseResult ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ParseResult {
    pub functions: Vec<FunctionInfo>,
    pub imports: Vec<ImportInfo>,
    pub exports: Vec<String>,
    pub classes: Vec<ClassInfo>,
    pub types: Vec<TypeInfo>,
    pub lines: u32,
}

// ── 默认适配器（已被 languages/ 下的具体适配器取代，仅测试使用）────────────

#[allow(dead_code)]
pub struct DefaultAdapter {
    lang: tree_sitter::Language,
}

impl DefaultAdapter {
    #[allow(dead_code)]
    pub fn new(lang: tree_sitter::Language) -> Self {
        Self { lang }
    }
}

impl LanguageAdapter for DefaultAdapter {
    fn language(&self) -> tree_sitter::Language {
        self.lang.clone()
    }
    fn extract_functions(&self, _tree: &Tree, _source: &[u8]) -> Vec<FunctionInfo> {
        vec![]
    }
    fn extract_imports(&self, _tree: &Tree, _source: &[u8]) -> Vec<ImportInfo> {
        vec![]
    }
    fn extract_exports(&self, _tree: &Tree, _source: &[u8]) -> Vec<String> {
        vec![]
    }
    fn extract_classes(&self, _tree: &Tree, _source: &[u8]) -> Vec<ClassInfo> {
        vec![]
    }
    fn extract_types(&self, _tree: &Tree, _source: &[u8]) -> Vec<TypeInfo> {
        vec![]
    }
}

// ── 解析器 ────────────────────────────────────────────────────────────────────

/// 解析单个源文件，返回结构化信息（已被 scanner.rs 直接调用 languages 适配器取代）
#[allow(dead_code)]
pub fn parse_file(
    _file_path: &Path,
    language: Language,
    source: &[u8],
    adapter: &dyn LanguageAdapter,
) -> anyhow::Result<ParseResult> {
    let ts_lang = adapter.language();
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&ts_lang)
        .map_err(|e| anyhow::anyhow!("set_language failed for {:?}: {}", language, e))?;

    let tree = parser
        .parse(source, None)
        .ok_or_else(|| anyhow::anyhow!("parse returned None for {:?}", language))?;

    let functions = adapter.extract_functions(&tree, source);
    let imports = adapter.extract_imports(&tree, source);
    let exports = adapter.extract_exports(&tree, source);
    let classes = adapter.extract_classes(&tree, source);
    let types = adapter.extract_types(&tree, source);
    let lines = source.iter().filter(|&&b| b == b'\n').count() as u32 + 1;

    Ok(ParseResult {
        functions,
        imports,
        exports,
        classes,
        types,
        lines,
    })
}

/// 根据语言枚举获取对应的 tree-sitter Language（仅测试使用）
#[allow(dead_code)]
pub fn get_ts_language(language: Language) -> tree_sitter::Language {
    match language {
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        Language::Go => tree_sitter_go::LANGUAGE.into(),
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::Java => tree_sitter_java::LANGUAGE.into(),
        Language::C => tree_sitter_c::LANGUAGE.into(),
        Language::Cpp => tree_sitter_cpp::LANGUAGE.into(),
    }
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_empty_js() {
        let lang = Language::JavaScript;
        let ts_lang = get_ts_language(lang);
        let adapter = DefaultAdapter::new(ts_lang);
        let result = parse_file(Path::new("test.js"), lang, b"", &adapter).unwrap();
        assert_eq!(result.lines, 1);
        assert!(result.functions.is_empty());
    }

    #[test]
    fn test_parse_simple_rust() {
        let lang = Language::Rust;
        let ts_lang = get_ts_language(lang);
        let adapter = DefaultAdapter::new(ts_lang);
        let src = b"fn main() { println!(\"hello\"); }";
        let result = parse_file(Path::new("main.rs"), lang, src, &adapter).unwrap();
        assert_eq!(result.lines, 1);
    }

    #[test]
    fn test_line_count() {
        let lang = Language::Python;
        let ts_lang = get_ts_language(lang);
        let adapter = DefaultAdapter::new(ts_lang);
        let src = b"a = 1\nb = 2\nc = 3\n";
        let result = parse_file(Path::new("test.py"), lang, src, &adapter).unwrap();
        assert_eq!(result.lines, 4); // 3 newlines + 1
    }
}
