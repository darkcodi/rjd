/// Utilities for handling dot notation paths in JSON
///
/// Format: `users[0].name` for nested structures
/// - Root path: empty JsonPath
/// - Object access: `.key` or `key` (when root)
/// - Array access: `[0]` (index in brackets)
///
/// Build a JsonPath from parent path and key
pub fn join_path(base: &crate::json_path::JsonPath, key: &str) -> crate::json_path::JsonPath {
    let mut new_path = base.clone();
    new_path.push(crate::json_path::PathSegment::Key(key.to_string()));
    new_path
}

/// Build a JsonPath from parent path and array index
pub fn join_array_path(
    base: &crate::json_path::JsonPath,
    index: usize,
) -> crate::json_path::JsonPath {
    let mut new_path = base.clone();
    new_path.push(crate::json_path::PathSegment::Index(index));
    new_path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_path::{JsonPath, PathSegment};

    #[test]
    fn test_join_path_root() {
        let result = join_path(&JsonPath::new(), "name");
        let expected = JsonPath::from_segments(vec![PathSegment::Key("name".to_string())]);
        assert_eq!(result, expected);
        assert_eq!(result.to_string(), "name");
    }

    #[test]
    fn test_join_path_nested() {
        let base = JsonPath::from_segments(vec![PathSegment::Key("user".to_string())]);
        let result = join_path(&base, "name");
        assert_eq!(result.to_string(), "user.name");

        let base2 = JsonPath::from_segments(vec![
            PathSegment::Key("user".to_string()),
            PathSegment::Key("address".to_string()),
        ]);
        let result2 = join_path(&base2, "city");
        assert_eq!(result2.to_string(), "user.address.city");
    }

    #[test]
    fn test_join_array_path() {
        let result = join_array_path(&JsonPath::new(), 0);
        assert_eq!(result.to_string(), "[0]");

        let base = JsonPath::from_segments(vec![PathSegment::Key("users".to_string())]);
        let result2 = join_array_path(&base, 0);
        assert_eq!(result2.to_string(), "users[0]");

        let base3: JsonPath = "users[0].friends".parse().unwrap();
        let result3 = join_array_path(&base3, 1);
        assert_eq!(result3.to_string(), "users[0].friends[1]");
    }

    #[test]
    fn test_join_path_returns_jsonpath() {
        let base = JsonPath::new();
        let result = join_path(&base, "test");
        // Verify the return type is JsonPath by checking it has the expected methods
        assert!(!result.is_empty());
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_join_array_path_returns_jsonpath() {
        let base = JsonPath::new();
        let result = join_array_path(&base, 0);
        // Verify the return type is JsonPath
        assert!(!result.is_empty());
        assert_eq!(result.len(), 1);
    }
}
