use clap::Parser;
use std::path::PathBuf;

// Import from library crate for error type
use rjd::RjdError;

/// Output format options
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    #[value(name = "changes")]
    Changes, // Default: {added, removed, modified}

    #[value(name = "after")]
    After, // Output the "after" state with only changed properties

    #[value(name = "rfc6902")]
    Rfc6902, // RFC 6902 compliant JSON Patch format
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Changes => write!(f, "changes"),
            OutputFormat::After => write!(f, "after"),
            OutputFormat::Rfc6902 => write!(f, "rfc6902"),
        }
    }
}

/// Command-line arguments for rjd
#[derive(Parser, Debug)]
#[command(name = "rjd")]
#[command(about = "Compare two JSON files or inline JSON strings")]
pub struct Args {
    /// First JSON file or inline JSON string
    pub file1: String,

    /// Second JSON file or inline JSON string (not required when using --stdin)
    #[arg(required = false)]
    pub file2: Option<String>,

    /// Read the second JSON input from stdin
    #[arg(long)]
    pub stdin: bool,

    /// Output format (default: changes)
    #[arg(short, long, default_value_t = OutputFormat::Changes, hide_default_value = true)]
    pub format: OutputFormat,

    /// Sort keys in output
    #[arg(long)]
    pub sort: bool,

    /// JSON file containing paths to ignore (can be specified multiple times)
    #[arg(long)]
    pub ignore_json: Vec<String>,

    /// Maximum file size in bytes (default: 104857600, env: RJD_MAX_FILE_SIZE)
    #[arg(long)]
    pub max_file_size: Option<u64>,

    /// Maximum JSON nesting depth (default: 1000, env: RJD_MAX_JSON_DEPTH)
    #[arg(long)]
    pub max_depth: Option<usize>,

    /// Follow symbolic links (default: false, env: RJD_FOLLOW_SYMLINKS)
    #[arg(long)]
    pub follow_symlinks: bool,

    /// Force input to be treated as inline JSON
    #[arg(long)]
    pub inline: bool,
}

impl Args {
    /// Validate command-line arguments
    pub fn validate(&self) -> Result<(), RjdError> {
        // If not using stdin, file2 must be provided
        if !self.stdin && self.file2.is_none() {
            return Err(RjdError::MissingFile2);
        }

        // Validate ignore files exist
        for ignore_path in &self.ignore_json {
            let path = PathBuf::from(ignore_path);
            if !path.exists() {
                return Err(RjdError::FileRead {
                    path,
                    source: std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        "Ignore file not found",
                    ),
                });
            }
        }

        Ok(())
    }
}
