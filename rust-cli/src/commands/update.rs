use clap::Args;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Args)]
pub struct UpdateArgs {
    /// Project directory
    pub dir: Option<String>,
    /// Additional glob patterns to exclude
    #[arg(long, num_args = 1..)]
    pub exclude: Vec<String>,
}

pub fn run(args: UpdateArgs) {
    let dir = args.dir.unwrap_or_else(|| ".".to_string());
    let root = PathBuf::from(&dir);
    let root = match root.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: cannot resolve directory '{}': {}", dir, e);
            std::process::exit(1);
        }
    };

    let codemap_dir = root.join(".codemap");

    // 加载现有图谱
    let mut graph = match crate::graph::load_graph(&codemap_dir) {
        Ok(g) => g,
        Err(e) => {
            eprintln!(
                "Error: could not load graph from {}/.codemap/: {}",
                root.display(),
                e
            );
            eprintln!("Run 'codegraph scan {}' first.", root.display());
            std::process::exit(1);
        }
    };

    // 遍历磁盘当前文件，计算哈希
    let files = crate::traverser::traverse_files(&root, &args.exclude);
    let has_cpp = crate::traverser::has_cpp_source_files(&files);

    let mut new_hashes: HashMap<String, String> = HashMap::new();
    let mut file_contents: HashMap<String, Vec<u8>> = HashMap::new();

    for abs_path in &files {
        if crate::traverser::detect_language(abs_path).is_none() {
            continue;
        }
        let content = match std::fs::read(abs_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let rel_path = abs_path
            .strip_prefix(&root)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .replace('\\', "/");

        let hash = crate::graph::compute_file_hash(&content);
        new_hashes.insert(rel_path.clone(), hash);
        file_contents.insert(rel_path, content);
    }

    // 从 meta.fileHashes 读取旧哈希（与 Node.js update 逻辑一致）
    // 若 meta.json 不存在或无 fileHashes，回退到从 graph.files 提取
    let old_hashes: HashMap<String, String> =
        match crate::graph::load_meta(&codemap_dir) {
            Ok(meta) if !meta.file_hashes.is_empty() => meta.file_hashes.into_iter().collect(),
            _ => graph
                .files
                .iter()
                .map(|(p, f)| (p.clone(), f.hash.clone()))
                .collect(),
        };

    // 检测变更
    let changes = crate::differ::detect_changed_files(&old_hashes, &new_hashes);

    if changes.is_empty() {
        println!("No changes detected.");
        return;
    }

    println!(
        "Changes: +{} added, ~{} modified, -{} removed",
        changes.added.len(),
        changes.modified.len(),
        changes.removed.len()
    );

    // 解析变更文件（新增 + 修改）
    let mut updated_files: HashMap<String, crate::graph::FileEntry> = HashMap::new();

    for rel_path in changes.added.iter().chain(changes.modified.iter()) {
        let content = match file_contents.get(rel_path) {
            Some(c) => c,
            None => continue,
        };

        // 重建绝对路径以检测语言
        let abs_path = root.join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
        let base_lang = match crate::traverser::detect_language(&abs_path) {
            Some(l) => l,
            None => continue,
        };
        let lang = crate::traverser::effective_language(&abs_path, base_lang, has_cpp);

        let adapter = crate::languages::get_adapter(lang);

        let mut ts_parser = tree_sitter::Parser::new();
        ts_parser.set_language(&adapter.language()).ok();
        let tree = match ts_parser.parse(content, None) {
            Some(t) => t,
            None => continue,
        };

        let lang_functions = adapter.extract_functions(&tree, content);
        let lang_imports = adapter.extract_imports(&tree, content);
        let lang_exports = adapter.extract_exports(&tree, content);
        let lang_classes = adapter.extract_classes(&tree, content);
        let lines = content.iter().filter(|&&b| b == b'\n').count() as u32 + 1;

        let functions = crate::scanner::convert_functions(&lang_functions);
        let classes = crate::scanner::convert_classes(&lang_classes);
        let types = crate::scanner::convert_types(&lang_classes, lang);
        let imports = crate::scanner::convert_imports(&lang_imports);
        let exports = crate::scanner::convert_exports(&lang_exports);

        let module_name = crate::scanner::detect_module_name(&abs_path, &root);
        let hash = new_hashes[rel_path].clone();

        updated_files.insert(
            rel_path.clone(),
            crate::graph::FileEntry {
                language: lang.as_str().to_string(),
                module: module_name,
                hash,
                lines,
                functions,
                classes,
                types,
                imports,
                exports,
                is_entry_point: crate::graph::is_entry_point(&abs_path),
            },
        );
    }

    // 合并变更到图谱
    crate::differ::merge_graph_update(&mut graph, updated_files, &changes.removed);

    // 更新扫描时间
    graph.scanned_at = crate::graph::chrono_now();

    // 保存更新后的图谱
    if let Err(e) = crate::graph::save_graph(&codemap_dir, &graph) {
        eprintln!("Error saving graph: {}", e);
        std::process::exit(1);
    }

    // 重新生成 slices（与 Node.js update 行为一致）
    if let Err(e) = crate::slicer::save_slices(&codemap_dir, &graph) {
        eprintln!("Warning: failed to save slices: {}", e);
    }

    println!("Update complete.");
    println!(
        "  +{} ~{} -{}",
        changes.added.len(),
        changes.modified.len(),
        changes.removed.len()
    );
    if !changes.added.is_empty() {
        println!("  Added: {}", changes.added.join(", "));
    }
    if !changes.modified.is_empty() {
        println!("  Modified: {}", changes.modified.join(", "));
    }
    if !changes.removed.is_empty() {
        println!("  Removed: {}", changes.removed.join(", "));
    }
}
