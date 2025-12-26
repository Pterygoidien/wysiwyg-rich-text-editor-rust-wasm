//! Layout Engine
//!
//! This module computes the visual layout of a document, transforming the logical
//! document structure into positioned display lines ready for rendering.
//!
//! # Responsibilities
//!
//! - **Text Wrapping**: Break paragraphs into lines that fit within column width
//! - **Pagination**: Distribute lines across pages, respecting page height constraints
//! - **Multi-Column Layout**: Support for 1-N column layouts with configurable gaps
//! - **Float Positioning**: Text wrapping around floating images
//! - **Position Mapping**: Bidirectional conversion between document positions and screen coordinates
//!
//! # Layout Pipeline
//!
//! ```text
//! Document → compute_layout() → Vec<DisplayLine>
//!                    ↓
//!            1. layout_paragraph()    - Wrap each paragraph into lines
//!            2. assign_page_positions() - Assign page/column/y positions
//! ```
//!
//! # Configuration
//!
//! Layout is controlled by `LayoutConfig` which specifies:
//! - Page dimensions and margins
//! - Number of columns and gap between them
//! - Font size, line height, and spacing
//!
//! # Display Lines
//!
//! The output is a `Vec<DisplayLine>`, where each `DisplayLine` represents:
//! - A portion of text from a single paragraph
//! - Its position (page, column, x, y coordinates)
//! - Rendering metadata (block type, list info, float adjustments)
//!
//! # Position Mapping
//!
//! The module provides functions for cursor/selection positioning:
//! - `para_to_display_pos()`: Convert (paragraph, offset) → (line, column)
//! - `display_to_para()`: Convert (line, column) → (paragraph, offset)
//! - `get_page_for_position()`: Find which page contains a position

use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use crate::document::{BlockType, Document, DocumentTable, HorizontalAlign, ImagePositionMode, ImageWrapStyle, ListType, Paragraph, TableWidthMode};

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
#[serde(rename_all = "camelCase")]
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
    /// Whether this is a table line
    #[serde(default)]
    pub is_table: bool,
    /// Table ID if this is a table line
    #[serde(default)]
    pub table_id: Option<String>,
    /// Computed table layout (for rendering)
    #[serde(default)]
    pub table_layout: Option<TableLayout>,
}

