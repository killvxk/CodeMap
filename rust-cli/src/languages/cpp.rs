use tree_sitter::{Language, Tree};
use super::{
    ClassInfo, ExportInfo, FunctionInfo, ImportInfo, LanguageAdapter,
    find_descendant_of_type, node_text, walk_nodes,
};
use super::c_lang::{extract_c_includes, extract_c_functions, extract_c_exports, extract_c_classes};

pub struct CppAdapter;

impl CppAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAdapter for CppAdapter {
    fn language(&self) -> Language {
        tree_sitter_cpp::LANGUAGE.into()
    }

    fn extract_functions(&self, tree: &Tree, source: &[u8]) -> Vec<FunctionInfo> {
        // C++ 函数提取与 C 相同，但名称可能含 :: 限定符
        extract_c_functions(tree, source)
    }

    fn extract_imports(&self, tree: &Tree, source: &[u8]) -> Vec<ImportInfo> {
        extract_c_includes(tree, source)
    }

    fn extract_exports(&self, tree: &Tree, source: &[u8]) -> Vec<ExportInfo> {
        extract_c_exports(tree, source)
    }

    fn extract_classes(&self, tree: &Tree, source: &[u8]) -> Vec<ClassInfo> {
        let mut classes = extract_c_classes(tree, source);
        // 额外处理 C++ 类方法
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() != "class_specifier" {
                return;
            }
            if node.child_by_field_name("body").is_none() {
                return;
            }
            if let Some(name_node) = node.child_by_field_name("name") {
                let class_name = node_text(name_node, source).to_string();
                if let Some(ci) = classes.iter_mut().find(|c| c.name == class_name && c.kind == "class") {
                    ci.methods = extract_cpp_methods(node, source);
                }
            }
        });
        // 添加 enum（与 Node.js 一致）
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() != "enum_specifier" {
                return;
            }
            if node.child_by_field_name("body").is_none() {
                return;
            }
            if let Some(n) = node.child_by_field_name("name") {
                classes.push(ClassInfo {
                    name: node_text(n, source).to_string(),
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    methods: Vec::new(),
                    kind: "enum".into(),
                });
            }
        });
        // 添加 namespace（与 Node.js 一致）
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() != "namespace_definition" {
                return;
            }
            if let Some(n) = node.child_by_field_name("name") {
                classes.push(ClassInfo {
                    name: node_text(n, source).to_string(),
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    methods: Vec::new(),
                    kind: "namespace".into(),
                });
            }
        });
        classes
    }
}

fn extract_cpp_methods(class_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut methods = Vec::new();
    walk_nodes(class_node, &mut |node| {
        if node.kind() == "function_definition" {
            if let Some(func_decl) = find_descendant_of_type(node, "function_declarator") {
                if let Some(name_node) = func_decl.child_by_field_name("declarator") {
                    methods.push(node_text(name_node, source).to_string());
                }
            }
        }
    });
    methods
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> tree_sitter::Tree {
        let adapter = CppAdapter::new();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&adapter.language()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_cpp_extract_functions() {
        let src = r#"
#include <string>

class Engine {
public:
    void start() {}
    void stop() {}
};

int main() {
    return 0;
}
"#;
        let tree = parse(src);
        let adapter = CppAdapter::new();
        let fns = adapter.extract_functions(&tree, src.as_bytes());
        assert!(fns.iter().any(|f| f.name == "main"));
    }

    #[test]
    fn test_cpp_extract_includes() {
        let src = "#include <vector>\n#include \"engine.h\"\n";
        let tree = parse(src);
        let adapter = CppAdapter::new();
        let imports = adapter.extract_imports(&tree, src.as_bytes());
        assert!(imports.iter().any(|i| i.source == "vector" && i.is_default));
        assert!(imports.iter().any(|i| i.source == "engine.h" && !i.is_default));
    }

    #[test]
    fn test_cpp_extract_classes() {
        let src = r#"
class Animal {
public:
    void speak() {}
};

struct Point {
    int x, y;
};
"#;
        let tree = parse(src);
        let adapter = CppAdapter::new();
        let classes = adapter.extract_classes(&tree, src.as_bytes());
        assert!(classes.iter().any(|c| c.name == "Animal" && c.kind == "class"));
        assert!(classes.iter().any(|c| c.name == "Point" && c.kind == "struct"));
    }

    #[test]
    fn test_cpp_namespace_exports() {
        let src = r#"
namespace MyLib {
    void helper() {}
}
"#;
        let tree = parse(src);
        let adapter = CppAdapter::new();
        let exports = adapter.extract_exports(&tree, src.as_bytes());
        // Node.js 行为：namespace 本身不出现在 exports 中
        assert!(!exports.iter().any(|e| e.name == "MyLib" && e.kind == "namespace"),
            "namespace should not appear in exports");
    }
}
