/// 变量追踪集成测试
///
/// 验证全链路：语言适配器 extract_variables → scanner 转换 → query 查询
/// 使用内联源码，不依赖外部 fixture 文件。
use codegraph::languages::{self, get_adapter};
use codegraph::scanner::{convert_variables, convert_functions, convert_imports};
use codegraph::query::{query_symbol, QueryOptions};
use codegraph::graph::{
    CodeGraph, FileEntry, GraphConfig, GraphSummary, ModuleEntry, ProjectInfo,
};
use codegraph::traverser::Language;
use std::collections::HashMap;

// ── 辅助函数 ──────────────────────────────────────────────────────────────────

fn parse_variables(lang: Language, source: &str) -> Vec<languages::VariableInfo> {
    let adapter = get_adapter(lang);
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&adapter.language()).unwrap();
    let tree = parser.parse(source.as_bytes(), None).unwrap();
    adapter.extract_variables(&tree, source.as_bytes())
}

// ── TypeScript 变量提取 ──────────────────────────────────────────────────────

const TS_SOURCE: &str = r#"
import { helper } from './utils';

export const MAX_RETRIES = 3;
const API_URL = "https://example.com";
let counter = 0;
export const handler = (req: Request) => { return req; };

export function login(user: string): boolean {
    return true;
}
"#;

#[test]
fn test_ts_variables_count() {
    let vars = parse_variables(Language::TypeScript, TS_SOURCE);
    // handler 是箭头函数，应被过滤；剩余 MAX_RETRIES, API_URL, counter
    assert_eq!(vars.len(), 3, "expected 3 variables, got {:?}",
        vars.iter().map(|v| &v.name).collect::<Vec<_>>());
}

#[test]
fn test_ts_const_kind() {
    let vars = parse_variables(Language::TypeScript, TS_SOURCE);
    let max = vars.iter().find(|v| v.name == "MAX_RETRIES").expect("MAX_RETRIES not found");
    assert_eq!(max.kind, "const");
    assert!(max.is_exported, "MAX_RETRIES should be exported");
}

#[test]
fn test_ts_let_kind() {
    let vars = parse_variables(Language::TypeScript, TS_SOURCE);
    let ctr = vars.iter().find(|v| v.name == "counter").expect("counter not found");
    assert_eq!(ctr.kind, "let");
    assert!(!ctr.is_exported, "counter should not be exported");
}

#[test]
fn test_ts_non_exported_const() {
    let vars = parse_variables(Language::TypeScript, TS_SOURCE);
    let url = vars.iter().find(|v| v.name == "API_URL").expect("API_URL not found");
    assert_eq!(url.kind, "const");
    assert!(!url.is_exported, "API_URL should not be exported");
}

// ── JavaScript 变量提取 ─────────────────────────────────────────────────────

const JS_SOURCE: &str = r#"
const BASE_URL = "http://localhost";
let requestCount = 0;
export const TIMEOUT = 5000;
const noop = () => {};

function main() {}
"#;

#[test]
fn test_js_variables_count() {
    let vars = parse_variables(Language::JavaScript, JS_SOURCE);
    // noop 是箭头函数应被过滤；剩余 BASE_URL, requestCount, TIMEOUT
    assert_eq!(vars.len(), 3, "expected 3 JS variables, got {:?}",
        vars.iter().map(|v| &v.name).collect::<Vec<_>>());
}

#[test]
fn test_js_exported_const() {
    let vars = parse_variables(Language::JavaScript, JS_SOURCE);
    let timeout = vars.iter().find(|v| v.name == "TIMEOUT").expect("TIMEOUT not found");
    assert_eq!(timeout.kind, "const");
    assert!(timeout.is_exported);
}

// ── Python 变量提取 ──────────────────────────────────────────────────────────

const PY_SOURCE: &str = r#"
MAX_CONNECTIONS = 100
_internal_flag = True
db_url = "postgres://localhost"

def connect():
    pass

class Config:
    pass
"#;

#[test]
fn test_py_variables_count() {
    let vars = parse_variables(Language::Python, PY_SOURCE);
    assert_eq!(vars.len(), 3, "expected 3 Python variables, got {:?}",
        vars.iter().map(|v| &v.name).collect::<Vec<_>>());
}

#[test]
fn test_py_exported_by_naming() {
    let vars = parse_variables(Language::Python, PY_SOURCE);
    let max = vars.iter().find(|v| v.name == "MAX_CONNECTIONS").expect("MAX_CONNECTIONS not found");
    assert!(max.is_exported, "non-underscore name should be exported");
    let internal = vars.iter().find(|v| v.name == "_internal_flag").expect("_internal_flag not found");
    assert!(!internal.is_exported, "underscore-prefixed should not be exported");
}

// ── Rust 变量提取 ────────────────────────────────────────────────────────────

const RS_SOURCE: &str = r#"
pub const MAX_SIZE: usize = 1024;
const INTERNAL_LIMIT: u32 = 50;
pub static COUNTER: AtomicU32 = AtomicU32::new(0);
static PRIVATE_STATE: bool = false;

pub fn process() {}
"#;

