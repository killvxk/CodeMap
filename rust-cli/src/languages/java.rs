use tree_sitter::{Language, Tree};
use super::{
    ClassInfo, ExportInfo, FunctionInfo, ImportInfo, LanguageAdapter,
    node_text, walk_nodes,
};

pub struct JavaAdapter;

impl JavaAdapter {
    pub fn new() -> Self {
        Self
    }
}

impl LanguageAdapter for JavaAdapter {
    fn language(&self) -> Language {
        tree_sitter_java::LANGUAGE.into()
    }

    fn extract_functions(&self, tree: &Tree, source: &[u8]) -> Vec<FunctionInfo> {
        let mut functions = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            if node.kind() != "method_declaration" && node.kind() != "constructor_declaration" {
                return;
            }
            let name_node = match node.child_by_field_name("name") {
                Some(n) => n,
                None => return,
            };
            let class_name = find_enclosing_class_name(node, source);
            let qualified_name = match &class_name {
                Some(c) => format!("{}.{}", c, node_text(name_node, source)),
                None => node_text(name_node, source).to_string(),
            };
            let params = node.child_by_field_name("parameters")
                .map(|p| extract_java_params(p, source))
                .unwrap_or_default();
            let is_exported = has_modifier(node, source, "public");
            functions.push(FunctionInfo {
                name: qualified_name,
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
            if node.kind() != "import_declaration" {
                return;
            }
            let text = node_text(node, source).trim().to_string();
            // "import java.util.List;" -> "java.util.List"
            let path = text
                .trim_start_matches("import")
                .trim_start_matches("static")
                .trim()
                .trim_end_matches(';')
                .trim()
                .to_string();
            if let Some(last_dot) = path.rfind('.') {
                let src = path[..last_dot].to_string();
                let symbol = path[last_dot + 1..].to_string();
                imports.push(ImportInfo {
                    source: src,
                    names: vec![symbol],
                    is_default: false,
                });
            } else {
                imports.push(ImportInfo {
                    source: path,
                    names: Vec::new(),
                    is_default: false,
                });
            }
        });
        imports
    }

    fn extract_exports(&self, tree: &Tree, source: &[u8]) -> Vec<ExportInfo> {
        let mut exports = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            let kind = match node.kind() {
                "class_declaration" => "class",
                "interface_declaration" => "interface",
                "enum_declaration" => "enum",
                _ => return,
            };
            if has_modifier(node, source, "public") {
                if let Some(n) = node.child_by_field_name("name") {
                    exports.push(ExportInfo {
                        name: node_text(n, source).to_string(),
                        kind: kind.into(),
                    });
                }
            }
        });
        exports
    }

    fn extract_classes(&self, tree: &Tree, source: &[u8]) -> Vec<ClassInfo> {
        let mut classes = Vec::new();
        walk_nodes(tree.root_node(), &mut |node| {
            let kind = match node.kind() {
                "class_declaration" => "class",
                "interface_declaration" => "interface",
                "enum_declaration" => "enum",
                _ => return,
            };
            if let Some(n) = node.child_by_field_name("name") {
                let methods = extract_java_methods(node, source);
                classes.push(ClassInfo {
                    name: node_text(n, source).to_string(),
                    start_line: node.start_position().row + 1,
                    end_line: node.end_position().row + 1,
                    methods,
                    kind: kind.into(),
                });
            }
        });
        classes
    }
}

fn find_enclosing_class_name(node: tree_sitter::Node, source: &[u8]) -> Option<String> {
    let mut current = node.parent();
    while let Some(n) = current {
        if matches!(n.kind(), "class_body" | "interface_body" | "enum_body") {
            if let Some(decl) = n.parent() {
                if let Some(name) = decl.child_by_field_name("name") {
                    return Some(node_text(name, source).to_string());
                }
            }
        }
        current = n.parent();
    }
    None
}

fn has_modifier(node: tree_sitter::Node, source: &[u8], modifier: &str) -> bool {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "modifiers" {
            let mut c = child.walk();
            for m in child.children(&mut c) {
                if node_text(m, source) == modifier {
                    return true;
                }
            }
        }
    }
    false
}

fn extract_java_params(params_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();
    for child in params_node.children(&mut cursor) {
        if child.kind() == "formal_parameter" || child.kind() == "spread_parameter" {
            if let Some(n) = child.child_by_field_name("name") {
                params.push(node_text(n, source).to_string());
            }
        }
    }
    params
}

fn extract_java_methods(class_node: tree_sitter::Node, source: &[u8]) -> Vec<String> {
    let mut methods = Vec::new();
    walk_nodes(class_node, &mut |node| {
        if node.kind() == "method_declaration" {
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

    fn parse(source: &str) -> tree_sitter::Tree {
        let adapter = JavaAdapter::new();
        let mut parser = tree_sitter::Parser::new();
        parser.set_language(&adapter.language()).unwrap();
        parser.parse(source, None).unwrap()
    }

    #[test]
    fn test_java_extract_functions() {
        let src = r#"
public class Greeter {
    public String greet(String name) {
        return "Hello " + name;
    }
    private void helper() {}
}
"#;
        let tree = parse(src);
        let adapter = JavaAdapter::new();
        let fns = adapter.extract_functions(&tree, src.as_bytes());
        assert!(fns.iter().any(|f| f.name == "Greeter.greet" && f.is_exported));
        assert!(fns.iter().any(|f| f.name == "Greeter.helper" && !f.is_exported));
    }

    #[test]
    fn test_java_extract_imports() {
        let src = "import java.util.List;\nimport java.io.*;\n";
        let tree = parse(src);
        let adapter = JavaAdapter::new();
        let imports = adapter.extract_imports(&tree, src.as_bytes());
        assert!(imports.iter().any(|i| i.source == "java.util" && i.names.contains(&"List".to_string())));
    }

    #[test]
    fn test_java_extract_classes() {
        let src = r#"
public class Animal {
    public void speak() {}
}
public interface Runnable {}
"#;
        let tree = parse(src);
        let adapter = JavaAdapter::new();
        let classes = adapter.extract_classes(&tree, src.as_bytes());
        assert!(classes.iter().any(|c| c.name == "Animal" && c.kind == "class"));
        assert!(classes.iter().any(|c| c.name == "Runnable" && c.kind == "interface"));
    }
}
