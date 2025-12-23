use clap::Parser;
use std::process;

mod cli;
mod diff;
mod error;
mod formatter;
mod loader;
mod path;
mod types;

// Re-export types for easier importing
pub use cli::Args;
pub use diff::diff;
pub use error::RjdError;
pub use formatter::create_formatter;
pub use loader::{load_json_file, load_json_input};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), RjdError> {
    // Parse command-line arguments
    let args = cli::Args::parse();

    // Load and parse JSON from either files or inline strings
    let old_json = load_json_input(&args.file1).map_err(|e| RjdError::Internal {
        message: format!("Failed to load '{}': {}", args.file1, e),
    })?;
    let new_json = load_json_input(&args.file2).map_err(|e| RjdError::Internal {
        message: format!("Failed to load '{}': {}", args.file2, e),
    })?;

    // Compute diff
    let changes = diff(&old_json, &new_json);

    // Format and output results
    let formatter = create_formatter(args.format, args.sort);
    let output = formatter
        .format(&changes)
        .map_err(|e| RjdError::Formatter {
            message: e.to_string(),
        })?;

    println!("{}", output);

    Ok(())
}