/// Describes width reduction due to a floating image
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FloatReduction {
    pub side: FloatSide,
    pub width: f64,
    /// X position of the float (relative to column start)
    pub float_x: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FloatSide {
    Left,
    Right,
}

/// Computed table layout for rendering
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TableLayout {
    /// Table ID
    pub table_id: String,
    /// Computed row heights in pixels
    pub row_heights: Vec<f64>,
    /// Computed column widths in pixels
    pub column_widths: Vec<f64>,
    /// Total table height in pixels
    pub total_height: f64,
    /// Total table width in pixels
    pub total_width: f64,
    /// Cell text layouts (row, col) -> wrapped lines
    pub cell_lines: Vec<Vec<Vec<String>>>,
}

/// Active floating image for text wrapping
#[derive(Debug, Clone)]
pub struct ActiveFloat {
    pub id: String,
    /// For move-with-text floats: line indices
    pub start_line: usize,
    pub end_line: usize,
    pub width: f64,
    pub side: FloatSide,
    /// For positioned floats, the page they're on
    pub page_index: Option<usize>,
    /// For fixed-position floats: Y coordinates (relative to margin)
    pub y_start: Option<f64>,
    pub y_end: Option<f64>,
    /// X position of the float (relative to margin, for fixed-position floats)
    pub x_position: Option<f64>,
}

/// Convert horizontal alignment to float side
fn align_to_float_side(align: HorizontalAlign) -> FloatSide {
    match align {
        HorizontalAlign::Right => FloatSide::Right,
        _ => FloatSide::Left,
    }
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

    // Pre-pass: Collect all images with fixed positions (already positioned floats)
    // These affect text layout based on their absolute Y position
    for image in &document.images {
        if image.wrap_style.is_float()
            && image.position_mode == ImagePositionMode::FixedPosition
            && image.y.is_some()
        {
            let y = image.y.unwrap();
            let x = image.x.unwrap_or(0.0);
            let image_height = image.cropped_height();
            let image_width = image.width.min(config.column_width());

            // Determine which side the float is on based on its X position
            // If image center is in left half of column, it's a left float; otherwise right
            let column_width = config.column_width();
            let image_center = x + image_width / 2.0;
            let side = if image_center < column_width / 2.0 {
                FloatSide::Left
            } else {
                FloatSide::Right
            };

            // Store Y coordinates for fixed-position floats
            // These will be checked against line Y positions during layout
            active_floats.push(ActiveFloat {
                id: image.id.clone(),
                start_line: 0, // Not used for fixed-position
                end_line: 0,   // Not used for fixed-position
                width: image_width,
                side,
                page_index: image.page_index,
                y_start: Some(y),
                y_end: Some(y + image_height),
                x_position: Some(x),
            });
        }
    }

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
            is_table: false,
            table_id: None,
            table_layout: None,
        }];
    }

    // Handle table paragraphs
    if let Some(table_id) = para.table_id() {
        if let Some(table) = document.tables.iter().find(|t| t.id == table_id) {
            let table_layout = compute_table_layout(table, config, measure_fn);
            let table_height = table_layout.total_height;

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
                is_image: false,
                image_id: None,
                image_height: Some(table_height / config.line_height_px()), // Convert to line units
                list_number: None,
                is_last_line: true,
                block_type: meta.block_type,
                list_type: ListType::None,
                float_reduction: None,
                is_table: true,
                table_id: Some(table_id.to_string()),
                table_layout: Some(table_layout),
            }];
        }
    }

    // Handle image paragraphs
    if let Some(image_id) = para.image_id() {
        if let Some(image) = document.images.iter().find(|img| img.id == image_id) {
            let clamped_width = image.width.min(config.column_width());
            let line_height = config.line_height_px();
            let image_height = image.cropped_height();

            // For float reduction: count all lines that overlap with the image
            // A line at Y overlaps if Y < image_height (its top is within the image bounds)
            // Use ceil() to include any line that has partial overlap
            let float_lines = if image_height <= 0.0 {
                0
            } else {
                (image_height / line_height).ceil() as usize
            };

            // For inline/top-bottom images: use ceil() for vertical space (image occupies full lines)
            let inline_image_lines = (image_height / line_height).ceil();

            // Check if this is a float image in move-with-text mode
            if image.wrap_style.is_float() && image.position_mode == ImagePositionMode::MoveWithText {
                // Register as active float for text lines that overlap with the image
                // start_line is current_line_count + 1 because the marker at current_line_count
                // has zero height and doesn't need reduction. Text starts at the next line.
                active_floats.push(ActiveFloat {
                    id: image_id.to_string(),
                    start_line: current_line_count + 1,
                    end_line: current_line_count + 1 + float_lines,
                    width: clamped_width,
                    side: align_to_float_side(image.horizontal_align),
                    page_index: None, // Will be determined during page assignment
                    y_start: None,    // Line-index based, not Y-based
                    y_end: None,
                    x_position: None, // Will be calculated based on alignment during layout
                });

                // Float images create a zero-height marker line
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
                    image_height: Some(0.0), // Zero height for float - doesn't take up space
                    list_number: None,
                    is_last_line: true,
                    block_type: meta.block_type,
                    list_type: ListType::None,
                    float_reduction: None,
                    is_table: false,
                    table_id: None,
                    table_layout: None,
                }];
            }

            // Check if this is a positioned float (already handled in pre-pass)
            if image.wrap_style.is_float() && image.position_mode == ImagePositionMode::FixedPosition {
                // Already registered in pre-pass, just create marker line
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
                    image_height: Some(0.0), // Zero height - position is absolute
                    list_number: None,
                    is_last_line: true,
                    block_type: meta.block_type,
                    list_type: ListType::None,
                    float_reduction: None,
                    is_table: false,
                    table_id: None,
                    table_layout: None,
                }];
            }

            // Behind/in-front images don't affect text flow
            if matches!(image.wrap_style, ImageWrapStyle::Behind | ImageWrapStyle::InFront) {
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
                    image_height: Some(0.0), // Zero height - doesn't affect text flow
                    list_number: None,
                    is_last_line: true,
                    block_type: meta.block_type,
                    list_type: ListType::None,
                    float_reduction: None,
                    is_table: false,
                    table_id: None,
                    table_layout: None,
                }];
            }

            // Inline or top-bottom image: takes up vertical space
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
                image_height: Some(inline_image_lines),
                list_number: None,
                is_last_line: true,
                block_type: meta.block_type,
                list_type: ListType::None,
                float_reduction: None,
                is_table: false,
                table_id: None,
                table_layout: None,
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

    // Calculate base formatting
    let font_size = meta.font_size.unwrap_or(config.font_size)
        * meta.block_type.font_size_multiplier();
    let list_indent = if meta.list_type != ListType::None {
        font_size * 1.5
    } else {
        0.0
    };
    let base_available_width = config.column_width() - list_indent;

    // Wrap the paragraph text
    let text = &para.text;
    let line_height = config.line_height_px();
    let column_width = config.column_width();
    if text.is_empty() {
        let estimated_y = current_line_count as f64 * line_height;
        let float_reduction = get_float_reduction(active_floats, current_line_count, estimated_y, line_height, column_width);
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
            is_table: false,
            table_id: None,
            table_layout: None,
        }];
    }

    // Word wrap the text with per-line float checking
    let mut lines: Vec<DisplayLine> = Vec::new();
    let mut current_start = 0;

    while current_start < text.len() {
        // Check for active floats affecting THIS line (not the first line)
        let line_index = current_line_count + lines.len();
        let estimated_y = line_index as f64 * line_height;
        let float_reduction = get_float_reduction(active_floats, line_index, estimated_y, line_height, column_width);
        let float_width = float_reduction.as_ref().map(|f| f.width + 10.0).unwrap_or(0.0);
        let available_width = base_available_width - float_width;

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
                float_reduction,
                is_table: false,
                table_id: None,
                table_layout: None,
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
            is_table: false,
            table_id: None,
            table_layout: None,
        });

        current_start = line_end;
    }

    // Mark last line
    if let Some(last) = lines.last_mut() {
        last.is_last_line = true;
    }

    lines
}

