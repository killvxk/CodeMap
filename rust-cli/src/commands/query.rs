use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct QueryArgs {
    /// Symbol or module name to query
    pub symbol: String,
    /// Filter by type: function, class, or type
    #[arg(long)]
    pub r#type: Option<String>,
    /// Project directory
    #[arg(long, default_value = ".")]
    pub dir: String,
    /// Query a module instead of a symbol
    #[arg(long)]
    pub module: bool,
}

pub fn run(args: QueryArgs) {
    let root = PathBuf::from(&args.dir);
    let root = match root.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: cannot resolve directory '{}': {}", args.dir, e);
            std::process::exit(1);
        }
    };

    let output_dir = root.join(".codemap");
    let graph = match crate::graph::load_graph(&output_dir) {
        Ok(g) => g,
        Err(e) => {
            eprintln!(
                "Error: failed to load code graph from '{}/.codemap/': {}",
                root.display(),
                e
            );
            eprintln!("Hint: run 'codegraph scan {}' first.", root.display());
            std::process::exit(1);
        }
    };

    if args.module {
        // 模块查询模式
        match crate::query::query_module(&graph, &args.symbol) {
            Some(result) => println!("{}", crate::query::format_module_result(&result)),
            None => {
                eprintln!("Module '{}' not found.", args.symbol);
                // 列出可用模块
                let mut mods: Vec<&str> = graph.modules.keys().map(|s| s.as_str()).collect();
                mods.sort();
                if !mods.is_empty() {
                    eprintln!("Available modules: {}", mods.join(", "));
                }
                std::process::exit(1);
            }
        }
    } else {
        // 符号查询模式
        let opts = crate::query::QueryOptions {
            type_filter: args.r#type.clone(),
        };
        let results = crate::query::query_symbol(&graph, &args.symbol, &opts);
        println!("{}", crate::query::format_symbol_results(&results));
    }
}
