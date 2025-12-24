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

## Path Notation

- `user.name` → nested object property
- `items[0]` → array element
- `users[0].email` → nested array/object property
