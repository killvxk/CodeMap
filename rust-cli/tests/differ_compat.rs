/// differ 集成测试
///
/// 移植自 ccplugin/cli/test/differ.test.js
use codegraph::differ::{detect_changed_files, merge_graph_update};
use codegraph::graph::{CodeGraph, FileEntry, ModuleEntry, GraphSummary, GraphConfig, ProjectInfo};
use std::collections::HashMap;

fn make_file_entry(module: &str, hash: &str, functions_count: usize, classes_count: usize) -> FileEntry {
    FileEntry {
        language: "typescript".to_string(),
        module: module.to_string(),
        hash: hash.to_string(),
        lines: 10,
        functions: (0..functions_count).map(|i| codegraph::graph::FunctionInfo {
            name: format!("fn{}", i),
            signature: format!("fn{}()", i),
            start_line: 1,
            end_line: 2,
        }).collect(),
        classes: (0..classes_count).map(|i| codegraph::graph::ClassInfo {
            name: format!("Class{}", i),
            start_line: 1,
            end_line: 5,
        }).collect(),
        types: vec![],
        imports: vec![],
        exports: vec![],
        is_entry_point: false,
    }
}

fn make_graph() -> CodeGraph {
    let mut files = HashMap::new();
    files.insert("src/auth/login.ts".to_string(), make_file_entry("auth", "sha256:aaa", 1, 0));
    files.insert("src/api/routes.ts".to_string(), make_file_entry("api", "sha256:bbb", 1, 0));
    files.insert("src/old/removed.ts".to_string(), make_file_entry("old", "sha256:ccc", 0, 0));

    let mut modules = HashMap::new();
    modules.insert("auth".to_string(), ModuleEntry {
        files: vec!["src/auth/login.ts".to_string()],
        depends_on: vec![],
        depended_by: vec!["api".to_string()],
    });
    modules.insert("api".to_string(), ModuleEntry {
        files: vec!["src/api/routes.ts".to_string()],
        depends_on: vec!["auth".to_string()],
        depended_by: vec![],
    });
    modules.insert("old".to_string(), ModuleEntry {
        files: vec!["src/old/removed.ts".to_string()],
        depends_on: vec![],
        depended_by: vec![],
    });

    CodeGraph {
        version: "1.0".to_string(),
        project: ProjectInfo { name: "test".to_string(), root: "/test".to_string() },
        scanned_at: "2026-01-01T00:00:00.000Z".to_string(),
        commit_hash: None,
        config: GraphConfig { languages: vec![], exclude_patterns: vec![] },
        summary: GraphSummary {
            total_files: 3,
            total_functions: 2,
            total_classes: 0,
            languages: [("typescript".to_string(), 3u32)].into_iter().collect(),
            modules: vec!["api".to_string(), "auth".to_string(), "old".to_string()],
            entry_points: vec![],
        },
        modules,
        files,
    }
}

// ── detectChangedFiles ───────────────────────────────────────────────────────

#[test]
fn test_detect_changed_identifies_all_categories() {
    let old_hashes: HashMap<String, String> = [
        ("src/auth/login.ts".to_string(), "sha256:aaa".to_string()),
        ("src/api/routes.ts".to_string(), "sha256:bbb".to_string()),
        ("src/old/removed.ts".to_string(), "sha256:ccc".to_string()),
    ].into_iter().collect();

    let new_hashes: HashMap<String, String> = [
        ("src/auth/login.ts".to_string(), "sha256:aaa".to_string()),  // unchanged
        ("src/api/routes.ts".to_string(), "sha256:xxx".to_string()),  // modified
        ("src/new/added.ts".to_string(), "sha256:ddd".to_string()),   // added
    ].into_iter().collect();

    let result = detect_changed_files(&old_hashes, &new_hashes);

    assert_eq!(result.added, vec!["src/new/added.ts"]);
    assert_eq!(result.modified, vec!["src/api/routes.ts"]);
    assert_eq!(result.removed, vec!["src/old/removed.ts"]);
    assert_eq!(result.unchanged, vec!["src/auth/login.ts"]);
}