#[test]
fn test_rust_variables_count() {
    let vars = parse_variables(Language::Rust, RS_SOURCE);
    assert_eq!(vars.len(), 4, "expected 4 Rust variables, got {:?}",
        vars.iter().map(|v| &v.name).collect::<Vec<_>>());
}

#[test]
fn test_rust_const_vs_static() {
    let vars = parse_variables(Language::Rust, RS_SOURCE);
    let max = vars.iter().find(|v| v.name == "MAX_SIZE").expect("MAX_SIZE not found");
    assert_eq!(max.kind, "const");
    assert!(max.is_exported);
    let ctr = vars.iter().find(|v| v.name == "COUNTER").expect("COUNTER not found");
    assert_eq!(ctr.kind, "static");
    assert!(ctr.is_exported);
}

#[test]
fn test_rust_private_variables() {
    let vars = parse_variables(Language::Rust, RS_SOURCE);
    let internal = vars.iter().find(|v| v.name == "INTERNAL_LIMIT").expect("INTERNAL_LIMIT not found");
    assert!(!internal.is_exported);
    let priv_state = vars.iter().find(|v| v.name == "PRIVATE_STATE").expect("PRIVATE_STATE not found");
    assert!(!priv_state.is_exported);
}

// ── Go 变量提取 ──────────────────────────────────────────────────────────────

const GO_SOURCE: &str = r#"
package main

var GlobalCount int = 0
var privateVal string = "hello"
const MaxRetries = 3
const internalLimit = 10

func main() {}
"#;

#[test]
fn test_go_variables_count() {
    let vars = parse_variables(Language::Go, GO_SOURCE);
    assert_eq!(vars.len(), 4, "expected 4 Go variables, got {:?}",
        vars.iter().map(|v| &v.name).collect::<Vec<_>>());
}

#[test]
fn test_go_exported_by_case() {
    let vars = parse_variables(Language::Go, GO_SOURCE);
    let gc = vars.iter().find(|v| v.name == "GlobalCount").expect("GlobalCount not found");
    assert!(gc.is_exported, "uppercase first letter = exported");
    assert_eq!(gc.kind, "var");
    let mr = vars.iter().find(|v| v.name == "MaxRetries").expect("MaxRetries not found");
    assert!(mr.is_exported);
    assert_eq!(mr.kind, "const");
    let pv = vars.iter().find(|v| v.name == "privateVal").expect("privateVal not found");
    assert!(!pv.is_exported, "lowercase first letter = private");
}

// ── C 变量提取 ───────────────────────────────────────────────────────────────

const C_SOURCE: &str = r#"
int globalCount = 0;
const int MAX = 100;
static int internalVal = 5;
extern int sharedVal;

void helper() {}
"#;

#[test]
fn test_c_variables_count() {
    let vars = parse_variables(Language::C, C_SOURCE);
    assert_eq!(vars.len(), 4, "expected 4 C variables, got {:?}",
        vars.iter().map(|v| &v.name).collect::<Vec<_>>());
}

#[test]
fn test_c_const_and_static() {
    let vars = parse_variables(Language::C, C_SOURCE);
    let max = vars.iter().find(|v| v.name == "MAX").expect("MAX not found");
    assert_eq!(max.kind, "const");
    assert!(max.is_exported);
    let internal = vars.iter().find(|v| v.name == "internalVal").expect("internalVal not found");
    assert!(!internal.is_exported, "static should not be exported");
}

// ── C++ 变量提取 ─────────────────────────────────────────────────────────────

const CPP_SOURCE: &str = r#"
int globalVal = 42;
const int MAX_CONN = 10;
static int internalCounter = 0;
constexpr double PI = 3.14;
"#;

#[test]
fn test_cpp_variables_count() {
    let vars = parse_variables(Language::Cpp, CPP_SOURCE);
    assert_eq!(vars.len(), 4, "expected 4 C++ variables, got {:?}",
        vars.iter().map(|v| &v.name).collect::<Vec<_>>());
}

#[test]
fn test_cpp_constexpr_is_const() {
    let vars = parse_variables(Language::Cpp, CPP_SOURCE);
    let pi = vars.iter().find(|v| v.name == "PI").expect("PI not found");
    assert_eq!(pi.kind, "const", "constexpr should be detected as const");
}

// ── Java 变量提取 ────────────────────────────────────────────────────────────

const JAVA_SOURCE: &str = r#"
public class AppConfig {
    public static final int MAX_THREADS = 8;
    private static int instanceCount = 0;
    public static String APP_NAME = "MyApp";

    public void run() {}
}
"#;

#[test]
fn test_java_variables_count() {
    let vars = parse_variables(Language::Java, JAVA_SOURCE);
    assert_eq!(vars.len(), 3, "expected 3 Java static variables, got {:?}",
        vars.iter().map(|v| &v.name).collect::<Vec<_>>());
}

#[test]
fn test_java_final_static_is_const() {
    let vars = parse_variables(Language::Java, JAVA_SOURCE);
    let max = vars.iter().find(|v| v.name == "MAX_THREADS").expect("MAX_THREADS not found");
    assert_eq!(max.kind, "const", "static final should be const");
    assert!(max.is_exported, "public should be exported");
}

