use clap::Args;
use std::path::PathBuf;

use crate::graph::load_graph;
use crate::impact::analyze_impact;

#[derive(Args)]
pub struct ImpactArgs {
    /// Module or file to analyze
    pub target: String,
    /// Maximum BFS depth for transitive dependants
    #[arg(long, default_value = "3")]
    pub depth: u32,
    /// Project directory
    #[arg(long, default_value = ".")]
    pub dir: String,
}

pub fn run(args: ImpactArgs) {
    let root_dir = match PathBuf::from(&args.dir).canonicalize() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: cannot resolve directory '{}': {}", args.dir, e);
            std::process::exit(1);
        }
    };
    let output_dir = root_dir.join(".codemap");

    let graph = match load_graph(&output_dir) {
        Ok(g) => g,
        Err(_) => {
            eprintln!("No code graph found. Run \"codegraph scan\" first.");
            std::process::exit(1);
        }
    };

    let result = analyze_impact(&graph, &args.target, args.depth);

    println!("Impact analysis for: {}", args.target);
    println!("  Target type: {}", result.target_type.as_str());
    println!("  Target module: {}", result.target_module);

    let direct_str = if result.direct_dependants.is_empty() {
        "(none)".to_string()
    } else {
        result.direct_dependants.join(", ")
    };
    println!("  Direct dependants: {direct_str}");

    let transitive_str = if result.transitive_dependants.is_empty() {
        "(none)".to_string()
    } else {
        result.transitive_dependants.join(", ")
    };
    println!("  Transitive dependants: {transitive_str}");

    println!(
        "  Impacted modules ({}): {}",
        result.impacted_modules.len(),
        result.impacted_modules.join(", ")
    );
    println!("  Impacted files ({}):", result.impacted_files.len());
    for file in &result.impacted_files {
        println!("    - {file}");
    }
}
