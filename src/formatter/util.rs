use serde_json::Value;

/// Recursively sort a JSON value's keys alphabetically
///
/// This ensures consistent output when the `--sort` option is used,
/// sorting keys in all objects at every level of nesting.
pub fn sort_json_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut sorted_map = serde_json::Map::new();
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            for key in keys {
                sorted_map.insert(key.clone(), sort_json_value(map.get(key).unwrap()));
            }
            Value::Object(sorted_map)
        }
        Value::Array(arr) => Value::Array(arr.iter().map(sort_json_value).collect()),
        _ => value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Map;

    #[test]
    fn test_sort_simple_object() {
        let mut map = Map::new();
        map.insert("z".to_string(), Value::String("last".to_string()));
        map.insert("a".to_string(), Value::String("first".to_string()));
        map.insert("m".to_string(), Value::String("middle".to_string()));

        let value = Value::Object(map);
        let sorted = sort_json_value(&value);

        let obj = sorted.as_object().unwrap();
        let keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        assert_eq!(keys, vec!["a", "m", "z"]);
    }

    #[test]
    fn test_sort_nested_object() {
        let mut inner = Map::new();
        inner.insert("z_inner".to_string(), Value::String("z".to_string()));
        inner.insert("a_inner".to_string(), Value::String("a".to_string()));

        let mut outer = Map::new();
        outer.insert("z_outer".to_string(), Value::String("z".to_string()));
        outer.insert("inner".to_string(), Value::Object(inner));
        outer.insert("a_outer".to_string(), Value::String("a".to_string()));

        let value = Value::Object(outer);
        let sorted = sort_json_value(&value);

        let obj = sorted.as_object().unwrap();
        let keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        assert_eq!(keys, vec!["a_outer", "inner", "z_outer"]);

        // Check nested object is also sorted
        let inner_obj = obj["inner"].as_object().unwrap();
        let inner_keys: Vec<&str> = inner_obj.keys().map(|s| s.as_str()).collect();
        assert_eq!(inner_keys, vec!["a_inner", "z_inner"]);
    }

    #[test]
    fn test_sort_array_preserves_order() {
        let arr = vec![
            Value::String("z".to_string()),
            Value::String("a".to_string()),
            Value::String("m".to_string()),
        ];

        let value = Value::Array(arr);
        let sorted = sort_json_value(&value);

        // Array order should be preserved
        let sorted_arr = sorted.as_array().unwrap();
        assert_eq!(sorted_arr.len(), 3);
        assert_eq!(sorted_arr[0], "z");
        assert_eq!(sorted_arr[1], "a");
        assert_eq!(sorted_arr[2], "m");
    }

    #[test]
    fn test_sort_array_with_objects() {
        let mut obj1 = Map::new();
        obj1.insert("z".to_string(), Value::String("first".to_string()));

        let mut obj2 = Map::new();
        obj2.insert("a".to_string(), Value::String("second".to_string()));

        let arr = vec![Value::Object(obj1), Value::Object(obj2)];

        let value = Value::Array(arr);
        let sorted = sort_json_value(&value);

        // Objects inside arrays should be sorted
        let sorted_arr = sorted.as_array().unwrap();
        let keys1: Vec<&str> = sorted_arr[0]
            .as_object()
            .unwrap()
            .keys()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(keys1, vec!["z"]);

        let keys2: Vec<&str> = sorted_arr[1]
            .as_object()
            .unwrap()
            .keys()
            .map(|s| s.as_str())
            .collect();
        assert_eq!(keys2, vec!["a"]);
    }

    #[test]
    fn test_sort_primitive_returns_same() {
        assert_eq!(sort_json_value(&Value::String("test".to_string())), "test");
        assert_eq!(sort_json_value(&Value::Number(42.into())), 42);
        assert_eq!(sort_json_value(&Value::Bool(true)), true);
        assert_eq!(sort_json_value(&Value::Null), Value::Null);
    }
}
