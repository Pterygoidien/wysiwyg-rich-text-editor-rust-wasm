//! Text Handling Utilities
//!
//! This module provides Unicode-aware text manipulation utilities essential for
//! correct handling of international text in the editor.
//!
//! # Why This Module Exists
//!
//! JavaScript strings and Rust strings handle Unicode differently:
//! - JavaScript uses UTF-16 code units
//! - Rust uses UTF-8 bytes
//! - Neither directly gives you "character count" for user-facing operations
//!
//! This module provides operations that work on Unicode scalar values (Rust's `char`),
//! which more closely matches user expectations for cursor movement and selection.
//!
//! # Key Functions
//!
//! - `char_count()`: Get the number of characters (not bytes or code units)
//! - `char_substring()`: Extract a substring by character indices
//! - `char_to_byte_index()` / `byte_to_char_index()`: Index conversion
//! - Word boundary detection for Ctrl+Arrow navigation
//!
//! # Text Shaping (Future)
//!
//! The `split_into_runs()` function is a placeholder for future integration with
//! rustybuzz for proper text shaping. Currently returns the entire text as a single
//! run, but could be extended to:
//! - Split by script (Latin, Arabic, CJK, etc.)
//! - Handle bidirectional text
//! - Apply font fallback
//!
//! # Limitations
//!
//! - Does not handle grapheme clusters (e.g., emoji with modifiers)
//! - Does not handle combining characters correctly for all cases
//! - For full Unicode correctness, consider using the `unicode-segmentation` crate

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

