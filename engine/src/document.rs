//! Document Model
//!
//! This module defines the core data structures that represent a document's content
//! and formatting. It provides a structured, serializable representation of rich text
//! documents with support for:
//!
//! - **Paragraphs**: Text content with inline formatting (bold, italic, colors, etc.)
//! - **Block Types**: Headings (H1-H4), blockquotes, and regular paragraphs
//! - **Lists**: Bulleted and numbered lists with proper counter management
//! - **Images**: Embedded images with positioning, sizing, and text wrapping options
//! - **Page Breaks**: Explicit page break markers for document pagination
//!
//! # Architecture
//!
//! The document model follows a hierarchical structure:
//!
//! ```text
//! Document
//! ├── paragraphs: Vec<Paragraph>
//! │   ├── text: String
//! │   ├── meta: ParagraphMeta (alignment, block type, list type)
//! │   └── styles: Vec<TextStyle> (inline formatting ranges)
//! └── images: Vec<DocumentImage>
//!     └── (id, src, dimensions, wrapping options)
//! ```
//!
//! # Special Markers
//!
//! The document uses Unicode characters as markers for special content:
//! - `U+FFFD` (Replacement Character): Page break marker
//! - `U+FFFC` (Object Replacement Character): Image placeholder, followed by image ID
//!
//! # Serialization
//!
//! All types implement `Serialize` and `Deserialize` for JSON persistence,
//! enabling document save/load functionality.

use serde::{Deserialize, Serialize};

/// The root document structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Document version for compatibility
    pub version: u32,
    /// All paragraphs in the document
    pub paragraphs: Vec<Paragraph>,
    /// All images in the document
    pub images: Vec<DocumentImage>,
}

impl Document {
    pub fn new() -> Self {
        Document {
            version: 1,
            paragraphs: vec![Paragraph::new(String::new())],
            images: Vec::new(),
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

/// A single paragraph in the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paragraph {
    /// The text content
    pub text: String,
    /// Paragraph metadata/formatting
    pub meta: ParagraphMeta,
    /// Inline text styles (ranges with formatting)
    #[serde(default)]
    pub styles: Vec<TextStyle>,
}

/// Inline text style for a range of characters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    /// Start character index (inclusive)
    pub start: usize,
    /// End character index (exclusive)
    pub end: usize,
    /// Bold formatting
    #[serde(default)]
    pub bold: bool,
    /// Italic formatting
    #[serde(default)]
    pub italic: bool,
    /// Underline formatting
    #[serde(default)]
    pub underline: bool,
    /// Strikethrough formatting
    #[serde(default)]
    pub strikethrough: bool,
    /// Text color (CSS color string)
    #[serde(default)]
    pub color: Option<String>,
    /// Background/highlight color (CSS color string)
    #[serde(default)]
    pub background: Option<String>,
}

impl TextStyle {
    pub fn new(start: usize, end: usize) -> Self {
        TextStyle {
            start,
            end,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            color: None,
            background: None,
        }
    }

    /// Check if this style has any formatting applied
    pub fn has_formatting(&self) -> bool {
        self.bold || self.italic || self.underline || self.strikethrough
            || self.color.is_some() || self.background.is_some()
    }

    /// Check if this style overlaps with a range
    pub fn overlaps(&self, start: usize, end: usize) -> bool {
        self.start < end && self.end > start
    }

    /// Check if this style contains a range
    pub fn contains(&self, start: usize, end: usize) -> bool {
        self.start <= start && self.end >= end
    }
}

impl Paragraph {
    pub fn new(text: String) -> Self {
        Paragraph {
            text,
            meta: ParagraphMeta::default(),
            styles: Vec::new(),
        }
    }

    pub fn with_meta(text: String, meta: ParagraphMeta) -> Self {
        Paragraph { text, meta, styles: Vec::new() }
    }

