//! Test assertion utilities and helpers
//!
//! Provides comprehensive assertion helpers specifically designed
//! for elif.rs applications, including custom assertion macros
//! and helper functions.

use serde_json::{Value as JsonValue};
use crate::{TestError, TestResult};

/// Collection of test assertions
pub struct TestAssertions;

impl TestAssertions {
    /// Assert that two JSON values are equal
    pub fn assert_json_eq(actual: &JsonValue, expected: &JsonValue) -> TestResult<()> {
        if actual != expected {
            return Err(TestError::Assertion {
                message: format!("JSON assertion failed:\nExpected: {}\nActual: {}", 
                    serde_json::to_string_pretty(expected).unwrap_or_default(),
                    serde_json::to_string_pretty(actual).unwrap_or_default()
                ),
            });
        }
        Ok(())
    }
    
    /// Assert that JSON contains expected fields/values
    pub fn assert_json_contains(actual: &JsonValue, expected: &JsonValue) -> TestResult<()> {
        if !json_contains(actual, expected) {
            return Err(TestError::Assertion {
                message: format!("JSON does not contain expected values:\nExpected to contain: {}\nActual: {}", 
                    serde_json::to_string_pretty(expected).unwrap_or_default(),
                    serde_json::to_string_pretty(actual).unwrap_or_default()
                ),
            });
        }
        Ok(())
    }
    
    /// Assert that a value is within a certain range
    pub fn assert_in_range<T>(value: T, min: T, max: T) -> TestResult<()> 
    where 
        T: PartialOrd + std::fmt::Display,
    {
        if value < min || value > max {
            return Err(TestError::Assertion {
                message: format!("Value {} is not in range [{}, {}]", value, min, max),
            });
        }
        Ok(())
    }
    
    /// Assert that a string matches a pattern
    pub fn assert_matches_pattern(text: &str, pattern: &str) -> TestResult<()> {
        use regex::Regex;
        
        let regex = Regex::new(pattern).map_err(|e| TestError::Assertion {
            message: format!("Invalid regex pattern '{}': {}", pattern, e),
        })?;
        
        if !regex.is_match(text) {
            return Err(TestError::Assertion {
                message: format!("Text '{}' does not match pattern '{}'", text, pattern),
            });
        }
        Ok(())
    }
    
    /// Assert that a collection contains an item
    pub fn assert_contains<T, I>(collection: &[T], item: &I) -> TestResult<()>
    where
        T: PartialEq<I>,
        T: std::fmt::Debug,
        I: std::fmt::Debug,
    {
        if !collection.iter().any(|x| x == item) {
            return Err(TestError::Assertion {
                message: format!("Collection {:?} does not contain item {:?}", collection, item),
            });
        }
        Ok(())
    }
    
    /// Assert that a collection has a specific length
    pub fn assert_length<T>(collection: &[T], expected_length: usize) -> TestResult<()> 
    where
        T: std::fmt::Debug,
    {
        if collection.len() != expected_length {
            return Err(TestError::Assertion {
                message: format!("Expected collection length {}, got {}: {:?}", 
                    expected_length, collection.len(), collection),
            });
        }
        Ok(())
    }
    
    /// Assert that a collection is empty
    pub fn assert_empty<T>(collection: &[T]) -> TestResult<()> 
    where
        T: std::fmt::Debug,
    {
        if !collection.is_empty() {
            return Err(TestError::Assertion {
                message: format!("Expected empty collection, got {:?}", collection),
            });
        }
        Ok(())
    }
    
    /// Assert that a collection is not empty
    pub fn assert_not_empty<T>(collection: &[T]) -> TestResult<()> 
    where
        T: std::fmt::Debug,
    {
        if collection.is_empty() {
            return Err(TestError::Assertion {
                message: "Expected non-empty collection, got empty collection".to_string(),
            });
        }
        Ok(())
    }
    
