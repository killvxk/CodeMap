/// fixture 兼容性验证测试
///
/// 对 ccplugin/cli/test/fixtures/sample-project/ 中的每种语言文件，
/// 用 Rust 适配器解析并验证提取结果与 Node.js 版本预期一致。
use codegraph::languages::get_adapter;
use codegraph::traverser::Language;

const FIXTURE_BASE: &str = "E:/2026/CodeMap/ccplugin/cli/test/fixtures/sample-project";

fn parse_fixture(lang: Language, rel_path: &str) -> (
    Vec<codegraph::languages::FunctionInfo>,
    Vec<codegraph::languages::ImportInfo>,
    Vec<codegraph::languages::ExportInfo>,
    Vec<codegraph::languages::ClassInfo>,
) {
    let path = format!("{}/{}", FIXTURE_BASE, rel_path);
    let source = std::fs::read(&path)
        .unwrap_or_else(|e| panic!("Cannot read {}: {}", path, e));

    let adapter = get_adapter(lang);
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&adapter.language()).unwrap();
    let tree = parser.parse(&source, None).unwrap();

    (
        adapter.extract_functions(&tree, &source),
        adapter.extract_imports(&tree, &source),
        adapter.extract_exports(&tree, &source),
        adapter.extract_classes(&tree, &source),
    )
}

// ── TypeScript: src/auth/login.ts ─────────────────────────────────────────────

#[test]
fn test_ts_login_functions() {
    let (fns, _, _, _) = parse_fixture(Language::TypeScript, "src/auth/login.ts");
    let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"login"), "expected 'login', got {:?}", names);
}

#[test]
fn test_ts_login_imports() {
    let (_, imports, _, _) = parse_fixture(Language::TypeScript, "src/auth/login.ts");
    assert!(
        imports.iter().any(|i| i.source.contains("db/users")),
        "expected import from '../db/users', got {:?}",
        imports.iter().map(|i| &i.source).collect::<Vec<_>>()
    );
    assert!(
        imports.iter().any(|i| i.source == "bcrypt"),
        "expected import 'bcrypt', got {:?}",
        imports.iter().map(|i| &i.source).collect::<Vec<_>>()
    );
}

#[test]
fn test_ts_login_exports() {
    let (_, _, exports, _) = parse_fixture(Language::TypeScript, "src/auth/login.ts");
    let names: Vec<&str> = exports.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"login"), "expected export 'login', got {:?}", names);
    assert!(names.contains(&"LoginOptions"), "expected export 'LoginOptions', got {:?}", names);
}

#[test]
fn test_ts_login_classes() {
    let (_, _, _, classes) = parse_fixture(Language::TypeScript, "src/auth/login.ts");
    // LoginOptions 是 interface
    assert!(
        classes.iter().any(|c| c.name == "LoginOptions" && c.kind == "interface"),
        "expected interface 'LoginOptions', got {:?}",
        classes.iter().map(|c| (&c.name, &c.kind)).collect::<Vec<_>>()
    );
}

// ── TypeScript: src/api/routes.ts ─────────────────────────────────────────────

#[test]
fn test_ts_routes_functions() {
    let (fns, _, _, _) = parse_fixture(Language::TypeScript, "src/api/routes.ts");
    let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"handleLogin"), "expected 'handleLogin', got {:?}", names);
}

#[test]
fn test_ts_routes_imports() {
    let (_, imports, _, _) = parse_fixture(Language::TypeScript, "src/api/routes.ts");
    assert!(
        imports.iter().any(|i| i.source.contains("auth/login") && i.names.contains(&"login".to_string())),
        "expected import {{login}} from '../auth/login', got {:?}",
        imports
    );
}

#[test]
fn test_ts_routes_exports() {
    let (_, _, exports, _) = parse_fixture(Language::TypeScript, "src/api/routes.ts");
    let names: Vec<&str> = exports.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"handleLogin"), "expected export 'handleLogin', got {:?}", names);
}

// ── Python: src/utils/helpers.py ─────────────────────────────────────────────

#[test]
fn test_py_helpers_functions() {
    let (fns, _, _, _) = parse_fixture(Language::Python, "src/utils/helpers.py");
    let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"process_data"), "expected 'process_data', got {:?}", names);
    assert!(names.contains(&"_internal_helper"), "expected '_internal_helper', got {:?}", names);
}

#[test]
fn test_py_helpers_imports() {
    let (_, imports, _, _) = parse_fixture(Language::Python, "src/utils/helpers.py");
    let sources: Vec<&str> = imports.iter().map(|i| i.source.as_str()).collect();
    assert!(sources.contains(&"os"), "expected import 'os', got {:?}", sources);
    assert!(sources.contains(&"sys"), "expected import 'sys', got {:?}", sources);
    assert!(
        imports.iter().any(|i| i.source == "os.path" && i.names.contains(&"join".to_string())),
        "expected 'from os.path import join', got {:?}", imports
    );
}

