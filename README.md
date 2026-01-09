# RJD - Rust JSON Diff

Compare JSON files or strings and output the differences.

## Usage

```bash
rjd file1.json file2.json                   # changes format (default)
rjd file1.json file2.json --format rfc6902  # RFC 6902 JSON Patch format
rjd file1.json file2.json --format after    # show changed properties only
rjd file1.json file2.json --sort            # sort keys alphabetically
rjd file1.json --stdin                      # read second input from stdin
rjd '{"name":"John"}' '{"name":"Jane"}'     # inline JSON
```

### Options

- `--sort, -s` - Sort keys alphabetically in the output (useful for consistent diffs)
- `--stdin` - Read the second JSON input from stdin instead of a file argument
- `--ignore-json <IGNORE_JSON>` - JSON file containing paths to ignore (can be specified multiple times)
- `--inline` - Force inputs to be treated as inline JSON (disables file path detection)
- `--max-file-size <SIZE>` - Maximum file size in bytes (default: 104857600 = 100MB). Prevents loading extremely large files.
- `--max-depth <DEPTH>` - Maximum JSON nesting depth (default: 1000). Prevents stack overflow from deeply nested JSON.
- `--follow-symlinks` - Follow symbolic links instead of rejecting them (default: reject symlinks for security)

### Environment Variables

- `RJD_MAX_FILE_SIZE` - Default maximum file size in bytes (overridden by `--max-file-size`)
- `RJD_MAX_JSON_DEPTH` - Default maximum JSON nesting depth (overridden by `--max-depth`)
- `RJD_FOLLOW_SYMLINKS` - Set to `1` or `true` to follow symlinks by default (overridden by `--follow-symlinks`)

### Resource Limits

RJD includes built-in resource limits to prevent denial-of-service attacks and accidental resource exhaustion:

- **File Size Limit**: Prevents loading extremely large files that could exhaust memory (default: 100MB)
- **JSON Depth Limit**: Prevents processing deeply nested JSON structures that could cause stack overflow (default: 1000 levels)

Both limits can be customized via environment variables or CLI flags. CLI flags take precedence over environment variables.

### Security Considerations

- **Symlinks**: By default, RJD rejects symbolic links to prevent unauthorized file access. Use `--follow-symlinks` only with trusted input.
- **Resource Limits**: Adjust limits based on your use case. Production environments should use conservative limits.
- **Input Validation**: All file paths are validated before processing. Nonexistent files or invalid ignore patterns will cause immediate errors.

## Examples

### File Comparison

```bash
rjd data/file1.json data/file2.json
```

**file1.json:**
```json
{
  "name": "Alice",
  "age": 25,
  "address": {
    "city": "NYC",
    "country": "USA"
  },
  "hobbies": ["reading"]
}
```

**file2.json:**
```json
{
  "name": "Alice",
  "age": 26,
  "address": {
    "city": "LA"
  },
  "hobbies": ["reading", "painting"]
}
```

**Output (changes format):**
```json
{
  "added": [
    {
      "path": "hobbies[1]",
      "value": "painting"
    }
  ],
  "removed": [
    {
      "path": "address.country",
      "value": "USA"
    }
  ],
  "modified": [
    {
      "path": "address.city",
      "old_value": "NYC",
      "new_value": "LA"
    },
    {
      "path": "age",
      "old_value": 25,
      "new_value": 26
    }
  ]
}
```

**Output (rfc6902 format):**
```json
[
  {
    "op": "add",
    "path": "/hobbies/1",
    "value": "painting"
  },
  {
    "op": "remove",
    "path": "/address/country"
  },
  {
    "op": "replace",
    "path": "/address/city",
    "value": "LA"
  },
  {
    "op": "replace",
    "path": "/age",
    "value": 26
  }
]
```

**Output (after format):**
```json
{
  "age": 26,
  "address": {
    "city": "LA"
  },
  "hobbies": [
    "reading",
    "painting"
  ]
}
```
The `after` format preserves the key order from `file2.json`, showing only properties that were added or modified.

### Inline JSON

