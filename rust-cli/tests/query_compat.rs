/// query 集成测试
///
/// 移植自 ccplugin/cli/test/query.test.js
/// 对 sample-project 运行 scan_project，验证 query 函数输出。
use codegraph::scanner::scan_project;
use codegraph::query::{query_symbol, query_module, query_dependants, query_dependencies, QueryOptions};
use std::path::Path;

const FIXTURE_DIR: &str =
    "E:/2026/CodeMap/ccplugin/cli/test/fixtures/sample-project";

fn fixture() -> codegraph::graph::CodeGraph {
    scan_project(Path::new(FIXTURE_DIR), &[]).expect("scan_project failed")
}

// ── querySymbol ──────────────────────────────────────────────────────────────

#[test]
fn test_query_symbol_login_found() {
    let g = fixture();
    let opts = QueryOptions { type_filter: None };
    let results = query_symbol(&g, "login", &opts);
    assert!(!results.is_empty(), "should find results for 'login'");
}

#[test]
fn test_query_symbol_login_function_kind() {
    let g = fixture();
    let opts = QueryOptions { type_filter: None };
    let results = query_symbol(&g, "login", &opts);
    let login_fn = results.iter().find(|r| r.kind == "function" && r.name == "login");
    assert!(login_fn.is_some(), "should find function named 'login'");
}

#[test]
fn test_query_symbol_login_module() {
    let g = fixture();
    let opts = QueryOptions { type_filter: None };
    let results = query_symbol(&g, "login", &opts);
    let login_fn = results.iter().find(|r| r.kind == "function" && r.name == "login").unwrap();
    assert_eq!(login_fn.module, "auth");
}

#[test]
fn test_query_symbol_login_has_lines() {
    let g = fixture();
    let opts = QueryOptions { type_filter: None };
    let results = query_symbol(&g, "login", &opts);
    let login_fn = results.iter().find(|r| r.kind == "function" && r.name == "login").unwrap();
    assert!(login_fn.lines.start > 0, "start line should be defined");
    assert!(login_fn.lines.end > 0, "end line should be defined");
}

#[test]
fn test_query_symbol_filter_by_function_type() {
    let g = fixture();
    let func_opts = QueryOptions { type_filter: Some("function".to_string()) };
    let results = query_symbol(&g, "login", &func_opts);
    let login_fn = results.iter().find(|r| r.name == "login" && r.kind == "function");
    assert!(login_fn.is_some(), "should find login as function with type=function filter");
}

#[test]
fn test_query_symbol_filter_by_type_excludes_function() {
    let g = fixture();
    let type_opts = QueryOptions { type_filter: Some("type".to_string()) };
    let results = query_symbol(&g, "login", &type_opts);
    // login is a function, not a type — should not appear with type filter
    let login_fn = results.iter().find(|r| r.name == "login" && r.kind == "function");
    assert!(login_fn.is_none(), "login function should not appear with type=type filter");
}

#[test]
fn test_query_symbol_unknown_returns_empty() {
    let g = fixture();
    let opts = QueryOptions { type_filter: None };
    let results = query_symbol(&g, "nonExistentSymbol12345", &opts);
    assert!(results.is_empty(), "unknown symbol should return empty");
}

// ── queryModule ──────────────────────────────────────────────────────────────

#[test]
fn test_query_module_auth_exists() {
    let g = fixture();
    let result = query_module(&g, "auth");
    assert!(result.is_some(), "module 'auth' should exist");
}

#[test]
fn test_query_module_auth_name() {
    let g = fixture();
    let result = query_module(&g, "auth").unwrap();
    assert_eq!(result.name, "auth");
}

#[test]
fn test_query_module_auth_has_files() {
    let g = fixture();
    let result = query_module(&g, "auth").unwrap();
    assert!(!result.files.is_empty(), "auth module should have files");
}

#[test]
fn test_query_module_auth_has_depends_on() {
    let g = fixture();
    let result = query_module(&g, "auth").unwrap();
    // dependsOn and dependedBy are arrays (may be empty)
    let _ = result.depends_on;
    let _ = result.depended_by;
}

#[test]
fn test_query_module_unknown_returns_none() {
    let g = fixture();
    let result = query_module(&g, "nonExistentModule");
    assert!(result.is_none(), "unknown module should return None");
}

// ── queryDependants ──────────────────────────────────────────────────────────

#[test]
fn test_query_dependants_auth_contains_api() {
    let g = fixture();
    let dependants = query_dependants(&g, "auth");
    assert!(dependants.contains(&"api".to_string()), "auth dependants should contain 'api'");
}

#[test]
fn test_query_dependants_unknown_returns_empty() {
    let g = fixture();
    let dependants = query_dependants(&g, "nonExistentModule");
    assert!(dependants.is_empty(), "unknown module dependants should be empty");
}

// ── queryDependencies ────────────────────────────────────────────────────────

#[test]
fn test_query_dependencies_api_contains_auth() {
    let g = fixture();
    let deps = query_dependencies(&g, "api");
    assert!(deps.contains(&"auth".to_string()), "api dependencies should contain 'auth'");
}

#[test]
fn test_query_dependencies_unknown_returns_empty() {
    let g = fixture();
    let deps = query_dependencies(&g, "nonExistentModule");
    assert!(deps.is_empty(), "unknown module dependencies should be empty");
}