#[test]
fn test_py_helpers_exports_dunder_all() {
    let (_, _, exports, _) = parse_fixture(Language::Python, "src/utils/helpers.py");
    // __all__ = ["process_data", "DataProcessor"]
    let names: Vec<&str> = exports.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names.len(), 2, "__all__ should yield exactly 2 exports, got {:?}", names);
    assert!(names.contains(&"process_data"), "expected 'process_data' in __all__, got {:?}", names);
    assert!(names.contains(&"DataProcessor"), "expected 'DataProcessor' in __all__, got {:?}", names);
}

#[test]
fn test_py_helpers_classes() {
    let (_, _, _, classes) = parse_fixture(Language::Python, "src/utils/helpers.py");
    assert!(
        classes.iter().any(|c| c.name == "DataProcessor"),
        "expected class 'DataProcessor', got {:?}",
        classes.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
    let dp = classes.iter().find(|c| c.name == "DataProcessor").unwrap();
    assert!(dp.methods.contains(&"__init__".to_string()), "expected method '__init__'");
    assert!(dp.methods.contains(&"run".to_string()), "expected method 'run'");
}

// ── Go: src/services/handler.go ──────────────────────────────────────────────

#[test]
fn test_go_handler_functions() {
    let (fns, _, _, _) = parse_fixture(Language::Go, "src/services/handler.go");
    let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"NewHandler"), "expected 'NewHandler', got {:?}", names);
    assert!(names.contains(&"ServeHTTP"), "expected 'ServeHTTP', got {:?}", names);
    assert!(names.contains(&"internalHelper"), "expected 'internalHelper', got {:?}", names);
}

#[test]
fn test_go_handler_exports() {
    let (_, _, exports, _) = parse_fixture(Language::Go, "src/services/handler.go");
    let names: Vec<&str> = exports.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"NewHandler"), "expected exported 'NewHandler', got {:?}", names);
    assert!(names.contains(&"ServeHTTP"), "expected exported 'ServeHTTP', got {:?}", names);
    assert!(!names.contains(&"internalHelper"), "'internalHelper' should NOT be exported, got {:?}", names);
}

#[test]
fn test_go_handler_imports() {
    let (_, imports, _, _) = parse_fixture(Language::Go, "src/services/handler.go");
    let sources: Vec<&str> = imports.iter().map(|i| i.source.as_str()).collect();
    assert!(sources.contains(&"fmt"), "expected import 'fmt', got {:?}", sources);
    assert!(sources.contains(&"net/http"), "expected import 'net/http', got {:?}", sources);
    assert!(sources.contains(&"encoding/json"), "expected import 'encoding/json', got {:?}", sources);
}

#[test]
fn test_go_handler_structs() {
    let (_, _, _, classes) = parse_fixture(Language::Go, "src/services/handler.go");
    let names: Vec<&str> = classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Handler"), "expected struct 'Handler', got {:?}", names);
    assert!(names.contains(&"Response"), "expected struct 'Response', got {:?}", names);
}

// ── Rust: src/core/engine.rs ─────────────────────────────────────────────────

#[test]
fn test_rust_engine_functions() {
    let (fns, _, _, _) = parse_fixture(Language::Rust, "src/core/engine.rs");
    let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"Engine::new"), "expected 'Engine::new', got {:?}", names);
    assert!(names.contains(&"Engine::run"), "expected 'Engine::run', got {:?}", names);
    assert!(names.contains(&"helper_function"), "expected 'helper_function', got {:?}", names);
    assert!(names.contains(&"public_function"), "expected 'public_function', got {:?}", names);
}

#[test]
fn test_rust_engine_exports() {
    let (_, _, exports, _) = parse_fixture(Language::Rust, "src/core/engine.rs");
    let names: Vec<&str> = exports.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"Engine"), "expected pub 'Engine', got {:?}", names);
    assert!(names.contains(&"Status"), "expected pub 'Status', got {:?}", names);
    assert!(names.contains(&"Processable"), "expected pub 'Processable', got {:?}", names);
    assert!(names.contains(&"public_function"), "expected pub 'public_function', got {:?}", names);
    assert!(!names.contains(&"helper_function"), "'helper_function' should NOT be exported, got {:?}", names);
}

#[test]
fn test_rust_engine_imports() {
    let (_, imports, _, _) = parse_fixture(Language::Rust, "src/core/engine.rs");
    assert!(
        imports.iter().any(|i| i.source.contains("std::io")),
        "expected import from 'std::io', got {:?}",
        imports.iter().map(|i| &i.source).collect::<Vec<_>>()
    );
}

