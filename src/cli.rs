use clap::Parser;
use std::path::PathBuf;

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Changes, // Default: {added, removed, modified}
}

impl std::str::FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "changes" => Ok(OutputFormat::Changes),
            _ => Err(format!("Unknown output format: {}", s)),
        }
    }
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Changes => write!(f, "changes"),
        }
    }
}

/// Command-line arguments for rjd
#[derive(Parser, Debug)]
#[command(name = "rjd")]
#[command(about = "Compare two JSON files and show differences")]
pub struct Args {
    /// First JSON file to compare
    pub file1: PathBuf,

    /// Second JSON file to compare
    pub file2: PathBuf,

    /// Output format (default: changes)
    #[arg(short, long, default_value = "changes")]
    pub format: OutputFormat,
}
