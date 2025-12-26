//! Render Command Generation
//!
//! This module generates render commands that the JavaScript frontend uses to
//! draw the document on an HTML5 Canvas. It implements a command pattern that
//! separates layout computation (Rust/WASM) from actual drawing (JavaScript).
//!
//! # Design Philosophy
//!
//! Rather than directly calling Canvas APIs (which would require expensive WASM↔JS
//! interop for every draw call), this module generates a batch of render commands
//! as JSON. The JavaScript side then executes these commands in a single efficient
//! loop.
//!
//! # Render Commands
//!
//! Available commands include:
//! - **SetFont**: Configure font family, size, bold/italic
//! - **SetFillColor/SetStrokeColor**: Set drawing colors
//! - **DrawText**: Render text at a position
//! - **DrawTextJustified**: Render justified text with word spacing
//! - **FillRect/StrokeRect**: Draw rectangles (backgrounds, borders)
//! - **FillCircle**: Draw circles (bullet points)
//! - **DrawImage**: Render an image with cropping
//! - **DrawUnderline/DrawStrikethrough**: Text decorations
//! - **DrawPageNumber**: Page number footer
//!
//! # Usage
//!
//! ```ignore
//! let commands = generate_render_commands(&display_lines, &document, &config, page_index);
//! let json = serde_json::to_string(&commands)?;
//! // Send json to JavaScript for execution
//! ```
//!
//! # Styled Text Rendering
//!
//! Text rendering handles inline styles by splitting lines into styled segments.
//! Each segment may have different bold, italic, color, or background settings,
//! and is rendered as a separate DrawText command with appropriate font settings.

use serde::{Deserialize, Serialize};

use crate::document::{BlockType, Document, DocumentTable, HorizontalAlign, ImagePositionMode, ImageWrapStyle, ListType, TextAlign, TextStyle};
use crate::layout::{DisplayLine, LayoutConfig, TableLayout};

/// A render command that can be sent to JavaScript for drawing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum RenderCommand {
    /// Set the current font
    SetFont {
        font: String,
        size: f64,
        bold: bool,
        italic: bool,
    },
    /// Set the fill color
    SetFillColor { color: String },
    /// Set the stroke color
    SetStrokeColor { color: String },
    /// Draw text at position
    DrawText { text: String, x: f64, y: f64 },
    /// Draw text with word spacing (for justified text)
    DrawTextJustified {
        words: Vec<String>,
        x: f64,
        y: f64,
        #[serde(rename = "wordSpacing")]
        word_spacing: f64,
    },
    /// Fill a rectangle
    FillRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    /// Stroke a rectangle
    StrokeRect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    /// Draw a line
    DrawLine {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        width: f64,
    },
    /// Draw a circle (for bullet points)
    FillCircle { x: f64, y: f64, radius: f64 },
    /// Draw an image
    DrawImage {
        #[serde(rename = "imageId")]
        image_id: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        #[serde(rename = "cropTop")]
        crop_top: f64,
        #[serde(rename = "cropRight")]
        crop_right: f64,
        #[serde(rename = "cropBottom")]
        crop_bottom: f64,
        #[serde(rename = "cropLeft")]
        crop_left: f64,
    },
    /// Draw the cursor
    DrawCursor { x: f64, y: f64, height: f64 },
    /// Draw selection highlight
    DrawSelection {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    },
    /// Draw page number
    DrawPageNumber { number: usize, x: f64, y: f64 },
    /// Draw underline
    DrawUnderline { x: f64, y: f64, width: f64 },
    /// Draw strikethrough
    DrawStrikethrough { x: f64, y: f64, width: f64 },
    /// Set global alpha (opacity) for behind/in-front images
    SetGlobalAlpha { alpha: f64 },
    /// Draw a table border line
    DrawTableBorder {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        width: f64,
        color: String,
    },
    /// Fill a table cell background
    FillCellBackground {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        color: String,
    },
}