    /// Assert that all items in a collection satisfy a predicate
    pub fn assert_all<T, F>(collection: &[T], predicate: F, message: &str) -> TestResult<()>
    where
        T: std::fmt::Debug,
        F: Fn(&T) -> bool,
    {
        let failing_items: Vec<&T> = collection.iter().filter(|item| !predicate(item)).collect();
        
        if !failing_items.is_empty() {
            return Err(TestError::Assertion {
                message: format!("{}: failing items: {:?}", message, failing_items),
            });
        }
        Ok(())
    }
    
    /// Assert that any item in a collection satisfies a predicate
    pub fn assert_any<T, F>(collection: &[T], predicate: F, message: &str) -> TestResult<()>
    where
        T: std::fmt::Debug,
        F: Fn(&T) -> bool,
    {
        if !collection.iter().any(predicate) {
            return Err(TestError::Assertion {
                message: format!("{}: no items match condition in {:?}", message, collection),
            });
        }
        Ok(())
    }
    
    /// Assert that two time values are close (within tolerance)
    pub fn assert_time_close(
        actual: chrono::DateTime<chrono::Utc>, 
        expected: chrono::DateTime<chrono::Utc>,
        tolerance_seconds: i64,
    ) -> TestResult<()> {
        let diff = (actual - expected).num_seconds().abs();
        if diff > tolerance_seconds {
            return Err(TestError::Assertion {
                message: format!("Time difference too large: {} seconds (tolerance: {} seconds)", 
                    diff, tolerance_seconds),
            });
        }
        Ok(())
    }
}

