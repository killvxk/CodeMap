pub mod typescript;
pub mod javascript;
pub mod python;
pub mod go_lang;
pub mod rust_lang;
pub mod java;
pub mod c_lang;
pub mod cpp;

// ---------------------------------------------------------------------------
// 公共数据结构
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct FunctionInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub params: Vec<String>,
    pub is_exported: bool,
}

#[derive(Debug, Clone)]
pub struct ImportInfo {
    pub source: String,
    pub names: Vec<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct ExportInfo {
    pub name: String,
    pub kind: String, // "function", "class", "type", "variable"
}

#[derive(Debug, Clone)]
pub struct ClassInfo {
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    pub methods: Vec<String>,
    pub kind: String, // "class", "interface", "struct", "enum", "trait"
}

// ---------------------------------------------------------------------------
// LanguageAdapter trait
// ---------------------------------------------------------------------------

pub trait LanguageAdapter: Send + Sync {
    fn language(&self) -> tree_sitter::Language;
    fn extract_functions(&self, tree: &tree_sitter::Tree, source: &[u8]) -> Vec<FunctionInfo>;
    fn extract_imports(&self, tree: &tree_sitter::Tree, source: &[u8]) -> Vec<ImportInfo>;
    fn extract_exports(&self, tree: &tree_sitter::Tree, source: &[u8]) -> Vec<ExportInfo>;
    fn extract_classes(&self, tree: &tree_sitter::Tree, source: &[u8]) -> Vec<ClassInfo>;
}

// ---------------------------------------------------------------------------
// 工厂函数
// ---------------------------------------------------------------------------

pub fn get_adapter(lang: crate::traverser::Language) -> Box<dyn LanguageAdapter> {
    match lang {
        crate::traverser::Language::TypeScript => Box::new(typescript::TypeScriptAdapter::new()),
        crate::traverser::Language::JavaScript => Box::new(javascript::JavaScriptAdapter::new()),
        crate::traverser::Language::Python => Box::new(python::PythonAdapter::new()),
        crate::traverser::Language::Go => Box::new(go_lang::GoAdapter::new()),
        crate::traverser::Language::Rust => Box::new(rust_lang::RustAdapter::new()),
        crate::traverser::Language::Java => Box::new(java::JavaAdapter::new()),
        crate::traverser::Language::C => Box::new(c_lang::CAdapter::new()),
        crate::traverser::Language::Cpp => Box::new(cpp::CppAdapter::new()),
    }
}

// ---------------------------------------------------------------------------
// 共享辅助函数
// ---------------------------------------------------------------------------

/// 深度优先遍历所有节点，对每个节点调用 visitor
/// 注意：使用递归实现，极端深层嵌套（>1000层）可能导致栈溢出
pub fn walk_nodes<F>(node: tree_sitter::Node, visitor: &mut F)
where
    F: FnMut(tree_sitter::Node),
{
    visitor(node);
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            walk_nodes(cursor.node(), visitor);
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }
}

/// 查找第一个指定类型的直接子节点
pub fn find_child_of_type<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == kind {
            return Some(child);
        }
    }
    None
}

/// 查找第一个指定类型的后代节点（BFS）
pub fn find_descendant_of_type<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Option<tree_sitter::Node<'a>> {
    let mut queue = std::collections::VecDeque::new();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        queue.push_back(child);
    }
    while let Some(current) = queue.pop_front() {
        if current.kind() == kind {
            return Some(current);
        }
        let mut c = current.walk();
        for child in current.children(&mut c) {
            queue.push_back(child);
        }
    }
    None
}

/// 去除字符串两端的引号
pub fn strip_quotes(s: &str) -> String {
    s.trim_matches(|c| c == '\'' || c == '"' || c == '`').to_string()
}

/// 从源码字节中提取节点文本
pub fn node_text<'a>(node: tree_sitter::Node, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or("")
}
