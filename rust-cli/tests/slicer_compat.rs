/// slicer 集成测试
///
/// 移植自 ccplugin/cli/test/slicer.test.js
use codegraph::scanner::scan_project;
use codegraph::slicer::{generate_overview, generate_slices, get_module_slice_with_deps, save_slices};
use std::path::Path;

const FIXTURE_DIR: &str =
    "E:/2026/CodeMap/ccplugin/cli/test/fixtures/sample-project";

fn fixture() -> codegraph::graph::CodeGraph {
    scan_project(Path::new(FIXTURE_DIR), &[]).expect("scan_project failed")
}

// ── generateOverview ─────────────────────────────────────────────────────────

#[test]
fn test_overview_has_project() {
    let g = fixture();
    let overview = generate_overview(&g);
    assert!(!overview.project.name.is_empty(), "overview should have project name");
}

#[test]
fn test_overview_has_scanned_at() {
    let g = fixture();
    let overview = generate_overview(&g);
    assert!(!overview.scanned_at.is_empty(), "overview should have scannedAt");
}

#[test]
fn test_overview_has_summary() {
    let g = fixture();
    let overview = generate_overview(&g);
    assert!(overview.summary.total_files > 0, "overview summary should have files");
}

#[test]
fn test_overview_has_modules() {
    let g = fixture();
    let overview = generate_overview(&g);
    assert!(!overview.modules.is_empty(), "overview should have modules");
}

#[test]
fn test_overview_compact_json() {
    let g = fixture();
    let overview = generate_overview(&g);
    let json = serde_json::to_string(&overview).unwrap();
    assert!(json.len() < 5000, "overview JSON should be < 5000 chars for small fixture, got {}", json.len());
}

#[test]
fn test_overview_auth_module_has_file_count() {
    let g = fixture();
    let overview = generate_overview(&g);
    let auth_mod = overview.modules.iter().find(|m| m.name == "auth");
    assert!(auth_mod.is_some(), "auth module should be in overview");
    assert!(auth_mod.unwrap().file_count > 0, "auth module should have files");
}

#[test]
fn test_overview_auth_module_has_exports() {
    let g = fixture();
    let overview = generate_overview(&g);
    let auth_mod = overview.modules.iter().find(|m| m.name == "auth").unwrap();
    // exports is a Vec (may be empty but should exist as field)
    let _ = &auth_mod.exports;
}

#[test]
fn test_overview_auth_module_has_depends_on() {
    let g = fixture();
    let overview = generate_overview(&g);
    let auth_mod = overview.modules.iter().find(|m| m.name == "auth").unwrap();
    let _ = &auth_mod.depends_on;
    let _ = &auth_mod.depended_by;
}

#[test]
fn test_overview_has_entry_points() {
    let g = fixture();
    let overview = generate_overview(&g);
    // entryPoints is a Vec (may be empty)
    let _ = &overview.entry_points;
}

// ── generateSlices ───────────────────────────────────────────────────────────

#[test]
fn test_slices_has_auth_module() {
    let g = fixture();
    let slices = generate_slices(&g);
    assert!(slices.contains_key("auth"), "slices should have 'auth' module");
}

#[test]
fn test_slices_has_api_module() {
    let g = fixture();
    let slices = generate_slices(&g);
    assert!(slices.contains_key("api"), "slices should have 'api' module");
}

#[test]
fn test_slices_auth_has_files() {
    let g = fixture();
    let slices = generate_slices(&g);
    let auth_slice = &slices["auth"];
    assert!(!auth_slice.files.is_empty(), "auth slice should have files");
}

#[test]
fn test_slices_auth_files_have_functions() {
    let g = fixture();
    let slices = generate_slices(&g);
    let auth_slice = &slices["auth"];
    // files[0] should have functions field
    let _ = &auth_slice.files[0].functions;
}

#[test]
fn test_slices_auth_files_have_imports() {
    let g = fixture();
    let slices = generate_slices(&g);
    let auth_slice = &slices["auth"];
    let _ = &auth_slice.files[0].imports;
}

#[test]
fn test_slices_api_depends_on_auth() {
    let g = fixture();
    let slices = generate_slices(&g);
    assert!(
        slices["api"].depends_on.contains(&"auth".to_string()),
        "api slice should depend on auth"
    );
}

#[test]
fn test_slices_auth_has_stats() {
    let g = fixture();
    let slices = generate_slices(&g);
    let auth_slice = &slices["auth"];
    assert!(auth_slice.stats.total_files > 0, "auth slice stats should have files");
}

// ── getModuleSliceWithDeps ───────────────────────────────────────────────────

#[test]
fn test_slice_with_deps_api_module() {
    let g = fixture();
    let result = get_module_slice_with_deps(&g, "api").expect("api module should exist");
    assert_eq!(result.slice.module, "api");
}

#[test]
fn test_slice_with_deps_api_has_files() {
    let g = fixture();
    let result = get_module_slice_with_deps(&g, "api").unwrap();
    assert!(!result.slice.files.is_empty(), "api slice should have files");
}

#[test]
fn test_slice_with_deps_api_has_dependencies() {
    let g = fixture();
    let result = get_module_slice_with_deps(&g, "api").unwrap();
    assert!(!result.dependencies.is_empty(), "api should have dependencies");
}

#[test]
fn test_slice_with_deps_api_auth_dep_exists() {
    let g = fixture();
    let result = get_module_slice_with_deps(&g, "api").unwrap();
    let auth_dep = result.dependencies.iter().find(|d| d.name == "auth");
    assert!(auth_dep.is_some(), "api dependencies should include 'auth'");
}

#[test]
fn test_slice_with_deps_auth_dep_has_exports() {
    let g = fixture();
    let result = get_module_slice_with_deps(&g, "api").unwrap();
    let auth_dep = result.dependencies.iter().find(|d| d.name == "auth").unwrap();
    let _ = &auth_dep.exports;
}

// ── saveSlices ───────────────────────────────────────────────────────────────

#[test]
fn test_save_slices_creates_overview_json() {
    let g = fixture();
    let output_dir = std::path::Path::new(FIXTURE_DIR).join(".codemap-test-slices-rust");
    save_slices(&output_dir, &g).expect("save_slices should succeed");

    let slices_dir = output_dir.join("slices");
    assert!(slices_dir.join("_overview.json").exists(), "_overview.json should be created");

    // cleanup
    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn test_save_slices_creates_module_files() {
    let g = fixture();
    let output_dir = std::path::Path::new(FIXTURE_DIR).join(".codemap-test-slices-rust2");
    save_slices(&output_dir, &g).expect("save_slices should succeed");

    let slices_dir = output_dir.join("slices");
    assert!(slices_dir.join("auth.json").exists(), "auth.json should be created");
    assert!(slices_dir.join("api.json").exists(), "api.json should be created");

    // cleanup
    let _ = std::fs::remove_dir_all(&output_dir);
}

#[test]
fn test_save_slices_overview_valid_json() {
    let g = fixture();
    let output_dir = std::path::Path::new(FIXTURE_DIR).join(".codemap-test-slices-rust3");
    save_slices(&output_dir, &g).expect("save_slices should succeed");

    let content = std::fs::read_to_string(output_dir.join("slices/_overview.json"))
        .expect("should read _overview.json");
    let parsed: serde_json::Value = serde_json::from_str(&content)
        .expect("_overview.json should be valid JSON");
    assert!(parsed.get("project").is_some(), "overview should have 'project' field");

    // cleanup
    let _ = std::fs::remove_dir_all(&output_dir);
}
