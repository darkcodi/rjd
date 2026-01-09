# RJD - Rust JSON Diff

[![crates.io](https://img.shields.io/crates/v/rjd)](https://crates.io/crates/rjd)
[![docs.rs](https://img.shields.io/docsrs/rjd)](https://docs.rs/rjd)
[![license](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)

Compare JSON files or strings and output the differences. Works as a CLI tool or Rust library.

## Installation

**CLI:**
```bash
cargo install rjd
```

**Library:**
```toml
[dependencies]
rjd = "1.2"
```

## CLI Usage

```bash
rjd file1.json file2.json                   # changes format (default)
rjd file1.json file2.json --format rfc6902  # RFC 6902 JSON Patch format
rjd file1.json file2.json --format after    # show changed properties only
rjd file1.json file2.json --sort            # sort keys alphabetically
rjd file1.json --stdin                      # read second input from stdin
rjd '{"a":1}' '{"a":2}'                     # inline JSON
```

### Options

- `--format <FORMAT>` - Output format: `changes` (default), `rfc6902`, `after`
- `--sort, -s` - Sort keys alphabetically
- `--stdin` - Read second input from stdin
- `--ignore-json <FILE>` - JSON file with paths to ignore (can be used multiple times)
- `--max-file-size <SIZE>` - Max file size in bytes (default: 100MB)
- `--max-depth <DEPTH>` - Max JSON nesting depth (default: 1000)
- `--follow-symlinks` - Follow symbolic links (default: reject for security)

### Environment Variables

- `RJD_MAX_FILE_SIZE` - Default max file size
- `RJD_MAX_JSON_DEPTH` - Default max depth
- `RJD_FOLLOW_SYMLINKS` - Set to `1` to follow symlinks by default

## Library Usage

```rust
use rjd::{diff, Changes};
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let old = json!({"name": "John", "age": 30});
    let new = json!({"name": "Jane", "age": 31});
    let changes = diff(&old, &new);

    // Iterate over changes
    for change in changes.modified.iter() {
        if let rjd::Change::Modified { path, old_value, new_value } = change {
            println!("{} changed from {} to {}", path, old_value, new_value);
        }
    }

    Ok(())
}
```

### Loading Files

```rust
use rjd::{load_json_file, diff};

let old = load_json_file("old.json")?;
let new = load_json_file("new.json")?;
let changes = diff(&old, &new);
```

### Output Formats

```rust
use rjd::create_formatter;

let formatter = create_formatter("rfc6902", false)?;
let output = formatter.format(&changes)?;
```

### Filtering Changes

```rust
// Zero-copy filtering for large diffs
let patterns = vec!["timestamp".to_string()];
let filtered: Vec<&rjd::Change> = changes
    .iter_filtered_changes(&patterns)
    .collect();
```

### Custom Config

```rust
use rjd::{LoadConfig, SymlinkPolicy};

let config = LoadConfig {
    max_file_size: 10_000_000,  // 10MB
    max_depth: 100,
    symlink_policy: SymlinkPolicy::Reject,
};

let json = load_json_file_with_config("data.json", &config)?;
```

## Output Formats

**Changes format** (default):
```json
{
  "added": [{"path": "email", "value": "x@y.com"}],
  "removed": [],
  "modified": [{"path": "age", "oldValue": 30, "newValue": 31}]
}
```

**RFC 6902 format**:
```json
[
  {"op": "add", "path": "/email", "value": "x@y.com"},
  {"op": "replace", "path": "/age", "value": 31}
]
```

**After format** (final state):
```json
{
  "name": "Jane",
  "age": 31,
  "email": "x@y.com"
}
```

## API

**Types:** `Change`, `Changes`, `JsonPath`, `RjdError`, `LoadConfig`, `SymlinkPolicy`

**Functions:** `diff()`, `load_json_file()`, `load_json_input()`, `create_formatter()`, `load_ignore_patterns()`

All functions return `Result<T, RjdError>`.

Full docs: [docs.rs/rjd](https://docs.rs/rjd)

## License

MIT OR Apache-2.0