/// A styled text segment for rendering
#[derive(Debug, Clone)]
struct StyledSegment {
    text: String,
    bold: bool,
    italic: bool,
    underline: bool,
    strikethrough: bool,
    color: String,
    background: Option<String>,
}

/// Get styled segments for a display line
/// Splits the line text based on overlapping styles
fn get_styled_segments(
    line_text: &str,
    line_start: usize,
    line_end: usize,
    styles: &[TextStyle],
    default_color: &str,
    _block_type: BlockType,
) -> Vec<StyledSegment> {
    if line_text.is_empty() {
        return vec![];
    }

    // Find all style boundaries within this line
    let mut boundaries: Vec<usize> = vec![line_start, line_end];
    for style in styles {
        if style.start > line_start && style.start < line_end {
            boundaries.push(style.start);
        }
        if style.end > line_start && style.end < line_end {
            boundaries.push(style.end);
        }
    }
    boundaries.sort();
    boundaries.dedup();

    // Create segments between boundaries
    let mut segments = Vec::new();
    for i in 0..boundaries.len() - 1 {
        let seg_start = boundaries[i];
        let seg_end = boundaries[i + 1];

        // Get text for this segment
        let text_start = seg_start - line_start;
        let text_end = seg_end - line_start;
        if text_start >= line_text.len() {
            continue;
        }
        let text = line_text[text_start..text_end.min(line_text.len())].to_string();
        if text.is_empty() {
            continue;
        }

        // Merge styles that apply to this segment
        let mut bold = false;
        let mut italic = false;
        let mut underline = false;
        let mut strikethrough = false;
        let mut color: Option<String> = None;
        let mut background: Option<String> = None;

        for style in styles {
            if style.start <= seg_start && style.end >= seg_end {
                // This style fully covers this segment
                if style.bold {
                    bold = true;
                }
                if style.italic {
                    italic = true;
                }
                if style.underline {
                    underline = true;
                }
                if style.strikethrough {
                    strikethrough = true;
                }
                if style.color.is_some() && color.is_none() {
                    color = style.color.clone();
                }
                if style.background.is_some() && background.is_none() {
                    background = style.background.clone();
                }
            }
        }

        segments.push(StyledSegment {
            text,
            bold,
            italic,
            underline,
            strikethrough,
            color: color.unwrap_or_else(|| default_color.to_string()),
            background,
        });
    }

    // If no segments were created (no styles), return the whole line as one segment
    if segments.is_empty() {
        segments.push(StyledSegment {
            text: line_text.to_string(),
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            color: default_color.to_string(),
            background: None,
        });
    }

    segments
}

/// Calculate image X position based on position mode, alignment and column
fn calculate_image_x(
    image: &crate::document::DocumentImage,
    column_index: usize,
    config: &LayoutConfig,
) -> f64 {
    // For fixed-position images, use the stored X coordinate directly
    if image.position_mode == ImagePositionMode::FixedPosition {
        if let Some(x) = image.x {
            return config.margin_left + x;
        }
    }

    // For move-with-text images or fallback, calculate based on alignment
    let col_offset = column_index as f64 * (config.column_width() + config.column_gap);
    let clamped_width = image.width.min(config.column_width());

    match image.horizontal_align {
        HorizontalAlign::Left => config.margin_left + col_offset,
        HorizontalAlign::Center => {
            config.margin_left + col_offset + (config.column_width() - clamped_width) / 2.0
        }
        HorizontalAlign::Right => {
            config.margin_left + col_offset + config.column_width() - clamped_width
        }
    }
}

