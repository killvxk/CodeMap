use std::collections::{HashMap, HashSet, VecDeque};

use crate::graph::{CodeGraph, ModuleEntry};

/// 影响分析结果
#[derive(Debug)]
pub struct ImpactResult {
    pub target_type: TargetType,
    pub target_module: String,
    pub direct_dependants: Vec<String>,
    pub transitive_dependants: Vec<String>,
    pub impacted_modules: Vec<String>,
    pub impacted_files: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TargetType {
    Module,
    File,
}

impl TargetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetType::Module => "module",
            TargetType::File => "file",
        }
    }
}

/// 分析修改某个模块或文件的影响范围。
///
/// target 可以是模块名或文件路径（支持部分匹配）。
/// max_depth 控制 BFS 最大深度（默认 3）。
pub fn analyze_impact(graph: &CodeGraph, target: &str, max_depth: u32) -> ImpactResult {
    // 1. 确定目标类型和所属模块
    let (target_type, target_module) = resolve_target(graph, target);

    // 2. 直接依赖方
    let direct_dependants = match graph.modules.get(&target_module) {
        Some(m) => m.depended_by.clone(),
        None => vec![],
    };

    // 3. BFS 传递依赖方
    let transitive_dependants = bfs_dependants(&graph.modules, &target_module, max_depth);

    // 4. 受影响模块 = 目标 + 所有传递依赖方
    let mut impacted_modules = vec![target_module.clone()];
    impacted_modules.extend(transitive_dependants.iter().cloned());

    // 5. 受影响文件 = 受影响模块的所有文件
    let mut impacted_files: Vec<String> = impacted_modules
        .iter()
        .filter_map(|m| graph.modules.get(m))
        .flat_map(|m| m.files.iter().cloned())
        .collect();
    impacted_files.sort();

    ImpactResult {
        target_type,
        target_module,
        direct_dependants,
        transitive_dependants,
        impacted_modules,
        impacted_files,
    }
}

fn resolve_target(graph: &CodeGraph, target: &str) -> (TargetType, String) {
    // 优先匹配模块名
    if graph.modules.contains_key(target) {
        return (TargetType::Module, target.to_string());
    }

    // 精确文件路径匹配
    if let Some(file) = graph.files.get(target) {
        return (TargetType::File, file.module.clone());
    }

    // 部分文件路径匹配
    if let Some(matched) = graph.files.keys().find(|f| f.contains(target)) {
        let module = graph.files[matched].module.clone();
        return (TargetType::File, module);
    }

    // 未找到 — 返回空结果
    (TargetType::Module, target.to_string())
}

/// BFS 遍历 dependedBy 边，返回所有传递依赖方（不含起始模块），按名称排序。
fn bfs_dependants(
    modules: &HashMap<String, ModuleEntry>,
    start: &str,
    max_depth: u32,
) -> Vec<String> {
    let mut visited: HashSet<String> = HashSet::new();
    visited.insert(start.to_string());

    let mut result: Vec<String> = Vec::new();
    // (module_name, current_depth)
    let mut queue: VecDeque<(String, u32)> = VecDeque::new();
    queue.push_back((start.to_string(), 0));

    while let Some((current, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }
        let Some(mod_entry) = modules.get(&current) else {
            continue;
        };
        for dep in &mod_entry.depended_by {
            if visited.insert(dep.clone()) {
                result.push(dep.clone());
                queue.push_back((dep.clone(), depth + 1));
            }
        }
    }

    result.sort();
    result
}

