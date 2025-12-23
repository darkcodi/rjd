/// Utilities for handling dot notation paths in JSON
///
/// Format: `users[0].name` for nested structures
/// - Root path: "" (empty string)
/// - Object access: `.key` or `key` (when root)
/// - Array access: `[0]` (index in brackets)

/// Build a path string from parent path and key
pub fn join_path(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_string()
    } else {
        format!("{}.{}", parent, key)
    }
}

/// Build a path string from parent path and array index
pub fn join_array_path(parent: &str, index: usize) -> String {
    if parent.is_empty() {
        format!("[{}]", index)
    } else {
        format!("{}[{}]", parent, index)
    }
}

/// Get the parent path (removes the last segment)
#[allow(dead_code)]
pub fn parent_path(path: &str) -> String {
    if path.is_empty() {
        return "".to_string();
    }

    // Handle array indices at the end
    if path.ends_with(']') {
        if let Some(pos) = path.rfind("[") {
            return path[..pos].to_string();
        }
    }

    // Handle object keys
    if let Some(pos) = path.rfind('.') {
        return path[..pos].to_string();
    }

    "".to_string()
}

/// Get the last segment of a path
#[allow(dead_code)]
pub fn last_segment(path: &str) -> &str {
    if path.is_empty() {
        return "";
    }

    // Handle array indices at the end
    if path.ends_with(']') {
        if let Some(start) = path.rfind('[') {
            return &path[start..];
        }
    }

    // Handle object keys
    if let Some(pos) = path.rfind('.') {
        return &path[pos + 1..];
    }

    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_path_root() {
        assert_eq!(join_path("", "name"), "name");
    }

    #[test]
    fn test_join_path_nested() {
        assert_eq!(join_path("user", "name"), "user.name");
        assert_eq!(join_path("user.address", "city"), "user.address.city");
    }

    #[test]
    fn test_join_array_path() {
        assert_eq!(join_array_path("", 0), "[0]");
        assert_eq!(join_array_path("users", 0), "users[0]");
        assert_eq!(
            join_array_path("users[0].friends", 1),
            "users[0].friends[1]"
        );
    }

    #[test]
    fn test_parent_path() {
        assert_eq!(parent_path("user.name"), "user");
        assert_eq!(parent_path("user.address.city"), "user.address");
        assert_eq!(parent_path("users[0]"), "users");
        assert_eq!(parent_path("users[0].name"), "users[0]");
        assert_eq!(parent_path("users[0].friends[1]"), "users[0].friends");
        assert_eq!(parent_path(""), "");
        assert_eq!(parent_path("single"), "");
    }

    #[test]
    fn test_last_segment() {
        assert_eq!(last_segment("user.name"), "name");
        assert_eq!(last_segment("user.address.city"), "city");
        assert_eq!(last_segment("users[0]"), "[0]");
        assert_eq!(last_segment("users[0].name"), "name");
        assert_eq!(last_segment("users[0].friends[1]"), "[1]");
        assert_eq!(last_segment("single"), "single");
        assert_eq!(last_segment(""), "");
    }
}
