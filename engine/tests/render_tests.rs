//! Tests for the render module

use editor_engine::*;

#[test]
fn test_empty_page_has_page_number() {
    let display_lines = vec![];
    let document = Document::new();
    let config = LayoutConfig::default();

    let commands = generate_render_commands(&display_lines, &document, &config, 0);

    // Should at least have page number command
    assert!(commands
        .iter()
        .any(|c| matches!(c, RenderCommand::DrawPageNumber { .. })));
}

#[test]
fn test_render_commands_serialization() {
    let cmd = RenderCommand::SetFillColor {
        color: "#000".to_string(),
    };

    let json = serde_json::to_string(&cmd).unwrap();
    assert!(json.contains("setFillColor"));
    assert!(json.contains("#000"));
}

#[test]
fn test_draw_text_command() {
    let cmd = RenderCommand::DrawText {
        text: "Hello".to_string(),
        x: 100.0,
        y: 200.0,
    };

    let json = serde_json::to_string(&cmd).unwrap();
    assert!(json.contains("drawText"));
    assert!(json.contains("Hello"));
}

#[test]
fn test_draw_image_command() {
    let cmd = RenderCommand::DrawImage {
        image_id: "img-123".to_string(),
        x: 50.0,
        y: 100.0,
        width: 200.0,
        height: 150.0,
        crop_top: 0.0,
        crop_right: 0.0,
        crop_bottom: 0.0,
        crop_left: 0.0,
    };

    let json = serde_json::to_string(&cmd).unwrap();
    assert!(json.contains("drawImage"));
    assert!(json.contains("img-123"));
}

#[test]
fn test_set_font_command() {
    let cmd = RenderCommand::SetFont {
        font: "Arial".to_string(),
        size: 16.0,
        bold: true,
        italic: false,
    };

    let json = serde_json::to_string(&cmd).unwrap();
    assert!(json.contains("setFont"));
    assert!(json.contains("Arial"));
    assert!(json.contains("bold"));
}