#[test]
fn test_detect_changed_identical_hashes() {
    let hashes: HashMap<String, String> = [
        ("src/a.ts".to_string(), "sha256:aaa".to_string()),
        ("src/b.ts".to_string(), "sha256:bbb".to_string()),
    ].into_iter().collect();

    let result = detect_changed_files(&hashes, &hashes.clone());
    assert!(result.added.is_empty());
    assert!(result.modified.is_empty());
    assert!(result.removed.is_empty());
    assert_eq!(result.unchanged.len(), 2);
    assert!(result.unchanged.contains(&"src/a.ts".to_string()));
    assert!(result.unchanged.contains(&"src/b.ts".to_string()));
}

#[test]
fn test_detect_changed_empty_old_hashes() {
    let new_hashes: HashMap<String, String> = [
        ("src/a.ts".to_string(), "sha256:aaa".to_string()),
        ("src/b.ts".to_string(), "sha256:bbb".to_string()),
    ].into_iter().collect();

    let result = detect_changed_files(&HashMap::new(), &new_hashes);
    assert_eq!(result.added.len(), 2);
    assert!(result.modified.is_empty());
    assert!(result.removed.is_empty());
    assert!(result.unchanged.is_empty());
}

// ── mergeGraphUpdate ─────────────────────────────────────────────────────────

#[test]
fn test_merge_removes_file() {
    let mut graph = make_graph();
    merge_graph_update(&mut graph, HashMap::new(), &["src/old/removed.ts".to_string()]);
    assert!(graph.files.get("src/old/removed.ts").is_none(), "removed file should be gone");
}

#[test]
fn test_merge_removes_empty_module() {
    let mut graph = make_graph();
    merge_graph_update(&mut graph, HashMap::new(), &["src/old/removed.ts".to_string()]);
    assert!(graph.modules.get("old").is_none(), "empty module should be removed");
}

#[test]
fn test_merge_updates_file() {
    let mut graph = make_graph();
    let mut updated = HashMap::new();
    updated.insert("src/api/routes.ts".to_string(), make_file_entry("api", "sha256:xxx", 2, 0));
    merge_graph_update(&mut graph, updated, &[]);
    let f = &graph.files["src/api/routes.ts"];
    assert_eq!(f.hash, "sha256:xxx");
    assert_eq!(f.functions.len(), 2);
}

#[test]
fn test_merge_adds_new_file_and_module() {
    let mut graph = make_graph();
    let mut updated = HashMap::new();
    updated.insert("src/new/added.ts".to_string(), make_file_entry("newmod", "sha256:ddd", 1, 1));
    merge_graph_update(&mut graph, updated, &[]);
    assert!(graph.files.get("src/new/added.ts").is_some(), "added file should exist");
    assert!(graph.modules.get("newmod").is_some(), "new module should exist");
    assert!(graph.modules["newmod"].files.contains(&"src/new/added.ts".to_string()));
}

#[test]
fn test_merge_recalculates_summary() {
    let mut graph = make_graph();
    let mut updated = HashMap::new();
    updated.insert("src/api/routes.ts".to_string(), make_file_entry("api", "sha256:xxx", 2, 0));
    updated.insert("src/new/added.ts".to_string(), make_file_entry("newmod", "sha256:ddd", 1, 1));
    merge_graph_update(&mut graph, updated, &["src/old/removed.ts".to_string()]);

    // login(1) + routes(2) + added(1) = 4 functions
    assert_eq!(graph.summary.total_functions, 4);
    // added has 1 class
    assert_eq!(graph.summary.total_classes, 1);
    // 3 files remain: login + routes + added
    assert_eq!(graph.summary.total_files, 3);
    assert!(graph.summary.modules.contains(&"newmod".to_string()));
    assert!(!graph.summary.modules.contains(&"old".to_string()));
}

#[test]
fn test_merge_nonexistent_file_no_panic() {
    let mut graph = make_graph();
    // Removing a non-existent file should not panic
    merge_graph_update(&mut graph, HashMap::new(), &["nonexistent.ts".to_string()]);
    assert_eq!(graph.summary.total_files, 3);
    assert!(graph.files.get("src/auth/login.ts").is_some());
}
