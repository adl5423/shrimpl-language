use clap::{Parser, Subcommand};
use std::error::Error;
use std::fs;
use std::process::{Command, Stdio};

mod ast;
mod docs;
mod interpreter;
mod parser;

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
    command: Option<Commands>,
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

    /// Start the Shrimpl language server (LSP) binary
    ///
    /// By default this runs the `shrimpl-lsp` executable found in PATH.
    /// Use `--exe` to point at a specific path, for example:
    /// `shrimpl lsp --exe target/debug/shrimpl-lsp`
    Lsp {
        /// Path to the shrimpl-lsp executable (or name in PATH)
        #[arg(short, long, default_value = "shrimpl-lsp")]
        exe: String,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    // Default behavior with no subcommand is the same as `run`.
    let command = cli.command.unwrap_or(Commands::Run);

    match command {
        Commands::Lsp { exe } => {
            // LSP mode: spawn the shrimpl-lsp binary and wire stdin/stdout/stderr through.
            start_lsp_subprocess(&exe)?;
        }

        Commands::Run => {
            let (source, program) = load_and_parse(&cli.file)?;
            // Avoid unused variable warning.
            let _ = source;

            // Read the configured port from the Shrimpl program so we can tell
            // the user exactly where the server will be listening.
            let port = program.server.port;

            // Friendly startup banner for `shrimpl --file app.shr run` (or just `shrimpl`).
            println!();
            println!("shr run");
            println!("----------------------------------------");
            println!("Shrimpl server is starting on http://localhost:{port}");
            println!("Open that URL in your browser (for example:");
            println!("  http://localhost:{port}/__shrimpl/ui");
            println!("to explore and test your Shrimpl API).");
            println!();
            println!("Press Ctrl+C to shut down the server.");
            println!("----------------------------------------");
            println!();

            // Run the HTTP server using actix.
            actix_web::rt::System::new().block_on(run_server(program))?;
        }

        Commands::Check => {
            let (_source, _program) = load_and_parse(&cli.file)?;
            // If parse succeeded, we are OK.
            println!("OK: {}", &cli.file);
        }

        Commands::Schema => {
            let (_source, program) = load_and_parse(&cli.file)?;
            let schema = docs::build_schema(&program);
            println!("{}", serde_json::to_string_pretty(&schema)?);
        }

        Commands::Diagnostics => {
            let (_source, program) = load_and_parse(&cli.file)?;
            let diags = docs::build_diagnostics(&program);
            println!("{}", serde_json::to_string_pretty(&diags)?);
        }
    }

    Ok(())
}

/// Read the Shrimpl source file and parse it into a Program.
fn load_and_parse(path: &str) -> Result<(String, ast::Program), Box<dyn Error>> {
    let source = fs::read_to_string(path).map_err(|e| format!("Failed to read {}: {}", path, e))?;

    let program = parse_program(&source).map_err(|e| format!("Parse error in {}: {}", path, e))?;

    Ok((source, program))
}

/// Start the external shrimpl-lsp process and wait for it to exit.
fn start_lsp_subprocess(exe: &str) -> Result<(), Box<dyn Error>> {
    let mut child = Command::new(exe)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|e| format!("Failed to start LSP server '{}': {}", exe, e))?;

    let status = child.wait()?;
    if !status.success() {
        return Err(format!("LSP server '{}' exited with status {}", exe, status).into());
    }

    Ok(())
}