// ── 测试 ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{
        CodeGraph, FileEntry, GraphConfig, GraphSummary, ModuleEntry, ProjectInfo,
    };
    use std::collections::HashMap;

    fn make_graph() -> CodeGraph {
        // 模块依赖关系：
        //   core  ← utils ← app
        //   core  ← app
        // 即 core.dependedBy = [utils, app], utils.dependedBy = [app]
        let mut modules = HashMap::new();
        modules.insert(
            "core".to_string(),
            ModuleEntry {
                files: vec!["src/core/mod.rs".to_string()],
                depends_on: vec![],
                depended_by: vec!["utils".to_string(), "app".to_string()],
            },
        );
        modules.insert(
            "utils".to_string(),
            ModuleEntry {
                files: vec!["src/utils/mod.rs".to_string()],
                depends_on: vec!["core".to_string()],
                depended_by: vec!["app".to_string()],
            },
        );
        modules.insert(
            "app".to_string(),
            ModuleEntry {
                files: vec!["src/main.rs".to_string()],
                depends_on: vec!["core".to_string(), "utils".to_string()],
                depended_by: vec![],
            },
        );

        let mut files = HashMap::new();
        files.insert(
            "src/core/mod.rs".to_string(),
            FileEntry {
                language: "rust".to_string(),
                module: "core".to_string(),
                hash: "sha256:abc".to_string(),
                lines: 10,
                functions: vec![],
                classes: vec![],
                types: vec![],
                imports: vec![],
                exports: vec![],
                is_entry_point: false,
            },
        );

        CodeGraph {
            version: "1.0".to_string(),
            project: ProjectInfo {
                name: "test".to_string(),
                root: "/tmp/test".to_string(),
            },
            scanned_at: "2026-01-01T00:00:00.000Z".to_string(),
            commit_hash: None,
            config: GraphConfig {
                languages: vec![],
                exclude_patterns: vec![],
            },
            summary: GraphSummary {
                total_files: 3,
                total_functions: 0,
                total_classes: 0,
                languages: HashMap::new(),
                modules: vec!["core".to_string(), "utils".to_string(), "app".to_string()],
                entry_points: vec![],
            },
            modules,
            files,
        }
    }

    #[test]
    fn test_impact_module_core() {
        let graph = make_graph();
        let result = analyze_impact(&graph, "core", 3);
        assert_eq!(result.target_type, TargetType::Module);
        assert_eq!(result.target_module, "core");
        // 直接依赖方
        let mut direct = result.direct_dependants.clone();
        direct.sort();
        assert_eq!(direct, vec!["app", "utils"]);
        // 传递依赖方（BFS depth=3）
        assert!(result.transitive_dependants.contains(&"app".to_string()));
        assert!(result.transitive_dependants.contains(&"utils".to_string()));
        // 受影响模块包含 core 自身
        assert!(result.impacted_modules.contains(&"core".to_string()));
    }

    #[test]
    fn test_impact_module_utils() {
        let graph = make_graph();
        let result = analyze_impact(&graph, "utils", 3);
        assert_eq!(result.direct_dependants, vec!["app"]);
        assert_eq!(result.transitive_dependants, vec!["app"]);
    }

    #[test]
    fn test_impact_module_app_no_dependants() {
        let graph = make_graph();
        let result = analyze_impact(&graph, "app", 3);
        assert!(result.direct_dependants.is_empty());
        assert!(result.transitive_dependants.is_empty());
        assert_eq!(result.impacted_modules, vec!["app"]);
    }

    #[test]
    fn test_impact_file_path() {
        let graph = make_graph();
        let result = analyze_impact(&graph, "src/core/mod.rs", 3);
        assert_eq!(result.target_type, TargetType::File);
        assert_eq!(result.target_module, "core");
    }

    #[test]
    fn test_impact_partial_file_path() {
        let graph = make_graph();
        let result = analyze_impact(&graph, "core/mod", 3);
        assert_eq!(result.target_type, TargetType::File);
        assert_eq!(result.target_module, "core");
    }

    #[test]
    fn test_impact_not_found() {
        let graph = make_graph();
        let result = analyze_impact(&graph, "nonexistent", 3);
        assert_eq!(result.target_module, "nonexistent");
        assert!(result.direct_dependants.is_empty());
        assert!(result.impacted_files.is_empty());
    }

    #[test]
    fn test_bfs_depth_limit() {
        let graph = make_graph();
        // depth=0 应该不追踪任何传递依赖
        let result = analyze_impact(&graph, "core", 0);
        assert!(result.transitive_dependants.is_empty());
    }

    #[test]
    fn test_impacted_files_sorted() {
        let graph = make_graph();
        let result = analyze_impact(&graph, "core", 3);
        let sorted = {
            let mut v = result.impacted_files.clone();
            v.sort();
            v
        };
        assert_eq!(result.impacted_files, sorted);
    }
}
