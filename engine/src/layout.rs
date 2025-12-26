//! Layout engine
//!
//! Computes the visual layout of the document, including:
//! - Line wrapping
//! - Pagination
//! - Multi-column layout
//! - Float positioning

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::document::{BlockType, Document, ListType, Paragraph};

/// Configuration for page layout
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    /// Page width in pixels
    pub page_width: f64,
    /// Page height in pixels
    pub page_height: f64,
    /// Top margin in pixels
    pub margin_top: f64,
    /// Right margin in pixels
    pub margin_right: f64,
    /// Bottom margin in pixels
    pub margin_bottom: f64,
    /// Left margin in pixels
    pub margin_left: f64,
    /// Number of columns
    pub columns: u8,
    /// Gap between columns in pixels
    pub column_gap: f64,
    /// Base font size in pixels
    pub font_size: f64,
    /// Line height multiplier
    pub line_height: f64,
    /// Letter spacing in pixels
    pub letter_spacing: f64,
    /// Paragraph spacing in pixels
    pub paragraph_spacing: f64,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        LayoutConfig {
            page_width: 816.0,  // US Letter at 96 DPI
            page_height: 1056.0,
            margin_top: 96.0,   // 1 inch
            margin_right: 96.0,
            margin_bottom: 96.0,
            margin_left: 96.0,
            columns: 1,
            column_gap: 48.0,
            font_size: 16.0,
            line_height: 1.5,
            letter_spacing: 0.0,
            paragraph_spacing: 12.0,
        }
    }
}

impl LayoutConfig {
    /// Get the content width (page width minus margins)
    pub fn content_width(&self) -> f64 {
        self.page_width - self.margin_left - self.margin_right
    }

    /// Get the content height (page height minus margins)
    pub fn content_height(&self) -> f64 {
        self.page_height - self.margin_top - self.margin_bottom
    }

    /// Get the width of a single column
    pub fn column_width(&self) -> f64 {
        let total_gap = self.column_gap * (self.columns as f64 - 1.0);
        (self.content_width() - total_gap) / self.columns as f64
    }

    /// Get the line height in pixels
    pub fn line_height_px(&self) -> f64 {
        self.font_size * self.line_height
    }
}

/// A computed display line
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayLine {
    /// Index of the source paragraph
    pub para_index: usize,
    /// Start character offset within the paragraph
    pub start_offset: usize,
    /// End character offset within the paragraph
    pub end_offset: usize,
    /// The text content of this line
    pub text: String,
    /// Page index (0-based)
    pub page_index: usize,
    /// Column index (0-based)
    pub column_index: usize,
    /// X position on the page
    pub x_position: f64,
    /// Y position on the page
    pub y_position: f64,
    /// Whether this is a page break marker
    pub is_page_break: bool,
    /// Whether this is an image line
    pub is_image: bool,
    /// Image ID if this is an image line
    pub image_id: Option<String>,
    /// Image height in line units
    pub image_height: Option<f64>,
    /// List number (for numbered lists)
    pub list_number: Option<usize>,
    /// Whether this is the last line of the paragraph
    pub is_last_line: bool,
    /// Block type for styling
    pub block_type: BlockType,
    /// List type for markers
    pub list_type: ListType,
    /// Float reduction for text wrapping around images
    pub float_reduction: Option<FloatReduction>,
}

