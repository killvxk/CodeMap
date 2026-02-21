use crate::graph::{CodeGraph, FileEntry, ModuleEntry};
use crate::path_utils::{posix_dirname, posix_normalize, strip_extension};
use std::collections::{HashMap, HashSet};

// ── 变更检测结果 ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ChangeSet {
    pub added: Vec<String>,
    pub modified: Vec<String>,
    pub removed: Vec<String>,
    #[allow(dead_code)]
    pub unchanged: Vec<String>,
}

impl ChangeSet {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.removed.is_empty()
    }
}

// ── 公共函数 ──────────────────────────────────────────────────────────────────

/// 对比旧哈希表与新哈希表，返回变更集合
///
/// - `old_hashes`: 图谱中已记录的 relPath → hash
/// - `new_hashes`: 磁盘当前扫描到的 relPath → hash
pub fn detect_changed_files(
    old_hashes: &HashMap<String, String>,
    new_hashes: &HashMap<String, String>,
) -> ChangeSet {
    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut removed = Vec::new();
    let mut unchanged = Vec::new();

    // 检查新文件集合
    for (path, new_hash) in new_hashes {
        match old_hashes.get(path) {
            None => added.push(path.clone()),
            Some(old_hash) if old_hash != new_hash => modified.push(path.clone()),
            _ => unchanged.push(path.clone()),
        }
    }

    // 找出已删除的文件
    for path in old_hashes.keys() {
        if !new_hashes.contains_key(path) {
            removed.push(path.clone());
        }
    }

    added.sort();
    modified.sort();
    removed.sort();
    unchanged.sort();

    ChangeSet { added, modified, removed, unchanged }
}

/// 将变更合并到现有图谱（原地修改）
///
/// - 删除 `removed_files` 中的文件条目及其模块引用
/// - 添加/更新 `updated_files` 中的文件条目
/// - 清理空模块
/// - 重新计算 summary 和模块依赖
pub fn merge_graph_update(
    graph: &mut CodeGraph,
    updated_files: HashMap<String, FileEntry>,
    removed_files: &[String],
) {
    // Step 1: 删除已移除的文件
    for file_path in removed_files {
        if let Some(file_data) = graph.files.remove(file_path) {
            if let Some(module) = graph.modules.get_mut(&file_data.module) {
                module.files.retain(|f| f != file_path);
            }
        }
    }

    // Step 2: 添加/更新变更文件
    for (file_path, file_data) in updated_files {
        // 若文件已存在且模块发生变化，从旧模块移除
        if let Some(existing) = graph.files.get(&file_path) {
            if existing.module != file_data.module {
                let old_mod = existing.module.clone();
                if let Some(m) = graph.modules.get_mut(&old_mod) {
                    m.files.retain(|f| f != &file_path);
                }
            }
        }

        // 确保目标模块存在
        graph.modules.entry(file_data.module.clone()).or_insert_with(|| ModuleEntry {
            files: vec![],
            depends_on: vec![],
            depended_by: vec![],
        });

        // 将文件加入模块（避免重复）
        let mod_name = file_data.module.clone();
        let module = graph.modules.get_mut(&mod_name).unwrap();
        if !module.files.contains(&file_path) {
            module.files.push(file_path.clone());
        }

        graph.files.insert(file_path, file_data);
    }

    // Step 3: 清理空模块
    graph.modules.retain(|_, m| !m.files.is_empty());

    // Step 4: 重新计算 summary 和依赖
    recalculate_summary(graph);
    rebuild_dependencies(graph);
}

// ── 内部函数 ──────────────────────────────────────────────────────────────────

/// 从当前文件数据重新计算 summary
fn recalculate_summary(graph: &mut CodeGraph) {
    let mut total_files = 0u32;
    let mut total_functions = 0u32;
    let mut total_classes = 0u32;
    let mut languages: HashMap<String, u32> = HashMap::new();

    for file_data in graph.files.values() {
        total_files += 1;
        total_functions += file_data.functions.len() as u32;
        total_classes += file_data.classes.len() as u32;
        *languages.entry(file_data.language.clone()).or_insert(0) += 1;
    }

    graph.summary.total_files = total_files;
    graph.summary.total_functions = total_functions;
    graph.summary.total_classes = total_classes;
    graph.summary.languages = languages.clone();

    let mut mod_list: Vec<String> = graph.modules.keys().cloned().collect();
    mod_list.sort();
    graph.summary.modules = mod_list;

    let mut entry_points: Vec<String> = graph
        .files
        .iter()
        .filter(|(_, f)| f.is_entry_point)
        .map(|(p, _)| p.clone())
        .collect();
    entry_points.sort();
    graph.summary.entry_points = entry_points;

    let mut lang_list: Vec<String> = languages.into_keys().collect();
    lang_list.sort();
    graph.config.languages = lang_list;
}

