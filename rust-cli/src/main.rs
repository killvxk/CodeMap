use clap::{Parser, Subcommand};

mod commands;
mod differ;
mod grammar_tests;
mod graph;
pub mod impact;
pub mod languages;
mod parser;
mod path_utils;
pub mod query;
mod scanner;
mod slicer;
mod traverser;

#[derive(Parser)]
#[command(name = "codegraph", about = "AST-based code graph generator", version = "0.2.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan a project directory and generate a code graph
    Scan(commands::scan::ScanArgs),
    /// Incrementally update the code graph for changed files
    Update(commands::update::UpdateArgs),
    /// Query the code graph for a symbol or module
    Query(commands::query::QueryArgs),
    /// Analyze the impact of changes to a module or file
    Impact(commands::impact::ImpactArgs),
    /// Show the status of the code graph for a project
    Status(commands::status::StatusArgs),
    /// Output module slice or overview as JSON
    Slice(commands::slice::SliceArgs),
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Scan(args) => commands::scan::run(args),
        Commands::Update(args) => commands::update::run(args),
        Commands::Query(args) => commands::query::run(args),
        Commands::Impact(args) => commands::impact::run(args),
        Commands::Status(args) => commands::status::run(args),
        Commands::Slice(args) => commands::slice::run(args),
    }
}
