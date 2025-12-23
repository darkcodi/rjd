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
pub use loader::load_json_file;

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args = cli::Args::parse();

    // Validate that files exist
    if !args.file1.exists() {
        return Err(Box::new(RjdError::FileRead {
            path: args.file1.clone(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", args.file1.display()),
            ),
        }));
    }

    if !args.file2.exists() {
        return Err(Box::new(RjdError::FileRead {
            path: args.file2.clone(),
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", args.file2.display()),
            ),
        }));
    }

    // Load and parse JSON files
    println!("Loading files...");
    let old_json = load_json_file(&args.file1)
        .map_err(|e| format!("Failed to load {}: {}", args.file1.display(), e))?;
    let new_json = load_json_file(&args.file2)
        .map_err(|e| format!("Failed to load {}: {}", args.file2.display(), e))?;

    // Compute diff
    println!("Computing diff...");
    let changes = diff(&old_json, &new_json);

    // Format and output results
    let formatter = create_formatter(args.format);
    let output = formatter.format(&changes)?;

    println!("{}", output);

    Ok(())
}
