use clap::Args;
use std::path::PathBuf;

#[derive(Args)]
pub struct SliceArgs {
    /// Module name (omit for overview)
    pub module: Option<String>,
    /// Include dependency info in module slice
    #[arg(long)]
    pub with_deps: bool,
    /// Project directory
    #[arg(long, default_value = ".")]
    pub dir: String,
}

pub fn run(args: SliceArgs) {
    let root = PathBuf::from(&args.dir);
    let root = match root.canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: cannot resolve directory '{}': {}", args.dir, e);
            std::process::exit(1);
        }
    };

    let codemap_dir = root.join(".codemap");
    let graph = match crate::graph::load_graph(&codemap_dir) {
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

    match args.module {
        None => {
            // 输出 overview
            let overview = crate::slicer::generate_overview(&graph);
            match serde_json::to_string_pretty(&overview) {
                Ok(json) => println!("{}", json),
                Err(e) => {
                    eprintln!("Serialization error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(mod_name) => {
            if args.with_deps {
                match crate::slicer::get_module_slice_with_deps(&graph, &mod_name) {
                    Ok(slice) => match serde_json::to_string_pretty(&slice) {
                        Ok(json) => println!("{}", json),
                        Err(e) => {
                            eprintln!("Serialization error: {}", e);
                            std::process::exit(1);
                        }
                    },
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                match graph.modules.get(&mod_name) {
                    Some(mod_data) => {
                        let slice =
                            crate::slicer::build_module_slice(&graph, &mod_name, mod_data);
                        match serde_json::to_string_pretty(&slice) {
                            Ok(json) => println!("{}", json),
                            Err(e) => {
                                eprintln!("Serialization error: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                    None => {
                        eprintln!("Error: module \"{}\" not found in graph.", mod_name);
                        std::process::exit(1);
                    }
                }
            }
        }
    }
}
