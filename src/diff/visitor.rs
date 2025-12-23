use serde_json::Value;

/// Visitor trait for traversing JSON values
///
/// This trait allows for flexible traversal of JSON structures.
/// Different implementations can perform different operations during traversal.
pub trait ValueVisitor {
    type Output;

    /// Visit a null value
    fn visit_null(
        &mut self,
        path: &str,
        old_value: Option<&Value>,
        new_value: Option<&Value>,
    ) -> Self::Output;

    /// Visit a boolean value
    fn visit_bool(
        &mut self,
        path: &str,
        old_value: Option<&bool>,
        new_value: Option<&bool>,
    ) -> Self::Output;

    /// Visit a number value
    fn visit_number(
        &mut self,
        path: &str,
        old_value: Option<&Value>,
        new_value: Option<&Value>,
    ) -> Self::Output;

    /// Visit a string value
    fn visit_string(
        &mut self,
        path: &str,
        old_value: Option<&String>,
        new_value: Option<&String>,
    ) -> Self::Output;

    /// Visit an array value
    fn visit_array(
        &mut self,
        path: &str,
        old_value: Option<&Vec<Value>>,
        new_value: Option<&Vec<Value>>,
    ) -> Self::Output;

    /// Visit an object value
    fn visit_object(
        &mut self,
        path: &str,
        old_value: Option<&serde_json::Map<String, Value>>,
        new_value: Option<&serde_json::Map<String, Value>>,
    ) -> Self::Output;

    /// Called when both values are the same (no change)
    fn visit_equal(&mut self, _path: &str, _value: &Value) -> Self::Output {
        // Default implementation - no output for equal values
        // Can be overridden by visitors that need to track equal values
        unimplemented!()
    }
}

/// Traverse two JSON values and call the appropriate visitor methods
pub fn traverse<V>(
    old: Option<&Value>,
    new: Option<&Value>,
    path: &str,
    visitor: &mut V,
) -> V::Output
where
    V: ValueVisitor + ValueVisitorExt,
{
    match (old, new) {
        (None, Some(new)) => {
            // Value was added
            match new {
                Value::Null => visitor.visit_null(path, None, Some(new)),
                Value::Bool(b) => visitor.visit_bool(path, None, Some(b)),
                Value::Number(_n) => visitor.visit_number(path, None, Some(new)),
                Value::String(s) => visitor.visit_string(path, None, Some(s)),
                Value::Array(a) => visitor.visit_array(path, None, Some(a)),
                Value::Object(o) => visitor.visit_object(path, None, Some(o)),
            }
        }
        (Some(old), None) => {
            // Value was removed
            match old {
                Value::Null => visitor.visit_null(path, Some(old), None),
                Value::Bool(b) => visitor.visit_bool(path, Some(b), None),
                Value::Number(_n) => visitor.visit_number(path, Some(old), None),
                Value::String(s) => visitor.visit_string(path, Some(s), None),
                Value::Array(a) => visitor.visit_array(path, Some(a), None),
                Value::Object(o) => visitor.visit_object(path, Some(o), None),
            }
        }
        (Some(old), Some(new)) => {
            if old == new {
                // Values are equal
                visitor.visit_equal(path, new)
            } else {
                // Values are different - check types
                match (old, new) {
                    (Value::Null, Value::Null) => visitor.visit_null(path, Some(old), Some(new)),
                    (Value::Bool(_), Value::Bool(_)) => {
                        visitor.visit_bool(path, old.as_bool().as_ref(), new.as_bool().as_ref())
                    }
                    (Value::Number(_), Value::Number(_)) => {
                        visitor.visit_number(path, Some(old), Some(new))
                    }
                    (Value::String(_), Value::String(_)) => visitor.visit_string(
                        path,
                        old.as_str().map(|s| s.to_string()).as_ref(),
                        new.as_str().map(|s| s.to_string()).as_ref(),
                    ),
                    (Value::Array(_), Value::Array(_)) => {
                        visitor.visit_array(path, old.as_array(), new.as_array())
                    }
                    (Value::Object(_), Value::Object(_)) => {
                        visitor.visit_object(path, old.as_object(), new.as_object())
                    }
                    (_, _) => {
                        // Type mismatch - treat as modification
                        visitor.visit_modified(path, Some(old), Some(new))
                    }
                }
            }
        }
        (None, None) => {
            // Both are None - this shouldn't happen in normal traversal
            // but we handle it gracefully
            unimplemented!()
        }
    }
}

/// Extends the ValueVisitor trait with additional methods
pub trait ValueVisitorExt: ValueVisitor {
    /// Visit a modified value (type changed or value changed)
    fn visit_modified(
        &mut self,
        path: &str,
        old_value: Option<&Value>,
        new_value: Option<&Value>,
    ) -> Self::Output
    where
        Self: Sized,
    {
        // Default implementation treats modified as a change
        match (old_value, new_value) {
            (Some(old), Some(new)) => traverse(Some(old), Some(new), path, self),
            (Some(old), None) => {
                // This shouldn't happen for modified values
                self.visit_object(path, old.as_object(), None)
            }
            (None, Some(new)) => {
                // This shouldn't happen for modified values
                self.visit_object(path, None, new.as_object())
            }
            (None, None) => unimplemented!(),
        }
    }
}
