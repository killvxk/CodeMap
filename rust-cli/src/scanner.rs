use crate::graph::{
    compute_file_hash, create_empty_graph, is_entry_point, save_graph, CodeGraph, FileEntry,
    FunctionInfo as GraphFunctionInfo, ClassInfo as GraphClassInfo,
    TypeInfo as GraphTypeInfo, ImportInfo as GraphImportInfo, ModuleEntry,
};
use crate::languages;
use crate::path_utils::{normalize_path, strip_extension};
use crate::traverser::{detect_language, effective_language, has_cpp_source_files, traverse_files, Language};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// 公共转换函数：languages 层类型 → graph 层类型
// ---------------------------------------------------------------------------

pub fn convert_functions(lang_functions: &[languages::FunctionInfo]) -> Vec<GraphFunctionInfo> {
    lang_functions.iter().map(|f| {
        let sig = if f.params.is_empty() {
            format!("{}()", f.name)
        } else {
            format!("{}({})", f.name, f.params.join(", "))
        };
        GraphFunctionInfo {
            name: f.name.clone(),
            signature: sig,
            start_line: f.start_line as u32,
            end_line: f.end_line as u32,
        }
    }).collect()
}

pub fn convert_classes(lang_classes: &[languages::ClassInfo]) -> Vec<GraphClassInfo> {
    lang_classes.iter()
        .filter(|c| matches!(c.kind.as_str(), "class" | "struct"))
        .map(|c| GraphClassInfo {
            name: c.name.clone(),
            start_line: c.start_line as u32,
            end_line: c.end_line as u32,
        }).collect()
}

pub fn convert_types(lang_classes: &[languages::ClassInfo], lang: Language) -> Vec<GraphTypeInfo> {
    lang_classes.iter()
        .filter(|c| {
            !(lang == Language::Python && c.kind.as_str() == "class")
        })
        .map(|c| GraphTypeInfo {
            name: c.name.clone(),
            kind: c.kind.clone(),
            start_line: c.start_line as u32,
            end_line: c.end_line as u32,
        }).collect()
}

pub fn convert_imports(lang_imports: &[languages::ImportInfo]) -> Vec<GraphImportInfo> {
    lang_imports.iter().map(|i| GraphImportInfo {
        source: i.source.clone(),
        symbols: i.names.clone(),
        is_external: !i.source.starts_with('.'),
    }).collect()
}

pub fn convert_exports(lang_exports: &[languages::ExportInfo]) -> Vec<String> {
    lang_exports.iter().map(|e| e.name.clone()).collect()
}

/// 根目录级别的常见目录名，跳过这些层级来确定模块名
const COMMON_ROOT_DIRS: &[&str] = &["src", "lib", "app", "source", "packages"];

/// 从文件路径推断模块名
///
/// 策略：
/// - 取相对路径，去掉文件名
/// - 跳过开头的 COMMON_ROOT_DIRS 目录
/// - 第一个剩余目录段即为模块名
/// - 若无目录段，返回 "_root"
pub fn detect_module_name(file_path: &Path, root_dir: &Path) -> String {
    let rel = match file_path.strip_prefix(root_dir) {
        Ok(r) => r,
        Err(_) => return "_root".to_string(),
    };

    // 收集目录段（去掉文件名）
    let mut segments: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();

    // 去掉最后一个（文件名）
    if !segments.is_empty() {
        segments.pop();
    }

    // 跳过开头的 COMMON_ROOT_DIRS
    while !segments.is_empty() && COMMON_ROOT_DIRS.contains(&segments[0].as_str()) {
        segments.remove(0);
    }

    if segments.is_empty() {
        "_root".to_string()
    } else {
        segments[0].clone()
    }
}

