use floatctl_core::stream::{ConvStream, RawValueStream};
use proptest::prelude::*;
use serde_json::json;
use std::io::Write;
use tempfile::NamedTempFile;

// Strategy to generate arbitrary JSON values
fn arb_json_value() -> impl Strategy<Value = serde_json::Value> {
    prop::collection::vec(
        prop_oneof![
            Just(json!(null)),
            any::<bool>().prop_map(|b| json!(b)),
            any::<i64>().prop_map(|n| json!(n)),
            any::<f64>()
                .prop_filter("finite floats only", |f| f.is_finite())
                .prop_map(|f| json!(f)),
            ".*".prop_map(|s| json!(s)),
        ],
        0..10,
    )
    .prop_map(|arr| json!(arr))
}

proptest! {
    /// Property: Streaming parser never panics on arbitrary JSON arrays
    #[test]
    fn prop_raw_value_stream_never_panics(values in prop::collection::vec(arb_json_value(), 0..100)) {
        let mut file = NamedTempFile::new().unwrap();

        // Write JSON array to temp file
        let json_array = serde_json::Value::Array(values.clone());
        writeln!(file, "{}", serde_json::to_string(&json_array).unwrap()).unwrap();
        file.flush().unwrap();

        // Parse with streaming parser
        let stream = RawValueStream::from_path(file.path()).unwrap();
        let mut count = 0;

        for result in stream {
            // Should not panic - may error on malformed JSON but we control input
            prop_assert!(result.is_ok());
            count += 1;
        }

        // Invariant: parsed count equals input count
        prop_assert_eq!(count, values.len());
    }

    /// Property: Streaming parser preserves order
    #[test]
    fn prop_stream_preserves_order(values in prop::collection::vec(any::<i64>(), 1..50)) {
        // Skip empty arrays - tested separately
        prop_assume!(!values.is_empty());

        let mut file = NamedTempFile::new().unwrap();

        // Create JSON array with numeric values
        let json_values: Vec<_> = values.iter().map(|&n| json!(n)).collect();
        let json_array = serde_json::Value::Array(json_values);
        writeln!(file, "{}", serde_json::to_string(&json_array).unwrap()).unwrap();
        file.flush().unwrap();

        // Get path before file gets moved
        let path = file.path().to_path_buf();

        // Parse and collect
        let stream = RawValueStream::from_path(&path).unwrap();
        let parsed: Vec<i64> = stream
            .map(|r| r.unwrap().as_i64().unwrap())
            .collect();

        // Invariant: order preserved
        prop_assert_eq!(parsed, values);
    }

    /// Property: NDJSON format handles empty lines gracefully
    #[test]
    fn prop_ndjson_handles_empty_lines(values in prop::collection::vec(any::<String>(), 1..50)) {
        // Skip empty arrays - empty files fail format detection (tested separately)
        prop_assume!(!values.is_empty());

        let mut file = NamedTempFile::new().unwrap();

        // Write NDJSON with random empty lines
        for (i, s) in values.iter().enumerate() {
            writeln!(file, "{}", json!({"text": s, "index": i})).unwrap();
            // Insert random empty lines
            if i % 3 == 0 {
                writeln!(file).unwrap();
            }
        }
        file.flush().unwrap();

        // Parse NDJSON
        let stream = RawValueStream::from_path(file.path()).unwrap();
        let mut count = 0;

        for result in stream {
            prop_assert!(result.is_ok());
            count += 1;
        }

        // Should skip empty lines
        prop_assert_eq!(count, values.len());
    }

    /// Property: Mixed whitespace doesn't affect parsing
    #[test]
    fn prop_whitespace_tolerance(values in prop::collection::vec(any::<i32>(), 1..20)) {
        let mut file = NamedTempFile::new().unwrap();

        // Write JSON array with excessive whitespace
        write!(file, "  \n\t  [  \n").unwrap();
        for (i, v) in values.iter().enumerate() {
            if i > 0 {
                write!(file, "  ,  \n  ").unwrap();
            }
            write!(file, "{}  ", v).unwrap();
        }
        writeln!(file, "\n  ]  ").unwrap();
        file.flush().unwrap();

        // Parse
        let stream = RawValueStream::from_path(file.path()).unwrap();
        let parsed: Vec<i32> = stream
            .map(|r| r.unwrap().as_i64().unwrap() as i32)
            .collect();

        prop_assert_eq!(parsed, values);
    }
}

#[test]
fn test_empty_array_handling() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "[]").unwrap();
    file.flush().unwrap();

    let stream = RawValueStream::from_path(file.path()).unwrap();
    let count = stream.count();

    assert_eq!(count, 0, "Empty array should yield zero elements");
}

#[test]
fn test_single_element_array() {
    let mut file = NamedTempFile::new().unwrap();
    let array = json!([{"test": 123}]);
    writeln!(file, "{}", serde_json::to_string(&array).unwrap()).unwrap();
    file.flush().unwrap();

    let stream = RawValueStream::from_path(file.path()).unwrap();
    let values: Vec<_> = stream.collect::<Result<Vec<_>, _>>().unwrap();

    assert_eq!(values.len(), 1);
    assert_eq!(values[0]["test"], 123);
}

#[test]
fn test_nested_arrays_preserved() {
    let mut file = NamedTempFile::new().unwrap();
    let array = json!([{"nested": [1, 2, 3]}, {"nested": [4, 5, 6]}]);
    writeln!(file, "{}", serde_json::to_string(&array).unwrap()).unwrap();
    file.flush().unwrap();

    let stream = RawValueStream::from_path(file.path()).unwrap();
    let values: Vec<_> = stream.collect::<Result<Vec<_>, _>>().unwrap();

    assert_eq!(values.len(), 2);
    assert_eq!(values[0]["nested"], json!([1, 2, 3]));
    assert_eq!(values[1]["nested"], json!([4, 5, 6]));
}

#[test]
fn test_conv_stream_auto_detect_json_array() {
    let mut file = NamedTempFile::new().unwrap();
    let array = json!([{"id": "conv1"}, {"id": "conv2"}]);
    write!(file, "  ").unwrap(); // Leading whitespace
    writeln!(file, "{}", serde_json::to_string(&array).unwrap()).unwrap();
    file.flush().unwrap();

    let stream = ConvStream::from_path(file.path()).unwrap();

    // Should detect as JSON array format
    match stream {
        ConvStream::Array(_) => {}, // Expected
        ConvStream::Ndjson(_) => panic!("Should detect JSON array format"),
    }
}

#[test]
fn test_conv_stream_auto_detect_ndjson() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(file, "{}", serde_json::to_string(&json!({"id": "conv1"})).unwrap()).unwrap();
    writeln!(file, "{}", serde_json::to_string(&json!({"id": "conv2"})).unwrap()).unwrap();
    file.flush().unwrap();

    let stream = ConvStream::from_path(file.path()).unwrap();

    // Should detect as NDJSON format
    match stream {
        ConvStream::Ndjson(_) => {}, // Expected
        ConvStream::Array(_) => panic!("Should detect NDJSON format"),
    }
}