/// Helper function for JSON containment checking
fn json_contains(actual: &JsonValue, expected: &JsonValue) -> bool {
    match (actual, expected) {
        (JsonValue::Object(actual_map), JsonValue::Object(expected_map)) => {
            for (key, expected_value) in expected_map {
                if let Some(actual_value) = actual_map.get(key) {
                    if !json_contains(actual_value, expected_value) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        },
        (JsonValue::Array(actual_arr), JsonValue::Array(expected_arr)) => {
            // For arrays, check if all expected items exist in actual array
            expected_arr.iter().all(|expected_item| {
                actual_arr.iter().any(|actual_item| json_contains(actual_item, expected_item))
            })
        },
        _ => actual == expected,
    }
}

/// Macros for common assertions
#[macro_export]
macro_rules! assert_json_eq {
    ($actual:expr, $expected:expr) => {
        $crate::assertions::TestAssertions::assert_json_eq($actual, $expected)?
    };
}

#[macro_export]
macro_rules! assert_json_contains {
    ($actual:expr, $expected:expr) => {
        $crate::assertions::TestAssertions::assert_json_contains($actual, $expected)?
    };
}

#[macro_export]
macro_rules! assert_in_range {
    ($value:expr, $min:expr, $max:expr) => {
        $crate::assertions::TestAssertions::assert_in_range($value, $min, $max)?
    };
}

#[macro_export]
macro_rules! assert_matches {
    ($text:expr, $pattern:expr) => {
        $crate::assertions::TestAssertions::assert_matches_pattern($text, $pattern)?
    };
}

#[macro_export]
macro_rules! assert_contains {
    ($collection:expr, $item:expr) => {
        $crate::assertions::TestAssertions::assert_contains($collection, $item)?
    };
}

#[macro_export]
macro_rules! assert_length {
    ($collection:expr, $length:expr) => {
        $crate::assertions::TestAssertions::assert_length($collection, $length)?
    };
}

#[macro_export]
macro_rules! assert_empty {
    ($collection:expr) => {
        $crate::assertions::TestAssertions::assert_empty($collection)?
    };
}

#[macro_export]
macro_rules! assert_not_empty {
    ($collection:expr) => {
        $crate::assertions::TestAssertions::assert_not_empty($collection)?
    };
}

#[macro_export]
macro_rules! assert_all {
    ($collection:expr, $predicate:expr, $message:expr) => {
        $crate::assertions::TestAssertions::assert_all($collection, $predicate, $message)?
    };
}

#[macro_export]
macro_rules! assert_any {
    ($collection:expr, $predicate:expr, $message:expr) => {
        $crate::assertions::TestAssertions::assert_any($collection, $predicate, $message)?
    };
}

#[macro_export]
macro_rules! assert_time_close {
    ($actual:expr, $expected:expr, $tolerance:expr) => {
        $crate::assertions::TestAssertions::assert_time_close($actual, $expected, $tolerance)?
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use chrono::{Utc, Duration};
    
    #[test]
    fn test_json_equality() -> TestResult<()> {
        let json1 = json!({"name": "John", "age": 30});
        let json2 = json!({"name": "John", "age": 30});
        let json3 = json!({"name": "Jane", "age": 25});
        
        TestAssertions::assert_json_eq(&json1, &json2)?;
        
        let result = TestAssertions::assert_json_eq(&json1, &json3);
        assert!(result.is_err());
        
        Ok(())
    }
    
    #[test]
    fn test_json_contains() -> TestResult<()> {
        let actual = json!({
            "user": {
                "name": "John",
                "age": 30,
                "active": true
            },
            "posts": [
                {"title": "Post 1"},
                {"title": "Post 2"}
            ]
        });
        
        let expected = json!({
            "user": {
                "name": "John"
            }
        });
        
        TestAssertions::assert_json_contains(&actual, &expected)?;
        
        let expected_fail = json!({
            "user": {
                "name": "Jane"
            }
        });
        
        let result = TestAssertions::assert_json_contains(&actual, &expected_fail);
        assert!(result.is_err());
        
        Ok(())
    }
    
    #[test]
    fn test_range_assertion() -> TestResult<()> {
        TestAssertions::assert_in_range(5, 1, 10)?;
        
        let result = TestAssertions::assert_in_range(15, 1, 10);
        assert!(result.is_err());
        
        Ok(())
    }
    
    #[test]
    fn test_pattern_matching() -> TestResult<()> {
        TestAssertions::assert_matches_pattern("test@example.com", r"^[^@]+@[^@]+\.[^@]+$")?;
        
        let result = TestAssertions::assert_matches_pattern("invalid-email", r"^[^@]+@[^@]+\.[^@]+$");
        assert!(result.is_err());
        
        Ok(())
    }
    
    #[test]
    fn test_collection_assertions() -> TestResult<()> {
        let collection = vec![1, 2, 3, 4, 5];
        
        TestAssertions::assert_contains(&collection, &3)?;
        TestAssertions::assert_length(&collection, 5)?;
        TestAssertions::assert_not_empty(&collection)?;
        
        TestAssertions::assert_all(&collection, |x| *x > 0, "All items should be positive")?;
        TestAssertions::assert_any(&collection, |x| *x > 4, "Some items should be greater than 4")?;
        
        let empty_collection: Vec<i32> = vec![];
        TestAssertions::assert_empty(&empty_collection)?;
        
        Ok(())
    }
    
    #[test]
    fn test_time_assertion() -> TestResult<()> {
        let now = Utc::now();
        let close_time = now + Duration::seconds(2);
        let far_time = now + Duration::seconds(60);
        
        TestAssertions::assert_time_close(close_time, now, 10)?;
        
        let result = TestAssertions::assert_time_close(far_time, now, 10);
        assert!(result.is_err());
        
        Ok(())
    }
    
    #[test]
    fn test_macro_usage() -> TestResult<()> {
        let json1 = json!({"test": "value"});
        let json2 = json!({"test": "value"});
        
        assert_json_eq!(&json1, &json2);
        assert_in_range!(5, 1, 10);
        
        let collection = vec![1, 2, 3];
        assert_contains!(&collection, &2);
        assert_length!(&collection, 3);
        
        Ok(())
    }
}