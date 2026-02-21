use tree_sitter::{Language, Tree};
use super::{
    ClassInfo, ExportInfo, FunctionInfo, ImportInfo, LanguageAdapter,
    node_text, walk_nodes,
};

pub struct RustAdapter;

impl RustAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAdapter for RustAdapter {
    fn language(&self) -> Language {
        tree_sitter_rust::LANGUAGE.into()
    }

    fn extract_functions(&self, tree: &Tree, source: &[u8]) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() != "function_item" {
                return;
            }
            let name_node = match node.child_by_field_name("name") {
                Some(n) => n,
                None => return,
            };
            let impl_type = get_impl_type(node, source);
            let name = if let Some(ref t) = impl_type {
                format!("{}::{}", t, node_text(name_node, source))
            } else {
                node_text(name_node, source).to_string()
            };
            let params = node.child_by_field_name("parameters")
                .map(|p| extract_rust_params(p, source))
                .unwrap_or_default();
            let is_exported = has_pub_visibility(node, source);
            functions.push(FunctionInfo {
                name,
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                params,
                is_exported,
            });
        });
        functions
    }

    fn extract_imports(&self, tree: &Tree, source: &[u8]) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() != "use_declaration" {
                return;
            }
            let mut result = ImportInfo {
                source: String::new(),
                names: Vec::new(),
                is_default: false,
            };
            parse_use_tree(node, source, &mut result);
            if !result.source.is_empty() {
                imports.push(result);
            }
        });
        imports
    }

    fn extract_exports(&self, tree: &Tree, source: &[u8]) -> Vec<ExportInfo> {
        let mut exports = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            if !has_pub_visibility(node, source) {
                return;
            }
            if is_inside_impl(node) {
                return;
            }
            let kind = match node.kind() {
                "function_item" => "function",
                "struct_item" => "struct",
                "enum_item" => "enum",
                "trait_item" => "trait",
                "type_item" => "type",
                "mod_item" => "module",
                _ => return,
            };
            if let Some(n) = node.child_by_field_name("name") {
                exports.push(ExportInfo {
                    name: node_text(n, source).to_string(),
                    kind: kind.into(),
                });
            }
        });
        exports
    }

    fn extract_classes(&self, tree: &Tree, source: &[u8]) -> Vec<ClassInfo> {
        let mut classes = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            match node.kind() {
                "struct_item" => {
                    if let Some(n) = node.child_by_field_name("name") {
                        classes.push(ClassInfo {
                            name: node_text(n, source).to_string(),
                            start_line: node.start_position().row + 1,
                            end_line: node.end_position().row + 1,
                            methods: Vec::new(),
                            kind: "struct".into(),
                        });
                    }
                }
                "enum_item" => {
                    if let Some(n) = node.child_by_field_name("name") {
                        classes.push(ClassInfo {
                            name: node_text(n, source).to_string(),
                            start_line: node.start_position().row + 1,
                            end_line: node.end_position().row + 1,
                            methods: Vec::new(),
                            kind: "enum".into(),
                        });
                    }
                }
                "trait_item" => {
                    if let Some(n) = node.child_by_field_name("name") {
                        classes.push(ClassInfo {
                            name: node_text(n, source).to_string(),
                            start_line: node.start_position().row + 1,
                            end_line: node.end_position().row + 1,
                            methods: Vec::new(),
                            kind: "trait".into(),
                        });
                    }
                }
                _ => {}
            }
        });
        classes
    }
}

fn get_impl_type(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut current = node.parent();
    while let Some(n) = current {
        if n.kind() == "impl_item" {
            return n.child_by_field_name("type")
                .map(|t| node_text(t, source).to_string());
        }
        current = n.parent();
    }
    None
}

fn has_pub_visibility(node: tree_sitter::Node, source: &[u8]) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            return node_text(child, source).contains("pub");
        }
    }
    false
}

fn is_inside_impl(node: tree_sitter::Node) -> bool {
    let mut current = node.parent();
    while let Some(n) = current {
        if n.kind() == "impl_item" {
            return true;
        }
        current = n.parent();
    }
    false
}

fn extract_rust_params(params_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        match child.kind() {
            "parameter" => {
                if let Some(pat) = child.child_by_field_name("pattern") {
                    params.push(node_text(pat, source).to_string());
                }
            }
            "self_parameter" | "variadic_parameter" => {
                params.push(node_text(child, source).to_string());
            }
            _ => {}
        }
    }
    params
}

fn parse_use_tree(node: tree_sitter::Node, source: &[u8], result: &mut ImportInfo) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "scoped_identifier" | "scoped_use_list" => {
                if let Some(path) = child.child_by_field_name("path") {
                    result.source = node_text(path, source).to_string();
                }
                if let Some(list) = super::find_child_of_type(child, "use_list") {
                    extract_use_list_symbols(list, source, &mut result.names);
                } else if let Some(name) = child.child_by_field_name("name") {
                    result.names.push(node_text(name, source).to_string());
                }
                return;
            }
            "identifier" => {
                result.source = node_text(child, source).to_string();
                result.names.push(result.source.clone());
                return;
            }
            "use_list" => {
                extract_use_list_symbols(child, source, &mut result.names);
            }
            _ => {}
        }
    }
}

fn extract_use_list_symbols(list_node: tree_sitter::Node, source: &[u8], symbols: &mut Vec<String>) {
    let mut cursor = list_node.walk();
    for child in list_node.children(&mut cursor) {
        match child.kind() {
            "identifier" | "self" => {
                symbols.push(node_text(child, source).to_string());
            }
            "scoped_identifier" => {
                if let Some(n) = child.child_by_field_name("name") {
                    symbols.push(node_text(n, source).to_string());
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> tree_sitter::Tree {
        let adapter = RustAdapter::new();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&adapter.language()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_rust_extract_functions() {
        let src = r#"
pub fn greet(name: &str) -> String {
    format!("Hello, {}", name)
}

fn helper() {}

struct Foo;
impl Foo {
    pub fn method(&self) {}
}
"#;
        let tree = parse(src);
        let adapter = RustAdapter::new();
        let fns = adapter.extract_functions(&tree, src.as_bytes());
        assert!(fns.iter().any(|f| f.name == "greet" && f.is_exported));
        assert!(fns.iter().any(|f| f.name == "helper" && !f.is_exported));
        assert!(fns.iter().any(|f| f.name == "Foo::method"));
    }

    #[test]
    fn test_rust_extract_imports() {
        let src = "use std::io::{Read, Write};\nuse crate::utils;\n";
        let tree = parse(src);
        let adapter = RustAdapter::new();
        let imports = adapter.extract_imports(&tree, src.as_bytes());
        assert!(imports.iter().any(|i| i.source == "std::io"));
    }

    #[test]
    fn test_rust_extract_classes() {
        let src = r#"
pub struct Server {
    host: String,
}
pub enum Status { Ok, Err }
pub trait Handler {}
"#;
        let tree = parse(src);
        let adapter = RustAdapter::new();
        let classes = adapter.extract_classes(&tree, src.as_bytes());
        assert!(classes.iter().any(|c| c.name == "Server" && c.kind == "struct"));
        assert!(classes.iter().any(|c| c.name == "Status" && c.kind == "enum"));
        assert!(classes.iter().any(|c| c.name == "Handler" && c.kind == "trait"));
    }
}
