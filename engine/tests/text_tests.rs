//! Tests for the text module

use editor_engine::*;

#[test]
fn test_char_count_ascii() {
    assert_eq!(char_count("hello"), 5);
}

#[test]
fn test_char_count_unicode() {
    assert_eq!(char_count("héllo"), 5);
    assert_eq!(char_count("你好"), 2);
    assert_eq!(char_count("👋🌍"), 2);
}

#[test]
fn test_char_count_empty() {
    assert_eq!(char_count(""), 0);
}

#[test]
fn test_char_count_mixed() {
    // Mix of ASCII, accented, CJK, and emoji
    assert_eq!(char_count("Hello 你好 👋"), 10);
}

#[test]
fn test_char_substring() {
    assert_eq!(char_substring("hello", 1, 4), "ell");
    assert_eq!(char_substring("héllo", 1, 4), "éll");
    assert_eq!(char_substring("你好世界", 1, 3), "好世");
}

#[test]
fn test_char_substring_full() {
    assert_eq!(char_substring("hello", 0, 5), "hello");
}

#[test]
fn test_char_substring_empty() {
    assert_eq!(char_substring("hello", 2, 2), "");
}

#[test]
fn test_char_substring_emoji() {
    assert_eq!(char_substring("👋🌍🎉", 1, 2), "🌍");
}

#[test]
fn test_word_boundaries() {
    let text = "hello world test";
    assert_eq!(next_word_boundary(text, 0), 6);
    assert_eq!(next_word_boundary(text, 6), 12);
    assert_eq!(prev_word_boundary(text, 11), 6);
    assert_eq!(prev_word_boundary(text, 6), 0);
}

#[test]
fn test_next_word_boundary_end() {
    let text = "hello";
    assert_eq!(next_word_boundary(text, 5), 5);
    assert_eq!(next_word_boundary(text, 10), 5);
}

#[test]
fn test_prev_word_boundary_start() {
    let text = "hello";
    assert_eq!(prev_word_boundary(text, 0), 0);
}

#[test]
fn test_word_boundary_multiple_spaces() {
    let text = "hello   world";
    assert_eq!(next_word_boundary(text, 0), 8);
}

#[test]
fn test_is_word_boundary() {
    assert!(is_word_boundary(' '));
    assert!(is_word_boundary('.'));
    assert!(is_word_boundary(','));
    assert!(is_word_boundary('\n'));
    assert!(is_word_boundary('\t'));

    assert!(!is_word_boundary('a'));
    assert!(!is_word_boundary('Z'));
    assert!(!is_word_boundary('5'));
}

#[test]
fn test_char_to_byte_index() {
    let text = "héllo";
    // 'h' is 1 byte, 'é' is 2 bytes
    assert_eq!(char_to_byte_index(text, 0), 0);
    assert_eq!(char_to_byte_index(text, 1), 1); // After 'h'
    assert_eq!(char_to_byte_index(text, 2), 3); // After 'é' (2 bytes)
}

#[test]
fn test_byte_to_char_index() {
    let text = "héllo";
    assert_eq!(byte_to_char_index(text, 0), 0);
    assert_eq!(byte_to_char_index(text, 1), 1); // 'h'
    assert_eq!(byte_to_char_index(text, 3), 2); // After 'é'
}

#[test]
fn test_split_into_runs_empty() {
    let runs = split_into_runs("");
    assert!(runs.is_empty());
}

#[test]
fn test_split_into_runs_simple() {
    let runs = split_into_runs("hello");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].text, "hello");
    assert_eq!(runs[0].start, 0);
    assert_eq!(runs[0].end, 5);
}