/// Describes width reduction due to a floating image
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatReduction {
    pub side: FloatSide,
    pub width: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FloatSide {
    Left,
    Right,
}

/// Active floating image for text wrapping
#[derive(Debug, Clone)]
pub struct ActiveFloat {
    pub id: String,
    pub start_line: usize,
    pub end_line: usize,
    pub width: f64,
    pub side: FloatSide,
}

/// Text measurement function signature (called from JS)
type MeasureFn<'a> = &'a js_sys::Function;

/// Compute the layout for the entire document
pub fn compute_layout(
    document: &Document,
    config: &LayoutConfig,
    measure_fn: MeasureFn,
) -> Vec<DisplayLine> {
    let mut display_lines: Vec<DisplayLine> = Vec::new();
    let mut active_floats: Vec<ActiveFloat> = Vec::new();
    let mut list_counters: Vec<usize> = Vec::new();

    // First pass: Generate display lines for each paragraph
    for (para_idx, para) in document.paragraphs.iter().enumerate() {
        let lines = layout_paragraph(
            para_idx,
            para,
            document,
            config,
            measure_fn,
            &mut active_floats,
            &mut list_counters,
            display_lines.len(),
        );
        display_lines.extend(lines);
    }

    // Second pass: Assign page and column positions
    assign_page_positions(&mut display_lines, config);

    display_lines
}

/// Layout a single paragraph into display lines
fn layout_paragraph(
    para_idx: usize,
    para: &Paragraph,
    document: &Document,
    config: &LayoutConfig,
    measure_fn: MeasureFn,
    active_floats: &mut Vec<ActiveFloat>,
    list_counters: &mut Vec<usize>,
    current_line_count: usize,
) -> Vec<DisplayLine> {
    let meta = &para.meta;

    // Handle page breaks
    if para.is_page_break() {
        return vec![DisplayLine {
            para_index: para_idx,
            start_offset: 0,
            end_offset: 1,
            text: String::new(),
            page_index: 0,
            column_index: 0,
            x_position: 0.0,
            y_position: 0.0,
            is_page_break: true,
            is_image: false,
            image_id: None,
            image_height: None,
            list_number: None,
            is_last_line: true,
            block_type: meta.block_type,
            list_type: meta.list_type,
            float_reduction: None,
        }];
    }

    // Handle image paragraphs
    if let Some(image_id) = para.image_id() {
        if let Some(image) = document.images.iter().find(|img| img.id == image_id) {
            // Check if this is a float image
            if image.wrap_style.is_float() && image.y.is_none() {
                // Register as active float
                let side = match image.horizontal_align {
                    crate::document::HorizontalAlign::Right => FloatSide::Right,
                    _ => FloatSide::Left,
                };
                let image_lines = (image.cropped_height() / config.line_height_px()).ceil() as usize;
                active_floats.push(ActiveFloat {
                    id: image_id.to_string(),
                    start_line: current_line_count,
                    end_line: current_line_count + image_lines,
                    width: image.width,
                    side,
                });

                // Float images don't create visible display lines
                return vec![DisplayLine {
                    para_index: para_idx,
                    start_offset: 0,
                    end_offset: para.text.len(),
                    text: String::new(),
                    page_index: 0,
                    column_index: 0,
                    x_position: 0.0,
                    y_position: 0.0,
                    is_page_break: false,
                    is_image: true,
                    image_id: Some(image_id.to_string()),
                    image_height: Some(0.0), // Zero height for float
                    list_number: None,
                    is_last_line: true,
                    block_type: meta.block_type,
                    list_type: ListType::None,
                    float_reduction: None,
                }];
            }

            // Non-float image
            let image_lines = (image.cropped_height() / config.line_height_px()).ceil();
            return vec![DisplayLine {
                para_index: para_idx,
                start_offset: 0,
                end_offset: para.text.len(),
                text: String::new(),
                page_index: 0,
                column_index: 0,
                x_position: 0.0,
                y_position: 0.0,
                is_page_break: false,
                is_image: true,
                image_id: Some(image_id.to_string()),
                image_height: Some(image_lines),
                list_number: None,
                is_last_line: true,
                block_type: meta.block_type,
                list_type: ListType::None,
                float_reduction: None,
            }];
        }
    }

    // Handle list numbering
    let list_number = match meta.list_type {
        ListType::Numbered => {
            let num = list_counters.last().copied().unwrap_or(0) + 1;
            if list_counters.is_empty() {
                list_counters.push(num);
            } else {
                *list_counters.last_mut().unwrap() = num;
            }
            Some(num)
        }
        ListType::Bullet => None,
        ListType::None => {
            list_counters.clear();
            None
        }
    };

    // Calculate available width
    let font_size = meta.font_size.unwrap_or(config.font_size)
        * meta.block_type.font_size_multiplier();
    let list_indent = if meta.list_type != ListType::None {
        font_size * 1.5
    } else {
        0.0
    };

    // Check for active floats affecting this line
    let float_reduction = get_float_reduction(active_floats, current_line_count);
    let float_width = float_reduction.as_ref().map(|f| f.width + 10.0).unwrap_or(0.0);

    let available_width = config.column_width() - list_indent - float_width;

    // Wrap the paragraph text
    let text = &para.text;
    if text.is_empty() {
        return vec![DisplayLine {
            para_index: para_idx,
            start_offset: 0,
            end_offset: 0,
            text: String::new(),
            page_index: 0,
            column_index: 0,
            x_position: 0.0,
            y_position: 0.0,
            is_page_break: false,
            is_image: false,
            image_id: None,
            image_height: None,
            list_number,
            is_last_line: true,
            block_type: meta.block_type,
            list_type: meta.list_type,
            float_reduction,
        }];
    }

    // Word wrap the text
    let mut lines: Vec<DisplayLine> = Vec::new();
    let mut current_start = 0;

    while current_start < text.len() {
        let remaining = &text[current_start..];

        // Measure remaining text
        let remaining_width = measure_text(measure_fn, remaining, font_size, config.letter_spacing);

        if remaining_width <= available_width {
            // Entire remaining text fits
            lines.push(DisplayLine {
                para_index: para_idx,
                start_offset: current_start,
                end_offset: text.len(),
                text: remaining.to_string(),
                page_index: 0,
                column_index: 0,
                x_position: 0.0,
                y_position: 0.0,
                is_page_break: false,
                is_image: false,
                image_id: None,
                image_height: None,
                list_number: if lines.is_empty() { list_number } else { None },
                is_last_line: true,
                block_type: meta.block_type,
                list_type: meta.list_type,
                float_reduction: float_reduction.clone(),
            });
            break;
        }

        // Find break point
        let mut line_end = current_start;
        let mut last_word_boundary = current_start;

        for (i, c) in text[current_start..].char_indices() {
            let pos = current_start + i;
            let test_text = &text[current_start..=pos];
            let width = measure_text(measure_fn, test_text, font_size, config.letter_spacing);

            if c == ' ' {
                last_word_boundary = pos + 1;
            }

            if width > available_width {
                // Exceeded width, break at last word boundary
                line_end = if last_word_boundary > current_start {
                    last_word_boundary
                } else {
                    pos.max(current_start + 1)
                };
                break;
            }

            line_end = pos + c.len_utf8();
        }

        // Ensure progress
        if line_end <= current_start {
            line_end = current_start + 1;
        }

        let line_text = text[current_start..line_end].to_string();
        lines.push(DisplayLine {
            para_index: para_idx,
            start_offset: current_start,
            end_offset: line_end,
            text: line_text,
            page_index: 0,
            column_index: 0,
            x_position: 0.0,
            y_position: 0.0,
            is_page_break: false,
            is_image: false,
            image_id: None,
            image_height: None,
            list_number: if lines.is_empty() { list_number } else { None },
            is_last_line: false,
            block_type: meta.block_type,
            list_type: meta.list_type,
            float_reduction: float_reduction.clone(),
        });

        current_start = line_end;
    }

    // Mark last line
    if let Some(last) = lines.last_mut() {
        last.is_last_line = true;
    }

    lines
}

/// Get float reduction for a given line index
fn get_float_reduction(floats: &[ActiveFloat], line_index: usize) -> Option<FloatReduction> {
    for float in floats {
        if line_index >= float.start_line && line_index < float.end_line {
            return Some(FloatReduction {
                side: float.side,
                width: float.width,
            });
        }
    }
    None
}

/// Assign page and column positions to all display lines
fn assign_page_positions(display_lines: &mut [DisplayLine], config: &LayoutConfig) {
    let mut current_y = 0.0;
    let mut current_page = 0;
    let mut current_column = 0;
    let max_column_height = config.content_height();
    let line_height = config.line_height_px();

    for dl in display_lines.iter_mut() {
        // Handle page breaks
        if dl.is_page_break {
            dl.page_index = current_page;
            dl.y_position = current_y;
            dl.column_index = current_column;
            // Move to next page
            current_page += 1;
            current_column = 0;
            current_y = 0.0;
            continue;
        }

        // Calculate line height for this line
        let this_line_height = if dl.is_image {
            dl.image_height.unwrap_or(1.0) * line_height
        } else {
            line_height
        };

        // Add paragraph spacing if last line
        let spacing_after = if dl.is_last_line {
            config.paragraph_spacing
        } else {
            0.0
        };

        // Check for overflow
        if current_y + this_line_height > max_column_height {
            // Move to next column or page
            if config.columns > 1 && current_column < (config.columns - 1) as usize {
                current_column += 1;
                current_y = 0.0;
            } else {
                current_page += 1;
                current_column = 0;
                current_y = 0.0;
            }
        }

        // Assign position
        dl.page_index = current_page;
        dl.column_index = current_column;
        dl.y_position = current_y;

        // Calculate X position
        let column_offset = current_column as f64 * (config.column_width() + config.column_gap);
        dl.x_position = config.margin_left + column_offset;

        // Advance Y
        current_y += this_line_height + spacing_after;
    }
}

/// Measure text width using the provided JS function
fn measure_text(
    measure_fn: &js_sys::Function,
    text: &str,
    font_size: f64,
    letter_spacing: f64,
) -> f64 {
    let this = JsValue::NULL;
    let text_js = JsValue::from_str(text);
    let size_js = JsValue::from_f64(font_size);

    match measure_fn.call2(&this, &text_js, &size_js) {
        Ok(result) => {
            let width = result.as_f64().unwrap_or(text.len() as f64 * font_size * 0.5);
            // Add letter spacing
            let spacing = if text.len() > 1 {
                (text.chars().count() - 1) as f64 * letter_spacing
            } else {
                0.0
            };
            width + spacing
        }
        Err(_) => text.len() as f64 * font_size * 0.5,
    }
}

/// Result of mapping a paragraph position to a display position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayPosition {
    /// Display line index
    pub line: usize,
    /// Column offset within the display line
    pub col: usize,
}

