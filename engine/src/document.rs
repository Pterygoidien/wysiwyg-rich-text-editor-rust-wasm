//! Document model definitions
//!
//! This module contains the core data structures that represent a document:
//! paragraphs, formatting, images, and metadata.

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
}

impl Paragraph {
    pub fn new(text: String) -> Self {
        Paragraph {
            text,
            meta: ParagraphMeta::default(),
        }
    }

    pub fn with_meta(text: String, meta: ParagraphMeta) -> Self {
        Paragraph { text, meta }
    }

    /// Check if this paragraph is a page break marker
    pub fn is_page_break(&self) -> bool {
        self.text == "\x0C" // Form feed character
    }

    /// Check if this paragraph is an image marker
    pub fn is_image(&self) -> bool {
        self.text.starts_with('\x01') // SOH character
    }

    /// Get image ID if this is an image paragraph
    pub fn image_id(&self) -> Option<&str> {
        if self.is_image() {
            Some(&self.text[1..])
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paragraph_page_break() {
        let para = Paragraph::new("\x0C".to_string());
        assert!(para.is_page_break());
    }

    #[test]
    fn test_paragraph_image() {
        let para = Paragraph::new("\x01image-123".to_string());
        assert!(para.is_image());
        assert_eq!(para.image_id(), Some("image-123"));
    }

    #[test]
    fn test_block_type_multipliers() {
        assert_eq!(BlockType::Heading1.font_size_multiplier(), 2.0);
        assert_eq!(BlockType::Paragraph.font_size_multiplier(), 1.0);
    }
}