/// Compute the layout for a table
fn compute_table_layout(
    table: &DocumentTable,
    config: &LayoutConfig,
    measure_fn: MeasureFn,
) -> TableLayout {
    let available_width = config.column_width();
    let line_height = config.line_height_px();
    let font_size = config.font_size;
    let cell_padding = 8.0; // 4px on each side
    let border = table.border_width;
    let num_cols = table.column_widths.len();

    // Total border width used by all vertical borders
    let total_border_width = (num_cols + 1) as f64 * border;

    // Width available for cell content (total minus borders)
    let content_width = available_width - total_border_width;

    // 1. Calculate column widths based on mode
    let column_widths: Vec<f64> = match table.width_mode {
        TableWidthMode::Fixed => table.column_widths.clone(),
        TableWidthMode::Percentage => {
            // Distribute content_width according to percentages
            table.column_widths
                .iter()
                .map(|w| content_width * w / 100.0)
                .collect()
        }
        TableWidthMode::Auto => {
            // For now, use percentage mode for auto too
            table.column_widths
                .iter()
                .map(|w| content_width * w / 100.0)
                .collect()
        }
    };

    // 2. Calculate cell text layouts and row heights
    // First pass: calculate base row heights without considering row spans
    let mut row_heights: Vec<f64> = Vec::new();
    let mut cell_lines: Vec<Vec<Vec<String>>> = Vec::new();

    for row in &table.rows {
        let mut row_cell_lines: Vec<Vec<String>> = Vec::new();
        let mut max_lines = 1;

        for (col_idx, cell) in row.cells.iter().enumerate() {
            // Skip covered cells - they don't contribute to row height calculation
            if cell.covered {
                row_cell_lines.push(vec![String::new()]);
                continue;
            }

            // Calculate the width of this cell (accounting for col_span)
            let mut cell_content_width = 0.0;
            for span_col in col_idx..(col_idx + cell.col_span).min(num_cols) {
                cell_content_width += column_widths.get(span_col).copied().unwrap_or(0.0);
            }
            // Add border widths between spanned columns
            if cell.col_span > 1 {
                cell_content_width += (cell.col_span - 1) as f64 * border;
            }
            cell_content_width -= cell_padding;

            // Wrap cell text
            let lines = wrap_text_for_cell(&cell.text, cell_content_width, font_size, measure_fn, config);

            // Only count lines for row height if this cell doesn't span multiple rows
            if cell.row_span == 1 {
                max_lines = max_lines.max(lines.len());
            }
            row_cell_lines.push(lines);
        }

        // Row height = max lines * line_height + padding
        let row_height = (max_lines as f64 * line_height + cell_padding).max(
            row.min_height.unwrap_or(line_height + cell_padding)
        );
        row_heights.push(row_height);
        cell_lines.push(row_cell_lines);
    }

    // Second pass: adjust row heights for cells with row spans
    for (row_idx, row) in table.rows.iter().enumerate() {
        for (col_idx, cell) in row.cells.iter().enumerate() {
            if cell.covered || cell.row_span <= 1 {
                continue;
            }

            // Calculate required height for this spanning cell
            let lines_count = cell_lines.get(row_idx)
                .and_then(|r| r.get(col_idx))
                .map(|l| l.len())
                .unwrap_or(1);
            let required_height = lines_count as f64 * line_height + cell_padding;

            // Calculate current total height of spanned rows
            let spanned_rows_end = (row_idx + cell.row_span).min(table.rows.len());
            let current_height: f64 = row_heights[row_idx..spanned_rows_end].iter().sum();
            let border_height = (cell.row_span - 1) as f64 * border;
            let current_total = current_height + border_height;

            // If required height > current total, distribute extra height
            if required_height > current_total {
                let extra = required_height - current_total;
                let extra_per_row = extra / cell.row_span as f64;
                for r in row_idx..spanned_rows_end {
                    row_heights[r] += extra_per_row;
                }
            }
        }
    }

    // 3. Calculate totals - table should span full available width
    let total_height = row_heights.iter().sum::<f64>() + (table.rows.len() + 1) as f64 * border;
    let total_width = available_width; // Full column width

    TableLayout {
        table_id: table.id.clone(),
        row_heights,
        column_widths,
        total_height,
        total_width,
        cell_lines,
    }
}