    /// Apply a style to a range of text
    /// This handles merging and splitting existing styles
    pub fn apply_style<F>(&mut self, start: usize, end: usize, modifier: F)
    where
        F: Fn(&mut TextStyle),
    {
        if start >= end {
            return;
        }

        // Collect styles that need to be modified
        let mut new_styles: Vec<TextStyle> = Vec::new();

        for style in self.styles.iter() {
            if style.overlaps(start, end) {
                // Style overlaps with the range we're modifying

                // Part before the range (unchanged)
                if style.start < start {
                    let mut before = style.clone();
                    before.end = start;
                    new_styles.push(before);
                }

                // Part after the range (unchanged)
                if style.end > end {
                    let mut after = style.clone();
                    after.start = end;
                    new_styles.push(after);
                }

                // The overlapping part (will be merged with the new style)
                let overlap_start = style.start.max(start);
                let overlap_end = style.end.min(end);
                let mut overlap = style.clone();
                overlap.start = overlap_start;
                overlap.end = overlap_end;
                modifier(&mut overlap);
                if overlap.has_formatting() {
                    new_styles.push(overlap);
                }
            } else {
                // Style doesn't overlap, keep it as is
                new_styles.push(style.clone());
            }
        }

        // Check if we need to add a new style for uncovered parts of the range
        // Find gaps in the range [start, end) that aren't covered by any style
        let mut covered: Vec<(usize, usize)> = new_styles
            .iter()
            .filter(|s| s.overlaps(start, end))
            .map(|s| (s.start.max(start), s.end.min(end)))
            .collect();
        covered.sort_by_key(|r| r.0);

        let mut pos = start;
        for (s, e) in covered {
            if pos < s {
                // Gap from pos to s
                let mut new_style = TextStyle::new(pos, s);
                modifier(&mut new_style);
                if new_style.has_formatting() {
                    new_styles.push(new_style);
                }
            }
            pos = e;
        }
        if pos < end {
            // Gap from pos to end
            let mut new_style = TextStyle::new(pos, end);
            modifier(&mut new_style);
            if new_style.has_formatting() {
                new_styles.push(new_style);
            }
        }

        // Sort by start position and merge adjacent styles with same formatting
        new_styles.sort_by_key(|s| s.start);
        self.styles = Self::merge_adjacent_styles(new_styles);
    }

    /// Merge adjacent styles that have identical formatting
    fn merge_adjacent_styles(styles: Vec<TextStyle>) -> Vec<TextStyle> {
        let mut result: Vec<TextStyle> = Vec::new();

        for style in styles {
            if let Some(last) = result.last_mut() {
                if last.end == style.start
                    && last.bold == style.bold
                    && last.italic == style.italic
                    && last.underline == style.underline
                    && last.strikethrough == style.strikethrough
                    && last.color == style.color
                    && last.background == style.background
                {
                    // Merge
                    last.end = style.end;
                    continue;
                }
            }
            result.push(style);
        }

        result
    }

    /// Get the style at a specific character position
    pub fn style_at(&self, pos: usize) -> Option<&TextStyle> {
        self.styles.iter().find(|s| s.start <= pos && s.end > pos)
    }

    /// Get all styles that overlap with a range
    pub fn styles_in_range(&self, start: usize, end: usize) -> Vec<&TextStyle> {
        self.styles.iter().filter(|s| s.overlaps(start, end)).collect()
    }

    /// Check if this paragraph is a page break marker
    /// Uses Unicode replacement character U+FFFD to match JavaScript implementation
    pub fn is_page_break(&self) -> bool {
        self.text == "\u{FFFD}"
    }

    /// Check if this paragraph is an image marker
    /// Uses Unicode object replacement character U+FFFC to match JavaScript implementation
    pub fn is_image(&self) -> bool {
        self.text.starts_with('\u{FFFC}')
    }

    /// Get image ID if this is an image paragraph
    pub fn image_id(&self) -> Option<&str> {
        if self.is_image() {
            // Skip the U+FFFC marker character (3 bytes in UTF-8)
            Some(&self.text[3..])
        } else {
            None
        }
    }
}

/// Paragraph formatting metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphMeta {
    /// Text alignment
    pub align: TextAlign,
    /// Block type (paragraph, heading, etc.)
    pub block_type: BlockType,
    /// List type
    pub list_type: ListType,
    /// Custom font size (if different from default)
    pub font_size: Option<f64>,
    /// Text color
    pub text_color: Option<String>,
}

