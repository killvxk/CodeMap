use tree_sitter::{Language, Tree};
use super::{
    ClassInfo, ExportInfo, FunctionInfo, ImportInfo, LanguageAdapter,
    find_child_of_type, node_text, strip_quotes, walk_nodes,
};

/// JavaScript 适配器（复用 TypeScript 逻辑，使用 JS grammar）
pub struct JavaScriptAdapter;

impl JavaScriptAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAdapter for JavaScriptAdapter {
    fn language(&self) -> Language {
        tree_sitter_javascript::LANGUAGE.into()
    }

    fn extract_functions(&self, tree: &Tree, source: &[u8]) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() == "function_declaration" {
                if let Some(name_node) = node.child_by_field_name("name") {
                    let name = node_text(name_node, source).to_string();
                    let params = node.child_by_field_name("parameters")
                        .map(|p| extract_js_params(p, source))
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
            // export const foo = (...) => {}
            if node.kind() == "lexical_declaration" {
                let is_top = node.parent().map(|p| {
                    p.kind() == "program" || p.kind() == "export_statement"
                }).unwrap_or(false);
                if is_top {
                    let mut cursor = node.walk();
                    for child in node.children(&mut cursor) {
                        if child.kind() == "variable_declarator" {
                            if let Some(val) = child.child_by_field_name("value") {
                                if val.kind() == "arrow_function" {
                                    if let Some(name_node) = child.child_by_field_name("name") {
                                        let name = node_text(name_node, source).to_string();
                                        let params = val.child_by_field_name("parameters")
                                            .map(|p| extract_js_params(p, source))
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
            let src_node = node.child_by_field_name("source")
                .or_else(|| find_child_of_type(node, "string"));
            let src = match src_node {
                Some(n) => strip_quotes(node_text(n, source)),
                None => return,
            };
            let mut names = Vec::new();
            if let Some(clause) = find_child_of_type(node, "import_clause") {
                if let Some(named) = find_child_of_type(clause, "named_imports") {
                    let mut c = named.walk();
                    for spec in named.children(&mut c) {
                        if spec.kind() == "import_specifier" {
                            let n = spec.child_by_field_name("name")
                                .or_else(|| spec.named_child(0));
                            if let Some(n) = n {
                                names.push(node_text(n, source).to_string());
                            }
                        }
                    }
                }
                let mut c = clause.walk();
                for child in clause.children(&mut c) {
                    if child.kind() == "identifier" {
                        names.push(node_text(child, source).to_string());
                    }
                }
            }
            imports.push(ImportInfo { source: src, names, is_default: false });
        });
        imports
    }

    fn extract_exports(&self, tree: &Tree, source: &[u8]) -> Vec<ExportInfo> {
        let mut exports = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() != "export_statement" {
                return;
            }
            if let Some(func) = find_child_of_type(node, "function_declaration") {
                if let Some(n) = func.child_by_field_name("name") {
                    exports.push(ExportInfo { name: node_text(n, source).to_string(), kind: "function".into() });
                }
            }
            if let Some(cls) = find_child_of_type(node, "class_declaration") {
                if let Some(n) = cls.child_by_field_name("name") {
                    exports.push(ExportInfo { name: node_text(n, source).to_string(), kind: "class".into() });
                }
            }
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
            if node.kind() == "class_declaration" {
                if let Some(n) = node.child_by_field_name("name") {
                    let mut methods = Vec::new();
                    walk_nodes(node, &mut |child| {
                        if child.kind() == "method_definition" {
                            if let Some(mn) = child.child_by_field_name("name") {
                                methods.push(node_text(mn, source).to_string());
                            }
                        }
                    });
                    classes.push(ClassInfo {
                        name: node_text(n, source).to_string(),
                        start_line: node.start_position().row + 1,
                        end_line: node.end_position().row + 1,
                        methods,
                        kind: "class".into(),
                    });
                }
            }
        });
        classes
    }
}

fn extract_js_params(params_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let text = node_text(params_node, source);
    let inner = text.trim_start_matches('(').trim_end_matches(')');
    if inner.trim().is_empty() {
        return Vec::new();
    }
    inner.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> tree_sitter::Tree {
        let adapter = JavaScriptAdapter::new();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&adapter.language()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_js_extract_functions() {
        let src = r#"
export function hello(name) {
    return 'Hello ' + name;
}
const add = (a, b) => a + b;
"#;
        let tree = parse(src);
        let adapter = JavaScriptAdapter::new();
        let fns = adapter.extract_functions(&tree, src.as_bytes());
        assert!(fns.iter().any(|f| f.name == "hello" && f.is_exported));
    }

    #[test]
    fn test_js_extract_imports() {
        let src = "import { readFile } from 'fs';\nimport path from 'path';\n";
        let tree = parse(src);
        let adapter = JavaScriptAdapter::new();
        let imports = adapter.extract_imports(&tree, src.as_bytes());
        assert_eq!(imports.len(), 2);
        assert_eq!(imports[0].source, "fs");
    }
}