/// Wrap text for a table cell, returning lines
/// Handles explicit newlines and word wrapping
fn wrap_text_for_cell(
    text: &str,
    max_width: f64,
    font_size: f64,
    measure_fn: MeasureFn,
    config: &LayoutConfig,
) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }

    let mut all_lines: Vec<String> = Vec::new();

    // First, split by explicit newlines
    for paragraph in text.split('\n') {
        if paragraph.is_empty() {
            all_lines.push(String::new());
            continue;
        }

        // Then wrap each paragraph
        let mut current_start = 0;

        while current_start < paragraph.len() {
            let remaining = &paragraph[current_start..];
            let remaining_width = measure_text(measure_fn, remaining, font_size, config.letter_spacing);

            if remaining_width <= max_width {
                all_lines.push(remaining.to_string());
                break;
            }

            // Find break point
            let mut line_end = current_start;
            let mut last_word_boundary = current_start;

            for (i, c) in paragraph[current_start..].char_indices() {
                let pos = current_start + i;
                let test_text = &paragraph[current_start..=pos];
                let width = measure_text(measure_fn, test_text, font_size, config.letter_spacing);

                if c == ' ' {
                    last_word_boundary = pos + 1;
                }

                if width > max_width {
                    line_end = if last_word_boundary > current_start {
                        last_word_boundary
                    } else {
                        pos.max(current_start + 1)
                    };
                    break;
                }

                line_end = pos + c.len_utf8();
            }

            if line_end <= current_start {
                line_end = current_start + 1;
            }

            all_lines.push(paragraph[current_start..line_end].to_string());
            current_start = line_end;
        }
    }

    if all_lines.is_empty() {
        all_lines.push(String::new());
    }

    all_lines
}

/// Get float reduction for a given line
///
/// For move-with-text floats: uses line_index to check overlap
/// For fixed-position floats: uses estimated_y to check Y-based overlap
///
/// Returns the float reduction including the X position of the float
fn get_float_reduction(
    floats: &[ActiveFloat],
    line_index: usize,
    estimated_y: f64,
    line_height: f64,
    column_width: f64,
) -> Option<FloatReduction> {
    for float in floats {
        // Check Y-based overlap for fixed-position floats
        if let (Some(y_start), Some(y_end)) = (float.y_start, float.y_end) {
            // Line occupies Y range [estimated_y, estimated_y + line_height)
            // Float occupies Y range [y_start, y_end)
            // They overlap if: estimated_y < y_end AND estimated_y + line_height > y_start
            if estimated_y < y_end && estimated_y + line_height > y_start {
                // For fixed-position floats, use stored X position
                let float_x = float.x_position.unwrap_or(0.0);
                return Some(FloatReduction {
                    side: float.side,
                    width: float.width,
                    float_x,
                });
            }
        } else {
            // Line-index based overlap for move-with-text floats
            if line_index >= float.start_line && line_index < float.end_line {
                // For move-with-text floats, calculate X based on side
                let float_x = match float.side {
                    FloatSide::Left => 0.0,
                    FloatSide::Right => column_width - float.width,
                };
                return Some(FloatReduction {
                    side: float.side,
                    width: float.width,
                    float_x,
                });
            }
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
        // Tables and images use image_height (in line units) for their height
        let this_line_height = if dl.is_image || dl.is_table {
            dl.image_height.unwrap_or(1.0) * line_height
        } else {
            line_height
        };

        // Add paragraph spacing if last line, but not for zero-height image markers
        let spacing_after = if dl.is_last_line && this_line_height > 0.0 {
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

