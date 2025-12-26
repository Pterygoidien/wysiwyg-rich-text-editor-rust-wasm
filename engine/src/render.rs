//! Render command generation
//!
//! This module generates render commands that the JavaScript side
//! can use to draw the document on a canvas.

use serde::{Deserialize, Serialize};

use crate::document::{BlockType, Document, ListType, TextAlign};
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
        image_id: String,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        crop_top: f64,
        crop_right: f64,
        crop_bottom: f64,
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

        // Calculate text width for alignment
        // Note: actual width calculation would need to call back to JS
        // For now, we'll let JS handle alignment
        let text_color = para_meta
            .text_color
            .clone()
            .unwrap_or_else(|| "#202124".to_string());
        commands.push(RenderCommand::SetFillColor { color: text_color });

        // Draw text based on alignment
        let text_y = y + (config.line_height_px() - font_size) / 2.0;

        match para_meta.align {
            TextAlign::Justify if !dl.is_last_line && !dl.text.is_empty() => {
                let words: Vec<String> = dl.text.split(' ').map(|s| s.to_string()).collect();
                if words.len() > 1 {
                    // Let JS calculate word spacing based on measured text width
                    commands.push(RenderCommand::DrawTextJustified {
                        words,
                        x: text_start_x,
                        y: text_y,
                        word_spacing: 0.0, // JS will calculate
                    });
                } else {
                    commands.push(RenderCommand::DrawText {
                        text: dl.text.clone(),
                        x: text_start_x,
                        y: text_y,
                    });
                }
            }
            _ => {
                // For left, center, right alignment, JS will position based on measured width
                commands.push(RenderCommand::DrawText {
                    text: dl.text.clone(),
                    x: text_start_x,
                    y: text_y,
                });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_page_has_page_number() {
        let display_lines = vec![];
        let document = Document::new();
        let config = LayoutConfig::default();

        let commands = generate_render_commands(&display_lines, &document, &config, 0);

        // Should at least have page number command
        assert!(commands.iter().any(|c| matches!(c, RenderCommand::DrawPageNumber { .. })));
    }
}
