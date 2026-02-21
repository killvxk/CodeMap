/// tree-sitter 语法加载验证
/// 验证 8 种语言（含 TSX）的语法可以正常加载
#[cfg(test)]
mod grammar_tests {
    use tree_sitter::Language;

    fn lang_typescript() -> Language {
        tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
    }

    fn lang_tsx() -> Language {
        tree_sitter_typescript::LANGUAGE_TSX.into()
    }

    fn lang_javascript() -> Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn lang_python() -> Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn lang_go() -> Language {
        tree_sitter_go::LANGUAGE.into()
    }

    fn lang_rust() -> Language {
        tree_sitter_rust::LANGUAGE.into()
    }

    fn lang_java() -> Language {
        tree_sitter_java::LANGUAGE.into()
    }

    fn lang_c() -> Language {
        tree_sitter_c::LANGUAGE.into()
    }

    fn lang_cpp() -> Language {
        tree_sitter_cpp::LANGUAGE.into()
    }

    #[test]
    fn test_typescript_grammar_loads() {
        let lang = lang_typescript();
        assert!(lang.node_kind_count() > 0);
    }

    #[test]
    fn test_tsx_grammar_loads() {
        let lang = lang_tsx();
        assert!(lang.node_kind_count() > 0);
    }

    #[test]
    fn test_javascript_grammar_loads() {
        let lang = lang_javascript();
        assert!(lang.node_kind_count() > 0);
    }

    #[test]
    fn test_python_grammar_loads() {
        let lang = lang_python();
        assert!(lang.node_kind_count() > 0);
    }

    #[test]
    fn test_go_grammar_loads() {
        let lang = lang_go();
        assert!(lang.node_kind_count() > 0);
    }

    #[test]
    fn test_rust_grammar_loads() {
        let lang = lang_rust();
        assert!(lang.node_kind_count() > 0);
    }

    #[test]
    fn test_java_grammar_loads() {
        let lang = lang_java();
        assert!(lang.node_kind_count() > 0);
    }

    #[test]
    fn test_c_grammar_loads() {
        let lang = lang_c();
        assert!(lang.node_kind_count() > 0);
    }

    #[test]
    fn test_cpp_grammar_loads() {
        let lang = lang_cpp();
        assert!(lang.node_kind_count() > 0);
    }

    #[test]
    fn test_parse_simple_typescript() {
        let lang = lang_typescript();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).expect("Failed to set TypeScript language");
        let tree = parser.parse("const x: number = 42;", None);
        assert!(tree.is_some());
    }

    #[test]
    fn test_parse_simple_python() {
        let lang = lang_python();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&lang).expect("Failed to set Python language");
        let tree = parser.parse("def hello(): pass", None);
        assert!(tree.is_some());
    }
}
