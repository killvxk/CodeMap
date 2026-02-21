use tree_sitter::{Language, Tree};
use super::{
    ClassInfo, ExportInfo, FunctionInfo, ImportInfo, LanguageAdapter,
    node_text, strip_quotes, walk_nodes,
};

pub struct PythonAdapter;

impl PythonAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAdapter for PythonAdapter {
    fn language(&self) -> Language {
        tree_sitter_python::LANGUAGE.into()
    }

    fn extract_functions(&self, tree: &Tree, source: &[u8]) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            // 只处理顶层函数（含 decorated_definition 包装）
            let func_node = unwrap_decorated(child, "function_definition");
            if let Some(func) = func_node {
                if let Some(name_node) = func.child_by_field_name("name") {
                    let name = node_text(name_node, source).to_string();
                    let params = func.child_by_field_name("parameters")
                        .map(|p| extract_python_params(p, source))
                        .unwrap_or_default();
                    functions.push(FunctionInfo {
                        name,
                        start_line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        params,
                        is_exported: true, // Python 默认公开
                    });
                }
            }
        }
        functions
    }

    fn extract_imports(&self, tree: &Tree, source: &[u8]) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            match node.kind() {
                "import_statement" => {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        match child.kind() {
                            "dotted_name" => {
                                let name = node_text(child, source).to_string();
                                imports.push(ImportInfo {
                                    source: name.clone(),
                                    names: vec![name],
                                    is_default: false,
                                });
                            }
                            "aliased_import" => {
                                let name_node = child.named_child(0);
                                if let Some(n) = name_node {
                                    let name = node_text(n, source).to_string();
                                    imports.push(ImportInfo {
                                        source: name.clone(),
                                        names: vec![name],
                                        is_default: false,
                                    });
                                }
                            }
                            _ => {}
                        }
                    }
                }
                "import_from_statement" => {
                    let module = node.child_by_field_name("module_name")
                        .map(|n| node_text(n, source).to_string())
                        .unwrap_or_default();
                    let mut names = Vec::new();
                    let mut past_import = false;
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "import" {
                            past_import = true;
                            continue;
                        }
                        if !past_import {
                            continue;
                        }
                        match child.kind() {
                            "dotted_name" | "identifier" => {
                                names.push(node_text(child, source).to_string());
                            }
                            "aliased_import" => {
                                if let Some(n) = child.named_child(0) {
                                    names.push(node_text(n, source).to_string());
                                }
                            }
                            "wildcard_import" => {
                                names.push("*".to_string());
                            }
                            _ => {}
                        }
                    }
                    imports.push(ImportInfo {
                        source: module,
                        names,
                        is_default: false,
                    });
                }
                _ => {}
            }
        });
        imports
    }

    fn extract_exports(&self, tree: &Tree, source: &[u8]) -> Vec<ExportInfo> {
        // 先尝试 __all__
        if let Some(all_exports) = extract_dunder_all(tree, source) {
            return all_exports.into_iter()
                .map(|name| ExportInfo { name, kind: "variable".into() })
                .collect();
        }
        // 回退：所有顶层函数和类
        let mut exports = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if let Some(func) = unwrap_decorated(child, "function_definition") {
                if let Some(n) = func.child_by_field_name("name") {
                    exports.push(ExportInfo {
                        name: node_text(n, source).to_string(),
                        kind: "function".into(),
                    });
                }
            } else if let Some(cls) = unwrap_decorated(child, "class_definition") {
                if let Some(n) = cls.child_by_field_name("name") {
                    exports.push(ExportInfo {
                        name: node_text(n, source).to_string(),
                        kind: "class".into(),
                    });
                }
            }
        }
        exports
    }

    fn extract_classes(&self, tree: &Tree, source: &[u8]) -> Vec<ClassInfo> {
        let mut classes = Vec::new();
        let root = tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if let Some(cls) = unwrap_decorated(child, "class_definition") {
                if let Some(name_node) = cls.child_by_field_name("name") {
                    let methods = extract_class_methods(cls, source);
                    classes.push(ClassInfo {
                        name: node_text(name_node, source).to_string(),
                        start_line: child.start_position().row + 1,
                        end_line: child.end_position().row + 1,
                        methods,
                        kind: "class".into(),
                    });
                }
            }
        }
        classes
    }
}