impl Default for ParagraphMeta {
    fn default() -> Self {
        ParagraphMeta {
            align: TextAlign::Left,
            block_type: BlockType::Paragraph,
            list_type: ListType::None,
            font_size: None,
            text_color: None,
        }
    }
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

/// Block-level element types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockType {
    #[serde(rename = "p")]
    Paragraph,
    #[serde(rename = "h1")]
    Heading1,
    #[serde(rename = "h2")]
    Heading2,
    #[serde(rename = "h3")]
    Heading3,
    #[serde(rename = "h4")]
    Heading4,
    #[serde(rename = "blockquote")]
    Blockquote,
}

impl BlockType {
    /// Get the font size multiplier for this block type
    pub fn font_size_multiplier(&self) -> f64 {
        match self {
            BlockType::Heading1 => 2.0,
            BlockType::Heading2 => 1.5,
            BlockType::Heading3 => 1.17,
            BlockType::Heading4 => 1.0,
            BlockType::Paragraph => 1.0,
            BlockType::Blockquote => 1.0,
        }
    }

    /// Check if this block type should be bold
    pub fn is_bold(&self) -> bool {
        matches!(
            self,
            BlockType::Heading1
                | BlockType::Heading2
                | BlockType::Heading3
                | BlockType::Heading4
        )
    }

    /// Check if this block type should be italic
    pub fn is_italic(&self) -> bool {
        matches!(self, BlockType::Blockquote)
    }
}

/// List type options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListType {
    None,
    Bullet,
    Numbered,
}

/// Image wrap style options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageWrapStyle {
    Inline,
    TopBottom,
    Square,
    Tight,
    Through,
    Behind,
    InFront,
}

impl ImageWrapStyle {
    /// Check if this wrap style causes text to flow around the image
    pub fn is_float(&self) -> bool {
        matches!(
            self,
            ImageWrapStyle::Square | ImageWrapStyle::Tight | ImageWrapStyle::Through
        )
    }
}

/// Horizontal alignment for images
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

/// An image in the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentImage {
    /// Unique identifier
    pub id: String,
    /// Image source URL or data URL
    pub src: String,
    /// Display width in pixels
    pub width: f64,
    /// Display height in pixels
    pub height: f64,
    /// Original/natural width
    pub natural_width: f64,
    /// Original/natural height
    pub natural_height: f64,
    /// Wrap style
    pub wrap_style: ImageWrapStyle,
    /// Horizontal alignment
    pub horizontal_align: HorizontalAlign,
    /// Absolute X position (if positioned)
    pub x: Option<f64>,
    /// Absolute Y position (if positioned)
    pub y: Option<f64>,
    /// Page index (if positioned)
    pub page_index: Option<usize>,
    /// Crop percentages
    pub crop_top: f64,
    pub crop_right: f64,
    pub crop_bottom: f64,
    pub crop_left: f64,
}

impl DocumentImage {
    pub fn new(id: String, src: String, width: f64, height: f64) -> Self {
        DocumentImage {
            id,
            src,
            width,
            height,
            natural_width: width,
            natural_height: height,
            wrap_style: ImageWrapStyle::Inline,
            horizontal_align: HorizontalAlign::Left,
            x: None,
            y: None,
            page_index: None,
            crop_top: 0.0,
            crop_right: 0.0,
            crop_bottom: 0.0,
            crop_left: 0.0,
        }
    }

    /// Get the effective display height after cropping
    pub fn cropped_height(&self) -> f64 {
        let crop_factor = (100.0 - self.crop_top - self.crop_bottom) / 100.0;
        self.height * crop_factor
    }

    /// Get the effective display width after cropping
    pub fn cropped_width(&self) -> f64 {
        let crop_factor = (100.0 - self.crop_left - self.crop_right) / 100.0;
        self.width * crop_factor
    }
}

