use clap::{Parser, Subcommand};
use std::fs;

mod parser;
mod ast;
mod interpreter;
mod docs;

use interpreter::http::run as run_server;
use parser::parse_program;

/// Shrimpl CLI
#[derive(Parser)]
#[command(name = "shrimpl")]
#[command(about = "Shrimpl language CLI", long_about = None)]
struct Cli {
    /// Path to the main Shrimpl file (default: app.shr)
    #[arg(global = true, short, long, default_value = "app.shr")]
    file: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the Shrimpl server
    Run,
    /// Check syntax and print diagnostics
    Check,
    /// Print schema JSON
    Schema,
    /// Print diagnostics JSON
    Diagnostics,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let source = fs::read_to_string(&cli.file)
        .map_err(|e| format!("Failed to read {}: {}", &cli.file, e))?;

    let program = parse_program(&source)
        .map_err(|e| format!("Parse error in {}: {}", &cli.file, e))?;

    match cli.command {
        Commands::Run => {
            // run actix server (blocking)
            actix_web::rt::System::new().block_on(run_server(program))?;
        }
        Commands::Check => {
            // If we reach here, parse succeeded.
            println!("OK: {}", &cli.file);
        }
        Commands::Schema => {
            let schema = docs::build_schema(&program);
            println!("{}", serde_json::to_string_pretty(&schema)?);
        }
        Commands::Diagnostics => {
            let diags = docs::build_diagnostics(&program);
            println!("{}", serde_json::to_string_pretty(&diags)?);
        }
    }

    Ok(())
}
