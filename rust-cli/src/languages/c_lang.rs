use tree_sitter::{Language, Tree};
use super::{
    ClassInfo, ExportInfo, FunctionInfo, ImportInfo, LanguageAdapter,
    find_descendant_of_type, node_text, walk_nodes,
};

pub struct CAdapter;

impl CAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAdapter for CAdapter {
    fn language(&self) -> Language {
        tree_sitter_c::LANGUAGE.into()
    }

    fn extract_functions(&self, tree: &Tree, source: &[u8]) -> Vec<FunctionInfo> {
        extract_c_functions(tree, source)
    }

    fn extract_imports(&self, tree: &Tree, source: &[u8]) -> Vec<ImportInfo> {
        extract_c_includes(tree, source)
    }

    fn extract_exports(&self, tree: &Tree, source: &[u8]) -> Vec<ExportInfo> {
        extract_c_exports(tree, source)
    }

    fn extract_classes(&self, tree: &Tree, source: &[u8]) -> Vec<ClassInfo> {
        extract_c_classes(tree, source)
    }
}

// ---------------------------------------------------------------------------
// 共享实现（C 和 C++ 共用）
// ---------------------------------------------------------------------------

pub fn extract_c_functions(tree: &Tree, source: &[u8]) -> Vec<FunctionInfo> {
    let mut functions = Vec::new();
    walk_nodes(tree.root_node(), &mut |node| {
        if node.kind() != "function_definition" {
            return;
        }
        let func_decl = match find_descendant_of_type(node, "function_declarator") {
            Some(n) => n,
            None => return,
        };
        let name_node = match func_decl.child_by_field_name("declarator") {
            Some(n) => n,
            None => return,
        };
        let name = node_text(name_node, source).to_string();
        let is_static = has_storage_class_static(node, source);
        let params = func_decl.child_by_field_name("parameters")
            .map(|p| extract_c_params(p, source))
            .unwrap_or_default();
        functions.push(FunctionInfo {
            name,
            start_line: node.start_position().row + 1,
            end_line: node.end_position().row + 1,
            params,
            is_exported: !is_static,
        });
    });
    functions
}

pub fn extract_c_includes(tree: &Tree, source: &[u8]) -> Vec<ImportInfo> {
    let mut imports = Vec::new();
    walk_nodes(tree.root_node(), &mut |node| {
        if node.kind() != "preproc_include" {
            return;
        }
        let path_node = super::find_child_of_type(node, "system_lib_string")
            .or_else(|| super::find_child_of_type(node, "string_literal"));
        let path_n = match path_node {
            Some(n) => n,
            None => return,
        };
        let is_system = path_n.kind() == "system_lib_string";
        let raw = node_text(path_n, source)
            .trim_matches(|c| c == '<' || c == '>' || c == '"')
            .to_string();
        imports.push(ImportInfo {
            source: raw,
            names: Vec::new(),
            is_default: is_system,
        });
    });
    imports
}

pub fn extract_c_exports(tree: &Tree, source: &[u8]) -> Vec<ExportInfo> {
    let mut exports = Vec::new();
    let mut seen = std::collections::HashSet::new();
    walk_nodes(tree.root_node(), &mut |node| {
        match node.kind() {
            "function_definition" => {
                if has_storage_class_static(node, source) {
                    return;
                }
                if let Some(func_decl) = find_descendant_of_type(node, "function_declarator") {
                    if let Some(name_node) = func_decl.child_by_field_name("declarator") {
                        let name = bare_identifier(node_text(name_node, source));
                        if seen.insert(name.clone()) {
                            exports.push(ExportInfo { name, kind: "function".into() });
                        }
                    }
                }
            }
            "struct_specifier" | "class_specifier" => {
                if node.child_by_field_name("body").is_none() {
                    return; // 跳过前向声明
                }
                if let Some(n) = node.child_by_field_name("name") {
                    let name = node_text(n, source).to_string();
                    if seen.insert(name.clone()) {
                        exports.push(ExportInfo { name, kind: "struct".into() });
                    }
                }
            }
            "enum_specifier" => {
                if let Some(n) = node.child_by_field_name("name") {
                    let name = node_text(n, source).to_string();
                    if seen.insert(name.clone()) {
                        exports.push(ExportInfo { name, kind: "enum".into() });
                    }
                }
            }
            "type_definition" => {
                if let Some(n) = find_descendant_of_type(node, "type_identifier") {
                    let name = node_text(n, source).to_string();
                    if seen.insert(name.clone()) {
                        exports.push(ExportInfo { name, kind: "typedef".into() });
                    }
                }
            }
            _ => {}
        }
    });
    exports
}

pub fn extract_c_classes(tree: &Tree, source: &[u8]) -> Vec<ClassInfo> {
    let mut classes = Vec::new();
    walk_nodes(tree.root_node(), &mut |node| {
        match node.kind() {
            "struct_specifier" | "class_specifier" => {
                if node.child_by_field_name("body").is_none() {
                    return;
                }
                if let Some(n) = node.child_by_field_name("name") {
                    let kind = if node.kind() == "class_specifier" { "class" } else { "struct" };
                    classes.push(ClassInfo {
                        name: node_text(n, source).to_string(),
                        start_line: node.start_position().row + 1,
                        end_line: node.end_position().row + 1,
                        methods: Vec::new(),
                        kind: kind.into(),
                    });
                }
            }
            _ => {}
        }
    });
    classes
}

fn has_storage_class_static(func_def: tree_sitter::Node, source: &[u8]) -> bool {
    let mut cursor = func_def.walk();
    for child in func_def.children(&mut cursor) {
        if child.kind() == "storage_class_specifier" && node_text(child, source) == "static" {
            return true;
        }
    }
    false
}

fn bare_identifier(text: &str) -> String {
    if let Some(idx) = text.rfind("::") {
        text[idx + 2..].to_string()
    } else {
        text.to_string()
    }
}

fn extract_c_params(params_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        if child.kind() == "parameter_declaration" {
            // 参数名通常是最后一个 identifier 或 pointer_declarator
            if let Some(decl) = child.child_by_field_name("declarator") {
                params.push(node_text(decl, source).trim_start_matches('*').to_string());
            }
        }
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(source: &str) -> tree_sitter::Tree {
        let adapter = CAdapter::new();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&adapter.language()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_c_extract_functions() {
        let src = r#"
#include <stdio.h>

int add(int a, int b) {
    return a + b;
}

static void helper() {}
"#;
        let tree = parse(src);
        let adapter = CAdapter::new();
        let fns = adapter.extract_functions(&tree, src.as_bytes());
        assert!(fns.iter().any(|f| f.name == "add" && f.is_exported));
        assert!(fns.iter().any(|f| f.name == "helper" && !f.is_exported));
    }

    #[test]
    fn test_c_extract_includes() {
        let src = "#include <stdio.h>\n#include \"mylib.h\"\n";
        let tree = parse(src);
        let adapter = CAdapter::new();
        let imports = adapter.extract_imports(&tree, src.as_bytes());
        assert!(imports.iter().any(|i| i.source == "stdio.h" && i.is_default));
        assert!(imports.iter().any(|i| i.source == "mylib.h" && !i.is_default));
    }

    #[test]
    fn test_c_extract_structs() {
        let src = r#"
struct Point {
    int x;
    int y;
};
"#;
        let tree = parse(src);
        let adapter = CAdapter::new();
        let classes = adapter.extract_classes(&tree, src.as_bytes());
        assert!(classes.iter().any(|c| c.name == "Point" && c.kind == "struct"));
    }
}