fn unwrap_decorated<'a>(node: tree_sitter::Node<'a>, expected: &str) -> Option<tree_sitter::Node<'a>> {
    if node.kind() == expected {
        return Some(node);
    }
    if node.kind() == "decorated_definition" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == expected {
                return Some(child);
            }
        }
    }
    None
}

fn extract_python_params(params_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        match child.kind() {
            "identifier" => params.push(node_text(child, source).to_string()),
            "typed_parameter" | "default_parameter" | "typed_default_parameter" => {
                if let Some(n) = child.named_child(0) {
                    params.push(node_text(n, source).to_string());
                }
            }
            _ => {}
        }
    }
    params
}

fn extract_class_methods(class_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut methods = Vec::new();
    if let Some(body) = class_node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if let Some(func) = unwrap_decorated(child, "function_definition") {
                if let Some(n) = func.child_by_field_name("name") {
                    methods.push(node_text(n, source).to_string());
                }
            }
        }
    }
    methods
}

fn extract_dunder_all(tree: &tree_sitter::Tree, source: &[u8]) -> Option<Vec<String>> {
    let root = tree.root_node();
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        // expression_statement > assignment
        if child.kind() == "expression_statement" {
            if let Some(expr) = child.named_child(0) {
                if expr.kind() == "assignment" {
                    if let Some(left) = expr.child_by_field_name("left") {
                        if node_text(left, source) == "__all__" {
                            if let Some(right) = expr.child_by_field_name("right") {
                                return extract_list_strings(right, source);
                            }
                        }
                    }
                }
            }
        }
        // 直接 assignment
        if child.kind() == "assignment" {
            if let Some(left) = child.child_by_field_name("left") {
                if node_text(left, source) == "__all__" {
                    if let Some(right) = child.child_by_field_name("right") {
                        return extract_list_strings(right, source);
                    }
                }
            }
        }
    }
    None
}

fn extract_list_strings(list_node: tree_sitter::Node, source: &[u8]) -> Option<Vec<String>> {
    if list_node.kind() != "list" {
        return None;
    }
    let mut strings = Vec::new();
    let mut cursor = list_node.walk();
    for child in list_node.children(&mut cursor) {
        if child.kind() == "string" {
            strings.push(strip_quotes(node_text(child, source)));
        }
    }
    Some(strings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> tree_sitter::Tree {
        let adapter = PythonAdapter::new();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&adapter.language()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_python_extract_functions() {
        let src = r#"
def greet(name):
    return f"Hello, {name}"

def helper():
    pass
"#;
        let tree = parse(src);
        let adapter = PythonAdapter::new();
        let fns = adapter.extract_functions(&tree, src.as_bytes());
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].name, "greet");
        assert_eq!(fns[1].name, "helper");
    }

    #[test]
    fn test_python_extract_imports() {
        let src = "import os\nfrom pathlib import Path\nfrom . import utils\n";
        let tree = parse(src);
        let adapter = PythonAdapter::new();
        let imports = adapter.extract_imports(&tree, src.as_bytes());
        assert!(imports.iter().any(|i| i.source == "os"));
        assert!(imports.iter().any(|i| i.source == "pathlib" && i.names.contains(&"Path".to_string())));
    }

    #[test]
    fn test_python_extract_classes() {
        let src = r#"
class Animal:
    def speak(self):
        pass
    def move(self):
        pass
"#;
        let tree = parse(src);
        let adapter = PythonAdapter::new();
        let classes = adapter.extract_classes(&tree, src.as_bytes());
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "Animal");
        assert!(classes[0].methods.contains(&"speak".to_string()));
    }

    #[test]
    fn test_python_dunder_all() {
        let src = r#"
__all__ = ["foo", "bar"]
def foo(): pass
def bar(): pass
def _private(): pass
"#;
        let tree = parse(src);
        let adapter = PythonAdapter::new();
        let exports = adapter.extract_exports(&tree, src.as_bytes());
        assert_eq!(exports.len(), 2);
        assert!(exports.iter().any(|e| e.name == "foo"));
        assert!(exports.iter().any(|e| e.name == "bar"));
    }
}
