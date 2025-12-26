//! Text handling utilities
//!
//! This module provides text-related utilities including:
//! - Unicode-aware text manipulation
//! - Text shaping preparation (for future rustybuzz integration)

use wasm_bindgen::prelude::*;

/// Get the character count (not byte count) of a string
#[wasm_bindgen]
pub fn char_count(text: &str) -> usize {
    text.chars().count()
}

/// Get a substring by character indices (not byte indices)
#[wasm_bindgen]
pub fn char_substring(text: &str, start: usize, end: usize) -> String {
    text.chars().skip(start).take(end - start).collect()
}

/// Find the byte index for a character index
pub fn char_to_byte_index(text: &str, char_index: usize) -> usize {
    text.char_indices()
        .nth(char_index)
        .map(|(i, _)| i)
        .unwrap_or(text.len())
}

/// Find the character index for a byte index
pub fn byte_to_char_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index.min(text.len())].chars().count()
}

/// Check if a character is a word boundary
pub fn is_word_boundary(c: char) -> bool {
    c.is_whitespace() || c.is_ascii_punctuation()
}

/// Find the next word boundary from a position
pub fn next_word_boundary(text: &str, from_char: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    if from_char >= len {
        return len;
    }

    // Skip current word
    let mut pos = from_char;
    while pos < len && !is_word_boundary(chars[pos]) {
        pos += 1;
    }

    // Skip whitespace
    while pos < len && chars[pos].is_whitespace() {
        pos += 1;
    }

    pos
}

/// Find the previous word boundary from a position
pub fn prev_word_boundary(text: &str, from_char: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();

    if from_char == 0 {
        return 0;
    }

    let mut pos = from_char - 1;

    // Skip whitespace
    while pos > 0 && chars[pos].is_whitespace() {
        pos -= 1;
    }

    // Skip word
    while pos > 0 && !is_word_boundary(chars[pos - 1]) {
        pos -= 1;
    }

    pos
}

/// Represents a text run with consistent formatting
#[derive(Debug, Clone)]
pub struct TextRun {
    pub text: String,
    pub start: usize,
    pub end: usize,
}

/// Split text into runs for shaping
/// Currently just returns the whole text as one run.
/// Future: could split by script, direction, or formatting.
pub fn split_into_runs(text: &str) -> Vec<TextRun> {
    if text.is_empty() {
        return vec![];
    }

    vec![TextRun {
        text: text.to_string(),
        start: 0,
        end: text.len(),
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_char_substring() {
        assert_eq!(char_substring("hello", 1, 4), "ell");
        assert_eq!(char_substring("héllo", 1, 4), "éll");
        assert_eq!(char_substring("你好世界", 1, 3), "好世");
    }

    #[test]
    fn test_word_boundaries() {
        let text = "hello world test";
        assert_eq!(next_word_boundary(text, 0), 6);
        assert_eq!(next_word_boundary(text, 6), 12);
        assert_eq!(prev_word_boundary(text, 11), 6);
        assert_eq!(prev_word_boundary(text, 6), 0);
    }
}
