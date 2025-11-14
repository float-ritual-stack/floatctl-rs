/// UTF-8 Safety Tests
///
/// These tests prevent regressions related to UTF-8 character boundary handling.
/// References: CLAUDE.md line 178 - "Fixed UTF-8 character boundary panic in truncation logic"
///
/// Key invariant: String slicing operations MUST use char_indices() to find byte positions
/// that align with UTF-8 character boundaries, otherwise panics occur.

use floatctl_core::conversation::Message;
use floatctl_core::stream::ConvStream;
use serde_json::json;
use std::io::Write;
use tempfile::NamedTempFile;

/// Test truncation-like operations with multi-byte UTF-8 characters
#[test]
fn test_message_parsing_with_emoji() {
    // Emoji are 4-byte UTF-8 sequences
    let json_value = json!({
        "id": "test-123",
        "role": "user",
        "text": "Hello 👋 World 🌍 Test 🚀",
        "timestamp": "2025-01-14T12:00:00Z"
    });

    let result = Message::from_export(0, json_value);
    assert!(result.is_ok(), "Should parse message with emoji without panic");

    let message = result.unwrap();
    assert_eq!(message.content, "Hello 👋 World 🌍 Test 🚀");
}

#[test]
fn test_message_parsing_with_cjk_characters() {
    // CJK characters are 3-byte UTF-8 sequences
    let json_value = json!({
        "id": "test-456",
        "role": "assistant",
        "text": "你好世界 こんにちは世界 안녕하세요",
        "timestamp": "2025-01-14T12:00:00Z"
    });

    let result = Message::from_export(0, json_value);
    assert!(result.is_ok(), "Should parse CJK text without panic");

    let message = result.unwrap();
    assert_eq!(message.content, "你好世界 こんにちは世界 안녕하세요");
}

#[test]
fn test_message_parsing_with_rtl_text() {
    // Arabic/Hebrew RTL text with combining marks
    let json_value = json!({
        "id": "test-789",
        "role": "user",
        "text": "مرحبا العالم שלום עולם",
        "timestamp": "2025-01-14T12:00:00Z"
    });

    let result = Message::from_export(0, json_value);
    assert!(result.is_ok(), "Should parse RTL text without panic");

    let message = result.unwrap();
    assert_eq!(message.content, "مرحبا العالم שלום עולם");
}

#[test]
fn test_message_parsing_with_combining_characters() {
    // Combining diacritics (é = e + combining acute)
    let text_decomposed = "e\u{0301}"; // é decomposed
    let json_value = json!({
        "id": "test-combine",
        "role": "user",
        "text": format!("Caf{} au lait", text_decomposed),
        "timestamp": "2025-01-14T12:00:00Z"
    });

    let result = Message::from_export(0, json_value);
    assert!(result.is_ok(), "Should parse combining characters without panic");
}

#[test]
fn test_message_parsing_with_zero_width_joiners() {
    // Zero-width joiners used in emoji sequences
    let text = "👨‍👩‍👧‍👦"; // Family emoji with ZWJ
    let json_value = json!({
        "id": "test-zwj",
        "role": "user",
        "text": text,
        "timestamp": "2025-01-14T12:00:00Z"
    });

    let result = Message::from_export(0, json_value);
    assert!(result.is_ok(), "Should parse ZWJ sequences without panic");

    let message = result.unwrap();
    assert_eq!(message.content, text);
}

#[test]
fn test_streaming_with_mixed_unicode() {
    let mut file = NamedTempFile::new().unwrap();

    // Mix of ASCII, emoji, CJK, RTL
    let conversations = vec![
        json!({
            "id": "conv1",
            "title": "ASCII Title",
            "created_at": "2025-01-14T12:00:00Z",
            "messages": []
        }),
        json!({
            "id": "conv2",
            "title": "Emoji 🎉 Title",
            "created_at": "2025-01-14T12:01:00Z",
            "messages": []
        }),
        json!({
            "id": "conv3",
            "title": "日本語タイトル",
            "created_at": "2025-01-14T12:02:00Z",
            "messages": []
        }),
        json!({
            "id": "conv4",
            "title": "عنوان عربي",
            "created_at": "2025-01-14T12:03:00Z",
            "messages": []
        }),
    ];

    let json_array = serde_json::Value::Array(conversations);
    writeln!(file, "{}", serde_json::to_string(&json_array).unwrap()).unwrap();
    file.flush().unwrap();

    // Parse with streaming - should not panic on any UTF-8
    let stream = ConvStream::from_path(file.path()).unwrap();
    let mut count = 0;

    for result in stream {
        assert!(result.is_ok(), "Streaming should handle mixed Unicode without panic");
        count += 1;
    }

    assert_eq!(count, 4);
}

