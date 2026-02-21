/// impact 集成测试
///
/// 移植自 ccplugin/cli/test/impact.test.js
use codegraph::scanner::scan_project;
use codegraph::impact::analyze_impact;
use std::path::Path;

const FIXTURE_DIR: &str =
    "E:/2026/CodeMap/ccplugin/cli/test/fixtures/sample-project";

fn fixture() -> codegraph::graph::CodeGraph {
    scan_project(Path::new(FIXTURE_DIR), &[]).expect("scan_project failed")
}

#[test]
fn test_impact_auth_target_type_module() {
    let g = fixture();
    let result = analyze_impact(&g, "auth", 3);
    assert_eq!(result.target_type.as_str(), "module");
}

#[test]
fn test_impact_auth_target_module() {
    let g = fixture();
    let result = analyze_impact(&g, "auth", 3);
    assert_eq!(result.target_module, "auth");
}

#[test]
fn test_impact_auth_direct_dependants_contains_api() {
    let g = fixture();
    let result = analyze_impact(&g, "auth", 3);
    assert!(
        result.direct_dependants.contains(&"api".to_string()),
        "auth direct dependants should contain 'api', got {:?}",
        result.direct_dependants
    );
}

#[test]
fn test_impact_auth_impacted_modules_contains_auth_and_api() {
    let g = fixture();
    let result = analyze_impact(&g, "auth", 3);
    assert!(result.impacted_modules.contains(&"auth".to_string()));
    assert!(result.impacted_modules.contains(&"api".to_string()));
}

#[test]
fn test_impact_file_target_type() {
    let g = fixture();
    let login_file = g.files.keys().find(|f| f.contains("login.ts"))
        .expect("login.ts should exist in graph").clone();
    let result = analyze_impact(&g, &login_file, 3);
    assert_eq!(result.target_type.as_str(), "file");
}

#[test]
fn test_impact_file_target_module() {
    let g = fixture();
    let login_file = g.files.keys().find(|f| f.contains("login.ts"))
        .expect("login.ts should exist in graph").clone();
    let result = analyze_impact(&g, &login_file, 3);
    assert_eq!(result.target_module, "auth");
}

#[test]
fn test_impact_file_impacted_files_has_auth_and_api() {
    let g = fixture();
    let login_file = g.files.keys().find(|f| f.contains("login.ts"))
        .expect("login.ts should exist in graph").clone();
    let result = analyze_impact(&g, &login_file, 3);
    assert!(!result.impacted_files.is_empty(), "impacted files should not be empty");
    let has_auth = result.impacted_files.iter().any(|f| f.contains("auth"));
    let has_api = result.impacted_files.iter().any(|f| f.contains("api"));
    assert!(has_auth, "impacted files should include auth files");
    assert!(has_api, "impacted files should include api files");
}

#[test]
fn test_impact_api_no_dependants() {
    let g = fixture();
    let result = analyze_impact(&g, "api", 3);
    assert_eq!(result.target_type.as_str(), "module");
    assert!(result.direct_dependants.is_empty(), "api has no dependants");
    assert!(result.transitive_dependants.is_empty());
    assert_eq!(result.impacted_modules, vec!["api"]);
}

#[test]
fn test_impact_depth_zero_no_transitive() {
    let g = fixture();
    let result = analyze_impact(&g, "auth", 0);
    assert!(result.transitive_dependants.is_empty(), "depth=0 should have no transitive dependants");
    assert_eq!(result.impacted_modules, vec!["auth"]);
}

#[test]
fn test_impact_unknown_target_graceful() {
    let g = fixture();
    let result = analyze_impact(&g, "nonExistentModule", 3);
    assert_eq!(result.target_module, "nonExistentModule");
    assert!(result.direct_dependants.is_empty());
    assert!(result.transitive_dependants.is_empty());
}