/// 扫描整个项目，构建 CodeGraph
pub fn scan_project(root_dir: &Path, exclude: &[String]) -> anyhow::Result<CodeGraph> {
    let project_name = root_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");
    let root_str = root_dir.to_string_lossy().replace('\\', "/");
    let mut graph = create_empty_graph(project_name, &root_str);

    // Step 1: 遍历文件
    let files = traverse_files(root_dir, exclude);
    let has_cpp = has_cpp_source_files(&files);

    // Step 2: 解析每个文件
    struct FileInfo {
        rel_path: String,
        language: String,
        module_name: String,
        hash: String,
        lines: u32,
        functions: Vec<crate::graph::FunctionInfo>,
        classes: Vec<crate::graph::ClassInfo>,
        types: Vec<crate::graph::TypeInfo>,
        imports: Vec<crate::graph::ImportInfo>,
        exports: Vec<String>,
        is_entry_point: bool,
    }

    let mut file_infos: Vec<(PathBuf, FileInfo)> = Vec::new();
    let mut language_counts: HashMap<String, u32> = HashMap::new();
    let mut total_functions = 0u32;
    let mut total_classes = 0u32;
    let mut module_set: HashSet<String> = HashSet::new();

    for abs_path in &files {
        let base_lang = match detect_language(abs_path) {
            Some(l) => l,
            None => continue,
        };
        let lang = effective_language(abs_path, base_lang, has_cpp);

        let content = match std::fs::read(abs_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let hash = compute_file_hash(&content);
        let adapter = languages::get_adapter(lang);

        // 用语言适配器解析
        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser.set_language(&adapter.language()).ok();
        let tree = match ts_parser.parse(&content, None) {
            Some(t) => t,
            None => continue,
        };

        let lang_functions = adapter.extract_functions(&tree, &content);
        let lang_imports = adapter.extract_imports(&tree, &content);
        let lang_exports = adapter.extract_exports(&tree, &content);
        let lang_classes = adapter.extract_classes(&tree, &content);
        let lines = content.iter().filter(|&&b| b == b'\n').count() as u32 + 1;

        // 转换为 graph 数据结构
        let functions = convert_functions(&lang_functions);
        let classes = convert_classes(&lang_classes);
        let types = convert_types(&lang_classes, lang);
        let imports = convert_imports(&lang_imports);
        let exports = convert_exports(&lang_exports);

        let module_name = detect_module_name(abs_path, root_dir);
        module_set.insert(module_name.clone());

        let lang_str = lang.as_str().to_string();
        *language_counts.entry(lang_str.clone()).or_insert(0) += 1;
        total_functions += functions.len() as u32;
        total_classes += classes.len() as u32;

        let rel_path = abs_path
            .strip_prefix(root_dir)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .replace('\\', "/");

        let entry = FileInfo {
            rel_path,
            language: lang_str,
            module_name,
            hash,
            lines,
            functions,
            classes,
            types,
            imports,
            exports,
            is_entry_point: is_entry_point(abs_path),
        };
        file_infos.push((abs_path.clone(), entry));
    }

    // Step 3: 初始化模块表
    let mut modules: HashMap<String, ModuleEntry> = HashMap::new();
    for mod_name in &module_set {
        modules.insert(
            mod_name.clone(),
            ModuleEntry {
                files: vec![],
                depends_on: vec![],
                depended_by: vec![],
            },
        );
    }

    // 构建路径 → 模块名的查找表（O(1) 导入解析）
    let mut path_lookup: HashMap<String, String> = HashMap::new();
    for (abs_path, info) in &file_infos {
        let norm = abs_path.to_string_lossy().replace('\\', "/");
        path_lookup.insert(norm.clone(), info.module_name.clone());
        // 无扩展名版本
        let without_ext = strip_extension(&norm);
        path_lookup
            .entry(without_ext)
            .or_insert_with(|| info.module_name.clone());
    }

    // Step 4: 填充 graph.files 并解析跨模块依赖
    let mut depends_on_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut depended_by_map: HashMap<String, HashSet<String>> = HashMap::new();
    for mod_name in &module_set {
        depends_on_map.insert(mod_name.clone(), HashSet::new());
        depended_by_map.insert(mod_name.clone(), HashSet::new());
    }

    for (abs_path, info) in &file_infos {
        // 解析导入依赖
        for imp in &info.imports {
            if imp.is_external {
                continue;
            }
            if let Some(target_mod) =
                resolve_import_module(abs_path, &imp.source, &path_lookup, &info.module_name)
            {
                if target_mod != info.module_name {
                    depends_on_map
                        .entry(info.module_name.clone())
                        .or_default()
                        .insert(target_mod.clone());
                    depended_by_map
                        .entry(target_mod)
                        .or_default()
                        .insert(info.module_name.clone());
                }
            }
        }

        // 写入 graph.files
        graph.files.insert(
            info.rel_path.clone(),
            FileEntry {
                language: info.language.clone(),
                module: info.module_name.clone(),
                hash: info.hash.clone(),
                lines: info.lines,
                functions: info.functions.clone(),
                classes: info.classes.clone(),
                types: info.types.clone(),
                imports: info.imports.clone(),
                exports: info.exports.clone(),
                is_entry_point: info.is_entry_point,
            },
        );

        // 将文件加入模块
        if let Some(m) = modules.get_mut(&info.module_name) {
            m.files.push(info.rel_path.clone());
        }
    }

    // Step 5: 填充 graph.modules（Set → 排序数组）
    for (mod_name, mod_entry) in &mut modules {
        let mut dep_on: Vec<String> = depends_on_map
            .get(mod_name)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        dep_on.sort();
        mod_entry.depends_on = dep_on;

        let mut dep_by: Vec<String> = depended_by_map
            .get(mod_name)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        dep_by.sort();
        mod_entry.depended_by = dep_by;
    }
    graph.modules = modules;

    // Step 6: 构建 summary
    graph.summary.total_files = file_infos.len() as u32;
    graph.summary.total_functions = total_functions;
    graph.summary.total_classes = total_classes;
    graph.summary.languages = language_counts.clone();
    let mut mod_list: Vec<String> = module_set.into_iter().collect();
    mod_list.sort();
    graph.summary.modules = mod_list;
    graph.summary.entry_points = graph
        .files
        .iter()
        .filter(|(_, f)| f.is_entry_point)
        .map(|(p, _)| p.clone())
        .collect();
    graph.summary.entry_points.sort();
    graph.config.languages = {
        let mut langs: Vec<String> = language_counts.into_keys().collect();
        langs.sort();
        langs
    };

    Ok(graph)
}

/// 解析相对导入，返回目标模块名
///
/// 注意：当前仅支持 JS/TS 的相对路径导入（以 `.` 开头）。
/// Go/Rust/Java/C/C++ 的 import 不以 `.` 开头，会被标记为 external 并跳过，
/// 因此这些语言的模块间依赖关系暂不解析。
fn resolve_import_module(
    importer_path: &Path,
    import_source: &str,
    path_lookup: &HashMap<String, String>,
    _fallback: &str,
) -> Option<String> {
    if !import_source.starts_with('.') {
        return None;
    }

    let importer_dir = importer_path.parent()?;
    let joined = importer_dir.join(import_source);
    // 手动规范化路径（解析 ".." 组件），避免 canonicalize 的 UNC 路径问题
    let resolved = normalize_path(&joined).replace('\\', "/");

    // 直接匹配
    if let Some(m) = path_lookup.get(&resolved) {
        return Some(m.clone());
    }

    // 无扩展名匹配（pathLookup 已索引）
    // index 文件解析：'./auth' → './auth/index'
    let index_path = format!("{}/index", resolved);
    if let Some(m) = path_lookup.get(&index_path) {
        return Some(m.clone());
    }

    None
}

/// 扫描并保存到 .codemap/ 目录
pub fn scan_and_save(root_dir: &Path, exclude: &[String]) -> anyhow::Result<CodeGraph> {
    let graph = scan_project(root_dir, exclude)?;
    let output_dir = root_dir.join(".codemap");
    save_graph(&output_dir, &graph)?;
    Ok(graph)
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_detect_module_name_root() {
        let root = Path::new("/project");
        let file = Path::new("/project/main.rs");
        assert_eq!(detect_module_name(file, root), "_root");
    }

    #[test]
    fn test_detect_module_name_src() {
        let root = Path::new("/project");
        let file = Path::new("/project/src/auth/login.ts");
        assert_eq!(detect_module_name(file, root), "auth");
    }

    #[test]
    fn test_detect_module_name_direct_subdir() {
        let root = Path::new("/project");
        let file = Path::new("/project/utils/helper.ts");
        assert_eq!(detect_module_name(file, root), "utils");
    }

    #[test]
    fn test_detect_module_name_src_root() {
        let root = Path::new("/project");
        let file = Path::new("/project/src/index.ts");
        assert_eq!(detect_module_name(file, root), "_root");
    }

    #[test]
    fn test_convert_functions_generates_signature() {
        let lang_fns = vec![
            crate::languages::FunctionInfo {
                name: "greet".to_string(),
                start_line: 1,
                end_line: 3,
                params: vec!["name".to_string(), "age".to_string()],
                is_exported: true,
            },
            crate::languages::FunctionInfo {
                name: "noop".to_string(),
                start_line: 5,
                end_line: 6,
                params: vec![],
                is_exported: false,
            },
        ];
        let result = convert_functions(&lang_fns);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].signature, "greet(name, age)");
        assert_eq!(result[1].signature, "noop()");
    }

    #[test]
    fn test_convert_classes_filters_non_class() {
        let lang_classes = vec![
            crate::languages::ClassInfo {
                name: "MyClass".to_string(),
                start_line: 1,
                end_line: 10,
                methods: vec![],
                kind: "class".to_string(),
            },
            crate::languages::ClassInfo {
                name: "MyTrait".to_string(),
                start_line: 12,
                end_line: 20,
                methods: vec![],
                kind: "trait".to_string(),
            },
            crate::languages::ClassInfo {
                name: "MyStruct".to_string(),
                start_line: 22,
                end_line: 30,
                methods: vec![],
                kind: "struct".to_string(),
            },
        ];
        let classes = convert_classes(&lang_classes);
        // 只保留 class 和 struct
        assert_eq!(classes.len(), 2);
        assert!(classes.iter().any(|c| c.name == "MyClass"));
        assert!(classes.iter().any(|c| c.name == "MyStruct"));
    }

    #[test]
    fn test_convert_types_excludes_python_class() {
        use crate::traverser::Language;
        let lang_classes = vec![
            crate::languages::ClassInfo {
                name: "MyClass".to_string(),
                start_line: 1,
                end_line: 10,
                methods: vec![],
                kind: "class".to_string(),
            },
            crate::languages::ClassInfo {
                name: "MyEnum".to_string(),
                start_line: 12,
                end_line: 20,
                methods: vec![],
                kind: "enum".to_string(),
            },
        ];
        // Python: class 不进入 types
        let types_py = convert_types(&lang_classes, Language::Python);
        assert_eq!(types_py.len(), 1);
        assert_eq!(types_py[0].name, "MyEnum");

        // TypeScript: class 进入 types
        let types_ts = convert_types(&lang_classes, Language::TypeScript);
        assert_eq!(types_ts.len(), 2);
    }

    #[test]
    fn test_convert_imports_is_external() {
        let lang_imports = vec![
            crate::languages::ImportInfo {
                source: "./utils".to_string(),
                names: vec!["helper".to_string()],
                is_default: false,
            },
            crate::languages::ImportInfo {
                source: "react".to_string(),
                names: vec!["useState".to_string()],
                is_default: false,
            },
        ];
        let imports = convert_imports(&lang_imports);
        assert_eq!(imports[0].is_external, false);
        assert_eq!(imports[1].is_external, true);
    }
}
