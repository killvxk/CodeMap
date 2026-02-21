/// scanner 集成测试
///
/// 移植自 ccplugin/cli/test/scanner.test.js
/// 对 sample-project 运行 scan_project，验证输出与 Node.js 版本一致。
use codegraph::scanner::scan_project;
use std::path::Path;

const FIXTURE_DIR: &str =
    "E:/2026/CodeMap/ccplugin/cli/test/fixtures/sample-project";

fn fixture() -> codegraph::graph::CodeGraph {
    scan_project(Path::new(FIXTURE_DIR), &[]).expect("scan_project failed")
}

// ── 基础 summary 验证（对应 scanner.test.js "should produce a valid graph with summary"）

#[test]
fn test_scan_valid_graph_version() {
    let g = fixture();
    assert_eq!(g.version, "1.0");
}

#[test]
fn test_scan_total_files() {
    let g = fixture();
    assert_eq!(g.summary.total_files, 7, "expected 7 source files");
}

#[test]
fn test_scan_languages_include_typescript() {
    let g = fixture();
    assert!(
        g.summary.languages.contains_key("typescript"),
        "expected 'typescript' in languages"
    );
}

#[test]
fn test_scan_language_counts() {
    let g = fixture();
    assert_eq!(g.summary.languages.get("typescript").copied().unwrap_or(0), 2);
    assert_eq!(g.summary.languages.get("rust").copied().unwrap_or(0), 1);
    assert_eq!(g.summary.languages.get("java").copied().unwrap_or(0), 1);
    assert_eq!(g.summary.languages.get("cpp").copied().unwrap_or(0), 1);
    assert_eq!(g.summary.languages.get("go").copied().unwrap_or(0), 1);
    assert_eq!(g.summary.languages.get("python").copied().unwrap_or(0), 1);
}

// ── 模块检测（对应 "should detect modules from directory structure"）

#[test]
fn test_scan_modules_detected() {
    let g = fixture();
    let mods = &g.summary.modules;
    for expected in &["api", "auth", "core", "models", "native", "services", "utils"] {
        assert!(mods.contains(&expected.to_string()), "missing module '{}'", expected);
    }
}

#[test]
fn test_scan_module_count() {
    let g = fixture();
    assert_eq!(g.summary.modules.len(), 7);
}

// ── 文件级别详情（对应 "should extract file-level details"）

#[test]
fn test_scan_login_ts_exists() {
    let g = fixture();
    assert!(
        g.files.contains_key("src/auth/login.ts"),
        "expected 'src/auth/login.ts' in files"
    );
}

#[test]
fn test_scan_login_ts_has_functions() {
    let g = fixture();
    let f = &g.files["src/auth/login.ts"];
    assert!(!f.functions.is_empty(), "login.ts should have functions");
    assert!(f.functions.iter().any(|fn_| fn_.name == "login"));
}

#[test]
fn test_scan_login_ts_has_imports() {
    let g = fixture();
    let f = &g.files["src/auth/login.ts"];
    assert!(!f.imports.is_empty(), "login.ts should have imports");
}

#[test]
fn test_scan_login_ts_exports_login() {
    let g = fixture();
    let f = &g.files["src/auth/login.ts"];
    assert!(f.exports.contains(&"login".to_string()), "login.ts should export 'login'");
}

// ── 模块依赖图（对应 "should build module dependency graph"）

#[test]
fn test_scan_api_module_exists() {
    let g = fixture();
    assert!(g.modules.contains_key("api"), "module 'api' should exist");
}

#[test]
fn test_scan_api_depends_on_auth() {
    let g = fixture();
    let api = &g.modules["api"];
    assert!(
        api.depends_on.contains(&"auth".to_string()),
        "api should depend on auth, got {:?}",
        api.depends_on
    );
}

#[test]
fn test_scan_auth_depended_by_api() {
    let g = fixture();
    let auth = &g.modules["auth"];
    assert!(
        auth.depended_by.contains(&"api".to_string()),
        "auth should be depended by api, got {:?}",
        auth.depended_by
    );
}

// ── 与 Node.js graph.json 的精确对比（交叉验证）

#[test]
fn test_cross_validate_total_functions() {
    let g = fixture();
    // Node.js graph.json: totalFunctions = 25
    assert_eq!(
        g.summary.total_functions, 25,
        "totalFunctions should match Node.js output (25)"
    );
}

#[test]
fn test_cross_validate_total_classes() {
    let g = fixture();
    // Node.js graph.json: totalClasses = 7
    assert_eq!(
        g.summary.total_classes, 7,
        "totalClasses should match Node.js output (7)"
    );
}

