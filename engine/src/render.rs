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

use crate::document::{BlockType, Document, ListType, TextAlign, TextStyle};
use crate::layout::{DisplayLine, LayoutConfig};

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

/// Generate render commands for a specific page
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

    // Render each line
    for dl in page_lines {
        if dl.is_page_break {
            continue;
        }

        if dl.is_image {
            if let Some(image_id) = &dl.image_id {
                if let Some(image) = document.images.iter().find(|img| &img.id == image_id) {
                    // Calculate image position
                    let col_offset =
                        dl.column_index as f64 * (config.column_width() + config.column_gap);
                    let x = config.margin_left + col_offset;
                    let y = config.margin_top + dl.y_position;

                    commands.push(RenderCommand::DrawImage {
                        image_id: image_id.clone(),
                        x,
                        y,
                        width: image.width,
                        height: image.cropped_height(),
                        crop_top: image.crop_top,
                        crop_right: image.crop_right,
                        crop_bottom: image.crop_bottom,
                        crop_left: image.crop_left,
                    });
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
        let float_offset = dl
            .float_reduction
            .as_ref()
            .filter(|f| f.side == crate::layout::FloatSide::Left)
            .map(|f| f.width + 10.0)
            .unwrap_or(0.0);

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