/// Generate render commands for a specific page
/// Uses multi-pass rendering for proper layering:
/// 1. Behind images (under text, with reduced opacity)
/// 2. Float images (square, tight, through - with text wrapping)
/// 3. Text and inline images
/// 4. In-front images (over text)
pub fn generate_render_commands(
    display_lines: &[DisplayLine],
    document: &Document,
    config: &LayoutConfig,
    page_index: usize,
) -> Vec<RenderCommand> {
    let mut commands: Vec<RenderCommand> = Vec::new();

    // Get lines for this page
    let page_lines: Vec<&DisplayLine> = display_lines
        .iter()
        .filter(|dl| dl.page_index == page_index)
        .collect();

    // ===== PASS 1: Behind images (rendered first, under text) =====
    commands.push(RenderCommand::SetGlobalAlpha { alpha: 0.5 });
    for dl in &page_lines {
        if dl.is_image {
            if let Some(image_id) = &dl.image_id {
                if let Some(image) = document.images.iter().find(|img| &img.id == image_id) {
                    if image.wrap_style == ImageWrapStyle::Behind {
                        let x = calculate_image_x(image, dl.column_index, config);
                        let y = if image.position_mode == ImagePositionMode::FixedPosition {
                            // Fixed position - use absolute Y on this page
                            if image.page_index == Some(page_index) {
                                config.margin_top + image.y.unwrap_or(0.0)
                            } else {
                                continue; // Not on this page
                            }
                        } else {
                            config.margin_top + dl.y_position
                        };

                        commands.push(RenderCommand::DrawImage {
                            image_id: image_id.clone(),
                            x,
                            y,
                            width: image.width.min(config.column_width()),
                            height: image.cropped_height(),
                            crop_top: image.crop_top,
                            crop_right: image.crop_right,
                            crop_bottom: image.crop_bottom,
                            crop_left: image.crop_left,
                        });
                    }
                }
            }
        }
    }
    commands.push(RenderCommand::SetGlobalAlpha { alpha: 1.0 });

    // ===== PASS 2: Float images (square, tight, through) =====
    for dl in &page_lines {
        if dl.is_image {
            if let Some(image_id) = &dl.image_id {
                if let Some(image) = document.images.iter().find(|img| &img.id == image_id) {
                    if image.wrap_style.is_float() {
                        let x = calculate_image_x(image, dl.column_index, config);
                        let y = if image.position_mode == ImagePositionMode::FixedPosition {
                            if image.page_index == Some(page_index) {
                                config.margin_top + image.y.unwrap_or(0.0)
                            } else {
                                continue;
                            }
                        } else {
                            config.margin_top + dl.y_position
                        };

                        commands.push(RenderCommand::DrawImage {
                            image_id: image_id.clone(),
                            x,
                            y,
                            width: image.width.min(config.column_width()),
                            height: image.cropped_height(),
                            crop_top: image.crop_top,
                            crop_right: image.crop_right,
                            crop_bottom: image.crop_bottom,
                            crop_left: image.crop_left,
                        });
                    }
                }
            }
        }
    }

    // ===== PASS 3: Text and inline/top-bottom images =====
    for dl in &page_lines {
        if dl.is_page_break {
            continue;
        }

        if dl.is_image {
            if let Some(image_id) = &dl.image_id {
                if let Some(image) = document.images.iter().find(|img| &img.id == image_id) {
                    // Only render inline and top-bottom images in this pass
                    if matches!(image.wrap_style, ImageWrapStyle::Inline | ImageWrapStyle::TopBottom) {
                        let x = calculate_image_x(image, dl.column_index, config);
                        let y = config.margin_top + dl.y_position;

                        commands.push(RenderCommand::DrawImage {
                            image_id: image_id.clone(),
                            x,
                            y,
                            width: image.width.min(config.column_width()),
                            height: image.cropped_height(),
                            crop_top: image.crop_top,
                            crop_right: image.crop_right,
                            crop_bottom: image.crop_bottom,
                            crop_left: image.crop_left,
                        });
                    }
                }
            }
            continue;
        }

        // Handle tables
        if dl.is_table {
            if let Some(table_id) = &dl.table_id {
                if let Some(table) = document.tables.iter().find(|t| &t.id == table_id) {
                    if let Some(layout) = &dl.table_layout {
                        let x = config.margin_left + dl.column_index as f64 * (config.column_width() + config.column_gap);
                        let y = config.margin_top + dl.y_position;
                        render_table(table, layout, x, y, &mut commands, config);
                    }
                }
            }
            continue;
        }

        // Calculate text position
        let y = config.margin_top + dl.y_position;
        let col_offset = dl.column_index as f64 * (config.column_width() + config.column_gap);

        // Get paragraph meta
        let default_meta = crate::document::ParagraphMeta::default();
        let para_meta = document
            .paragraphs
            .get(dl.para_index)
            .map(|p| &p.meta)
            .unwrap_or(&default_meta);

        // Calculate font size
        let base_font_size = para_meta.font_size.unwrap_or(config.font_size);
        let font_size = base_font_size * dl.block_type.font_size_multiplier();

        // Set font
        commands.push(RenderCommand::SetFont {
            font: "Arial".to_string(), // TODO: make configurable
            size: font_size,
            bold: dl.block_type.is_bold(),
            italic: dl.block_type.is_italic(),
        });

        // Calculate list indent and float offset
        let list_indent = if dl.list_type != ListType::None {
            font_size * 1.5
        } else {
            0.0
        };

        // For floats, text must avoid the float's X region
        // float_x is relative to column start, we need to offset text accordingly
        let float_offset = if let Some(ref fr) = dl.float_reduction {
            // The float occupies X range [float_x, float_x + width]
            // Text must be outside this range
            // For left-side floats (or floats starting from x=0): text starts after float
            // For right-side floats: text ends before float (handled by reduced width)

            // Calculate where the float ends (relative to column start)
            let float_end_x = fr.float_x + fr.width + 10.0; // 10px gap

            // If float starts near the left edge, offset text to the right
            if fr.float_x < config.column_width() / 2.0 {
                float_end_x
            } else {
                0.0 // Float is on right side, no left offset needed
            }
        } else {
            0.0
        };

        let text_start_x = config.margin_left + col_offset + list_indent + float_offset;

        // Draw list marker
        if dl.start_offset == 0 && dl.list_type != ListType::None {
            commands.push(RenderCommand::SetFillColor {
                color: "#202124".to_string(),
            });

            match dl.list_type {
                ListType::Bullet => {
                    let bullet_x = config.margin_left + col_offset + font_size * 0.5;
                    let bullet_y = y + config.line_height_px() / 2.0;
                    commands.push(RenderCommand::FillCircle {
                        x: bullet_x,
                        y: bullet_y,
                        radius: font_size * 0.15,
                    });
                }
                ListType::Numbered => {
                    if let Some(num) = dl.list_number {
                        let marker_x = config.margin_left + col_offset + font_size * 1.2;
                        let marker_y = y + (config.line_height_px() - font_size) / 2.0;
                        commands.push(RenderCommand::DrawText {
                            text: format!("{}.", num),
                            x: marker_x,
                            y: marker_y,
                        });
                    }
                }
                ListType::None => {}
            }
        }

        // Draw blockquote indicator
        if dl.block_type == BlockType::Blockquote && dl.start_offset == 0 {
            commands.push(RenderCommand::SetFillColor {
                color: "#ccc".to_string(),
            });
            commands.push(RenderCommand::FillRect {
                x: config.margin_left + col_offset,
                y,
                width: 3.0,
                height: config.line_height_px(),
            });
        }

        // Get paragraph styles for this line
        let para_styles = document
            .paragraphs
            .get(dl.para_index)
            .map(|p| &p.styles[..])
            .unwrap_or(&[]);

        // Default text color
        let default_color = para_meta
            .text_color
            .clone()
            .unwrap_or_else(|| "#202124".to_string());

        // Draw text based on alignment
        let text_y = y + (config.line_height_px() - font_size) / 2.0;

        // Get styled segments for this line
        let segments = get_styled_segments(
            &dl.text,
            dl.start_offset,
            dl.end_offset,
            para_styles,
            &default_color,
            dl.block_type,
        );

        // Render each styled segment
        let current_x = text_start_x;
        for segment in &segments {
            // Set font for this segment
            commands.push(RenderCommand::SetFont {
                font: "Arial".to_string(),
                size: font_size,
                bold: segment.bold || dl.block_type.is_bold(),
                italic: segment.italic || dl.block_type.is_italic(),
            });

            // Draw background/highlight if present
            if let Some(ref bg_color) = segment.background {
                commands.push(RenderCommand::SetFillColor {
                    color: bg_color.clone(),
                });
                // Note: width will need to be calculated by JS, using placeholder
                commands.push(RenderCommand::FillRect {
                    x: current_x,
                    y,
                    width: 0.0, // JS will calculate based on text measurement
                    height: config.line_height_px(),
                });
            }

            // Set text color
            commands.push(RenderCommand::SetFillColor {
                color: segment.color.clone(),
            });

            // Draw text
            if para_meta.align == TextAlign::Justify && !dl.is_last_line && !dl.text.is_empty() && segments.len() == 1 {
                // Only use justified rendering for unstyled single-segment lines
                let words: Vec<String> = segment.text.split(' ').map(|s| s.to_string()).collect();
                if words.len() > 1 {
                    commands.push(RenderCommand::DrawTextJustified {
                        words,
                        x: current_x,
                        y: text_y,
                        word_spacing: 0.0,
                    });
                } else {
                    commands.push(RenderCommand::DrawText {
                        text: segment.text.clone(),
                        x: current_x,
                        y: text_y,
                    });
                }
            } else {
                commands.push(RenderCommand::DrawText {
                    text: segment.text.clone(),
                    x: current_x,
                    y: text_y,
                });
            }

            // Draw underline if needed (JS needs to measure text width)
            if segment.underline {
                commands.push(RenderCommand::SetStrokeColor {
                    color: segment.color.clone(),
                });
                commands.push(RenderCommand::DrawUnderline {
                    x: current_x,
                    y: text_y + font_size + 2.0,
                    width: 0.0, // JS will calculate
                });
            }

            // Draw strikethrough if needed
            if segment.strikethrough {
                commands.push(RenderCommand::SetStrokeColor {
                    color: segment.color.clone(),
                });
                commands.push(RenderCommand::DrawStrikethrough {
                    x: current_x,
                    y: text_y + font_size / 2.0,
                    width: 0.0, // JS will calculate
                });
            }

            // Note: current_x advancement will be handled by JS based on text measurement
            // We're emitting relative positions here
        }
    }

    // ===== PASS 4: In-front images (rendered last, over text) =====
    for dl in &page_lines {
        if dl.is_image {
            if let Some(image_id) = &dl.image_id {
                if let Some(image) = document.images.iter().find(|img| &img.id == image_id) {
                    if image.wrap_style == ImageWrapStyle::InFront {
                        let x = calculate_image_x(image, dl.column_index, config);
                        let y = if image.position_mode == ImagePositionMode::FixedPosition {
                            if image.page_index == Some(page_index) {
                                config.margin_top + image.y.unwrap_or(0.0)
                            } else {
                                continue;
                            }
                        } else {
                            config.margin_top + dl.y_position
                        };

                        commands.push(RenderCommand::DrawImage {
                            image_id: image_id.clone(),
                            x,
                            y,
                            width: image.width.min(config.column_width()),
                            height: image.cropped_height(),
                            crop_top: image.crop_top,
                            crop_right: image.crop_right,
                            crop_bottom: image.crop_bottom,
                            crop_left: image.crop_left,
                        });
                    }
                }
            }
        }
    }

    // Draw page number
    commands.push(RenderCommand::SetFillColor {
        color: "#999".to_string(),
    });
    commands.push(RenderCommand::SetFont {
        font: "Arial".to_string(),
        size: 10.0,
        bold: false,
        italic: false,
    });
    commands.push(RenderCommand::DrawPageNumber {
        number: page_index + 1,
        x: config.page_width / 2.0,
        y: config.page_height - 20.0,
    });

    commands
}

