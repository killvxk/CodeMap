/// 符号查询引擎
///
/// 在 CodeGraph 中按名称搜索函数、类、类型，支持模糊匹配和类型过滤。
/// 逻辑与 ccplugin/cli/src/query.js 保持一致。
use crate::graph::{CodeGraph, FileEntry};

// ── 查询结果结构 ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone)]
pub struct SymbolResult {
    pub kind: String,       // "function" | "class" | "type"
    pub name: String,
    pub signature: Option<String>,
    pub file: String,
    pub module: String,
    pub lines: LineRange,
    /// 同文件中导入的其他符号（非自身）
    pub file_imports: Vec<String>,
    /// 导入了该符号的其他文件（"module:file" 格式）
    pub imported_by: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModuleResult {
    pub name: String,
    pub files: Vec<String>,
    pub depends_on: Vec<String>,
    pub depended_by: Vec<String>,
}

// ── 查询选项 ──────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct QueryOptions {
    /// 限制搜索类型："function" | "class" | "type"，None 表示全部
    pub type_filter: Option<String>,
}

// ── 核心查询函数 ──────────────────────────────────────────────────────────────

/// 在图谱中搜索匹配的符号（函数、类、类型）。
///
/// 匹配规则：符号名称等于 symbol_name，或包含 symbol_name（子串匹配）。
pub fn query_symbol(graph: &CodeGraph, symbol_name: &str, opts: &QueryOptions) -> Vec<SymbolResult> {
    let mut results = Vec::new();
    let type_filter = opts.type_filter.as_deref();

    for (file_path, file_data) in &graph.files {
        // 搜索函数
        if type_filter.is_none() || type_filter == Some("function") {
            for func in &file_data.functions {
                if matches_symbol(&func.name, symbol_name) {
                    let file_imports = collect_file_imports(file_data, &func.name);
                    let imported_by = find_callers(graph, file_path, &func.name);
                    results.push(SymbolResult {
                        kind: "function".into(),
                        name: func.name.clone(),
                        signature: Some(func.signature.clone()),
                        file: file_path.clone(),
                        module: file_data.module.clone(),
                        lines: LineRange { start: func.start_line, end: func.end_line },
                        file_imports,
                        imported_by,
                    });
                }
            }
        }

        // 搜索类
        if type_filter.is_none() || type_filter == Some("class") {
            for cls in &file_data.classes {
                if matches_symbol(&cls.name, symbol_name) {
                    let imported_by = find_callers(graph, file_path, &cls.name);
                    results.push(SymbolResult {
                        kind: "class".into(),
                        name: cls.name.clone(),
                        signature: None,
                        file: file_path.clone(),
                        module: file_data.module.clone(),
                        lines: LineRange { start: cls.start_line, end: cls.end_line },
                        file_imports: vec![],
                        imported_by,
                    });
                }
            }
        }

        // 搜索类型
        if type_filter.is_none() || type_filter == Some("type") {
            for tp in &file_data.types {
                if matches_symbol(&tp.name, symbol_name) {
                    let imported_by = find_callers(graph, file_path, &tp.name);
                    results.push(SymbolResult {
                        kind: "type".into(),
                        name: tp.name.clone(),
                        signature: None,
                        file: file_path.clone(),
                        module: file_data.module.clone(),
                        lines: LineRange { start: tp.start_line, end: tp.end_line },
                        file_imports: vec![],
                        imported_by,
                    });
                }
            }
        }
    }

    // 按文件路径排序，保证输出稳定
    results.sort_by(|a, b| a.file.cmp(&b.file).then(a.name.cmp(&b.name)));
    results
}

/// 查询模块信息。
pub fn query_module(graph: &CodeGraph, module_name: &str) -> Option<ModuleResult> {
    let mod_data = graph.modules.get(module_name)?;
    Some(ModuleResult {
        name: module_name.to_string(),
        files: mod_data.files.clone(),
        depends_on: mod_data.depends_on.clone(),
        depended_by: mod_data.depended_by.clone(),
    })
}

/// 返回依赖该模块的模块列表。
pub fn query_dependants(graph: &CodeGraph, module_name: &str) -> Vec<String> {
    graph.modules.get(module_name)
        .map(|m| m.depended_by.clone())
        .unwrap_or_default()
}

/// 返回该模块依赖的模块列表。
pub fn query_dependencies(graph: &CodeGraph, module_name: &str) -> Vec<String> {
    graph.modules.get(module_name)
        .map(|m| m.depends_on.clone())
        .unwrap_or_default()
}