#[test]
fn test_java_private_static() {
    let vars = parse_variables(Language::Java, JAVA_SOURCE);
    let ic = vars.iter().find(|v| v.name == "instanceCount").expect("instanceCount not found");
    assert_eq!(ic.kind, "static");
    assert!(!ic.is_exported, "private should not be exported");
}

// ── 全链路测试：变量 → 转换 → 查询 ──────────────────────────────────────────

fn make_variable_graph() -> CodeGraph {
    // 用 TS 源码解析出变量，走完整转换链路
    let adapter = get_adapter(Language::TypeScript);
    let mut parser = tree_sitter::Parser::new();
    parser.set_language(&adapter.language()).unwrap();
    let tree = parser.parse(TS_SOURCE.as_bytes(), None).unwrap();

    let lang_vars = adapter.extract_variables(&tree, TS_SOURCE.as_bytes());
    let lang_fns = adapter.extract_functions(&tree, TS_SOURCE.as_bytes());
    let lang_imports = adapter.extract_imports(&tree, TS_SOURCE.as_bytes());

    let variables = convert_variables(&lang_vars);
    let functions = convert_functions(&lang_fns);
    let imports = convert_imports(&lang_imports);

    let mut files = HashMap::new();
    files.insert(
        "src/app.ts".to_string(),
        FileEntry {
            language: "typescript".into(),
            module: "app".into(),
            hash: "sha256:test".into(),
            lines: 12,
            functions,
            classes: vec![],
            types: vec![],
            variables,
            imports,
            exports: vec!["MAX_RETRIES".into(), "handler".into(), "login".into()],
            is_entry_point: false,
            symbol_refs: std::collections::BTreeMap::new(),
        },
    );

    let mut modules = HashMap::new();
    modules.insert("app".into(), ModuleEntry {
        files: vec!["src/app.ts".into()],
        depends_on: vec![],
        depended_by: vec![],
    });

    CodeGraph {
        version: "1.0".into(),
        project: ProjectInfo { name: "test".into(), root: "/test".into() },
        scanned_at: "2026-01-01T00:00:00.000Z".into(),
        commit_hash: None,
        config: GraphConfig { languages: vec![], exclude_patterns: vec![] },
        summary: GraphSummary {
            total_files: 1,
            total_functions: 2,
            total_classes: 0,
            total_variables: 3,
            languages: HashMap::new(),
            modules: vec!["app".into()],
            entry_points: vec![],
        },
        modules,
        files,
    }
}

#[test]
fn test_query_variable_by_name() {
    let graph = make_variable_graph();
    let opts = QueryOptions { type_filter: Some("variable".into()) };
    let results = query_symbol(&graph, "MAX_RETRIES", &opts);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].kind, "variable");
    assert_eq!(results[0].name, "MAX_RETRIES");
}

#[test]
fn test_query_variable_has_signature() {
    let graph = make_variable_graph();
    let opts = QueryOptions { type_filter: Some("variable".into()) };
    let results = query_symbol(&graph, "MAX_RETRIES", &opts);
    assert_eq!(results[0].signature.as_deref(), Some("const MAX_RETRIES"));
}

#[test]
fn test_query_variable_excluded_by_function_filter() {
    let graph = make_variable_graph();
    let opts = QueryOptions { type_filter: Some("function".into()) };
    let results = query_symbol(&graph, "MAX_RETRIES", &opts);
    assert!(results.is_empty(), "variable should not appear with function filter");
}

#[test]
fn test_query_no_filter_includes_variables() {
    let graph = make_variable_graph();
    let opts = QueryOptions { type_filter: None };
    let results = query_symbol(&graph, "MAX_RETRIES", &opts);
    assert_eq!(results.len(), 1, "variable should appear without type filter");
    assert_eq!(results[0].kind, "variable");
}

// ── 向后兼容：旧 JSON 无 variables 字段 ─────────────────────────────────────

#[test]
fn test_backward_compat_no_variables_field() {
    let json = r#"{
        "version": "1.0",
        "project": {"name": "old", "root": "/old"},
        "scannedAt": "2025-01-01T00:00:00.000Z",
        "config": {"languages": [], "excludePatterns": []},
        "summary": {"totalFiles": 1, "totalFunctions": 1, "totalClasses": 0, "languages": {}, "modules": ["m"], "entryPoints": []},
        "modules": {"m": {"files": ["a.ts"], "dependsOn": [], "dependedBy": []}},
        "files": {
            "a.ts": {
                "language": "typescript",
                "module": "m",
                "hash": "sha256:old",
                "lines": 5,
                "functions": [],
                "classes": [],
                "types": [],
                "imports": [],
                "exports": [],
                "isEntryPoint": false
            }
        }
    }"#;
    let graph: CodeGraph = serde_json::from_str(json).expect("should deserialize old JSON without variables");
    let file = graph.files.get("a.ts").unwrap();
    assert!(file.variables.is_empty(), "missing variables field should default to empty vec");
    assert_eq!(graph.summary.total_variables, 0, "missing totalVariables should default to 0");
}