#[test]
fn test_rust_engine_classes() {
    let (_, _, _, classes) = parse_fixture(Language::Rust, "src/core/engine.rs");
    let names: Vec<&str> = classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Engine"), "expected struct 'Engine', got {:?}", names);
    assert!(names.contains(&"Status"), "expected enum 'Status', got {:?}", names);
    assert!(names.contains(&"Processable"), "expected trait 'Processable', got {:?}", names);
}

// ── Java: src/models/User.java ────────────────────────────────────────────────

#[test]
fn test_java_user_functions() {
    let (fns, _, _, _) = parse_fixture(Language::Java, "src/models/User.java");
    let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"User.getName"), "expected 'User.getName', got {:?}", names);
    assert!(names.contains(&"User.setEmail"), "expected 'User.setEmail', got {:?}", names);
    assert!(names.contains(&"User.validate"), "expected 'User.validate', got {:?}", names);
}

#[test]
fn test_java_user_imports() {
    let (_, imports, _, _) = parse_fixture(Language::Java, "src/models/User.java");
    assert!(
        imports.iter().any(|i| i.source == "java.util" && i.names.contains(&"List".to_string())),
        "expected 'import java.util.List', got {:?}", imports
    );
    assert!(
        imports.iter().any(|i| i.source == "java.util" && i.names.contains(&"Optional".to_string())),
        "expected 'import java.util.Optional', got {:?}", imports
    );
}

#[test]
fn test_java_user_exports() {
    let (_, _, exports, _) = parse_fixture(Language::Java, "src/models/User.java");
    let names: Vec<&str> = exports.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"User"), "expected public class 'User', got {:?}", names);
    // UserService 和 UserRole 没有 public 修饰符
    assert!(!names.contains(&"UserService"), "'UserService' has no public modifier, got {:?}", names);
}

#[test]
fn test_java_user_classes() {
    let (_, _, _, classes) = parse_fixture(Language::Java, "src/models/User.java");
    let names: Vec<&str> = classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"User"), "expected class 'User', got {:?}", names);
    assert!(names.contains(&"UserService"), "expected interface 'UserService', got {:?}", names);
    assert!(names.contains(&"UserRole"), "expected enum 'UserRole', got {:?}", names);
    let user = classes.iter().find(|c| c.name == "User").unwrap();
    assert!(user.methods.contains(&"getName".to_string()), "expected method 'getName'");
}

// ── C++: src/native/engine.cpp ────────────────────────────────────────────────

#[test]
fn test_cpp_engine_functions() {
    let (fns, _, _, _) = parse_fixture(Language::Cpp, "src/native/engine.cpp");
    let names: Vec<&str> = fns.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"Engine::Engine"), "expected 'Engine::Engine', got {:?}", names);
    assert!(names.contains(&"Engine::start"), "expected 'Engine::start', got {:?}", names);
    assert!(names.contains(&"Engine::stop"), "expected 'Engine::stop', got {:?}", names);
    assert!(names.contains(&"initialize"), "expected 'initialize', got {:?}", names);
}

#[test]
fn test_cpp_engine_exports() {
    let (_, _, exports, _) = parse_fixture(Language::Cpp, "src/native/engine.cpp");
    let names: Vec<&str> = exports.iter().map(|e| e.name.as_str()).collect();
    // static internalHelper 不应导出
    assert!(!names.contains(&"internalHelper"), "'internalHelper' is static, should NOT be exported, got {:?}", names);
    assert!(names.contains(&"initialize"), "expected export 'initialize', got {:?}", names);
}

#[test]
fn test_cpp_engine_includes() {
    let (_, imports, _, _) = parse_fixture(Language::Cpp, "src/native/engine.cpp");
    let sources: Vec<&str> = imports.iter().map(|i| i.source.as_str()).collect();
    assert!(sources.contains(&"iostream"), "expected #include <iostream>, got {:?}", sources);
    assert!(sources.contains(&"vector"), "expected #include <vector>, got {:?}", sources);
    assert!(sources.contains(&"utils.h"), "expected #include \"utils.h\", got {:?}", sources);
}

#[test]
fn test_cpp_engine_classes() {
    let (_, _, _, classes) = parse_fixture(Language::Cpp, "src/native/engine.cpp");
    let names: Vec<&str> = classes.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Engine"), "expected class 'Engine', got {:?}", names);
    assert!(names.contains(&"Config"), "expected struct 'Config', got {:?}", names);
}

#[test]
fn test_cpp_namespace_not_in_exports() {
    let (_, _, exports, _) = parse_fixture(Language::Cpp, "src/native/engine.cpp");
    let names: Vec<&str> = exports.iter().map(|e| e.name.as_str()).collect();
    // Node.js 行为：C++ exports 包含 class/struct/enum/function，不包含 namespace
    assert!(names.contains(&"Engine"), "expected 'Engine' export, got {:?}", names);
    assert!(names.contains(&"Config"), "expected 'Config' export, got {:?}", names);
    assert!(!names.contains(&"engine"), "namespace 'engine' should NOT be in exports, got {:?}", names);
}
