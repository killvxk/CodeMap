use clap::Args;
use std::path::PathBuf;

use crate::graph::{load_graph, load_meta};

#[derive(Args)]
pub struct StatusArgs {
    /// Project directory
    pub dir: Option<String>,
}

pub fn run(args: StatusArgs) {
    let dir = args.dir.unwrap_or_else(|| ".".to_string());
    let root_dir = match PathBuf::from(&dir).canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: cannot resolve directory '{}': {}", dir, e);
            std::process::exit(1);
        }
    };
    let output_dir = root_dir.join(".codemap");

    if !output_dir.exists() {
        eprintln!("No code graph found. Run \"codegraph scan\" first.");
        std::process::exit(1);
    }

    let graph = match load_graph(&output_dir) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error loading code graph: {e}");
            std::process::exit(1);
        }
    };

    let meta = load_meta(&output_dir).ok();

    println!("Project: {}", graph.project.name);
    println!("Scanned at: {}", graph.scanned_at);
    println!("Commit: {}", graph.commit_hash.as_deref().unwrap_or("(none)"));
    println!("Files: {}", graph.summary.total_files);
    println!("Functions: {}", graph.summary.total_functions);
    println!("Classes: {}", graph.summary.total_classes);
    println!("Modules: {}", graph.summary.modules.join(", "));

    // 语言分布
    if !graph.summary.languages.is_empty() {
        let mut lang_entries: Vec<_> = graph.summary.languages.iter().collect();
        lang_entries.sort_by_key(|(k, _)| k.as_str());
        let lang_str: Vec<String> = lang_entries
            .iter()
            .map(|(lang, count)| format!("{lang}({count})"))
            .collect();
        println!("Languages: {}", lang_str.join(", "));
    }

    // 上次更新时间（来自 meta）
    if let Some(ref m) = meta {
        println!("Last update: {}", m.last_scan_at);
    }

    // 已追踪文件数
    let tracked = meta
        .as_ref()
        .map(|m| m.file_hashes.len())
        .unwrap_or(0);
    println!("Tracked files: {tracked}");
}