```bash
# Simple
rjd '{"a":1}' '{"a":2}'
# → {
#     "added": [],
#     "removed": [],
#     "modified": [
#       {
#         "path": "a",
#         "old_value": 1,
#         "new_value": 2
#       }
#     ]
#   }

# Nested
rjd '{"user":{"age":30}}' '{"user":{"age":31,"email":"x@y.com"}}'
# → {
#     "added": [
#       {
#         "path": "user.email",
#         "value": "x@y.com"
#       }
#     ],
#     "removed": [],
#     "modified": [
#       {
#         "path": "user.age",
#         "old_value": 30,
#         "new_value": 31
#       }
#     ]
#   }

# Arrays
rjd '{"items":[{"id":1}]}' '{"items":[{"id":2}]}'
# → {
#     "added": [],
#     "removed": [],
#     "modified": [
#       {
#         "path": "items[0].id",
#         "old_value": 1,
#         "new_value": 2
#       }
#     ]
#   }
```

### Stdin

```bash
# Read second input from stdin
echo '{"name": "Jane"}' | rjd '{"name": "John"}' --stdin

# Pipe from a file
cat file.json | rjd baseline.json --stdin

# Use with process substitution
rjd baseline.json --stdin < new.json
```

### Resource Limits

```bash
# Set custom file size limit (50MB)
rjd file1.json file2.json --max-file-size 52428800

# Set custom JSON depth limit
rjd file1.json file2.json --max-depth 500

# Use environment variables
export RJD_MAX_FILE_SIZE=52428800
export RJD_MAX_JSON_DEPTH=500
rjd file1.json file2.json
```

### Inline JSON Mode

```bash
# Force inputs to be treated as inline JSON
rjd --inline '{"a":1}' '{"a":2}'

# Useful when file names conflict with JSON syntax
rjd --inline 'file.json' 'file2.json'
```

### Symlink Handling

```bash
# Default behavior: reject symlinks (secure)
rjd data.json config.json
# Error: Symlink rejected: data.json

# Follow symlinks (use with trusted input)
rjd --follow-symlinks data.json config.json

# Set via environment variable
export RJD_FOLLOW_SYMLINKS=1
rjd data.json config.json
```

## Path Notation

- `user.name` → nested object property
- `items[0]` → array element
- `users[0].email` → nested array/object property

## Library Usage

RJD can be used as a library in your Rust projects. Add to your `Cargo.toml`:

```toml
[dependencies]
rjd = "1.2"
```

### Basic Diffing

```rust
use rjd::{diff, Changes};
use serde_json::json;

let old = json!({"name": "John", "age": 30});
let new = json!({"name": "Jane", "age": 30});
let changes = diff(&old, &new);

// Iterate over changes
for change in changes.iter_added() {
    println!("Added: {}", change.path());
}
```

### Error Handling

Formatters now return `Result` for safe error handling:

```rust
use rjd::create_formatter;

// Invalid format returns Result::Err
match create_formatter("invalid_format", false) {
    Ok(formatter) => {
        let output = formatter.format(&changes)?;
        println!("{}", output);
    }
    Err(e) => {
        eprintln!("Error creating formatter: {}", e);
        // Error message: "Unknown format 'invalid_format'. Valid formats are: changes, after, rfc6902"
    }
}
```

### Performance for Large Diffs

For diffs with thousands of changes, use the iterator-based filtering to avoid allocations:

```rust
use rjd::Changes;

// Old approach (clones changes)
let filtered = changes.filter_ignore_patterns(&patterns);

// New approach (zero-copy iterator)
let filtered: Vec<&Change> = changes
    .iter_filtered_changes(&patterns)
    .collect();

// Or use directly with lazy evaluation
for change in changes.iter_filtered_changes(&patterns) {
    // Process change without allocating
}
```

**Performance impact**: For diffs with 10,000 changes, the iterator approach reduces allocations by 50-90% and scales linearly with change count.

### Root Path Handling

When diffing primitive values or replacing the entire JSON value, changes will have an empty path (`""`):

```rust
let old = json!("value1");
let new = json!("value2");
let changes = diff(&old, &new);

// Root-level change has empty path
assert_eq!(changes.modified[0].path().to_string(), "");
```

## Performance Notes

- **Large Files**: RJD handles files with 10,000+ changes efficiently
- **Memory Usage**: Iterator-based filtering (see above) minimizes memory for large diffs
- **Resource Limits**: Use `--max-file-size` and `--max-depth` to prevent excessive memory usage
- **Filtering**: The `--ignore-json` flag uses optimized pattern matching with O(1) lookup