// ── 内部辅助函数 ──────────────────────────────────────────────────────────────

/// 符号名称匹配：精确匹配或子串包含
fn matches_symbol(name: &str, query: &str) -> bool {
    name == query || name.contains(query)
}

/// 收集同文件中导入的符号（排除自身）
fn collect_file_imports(file_data: &FileEntry, self_name: &str) -> Vec<String> {
    file_data.imports
        .iter()
        .flat_map(|imp| imp.symbols.iter())
        .filter(|s| s.as_str() != self_name)
        .cloned()
        .collect()
}

/// 查找导入了指定符号的其他文件，返回 "module:file" 格式列表
fn find_callers(graph: &CodeGraph, source_file: &str, symbol_name: &str) -> Vec<String> {
    let mut callers = Vec::new();
    for (file_path, file_data) in &graph.files {
        if file_path == source_file {
            continue;
        }
        for imp in &file_data.imports {
            if imp.symbols.iter().any(|s| s == symbol_name) {
                callers.push(format!("{}:{}", file_data.module, file_path));
                break;
            }
        }
    }
    callers.sort();
    callers
}

// ── 格式化输出 ────────────────────────────────────────────────────────────────

/// 将查询结果格式化为人类可读的文本
pub fn format_symbol_results(results: &[SymbolResult]) -> String {
    if results.is_empty() {
        return "No symbols found.".to_string();
    }
    let mut out = String::new();
    for r in results {
        out.push_str(&format!(
            "[{}] {} ({}:{})\n",
            r.kind, r.name, r.file, r.lines.start
        ));
        if let Some(sig) = &r.signature {
            if !sig.is_empty() && sig != &r.name {
                out.push_str(&format!("  signature: {}\n", sig));
            }
        }
        out.push_str(&format!("  module:    {}\n", r.module));
        out.push_str(&format!("  lines:     {}-{}\n", r.lines.start, r.lines.end));
        if !r.imported_by.is_empty() {
            out.push_str(&format!("  importedBy: {}\n", r.imported_by.join(", ")));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

/// 将模块查询结果格式化为人类可读的文本
pub fn format_module_result(result: &ModuleResult) -> String {
    let mut out = format!("module: {}\n", result.name);
    out.push_str(&format!("  files ({}):\n", result.files.len()));
    for f in &result.files {
        out.push_str(&format!("    {}\n", f));
    }
    if !result.depends_on.is_empty() {
        out.push_str(&format!("  dependsOn: {}\n", result.depends_on.join(", ")));
    }
    if !result.depended_by.is_empty() {
        out.push_str(&format!("  dependedBy: {}\n", result.depended_by.join(", ")));
    }
    out.trim_end().to_string()
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{
        ClassInfo, CodeGraph, FileEntry, FunctionInfo, GraphConfig, GraphSummary,
        ImportInfo, ModuleEntry, ProjectInfo, TypeInfo,
    };
    use std::collections::HashMap;

    fn make_graph() -> CodeGraph {
        let mut files = HashMap::new();

        // auth/login.ts
        files.insert(
            "auth/login.ts".to_string(),
            FileEntry {
                language: "typescript".into(),
                module: "auth".into(),
                hash: "sha256:abc".into(),
                lines: 30,
                functions: vec![
                    FunctionInfo {
                        name: "login".into(),
                        signature: "login(user: string, pass: string): boolean".into(),
                        start_line: 5,
                        end_line: 15,
                    },
                    FunctionInfo {
                        name: "logout".into(),
                        signature: "logout(): void".into(),
                        start_line: 17,
                        end_line: 20,
                    },
                ],
                classes: vec![ClassInfo {
                    name: "AuthService".into(),
                    start_line: 1,
                    end_line: 30,
                }],
                types: vec![TypeInfo {
                    name: "UserToken".into(),
                    kind: "type".into(),
                    start_line: 2,
                    end_line: 2,
                }],
                imports: vec![ImportInfo {
                    source: "./utils".into(),
                    symbols: vec!["hashPassword".into()],
                    is_external: false,
                }],
                exports: vec!["login".into(), "logout".into(), "AuthService".into()],
                is_entry_point: false,
            },
        );

        // utils/helper.ts
        files.insert(
            "utils/helper.ts".to_string(),
            FileEntry {
                language: "typescript".into(),
                module: "utils".into(),
                hash: "sha256:def".into(),
                lines: 10,
                functions: vec![FunctionInfo {
                    name: "hashPassword".into(),
                    signature: "hashPassword(pw: string): string".into(),
                    start_line: 1,
                    end_line: 8,
                }],
                classes: vec![],
                types: vec![],
                imports: vec![],
                exports: vec!["hashPassword".into()],
                is_entry_point: false,
            },
        );

        let mut modules = HashMap::new();
        modules.insert(
            "auth".into(),
            ModuleEntry {
                files: vec!["auth/login.ts".into()],
                depends_on: vec!["utils".into()],
                depended_by: vec![],
            },
        );
        modules.insert(
            "utils".into(),
            ModuleEntry {
                files: vec!["utils/helper.ts".into()],
                depends_on: vec![],
                depended_by: vec!["auth".into()],
            },
        );

        CodeGraph {
            version: "1.0".into(),
            project: ProjectInfo { name: "test".into(), root: "/test".into() },
            scanned_at: "2026-01-01T00:00:00.000Z".into(),
            commit_hash: None,
            config: GraphConfig { languages: vec![], exclude_patterns: vec![] },
            summary: GraphSummary {
                total_files: 2,
                total_functions: 3,
                total_classes: 1,
                languages: HashMap::new(),
                modules: vec!["auth".into(), "utils".into()],
                entry_points: vec![],
            },
            modules,
            files,
        }
    }

    #[test]
    fn test_query_exact_match() {
        let graph = make_graph();
        let results = query_symbol(&graph, "login", &QueryOptions::default());
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "login");
        assert_eq!(results[0].kind, "function");
        assert_eq!(results[0].module, "auth");
        assert_eq!(results[0].lines.start, 5);
    }

    #[test]
    fn test_query_substring_match() {
        let graph = make_graph();
        let results = query_symbol(&graph, "log", &QueryOptions::default());
        // "login" 和 "logout" 都包含 "log"
        assert_eq!(results.len(), 2);
        let names: Vec<&str> = results.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"login"));
        assert!(names.contains(&"logout"));
    }

    #[test]
    fn test_query_type_filter_function() {
        let graph = make_graph();
        let opts = QueryOptions { type_filter: Some("function".into()) };
        let results = query_symbol(&graph, "Auth", &opts);
        // "AuthService" 是 class，过滤后不应出现
        assert!(results.is_empty());
    }

    #[test]
    fn test_query_type_filter_class() {
        let graph = make_graph();
        let opts = QueryOptions { type_filter: Some("class".into()) };
        let results = query_symbol(&graph, "Auth", &opts);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "AuthService");
        assert_eq!(results[0].kind, "class");
    }

    #[test]
    fn test_query_type_filter_type() {
        let graph = make_graph();
        let opts = QueryOptions { type_filter: Some("type".into()) };
        let results = query_symbol(&graph, "Token", &opts);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "UserToken");
        assert_eq!(results[0].kind, "type");
    }

    #[test]
    fn test_query_no_match() {
        let graph = make_graph();
        let results = query_symbol(&graph, "nonexistent_xyz", &QueryOptions::default());
        assert!(results.is_empty());
    }

    #[test]
    fn test_find_callers() {
        let graph = make_graph();
        // auth/login.ts 导入了 hashPassword
        let callers = find_callers(&graph, "utils/helper.ts", "hashPassword");
        assert_eq!(callers.len(), 1);
        assert!(callers[0].contains("auth"));
    }

    #[test]
    fn test_query_module() {
        let graph = make_graph();
        let result = query_module(&graph, "auth").unwrap();
        assert_eq!(result.name, "auth");
        assert!(result.depends_on.contains(&"utils".to_string()));
    }

    #[test]
    fn test_query_module_not_found() {
        let graph = make_graph();
        assert!(query_module(&graph, "nonexistent").is_none());
    }

    #[test]
    fn test_query_dependants() {
        let graph = make_graph();
        let deps = query_dependants(&graph, "utils");
        assert!(deps.contains(&"auth".to_string()));
    }

    #[test]
    fn test_query_dependencies() {
        let graph = make_graph();
        let deps = query_dependencies(&graph, "auth");
        assert!(deps.contains(&"utils".to_string()));
    }

    #[test]
    fn test_format_symbol_results_empty() {
        let out = format_symbol_results(&[]);
        assert_eq!(out, "No symbols found.");
    }

    #[test]
    fn test_format_symbol_results() {
        let graph = make_graph();
        let results = query_symbol(&graph, "login", &QueryOptions::default());
        let out = format_symbol_results(&results);
        assert!(out.contains("[function]"));
        assert!(out.contains("login"));
        assert!(out.contains("auth/login.ts"));
    }
}
