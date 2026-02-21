use tree_sitter::{Language, Tree};
use super::{
    ClassInfo, ExportInfo, FunctionInfo, ImportInfo, LanguageAdapter,
    node_text, strip_quotes, walk_nodes,
};

pub struct GoAdapter;

impl GoAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAdapter for GoAdapter {
    fn language(&self) -> Language {
        tree_sitter_go::LANGUAGE.into()
    }

    fn extract_functions(&self, tree: &Tree, source: &[u8]) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            match node.kind() {
                "function_declaration" => {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        let name = node_text(name_node, source).to_string();
                        let params = node.child_by_field_name("parameters")
                            .map(|p| extract_go_params(p, source))
                            .unwrap_or_default();
                        let is_exported = is_go_exported(&name);
                        functions.push(FunctionInfo {
                            name,
                            start_line: node.start_position().row + 1,
                            end_line: node.end_position().row + 1,
                            params,
                            is_exported,
                        });
                    }
                }
                "method_declaration" => {
                    if let Some(name_node) = node.child_by_field_name("name") {
                        let name = node_text(name_node, source).to_string();
                        let params = node.child_by_field_name("parameters")
                            .map(|p| extract_go_params(p, source))
                            .unwrap_or_default();
                        let is_exported = is_go_exported(&name);
                        functions.push(FunctionInfo {
                            name,
                            start_line: node.start_position().row + 1,
                            end_line: node.end_position().row + 1,
                            params,
                            is_exported,
                        });
                    }
                }
                _ => {}
            }
        });
        functions
    }

    fn extract_imports(&self, tree: &Tree, source: &[u8]) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() != "import_spec" {
                return;
            }
            let mut path_node = None;
            let mut alias_node = None;
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                match child.kind() {
                    "interpreted_string_literal" => path_node = Some(child),
                    "package_identifier" | "identifier" | "dot" | "blank_identifier" => {
                        alias_node = Some(child);
                    }
                    _ => {}
                }
            }
            let path_n = match path_node {
                Some(n) => n,
                None => return,
            };
            let src = strip_quotes(node_text(path_n, source));
            let symbol = if let Some(alias) = alias_node {
                node_text(alias, source).to_string()
            } else {
                src.split('/').last().unwrap_or(&src).to_string()
            };
            imports.push(ImportInfo {
                source: src,
                names: vec![symbol],
                is_default: false,
            });
        });
        imports
    }

    fn extract_exports(&self, tree: &Tree, source: &[u8]) -> Vec<ExportInfo> {
        let mut exports = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            match node.kind() {
                "function_declaration" | "method_declaration" => {
                    if let Some(n) = node.child_by_field_name("name") {
                        let name = node_text(n, source).to_string();
                        if is_go_exported(&name) {
                            exports.push(ExportInfo { name, kind: "function".into() });
                        }
                    }
                }
                "type_spec" => {
                    if let Some(n) = node.child_by_field_name("name") {
                        let name = node_text(n, source).to_string();
                        if is_go_exported(&name) {
                            let kind = node.child_by_field_name("type")
                                .map(|t| match t.kind() {
                                    "struct_type" => "struct",
                                    "interface_type" => "interface",
                                    _ => "type",
                                })
                                .unwrap_or("type");
                            exports.push(ExportInfo { name, kind: kind.into() });
                        }
                    }
                }
                _ => {}
            }
        });
        exports
    }

    fn extract_classes(&self, tree: &Tree, source: &[u8]) -> Vec<ClassInfo> {
        let mut classes = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() != "type_spec" {
                return;
            }
            let type_node = match node.child_by_field_name("type") {
                Some(t) => t,
                None => return,
            };
            let kind = match type_node.kind() {
                "struct_type" => "struct",
                "interface_type" => "interface",
                _ => return,
            };
            if let Some(name_node) = node.child_by_field_name("name") {
                let decl_node = node.parent()
                    .filter(|p| p.kind() == "type_declaration")
                    .unwrap_or(node);
                classes.push(ClassInfo {
                    name: node_text(name_node, source).to_string(),
                    start_line: decl_node.start_position().row + 1,
                    end_line: decl_node.end_position().row + 1,
                    methods: Vec::new(),
                    kind: kind.into(),
                });
            }
        });
        classes
    }
}

fn is_go_exported(name: &str) -> bool {
    name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
}

fn extract_go_params(params_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        if child.kind() == "parameter_declaration" || child.kind() == "variadic_parameter_declaration" {
            // 参数名在第一个 identifier 子节点
            let mut c = child.walk();
            for p in child.children(&mut c) {
                if p.kind() == "identifier" {
                    params.push(node_text(p, source).to_string());
                    break;
                }
            }
        }
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> tree_sitter::Tree {
        let adapter = GoAdapter::new();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&adapter.language()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_go_extract_functions() {
        let src = r#"package main

func Hello(name string) string {
    return "Hello " + name
}

func helper() {}
"#;
        let tree = parse(src);
        let adapter = GoAdapter::new();
        let fns = adapter.extract_functions(&tree, src.as_bytes());
        assert!(fns.iter().any(|f| f.name == "Hello" && f.is_exported));
        assert!(fns.iter().any(|f| f.name == "helper" && !f.is_exported));
    }

    #[test]
    fn test_go_extract_imports() {
        let src = r#"package main

import (
    "fmt"
    "net/http"
)
"#;
        let tree = parse(src);
        let adapter = GoAdapter::new();
        let imports = adapter.extract_imports(&tree, src.as_bytes());
        assert!(imports.iter().any(|i| i.source == "fmt"));
        assert!(imports.iter().any(|i| i.source == "net/http"));
    }

    #[test]
    fn test_go_extract_structs() {
        let src = r#"package main

type Server struct {
    host string
    port int
}
"#;
        let tree = parse(src);
        let adapter = GoAdapter::new();
        let classes = adapter.extract_classes(&tree, src.as_bytes());
        assert_eq!(classes.len(), 1);
        assert_eq!(classes[0].name, "Server");
        assert_eq!(classes[0].kind, "struct");
    }
}
