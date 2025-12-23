use clap::Parser;

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

    /// Second JSON file or inline JSON string
    pub file2: String,

    /// Output format (default: rfc6902)
    #[arg(short, long, default_value_t = OutputFormat::Rfc6902, hide_default_value = true)]
    pub format: OutputFormat,
}
