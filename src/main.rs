use clap::Parser;
use std::process;

mod cli;

// Import from library crate
use rjd::create_formatter;
use rjd::diff;
use rjd::load_all_ignore_patterns;
use rjd::RjdError;
use rjd::{
    load_json_input_with_config_policy_and_inline, load_json_stdin_with_config, LoadConfig,
    SymlinkPolicy,
};

fn main() {
    if let Err(err) = run() {
        eprintln!("Error: {}", err);
        process::exit(1);
    }
}

fn run() -> Result<(), RjdError> {
    // Parse command-line arguments
    let args = cli::Args::parse();

    // Validate arguments
    args.validate()?;

    // Create LoadConfig from environment variables and merge with CLI flags
    let config = LoadConfig::from_env().merge_with_cli(args.max_file_size, args.max_depth);

    // Determine symlink policy from CLI flag or environment variable
    let follow_symlinks_env = std::env::var("RJD_FOLLOW_SYMLINKS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(false);
    let symlink_policy = if args.follow_symlinks || follow_symlinks_env {
        SymlinkPolicy::Follow
    } else {
        SymlinkPolicy::Reject
    };

    // Load and parse JSON from either files or inline strings
    let old_json = load_json_input_with_config_policy_and_inline(
        &args.file1,
        &config,
        symlink_policy,
        args.inline,
    )
    .map_err(|e| RjdError::Internal {
        message: format!("Failed to load '{}': {}", args.file1, e),
    })?;

    let new_json = if args.stdin {
        load_json_stdin_with_config(&config).map_err(|e| RjdError::Internal {
            message: format!("Failed to load from stdin: {}", e),
        })?
    } else {
        let file2 = args
            .file2
            .expect("file2 is required when --stdin is not used");
        load_json_input_with_config_policy_and_inline(&file2, &config, symlink_policy, args.inline)
            .map_err(|e| RjdError::Internal {
                message: format!("Failed to load '{}': {}", file2, e),
            })?
    };

    // Compute diff
    let mut changes = diff(&old_json, &new_json);

    // Load and apply ignore patterns if specified
    if !args.ignore_json.is_empty() {
        let patterns =
            load_all_ignore_patterns(&args.ignore_json).map_err(|e| RjdError::Internal {
                message: e.to_string(),
            })?;
        changes = changes.filter_ignore_patterns(&patterns);
    }

    // Format and output results
    let format_str = args.format.to_string();
    let formatter = create_formatter(&format_str, args.sort).map_err(|e| RjdError::Formatter {
        message: e.to_string(),
    })?;
    let output = formatter
        .format(&changes)
        .map_err(|e| RjdError::Formatter {
            message: e.to_string(),
        })?;

    println!("{}", output);

    Ok(())
}