/// Result of mapping a display position to a paragraph position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphPosition {
    /// Paragraph index
    pub para: usize,
    /// Character offset within the paragraph
    pub offset: usize,
}

/// Convert a paragraph position (para index, char offset) to a display line position.
/// Used for mapping cursor/selection positions to rendered coordinates.
pub fn para_to_display_pos(
    display_lines: &[DisplayLine],
    para: usize,
    offset: usize,
) -> DisplayPosition {
    for (i, dl) in display_lines.iter().enumerate() {
        if dl.para_index == para && offset >= dl.start_offset && offset <= dl.end_offset {
            return DisplayPosition {
                line: i,
                col: offset - dl.start_offset,
            };
        }
    }

    // Fallback to last line
    let last_line = display_lines.len().saturating_sub(1);
    let last_col = display_lines
        .last()
        .map(|dl| dl.text.len())
        .unwrap_or(0);

    DisplayPosition {
        line: last_line,
        col: last_col,
    }
}

/// Convert a display line position to a paragraph position.
/// Used for mapping click coordinates back to document positions.
pub fn display_to_para(
    display_lines: &[DisplayLine],
    line: usize,
    col: usize,
) -> ParagraphPosition {
    if line >= display_lines.len() {
        // Beyond end of document
        if let Some(last) = display_lines.last() {
            return ParagraphPosition {
                para: last.para_index,
                offset: last.end_offset,
            };
        }
        return ParagraphPosition { para: 0, offset: 0 };
    }

    let dl = &display_lines[line];
    let clamped_col = col.min(dl.text.len());

    ParagraphPosition {
        para: dl.para_index,
        offset: dl.start_offset + clamped_col,
    }
}