#[test]
fn test_edge_case_4_byte_utf8_at_boundary() {
    // Regression test for slicing exactly at a 4-byte character boundary
    let emoji_text = "🚀".repeat(100); // 400 bytes (each emoji is 4 bytes)

    let json_value = json!({
        "id": "test-boundary",
        "role": "user",
        "text": emoji_text,
        "timestamp": "2025-01-14T12:00:00Z"
    });

    let result = Message::from_export(0, json_value);
    assert!(result.is_ok(), "Should handle long emoji strings without panic");

    let message = result.unwrap();
    assert_eq!(message.content.chars().count(), 100);
}

#[test]
fn test_mixed_byte_lengths_conversation() {
    let mut file = NamedTempFile::new().unwrap();

    // Conversation with messages of varying UTF-8 byte lengths
    let conversation = json!({
        "id": "mixed-utf8",
        "title": "Mixed UTF-8 Test",
        "created_at": "2025-01-14T12:00:00Z",
        "messages": [
            {
                "id": "msg1",
                "role": "user",
                "text": "A",  // 1 byte
                "timestamp": "2025-01-14T12:00:00Z"
            },
            {
                "id": "msg2",
                "role": "assistant",
                "text": "Ü",  // 2 bytes (Latin-1 Supplement)
                "timestamp": "2025-01-14T12:00:01Z"
            },
            {
                "id": "msg3",
                "role": "user",
                "text": "中",  // 3 bytes (CJK)
                "timestamp": "2025-01-14T12:00:02Z"
            },
            {
                "id": "msg4",
                "role": "assistant",
                "text": "🚀",  // 4 bytes (Emoji)
                "timestamp": "2025-01-14T12:00:03Z"
            }
        ]
    });

    writeln!(file, "[{}]", serde_json::to_string(&conversation).unwrap()).unwrap();
    file.flush().unwrap();

    let stream = ConvStream::from_path(file.path()).unwrap();
    let conversations: Vec<_> = stream.collect::<Result<Vec<_>, _>>().unwrap();

    assert_eq!(conversations.len(), 1);
    let conv = &conversations[0];
    assert_eq!(conv.messages.len(), 4);

    // Verify each message parsed correctly
    assert_eq!(conv.messages[0].content, "A");
    assert_eq!(conv.messages[1].content, "Ü");
    assert_eq!(conv.messages[2].content, "中");
    assert_eq!(conv.messages[3].content, "🚀");
}

/// Test that validates correct char_indices() usage for truncation
///
/// This is the pattern that SHOULD be used to avoid UTF-8 boundary panics:
/// ```
/// let byte_pos = text.char_indices().nth(char_count).map(|(idx, _)| idx).unwrap_or(text.len());
/// let truncated = &text[..byte_pos];
/// ```
#[test]
fn test_safe_truncation_pattern() {
    let text = "Hello 🌍 World";

    // SAFE: Use char_indices() to find byte position
    let char_limit = 8;
    let byte_pos = text
        .char_indices()
        .nth(char_limit)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len());

    let truncated = &text[..byte_pos];

    // Should not panic, and result should be valid UTF-8
    assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());
    assert_eq!(truncated.chars().count(), char_limit);
}

#[test]
#[should_panic(expected = "byte index")]
fn test_unsafe_truncation_pattern_panics() {
    let text = "Hello 🌍 World";

    // UNSAFE: Slicing by byte position without checking UTF-8 boundaries
    // This WILL panic if we slice in the middle of the emoji (4-byte sequence)
    let _bad_truncate = &text[..8]; // 8 bytes lands in middle of 🌍
}

#[test]
fn test_grapheme_cluster_handling() {
    // Some "characters" are multiple Unicode code points (grapheme clusters)
    let text = "नमस्ते"; // Devanagari - combines multiple code points

    let json_value = json!({
        "id": "test-grapheme",
        "role": "user",
        "text": text,
        "timestamp": "2025-01-14T12:00:00Z"
    });

    let result = Message::from_export(0, json_value);
    assert!(result.is_ok(), "Should parse grapheme clusters without panic");

    let message = result.unwrap();
    assert_eq!(message.content, text);
}
