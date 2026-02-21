use tree_sitter::{Language, Tree};
use super::{
    ClassInfo, ExportInfo, FunctionInfo, ImportInfo, LanguageAdapter,
    find_child_of_type, node_text, strip_quotes, walk_nodes,
};

pub struct TypeScriptAdapter {
    tsx: bool,
}

impl TypeScriptAdapter {
    pub fn new() -> Self {
        Self { tsx: false }
    }
    pub fn new_tsx() -> Self {
        Self { tsx: true }
    }
}

impl LanguageAdapter for TypeScriptAdapter {
    fn language(&self) -> Language {
        if self.tsx {
            tree_sitter_typescript::LANGUAGE_TSX.into()
        } else {
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
        }
    }

    fn extract_functions(&self, tree: &Tree, source: &[u8]) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() == "function_declaration" {
                if let Some(fn_info) = parse_function_declaration(node, source) {
                    functions.push(fn_info);
                }
            }
            // export const foo = (args) => { ... }
            if node.kind() == "lexical_declaration" {
                let parent = node.parent();
                let is_top_level = parent.map(|p| {
                    p.kind() == "program" || p.kind() == "export_statement"
                }).unwrap_or(false);
                if is_top_level {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "variable_declarator" {
                            let value = child.child_by_field_name("value");
                            if let Some(val) = value {
                                if val.kind() == "arrow_function" {
                                    if let Some(name_node) = child.child_by_field_name("name") {
                                        let name = node_text(name_node, source).to_string();
                                        let params = val.child_by_field_name("parameters")
                                            .map(|p| extract_params_text(p, source))
                                            .unwrap_or_default();
                                        let is_exported = node.parent()
                                            .map(|p| p.kind() == "export_statement")
                                            .unwrap_or(false);
                                        functions.push(FunctionInfo {
                                            name,
                                            start_line: node.start_position().row + 1,
                                            end_line: node.end_position().row + 1,
                                            params,
                                            is_exported,
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
        functions
    }

    fn extract_imports(&self, tree: &Tree, source: &[u8]) -> Vec<ImportInfo> {
        let mut imports = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() != "import_statement" {
                return;
            }
            let source_node = node.child_by_field_name("source")
                .or_else(|| find_child_of_type(node, "string"));
            let src = match source_node {
                Some(n) => strip_quotes(node_text(n, source)),
                None => return,
            };
            let mut names = Vec::new();
            if let Some(import_clause) = find_child_of_type(node, "import_clause") {
                // named imports { a, b }
                if let Some(named) = find_child_of_type(import_clause, "named_imports") {
                    let mut c = named.walk();
                    for spec in named.children(&mut c) {
                        if spec.kind() == "import_specifier" {
                            let name_node = spec.child_by_field_name("name")
                                .or_else(|| spec.named_child(0));
                            if let Some(n) = name_node {
                                names.push(node_text(n, source).to_string());
                            }
                        }
                    }
                }
                // default import
                let mut c = import_clause.walk();
                for child in import_clause.children(&mut c) {
                    if child.kind() == "identifier" {
                        names.push(node_text(child, source).to_string());
                    }
                }
            }
            imports.push(ImportInfo {
                source: src,
                names,
                is_default: false,
            });
        });
        imports
    }

    fn extract_exports(&self, tree: &Tree, source: &[u8]) -> Vec<ExportInfo> {
        let mut exports = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() != "export_statement" {
                return;
            }
            // export function foo
            if let Some(func) = find_child_of_type(node, "function_declaration") {
                if let Some(n) = func.child_by_field_name("name") {
                    exports.push(ExportInfo { name: node_text(n, source).to_string(), kind: "function".into() });
                }
            }
            // export class Foo
            if let Some(cls) = find_child_of_type(node, "class_declaration") {
                if let Some(n) = cls.child_by_field_name("name") {
                    exports.push(ExportInfo { name: node_text(n, source).to_string(), kind: "class".into() });
                }
            }
            // export interface Foo
            if let Some(iface) = find_child_of_type(node, "interface_declaration") {
                if let Some(n) = iface.child_by_field_name("name") {
                    exports.push(ExportInfo { name: node_text(n, source).to_string(), kind: "interface".into() });
                }
            }
            // export type Foo = ...
            if let Some(ta) = find_child_of_type(node, "type_alias_declaration") {
                if let Some(n) = ta.child_by_field_name("name") {
                    exports.push(ExportInfo { name: node_text(n, source).to_string(), kind: "type".into() });
                }
            }
            // export const/let/var
            if let Some(lex) = find_child_of_type(node, "lexical_declaration") {
                let mut c = lex.walk();
                for decl in lex.children(&mut c) {
                    if decl.kind() == "variable_declarator" {
                        if let Some(n) = decl.child_by_field_name("name") {
                            exports.push(ExportInfo { name: node_text(n, source).to_string(), kind: "variable".into() });
                        }
                    }
                }
            }
            // export { a, b }
            if let Some(clause) = find_child_of_type(node, "export_clause") {
                let mut c = clause.walk();
                for spec in clause.children(&mut c) {
                    if spec.kind() == "export_specifier" {
                        let n = spec.child_by_field_name("name")
                            .or_else(|| spec.named_child(0));
                        if let Some(n) = n {
                            exports.push(ExportInfo { name: node_text(n, source).to_string(), kind: "variable".into() });
                        }
                    }
                }
            }
        });
        exports
    }

    fn extract_classes(&self, tree: &Tree, source: &[u8]) -> Vec<ClassInfo> {
        let mut classes = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            match node.kind() {
                "class_declaration" => {
                    if let Some(n) = node.child_by_field_name("name") {
                        let methods = extract_class_methods(node, source);
                        classes.push(ClassInfo {
                            name: node_text(n, source).to_string(),
                            start_line: node.start_position().row + 1,
                            end_line: node.end_position().row + 1,
                            methods,
                            kind: "class".into(),
                        });
                    }
                }
                "interface_declaration" => {
                    if let Some(n) = node.child_by_field_name("name") {
                        classes.push(ClassInfo {
                            name: node_text(n, source).to_string(),
                            start_line: node.start_position().row + 1,
                            end_line: node.end_position().row + 1,
                            methods: Vec::new(),
                            kind: "interface".into(),
                        });
                    }
                }
                _ => {}
            }
        });
        classes
    }
}

fn parse_function_declaration(node: tree_sitter::Node, source: &[u8]) -> Option<FunctionInfo> {
    let name_node = node.child_by_field_name("name")?;
    let name = node_text(name_node, source).to_string();
    let params = node.child_by_field_name("parameters")
        .map(|p| extract_params_text(p, source))
        .unwrap_or_default();
    let is_exported = node.parent()
        .map(|p| p.kind() == "export_statement")
        .unwrap_or(false);
    Some(FunctionInfo {
        name,
        start_line: node.start_position().row + 1,
        end_line: node.end_position().row + 1,
        params,
        is_exported,
    })
}

fn extract_params_text(params_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let text = node_text(params_node, source);
    // 简单提取参数名：去掉括号，按逗号分割
    let inner = text.trim_start_matches('(').trim_end_matches(')');
    if inner.trim().is_empty() {
        return Vec::new();
    }
    inner.split(',')
        .map(|s| s.trim().split(':').next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn extract_class_methods(class_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut methods = Vec::new();
    walk_nodes(class_node, &mut |node| {
        if node.kind() == "method_definition" {
            if let Some(n) = node.child_by_field_name("name") {
                methods.push(node_text(n, source).to_string());
            }
        }
    });
    methods
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str, tsx: bool) -> tree_sitter::Tree {
        let adapter = if tsx { TypeScriptAdapter::new_tsx() } else { TypeScriptAdapter::new() };
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&adapter.language()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_ts_extract_functions() {
        let src = r#"
export function greet(name: string): string {
    return `Hello, ${name}`;
}
function helper() {}
"#;
        let tree = parse(src, false);
        let adapter = TypeScriptAdapter::new();
        let fns = adapter.extract_functions(&tree, src.as_bytes());
        assert_eq!(fns.len(), 2);
        assert_eq!(fns[0].name, "greet");
        assert!(fns[0].is_exported);
        assert_eq!(fns[1].name, "helper");
        assert!(!fns[1].is_exported);
    }

    #[test]
    fn test_ts_extract_imports() {
        let src = r#"import { foo, bar } from './utils';
import React from 'react';
"#;
        let tree = parse(src, false);
        let adapter = TypeScriptAdapter::new();
        let imports = adapter.extract_imports(&tree, src.as_bytes());
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].source, "./utils");
        assert!(imports[0].names.contains(&"foo".to_string()));
        assert!(imports[0].names.contains(&"bar".to_string()));
        assert_eq!(imports[1].source, "react");
    }

    #[test]
    fn test_ts_extract_exports() {
        let src = r#"
export function myFunc() {}
export class MyClass {}
export interface MyInterface {}
export type MyType = string;
export const MY_CONST = 42;
"#;
        let tree = parse(src, false);
        let adapter = TypeScriptAdapter::new();
        let exports = adapter.extract_exports(&tree, src.as_bytes());
        let names: Vec<&str> = exports.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"myFunc"));
        assert!(names.contains(&"MyClass"));
        assert!(names.contains(&"MyInterface"));
        assert!(names.contains(&"MyType"));
        assert!(names.contains(&"MY_CONST"));
    }

    #[test]
    fn test_ts_extract_classes() {
        let src = r#"
class Animal {
    speak() {}
    move() {}
}
interface Runnable {}
"#;
        let tree = parse(src, false);
        let adapter = TypeScriptAdapter::new();
        let classes = adapter.extract_classes(&tree, src.as_bytes());
        assert!(classes.iter().any(|c| c.name == "Animal" && c.kind == "class"));
        assert!(classes.iter().any(|c| c.name == "Runnable" && c.kind == "interface"));
        let animal = classes.iter().find(|c| c.name == "Animal").unwrap();
        assert!(animal.methods.contains(&"speak".to_string()));
        assert!(animal.methods.contains(&"move".to_string()));
    }
}