/// Render a table with borders and cell contents
fn render_table(
    table: &DocumentTable,
    layout: &TableLayout,
    x: f64,
    y: f64,
    commands: &mut Vec<RenderCommand>,
    config: &LayoutConfig,
) {
    let border = table.border_width;
    let border_color = &table.border_color;
    let line_height = config.line_height_px();
    let font_size = config.font_size;
    let cell_padding = 4.0;

    // 1. Draw cell backgrounds
    let mut current_y = y + border;
    for (row_idx, row) in table.rows.iter().enumerate() {
        let row_height = layout.row_heights.get(row_idx).copied().unwrap_or(line_height);
        let mut current_x = x + border;

        for (col_idx, cell) in row.cells.iter().enumerate() {
            let col_width = layout.column_widths.get(col_idx).copied().unwrap_or(100.0);

            // Draw cell background if set
            if let Some(ref bg) = cell.background {
                commands.push(RenderCommand::FillCellBackground {
                    x: current_x,
                    y: current_y,
                    width: col_width,
                    height: row_height,
                    color: bg.clone(),
                });
            }
            current_x += col_width + border;
        }
        current_y += row_height + border;
    }

    // 2. Draw horizontal border lines
    let mut line_y = y;
    for row_height in &layout.row_heights {
        commands.push(RenderCommand::DrawTableBorder {
            x1: x,
            y1: line_y,
            x2: x + layout.total_width,
            y2: line_y,
            width: border,
            color: border_color.clone(),
        });
        line_y += row_height + border;
    }
    // Bottom border
    commands.push(RenderCommand::DrawTableBorder {
        x1: x,
        y1: line_y,
        x2: x + layout.total_width,
        y2: line_y,
        width: border,
        color: border_color.clone(),
    });

    // 3. Draw vertical border lines
    let mut col_x = x;
    for col_width in &layout.column_widths {
        commands.push(RenderCommand::DrawTableBorder {
            x1: col_x,
            y1: y,
            x2: col_x,
            y2: y + layout.total_height,
            width: border,
            color: border_color.clone(),
        });
        col_x += col_width + border;
    }
    // Right border
    commands.push(RenderCommand::DrawTableBorder {
        x1: col_x,
        y1: y,
        x2: col_x,
        y2: y + layout.total_height,
        width: border,
        color: border_color.clone(),
    });

    // 4. Draw cell text
    commands.push(RenderCommand::SetFont {
        font: "Arial".to_string(),
        size: font_size,
        bold: false,
        italic: false,
    });
    commands.push(RenderCommand::SetFillColor {
        color: "#202124".to_string(),
    });

    current_y = y + border + cell_padding;
    for (row_idx, row_cell_lines) in layout.cell_lines.iter().enumerate() {
        let row_height = layout.row_heights.get(row_idx).copied().unwrap_or(line_height);
        let mut current_x = x + border + cell_padding;

        for (col_idx, cell_lines) in row_cell_lines.iter().enumerate() {
            let col_width = layout.column_widths.get(col_idx).copied().unwrap_or(100.0);

            // Get cell alignment
            let cell_align = table.rows.get(row_idx)
                .and_then(|r| r.cells.get(col_idx))
                .map(|c| c.align)
                .unwrap_or(TextAlign::Left);

            // Render each line of cell text
            let mut text_y = current_y;
            for line in cell_lines {
                if !line.is_empty() {
                    // Calculate x position based on alignment
                    let text_x = match cell_align {
                        TextAlign::Left => current_x,
                        TextAlign::Center => current_x + (col_width - 2.0 * cell_padding) / 2.0,
                        TextAlign::Right => current_x + col_width - 2.0 * cell_padding,
                        TextAlign::Justify => current_x,
                    };

                    commands.push(RenderCommand::DrawText {
                        text: line.clone(),
                        x: text_x,
                        y: text_y,
                    });
                }
                text_y += line_height;
            }
            current_x += col_width + border;
        }
        current_y += row_height + border;
    }
}