/// Get the page index for a given paragraph and offset
pub fn get_page_for_position(
    display_lines: &[DisplayLine],
    para: usize,
    offset: usize,
) -> usize {
    let pos = para_to_display_pos(display_lines, para, offset);
    display_lines
        .get(pos.line)
        .map(|dl| dl.page_index)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layout_config_defaults() {
        let config = LayoutConfig::default();
        assert!(config.content_width() > 0.0);
        assert!(config.content_height() > 0.0);
    }

    #[test]
    fn test_column_width_single() {
        let config = LayoutConfig {
            columns: 1,
            ..Default::default()
        };
        assert_eq!(config.column_width(), config.content_width());
    }

    #[test]
    fn test_column_width_double() {
        let config = LayoutConfig {
            columns: 2,
            column_gap: 48.0,
            ..Default::default()
        };
        let expected = (config.content_width() - 48.0) / 2.0;
        assert!((config.column_width() - expected).abs() < 0.001);
    }

    // Helper to create test display lines
    fn create_test_display_lines() -> Vec<DisplayLine> {
        vec![
            // Paragraph 0, line 0: "Hello "
            DisplayLine {
                para_index: 0,
                start_offset: 0,
                end_offset: 6,
                text: "Hello ".to_string(),
                page_index: 0,
                column_index: 0,
                x_position: 96.0,
                y_position: 0.0,
                is_page_break: false,
                is_image: false,
                image_id: None,
                image_height: None,
                list_number: None,
                is_last_line: false,
                block_type: BlockType::Paragraph,
                list_type: ListType::None,
                float_reduction: None,
            },
            // Paragraph 0, line 1: "World"
            DisplayLine {
                para_index: 0,
                start_offset: 6,
                end_offset: 11,
                text: "World".to_string(),
                page_index: 0,
                column_index: 0,
                x_position: 96.0,
                y_position: 24.0,
                is_page_break: false,
                is_image: false,
                image_id: None,
                image_height: None,
                list_number: None,
                is_last_line: true,
                block_type: BlockType::Paragraph,
                list_type: ListType::None,
                float_reduction: None,
            },
            // Paragraph 1, line 0: "Second paragraph"
            DisplayLine {
                para_index: 1,
                start_offset: 0,
                end_offset: 16,
                text: "Second paragraph".to_string(),
                page_index: 0,
                column_index: 0,
                x_position: 96.0,
                y_position: 48.0,
                is_page_break: false,
                is_image: false,
                image_id: None,
                image_height: None,
                list_number: None,
                is_last_line: true,
                block_type: BlockType::Paragraph,
                list_type: ListType::None,
                float_reduction: None,
            },
        ]
    }

    #[test]
    fn test_para_to_display_pos_first_line() {
        let lines = create_test_display_lines();
        let pos = para_to_display_pos(&lines, 0, 3);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.col, 3);
    }

    #[test]
    fn test_para_to_display_pos_wrapped_line() {
        let lines = create_test_display_lines();
        // Offset 8 is "or" in "World" (offset 6-11 on line 1)
        let pos = para_to_display_pos(&lines, 0, 8);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.col, 2); // 8 - 6 = 2
    }

    #[test]
    fn test_para_to_display_pos_second_paragraph() {
        let lines = create_test_display_lines();
        let pos = para_to_display_pos(&lines, 1, 7);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.col, 7);
    }

    #[test]
    fn test_display_to_para_first_line() {
        let lines = create_test_display_lines();
        let pos = display_to_para(&lines, 0, 3);
        assert_eq!(pos.para, 0);
        assert_eq!(pos.offset, 3);
    }

    #[test]
    fn test_display_to_para_wrapped_line() {
        let lines = create_test_display_lines();
        let pos = display_to_para(&lines, 1, 2);
        assert_eq!(pos.para, 0);
        assert_eq!(pos.offset, 8); // 6 + 2
    }

    #[test]
    fn test_display_to_para_second_paragraph() {
        let lines = create_test_display_lines();
        let pos = display_to_para(&lines, 2, 7);
        assert_eq!(pos.para, 1);
        assert_eq!(pos.offset, 7);
    }

    #[test]
    fn test_display_to_para_beyond_end() {
        let lines = create_test_display_lines();
        let pos = display_to_para(&lines, 100, 5);
        assert_eq!(pos.para, 1);
        assert_eq!(pos.offset, 16); // End of last paragraph
    }

    #[test]
    fn test_get_page_for_position() {
        let lines = create_test_display_lines();
        let page = get_page_for_position(&lines, 0, 3);
        assert_eq!(page, 0);
    }
}
