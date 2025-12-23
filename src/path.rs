/// Utilities for handling dot notation paths in JSON
///
/// Format: `users[0].name` for nested structures
/// - Root path: "" (empty string)
/// - Object access: `.key` or `key` (when root)
/// - Array access: `[0]` (index in brackets)
///
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
}