/// 从文件级 import 数据重建模块级 dependsOn / dependedBy
///
/// 注意：当前仅解析以 `.` 开头的相对路径导入（JS/TS），
/// 非 JS/TS 语言的 import 被标记为 external 而跳过。
fn rebuild_dependencies(graph: &mut CodeGraph) {
    // 构建 relPath → moduleName 查找表
    let mut path_lookup: HashMap<String, String> = HashMap::new();
    for (rel_path, file_data) in &graph.files {
        let norm = rel_path.replace('\\', "/");
        path_lookup.insert(norm.clone(), file_data.module.clone());
        // 无扩展名版本
        let without_ext = strip_extension(&norm);
        path_lookup.entry(without_ext).or_insert_with(|| file_data.module.clone());
    }

    // 用 Set 收集依赖关系
    let mut depends_on: HashMap<String, HashSet<String>> = HashMap::new();
    let mut depended_by: HashMap<String, HashSet<String>> = HashMap::new();
    for mod_name in graph.modules.keys() {
        depends_on.insert(mod_name.clone(), HashSet::new());
        depended_by.insert(mod_name.clone(), HashSet::new());
    }

    for (rel_path, file_data) in &graph.files {
        let module_name = &file_data.module;
        let norm_path = rel_path.replace('\\', "/");

        for imp in &file_data.imports {
            if imp.is_external || !imp.source.starts_with('.') {
                continue;
            }

            // 解析相对 import 路径（posix 风格）
            let importer_dir = posix_dirname(&norm_path);
            let resolved = posix_normalize(&format!("{}/{}", importer_dir, imp.source));

            let target = path_lookup
                .get(&resolved)
                .or_else(|| path_lookup.get(&format!("{}/index", resolved)))
                .cloned();

            if let Some(target_mod) = target {
                if &target_mod != module_name {
                    if let Some(set) = depends_on.get_mut(module_name) {
                        set.insert(target_mod.clone());
                    }
                    if let Some(set) = depended_by.get_mut(&target_mod) {
                        set.insert(module_name.clone());
                    }
                }
            }
        }
    }

    // 写回图谱
    for (mod_name, module) in &mut graph.modules {
        let mut dep_on: Vec<String> =
            depends_on.get(mod_name).map(|s| s.iter().cloned().collect()).unwrap_or_default();
        dep_on.sort();
        module.depends_on = dep_on;

        let mut dep_by: Vec<String> =
            depended_by.get(mod_name).map(|s| s.iter().cloned().collect()).unwrap_or_default();
        dep_by.sort();
        module.depended_by = dep_by;
    }
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{create_empty_graph, FileEntry, ModuleEntry};

    fn make_file_entry(module: &str) -> FileEntry {
        FileEntry {
            language: "typescript".to_string(),
            module: module.to_string(),
            hash: "sha256:aabbccdd11223344".to_string(),
            lines: 10,
            functions: vec![],
            classes: vec![],
            types: vec![],
            imports: vec![],
            exports: vec![],
            is_entry_point: false,
        }
    }

    // ── detect_changed_files ──────────────────────────────────────────────────

    #[test]
    fn test_detect_no_changes() {
        let hashes: HashMap<String, String> = [
            ("a.ts".to_string(), "hash1".to_string()),
            ("b.ts".to_string(), "hash2".to_string()),
        ]
        .into();
        let cs = detect_changed_files(&hashes, &hashes);
        assert!(cs.added.is_empty());
        assert!(cs.modified.is_empty());
        assert!(cs.removed.is_empty());
        assert_eq!(cs.unchanged.len(), 2);
        assert!(cs.is_empty());
    }

    #[test]
    fn test_detect_added() {
        let old: HashMap<String, String> = [("a.ts".to_string(), "h1".to_string())].into();
        let new: HashMap<String, String> = [
            ("a.ts".to_string(), "h1".to_string()),
            ("b.ts".to_string(), "h2".to_string()),
        ]
        .into();
        let cs = detect_changed_files(&old, &new);
        assert_eq!(cs.added, vec!["b.ts"]);
        assert!(cs.modified.is_empty());
        assert!(cs.removed.is_empty());
    }

    #[test]
    fn test_detect_modified() {
        let old: HashMap<String, String> = [("a.ts".to_string(), "h1".to_string())].into();
        let new: HashMap<String, String> = [("a.ts".to_string(), "h2".to_string())].into();
        let cs = detect_changed_files(&old, &new);
        assert!(cs.added.is_empty());
        assert_eq!(cs.modified, vec!["a.ts"]);
        assert!(cs.removed.is_empty());
    }

    #[test]
    fn test_detect_removed() {
        let old: HashMap<String, String> = [
            ("a.ts".to_string(), "h1".to_string()),
            ("b.ts".to_string(), "h2".to_string()),
        ]
        .into();
        let new: HashMap<String, String> = [("a.ts".to_string(), "h1".to_string())].into();
        let cs = detect_changed_files(&old, &new);
        assert!(cs.added.is_empty());
        assert!(cs.modified.is_empty());
        assert_eq!(cs.removed, vec!["b.ts"]);
    }

    #[test]
    fn test_detect_sorted_output() {
        let old: HashMap<String, String> = HashMap::new();
        let new: HashMap<String, String> = [
            ("z.ts".to_string(), "h1".to_string()),
            ("a.ts".to_string(), "h2".to_string()),
            ("m.ts".to_string(), "h3".to_string()),
        ]
        .into();
        let cs = detect_changed_files(&old, &new);
        assert_eq!(cs.added, vec!["a.ts", "m.ts", "z.ts"]);
    }

    // ── merge_graph_update ────────────────────────────────────────────────────

    #[test]
    fn test_merge_remove_file() {
        let mut graph = create_empty_graph("test", "/tmp/test");
        graph.files.insert("src/a.ts".to_string(), make_file_entry("auth"));
        graph.modules.insert(
            "auth".to_string(),
            ModuleEntry {
                files: vec!["src/a.ts".to_string()],
                depends_on: vec![],
                depended_by: vec![],
            },
        );
        graph.summary.total_files = 1;

        merge_graph_update(&mut graph, HashMap::new(), &["src/a.ts".to_string()]);

        assert!(!graph.files.contains_key("src/a.ts"));
        // 空模块应被清理
        assert!(!graph.modules.contains_key("auth"));
        assert_eq!(graph.summary.total_files, 0);
    }

    #[test]
    fn test_merge_add_file() {
        let mut graph = create_empty_graph("test", "/tmp/test");

        let mut updated = HashMap::new();
        updated.insert("src/b.ts".to_string(), make_file_entry("utils"));

        merge_graph_update(&mut graph, updated, &[]);

        assert!(graph.files.contains_key("src/b.ts"));
        assert!(graph.modules.contains_key("utils"));
        assert_eq!(graph.modules["utils"].files, vec!["src/b.ts"]);
        assert_eq!(graph.summary.total_files, 1);
    }

    #[test]
    fn test_merge_module_change() {
        let mut graph = create_empty_graph("test", "/tmp/test");
        graph.files.insert("src/a.ts".to_string(), make_file_entry("old_mod"));
        graph.modules.insert(
            "old_mod".to_string(),
            ModuleEntry {
                files: vec!["src/a.ts".to_string()],
                depends_on: vec![],
                depended_by: vec![],
            },
        );

        let mut updated = HashMap::new();
        updated.insert("src/a.ts".to_string(), make_file_entry("new_mod"));
        merge_graph_update(&mut graph, updated, &[]);

        // 旧模块应被清理（空了）
        assert!(!graph.modules.contains_key("old_mod"));
        // 新模块应存在
        assert!(graph.modules.contains_key("new_mod"));
        assert_eq!(graph.files["src/a.ts"].module, "new_mod");
    }

    #[test]
    fn test_rebuild_dependencies() {
        use crate::graph::ImportInfo;

        let mut graph = create_empty_graph("test", "/tmp/test");

        let mut auth_file = make_file_entry("auth");
        auth_file.imports = vec![ImportInfo {
            source: "../utils/helper".to_string(),
            symbols: vec![],
            is_external: false,
        }];
        graph.files.insert("src/auth/login.ts".to_string(), auth_file);

        let utils_file = make_file_entry("utils");
        graph.files.insert("src/utils/helper.ts".to_string(), utils_file);

        graph.modules.insert(
            "auth".to_string(),
            ModuleEntry {
                files: vec!["src/auth/login.ts".to_string()],
                depends_on: vec![],
                depended_by: vec![],
            },
        );
        graph.modules.insert(
            "utils".to_string(),
            ModuleEntry {
                files: vec!["src/utils/helper.ts".to_string()],
                depends_on: vec![],
                depended_by: vec![],
            },
        );

        rebuild_dependencies(&mut graph);

        assert_eq!(graph.modules["auth"].depends_on, vec!["utils"]);
        assert_eq!(graph.modules["utils"].depended_by, vec!["auth"]);
    }
}