#[test]
fn test_cross_validate_file_hashes() {
    let g = fixture();
    // 验证文件哈希与 Node.js meta.json 中的 fileHashes 一致
    let expected = [
        ("src/api/routes.ts",      "sha256:45be6689959ab2de"),
        ("src/auth/login.ts",      "sha256:8b6c5eca0e26a23f"),
        ("src/core/engine.rs",     "sha256:5ab3dc4e622ea75f"),
        ("src/models/User.java",   "sha256:6ba066e543f6f9a2"),
        ("src/native/engine.cpp",  "sha256:7823bcaa1f7f3e5f"),
        ("src/services/handler.go","sha256:094e3fb44dd7fe3b"),
        ("src/utils/helpers.py",   "sha256:b6769e7c6264de45"),
    ];
    for (path, hash) in &expected {
        let file = g.files.get(*path)
            .unwrap_or_else(|| panic!("file '{}' not found in graph", path));
        assert_eq!(
            file.hash, *hash,
            "hash mismatch for '{}': got '{}', expected '{}'",
            path, file.hash, hash
        );
    }
}

#[test]
fn test_cross_validate_routes_ts() {
    let g = fixture();
    let f = &g.files["src/api/routes.ts"];
    assert_eq!(f.language, "typescript");
    assert_eq!(f.module, "api");
    assert_eq!(f.lines, 7);
    assert_eq!(f.functions.len(), 1);
    assert_eq!(f.functions[0].name, "handleLogin");
    assert_eq!(f.functions[0].start_line, 3);
    assert_eq!(f.functions[0].end_line, 6);
    assert!(f.exports.contains(&"handleLogin".to_string()));
}

#[test]
fn test_cross_validate_login_ts() {
    let g = fixture();
    let f = &g.files["src/auth/login.ts"];
    assert_eq!(f.language, "typescript");
    assert_eq!(f.module, "auth");
    assert_eq!(f.lines, 15);
    assert_eq!(f.functions.len(), 1);
    assert_eq!(f.functions[0].name, "login");
    // Node.js: startLine=9, endLine=14
    assert_eq!(f.functions[0].start_line, 9);
    assert_eq!(f.functions[0].end_line, 14);
}

#[test]
fn test_cross_validate_engine_rs_functions() {
    let g = fixture();
    let f = &g.files["src/core/engine.rs"];
    assert_eq!(f.language, "rust");
    assert_eq!(f.functions.len(), 6);
    let names: Vec<&str> = f.functions.iter().map(|fn_| fn_.name.as_str()).collect();
    assert!(names.contains(&"Engine::new"));
    assert!(names.contains(&"Engine::run"));
    assert!(names.contains(&"Engine::internal_method"));
    assert!(names.contains(&"Engine::process"));
    assert!(names.contains(&"helper_function"));
    assert!(names.contains(&"public_function"));
}

#[test]
fn test_cross_validate_engine_rs_exports() {
    let g = fixture();
    let f = &g.files["src/core/engine.rs"];
    assert!(f.exports.contains(&"Engine".to_string()));
    assert!(f.exports.contains(&"Status".to_string()));
    assert!(f.exports.contains(&"Processable".to_string()));
    assert!(f.exports.contains(&"public_function".to_string()));
}

#[test]
fn test_cross_validate_handler_go_functions() {
    let g = fixture();
    let f = &g.files["src/services/handler.go"];
    assert_eq!(f.functions.len(), 3);
    let names: Vec<&str> = f.functions.iter().map(|fn_| fn_.name.as_str()).collect();
    assert!(names.contains(&"NewHandler"));
    assert!(names.contains(&"ServeHTTP"));
    assert!(names.contains(&"internalHelper"));
}

#[test]
fn test_cross_validate_helpers_py_exports() {
    let g = fixture();
    let f = &g.files["src/utils/helpers.py"];
    // __all__ = ["process_data", "DataProcessor"]
    assert_eq!(f.exports.len(), 2);
    assert!(f.exports.contains(&"process_data".to_string()));
    assert!(f.exports.contains(&"DataProcessor".to_string()));
}

#[test]
fn test_cross_validate_user_java_types() {
    let g = fixture();
    let f = &g.files["src/models/User.java"];
    assert_eq!(f.types.len(), 3);
    let type_names: Vec<&str> = f.types.iter().map(|t| t.name.as_str()).collect();
    assert!(type_names.contains(&"User"));
    assert!(type_names.contains(&"UserService"));
    assert!(type_names.contains(&"UserRole"));
}

// ── 可读取 Node.js 生成的 graph.json（格式兼容性）

#[test]
fn test_load_nodejs_graph_json() {
    use codegraph::graph::load_graph;
    let codemap_dir = std::path::Path::new(FIXTURE_DIR).join(".codemap");
    let graph = load_graph(&codemap_dir).expect("should load Node.js graph.json");
    assert_eq!(graph.version, "1.0");
    assert_eq!(graph.summary.total_files, 7);
    assert!(graph.files.contains_key("src/auth/login.ts"));
}
