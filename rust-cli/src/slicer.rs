use crate::graph::{CodeGraph, ModuleEntry};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ── 输出数据结构 ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleStats {
    #[serde(rename = "totalFiles")]
    pub total_files: u32,
    #[serde(rename = "totalFunctions")]
    pub total_functions: u32,
    #[serde(rename = "totalClasses")]
    pub total_classes: u32,
    #[serde(rename = "totalLines")]
    pub total_lines: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewModule {
    pub name: String,
    pub path: String,
    #[serde(rename = "fileCount")]
    pub file_count: u32,
    pub exports: Vec<String>,
    #[serde(rename = "dependsOn")]
    pub depends_on: Vec<String>,
    #[serde(rename = "dependedBy")]
    pub depended_by: Vec<String>,
    pub stats: ModuleStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Overview {
    pub project: crate::graph::ProjectInfo,
    #[serde(rename = "scannedAt")]
    pub scanned_at: String,
    #[serde(rename = "commitHash")]
    pub commit_hash: Option<String>,
    pub summary: crate::graph::GraphSummary,
    pub modules: Vec<OverviewModule>,
    #[serde(rename = "entryPoints")]
    pub entry_points: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SliceFile {
    pub path: String,
    pub language: String,
    pub lines: u32,
    pub functions: Vec<crate::graph::FunctionInfo>,
    pub classes: Vec<crate::graph::ClassInfo>,
    pub types: Vec<crate::graph::TypeInfo>,
    pub imports: Vec<crate::graph::ImportInfo>,
    pub exports: Vec<String>,
    #[serde(rename = "isEntryPoint")]
    pub is_entry_point: bool,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSlice {
    pub module: String,
    pub path: String,
    pub files: Vec<SliceFile>,
    pub exports: Vec<String>,
    #[serde(rename = "dependsOn")]
    pub depends_on: Vec<String>,
    #[serde(rename = "dependedBy")]
    pub depended_by: Vec<String>,
    pub stats: ModuleStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepInfo {
    pub name: String,
    pub exports: Vec<String>,
    #[serde(rename = "fileCount")]
    pub file_count: u32,
    pub stats: ModuleStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSliceWithDeps {
    #[serde(flatten)]
    pub slice: ModuleSlice,
    pub dependencies: Vec<DepInfo>,
}

// ── 公共函数 ──────────────────────────────────────────────────────────────────

/// 生成项目概览（_overview.json）
pub fn generate_overview(graph: &CodeGraph) -> Overview {
    let modules: Vec<OverviewModule> = graph
        .modules
        .iter()
        .map(|(mod_name, mod_data)| {
            let (all_exports, stats) = collect_module_stats(graph, mod_data);
            let path = module_path(mod_data, mod_name);
            OverviewModule {
                name: mod_name.clone(),
                path,
                file_count: mod_data.files.len() as u32,
                exports: dedup_sorted(all_exports),
                depends_on: mod_data.depends_on.clone(),
                depended_by: mod_data.depended_by.clone(),
                stats,
            }
        })
        .collect();

    Overview {
        project: graph.project.clone(),
        scanned_at: graph.scanned_at.clone(),
        commit_hash: graph.commit_hash.clone(),
        summary: graph.summary.clone(),
        modules,
        entry_points: graph.summary.entry_points.clone(),
    }
}

/// 生成所有模块的切片
#[allow(dead_code)]
pub fn generate_slices(graph: &CodeGraph) -> std::collections::HashMap<String, ModuleSlice> {
    graph
        .modules
        .iter()
        .map(|(mod_name, mod_data)| {
            let slice = build_module_slice(graph, mod_name, mod_data);
            (mod_name.clone(), slice)
        })
        .collect()
}

/// 构建单个模块的完整切片
pub fn build_module_slice(graph: &CodeGraph, mod_name: &str, mod_data: &ModuleEntry) -> ModuleSlice {
    let mut files: Vec<SliceFile> = Vec::new();
    let mut all_exports: Vec<String> = Vec::new();
    let mut total_functions = 0u32;
    let mut total_classes = 0u32;
    let mut total_lines = 0u32;

    for file_path in &mod_data.files {
        if let Some(file_data) = graph.files.get(file_path) {
            all_exports.extend(file_data.exports.clone());
            total_functions += file_data.functions.len() as u32;
            total_classes += file_data.classes.len() as u32;
            total_lines += file_data.lines;

            files.push(SliceFile {
                path: file_path.clone(),
                language: file_data.language.clone(),
                lines: file_data.lines,
                functions: file_data.functions.clone(),
                classes: file_data.classes.clone(),
                types: file_data.types.clone(),
                imports: file_data.imports.clone(),
                exports: file_data.exports.clone(),
                is_entry_point: file_data.is_entry_point,
                hash: file_data.hash.clone(),
            });
        }
    }

    ModuleSlice {
        module: mod_name.to_string(),
        path: module_path(mod_data, mod_name),
        files,
        exports: dedup_sorted(all_exports),
        depends_on: mod_data.depends_on.clone(),
        depended_by: mod_data.depended_by.clone(),
        stats: ModuleStats {
            total_files: mod_data.files.len() as u32,
            total_functions,
            total_classes,
            total_lines,
        },
    }
}

/// 获取模块切片并附带依赖信息（--with-deps）
pub fn get_module_slice_with_deps(
    graph: &CodeGraph,
    module_name: &str,
) -> anyhow::Result<ModuleSliceWithDeps> {
    let mod_data = graph
        .modules
        .get(module_name)
        .ok_or_else(|| anyhow::anyhow!("Module \"{}\" not found in graph", module_name))?;

    let slice = build_module_slice(graph, module_name, mod_data);

    let dependencies: Vec<DepInfo> = mod_data
        .depends_on
        .iter()
        .map(|dep_name| {
            if let Some(dep_data) = graph.modules.get(dep_name) {
                let (dep_exports, dep_stats) = collect_module_stats(graph, dep_data);
                DepInfo {
                    name: dep_name.clone(),
                    exports: dedup_sorted(dep_exports),
                    file_count: dep_data.files.len() as u32,
                    stats: dep_stats,
                }
            } else {
                DepInfo {
                    name: dep_name.clone(),
                    exports: vec![],
                    file_count: 0,
                    stats: ModuleStats {
                        total_files: 0,
                        total_functions: 0,
                        total_classes: 0,
                        total_lines: 0,
                    },
                }
            }
        })
        .collect();

    Ok(ModuleSliceWithDeps { slice, dependencies })
}

/// 保存 overview 和各模块切片到 {output_dir}/slices/
#[allow(dead_code)]
pub fn save_slices(output_dir: &Path, graph: &CodeGraph) -> anyhow::Result<()> {
    let slices_dir = output_dir.join("slices");
    std::fs::create_dir_all(&slices_dir)?;

    // 保存 _overview.json
    let overview = generate_overview(graph);
    let overview_json = serde_json::to_string_pretty(&overview)?;
    std::fs::write(slices_dir.join("_overview.json"), overview_json)?;

    // 保存各模块切片
    let slices = generate_slices(graph);
    for (mod_name, slice) in &slices {
        let slice_json = serde_json::to_string_pretty(slice)?;
        std::fs::write(slices_dir.join(format!("{}.json", mod_name)), slice_json)?;
    }

    Ok(())
}

// ── 内部工具函数 ──────────────────────────────────────────────────────────────

fn collect_module_stats(
    graph: &CodeGraph,
    mod_data: &ModuleEntry,
) -> (Vec<String>, ModuleStats) {
    let mut all_exports: Vec<String> = Vec::new();
    let mut total_functions = 0u32;
    let mut total_classes = 0u32;
    let mut total_lines = 0u32;

    for file_path in &mod_data.files {
        if let Some(file_data) = graph.files.get(file_path) {
            all_exports.extend(file_data.exports.clone());
            total_functions += file_data.functions.len() as u32;
            total_classes += file_data.classes.len() as u32;
            total_lines += file_data.lines;
        }
    }

    let stats = ModuleStats {
        total_files: mod_data.files.len() as u32,
        total_functions,
        total_classes,
        total_lines,
    };
    (all_exports, stats)
}

fn module_path(mod_data: &ModuleEntry, mod_name: &str) -> String {
    if let Some(first_file) = mod_data.files.first() {
        // 取第一个文件的目录
        let p = Path::new(first_file);
        if let Some(parent) = p.parent() {
            let s = parent.to_string_lossy().replace('\\', "/");
            if !s.is_empty() {
                return s;
            }
        }
    }
    mod_name.to_string()
}

fn dedup_sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v.dedup();
    v
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{create_empty_graph, FileEntry, ModuleEntry};

    fn make_test_graph() -> CodeGraph {
        let mut graph = create_empty_graph("testproject", "/tmp/testproject");

        graph.files.insert(
            "src/main.rs".to_string(),
            FileEntry {
                language: "rust".to_string(),
                module: "_root".to_string(),
                hash: "sha256:abcdef123456".to_string(),
                lines: 10,
                functions: vec![],
                classes: vec![],
                types: vec![],
                imports: vec![],
                exports: vec!["main".to_string()],
                is_entry_point: true,
            },
        );

        graph.modules.insert(
            "_root".to_string(),
            ModuleEntry {
                files: vec!["src/main.rs".to_string()],
                depends_on: vec![],
                depended_by: vec![],
            },
        );

        graph.summary.total_files = 1;
        graph.summary.modules = vec!["_root".to_string()];
        graph.summary.entry_points = vec!["src/main.rs".to_string()];
        graph
    }

    #[test]
    fn test_generate_overview() {
        let graph = make_test_graph();
        let overview = generate_overview(&graph);
        assert_eq!(overview.project.name, "testproject");
        assert_eq!(overview.modules.len(), 1);
        assert_eq!(overview.modules[0].name, "_root");
        assert_eq!(overview.modules[0].file_count, 1);
    }

    #[test]
    fn test_build_module_slice() {
        let graph = make_test_graph();
        let mod_data = graph.modules.get("_root").unwrap();
        let slice = build_module_slice(&graph, "_root", mod_data);
        assert_eq!(slice.module, "_root");
        assert_eq!(slice.files.len(), 1);
        assert_eq!(slice.exports, vec!["main"]);
        assert_eq!(slice.stats.total_files, 1);
        assert_eq!(slice.stats.total_lines, 10);
    }

    #[test]
    fn test_get_module_slice_with_deps_not_found() {
        let graph = make_test_graph();
        let result = get_module_slice_with_deps(&graph, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_dedup_sorted() {
        let v = vec!["b".to_string(), "a".to_string(), "b".to_string()];
        let result = dedup_sorted(v);
        assert_eq!(result, vec!["a", "b"]);
    }
}
