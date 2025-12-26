//! Tests for the layout module

use editor_engine::*;

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

#[test]
fn test_line_height_px() {
    let config = LayoutConfig {
        font_size: 16.0,
        line_height: 1.5,
        ..Default::default()
    };
    assert_eq!(config.line_height_px(), 24.0);
}

#[test]
fn test_content_dimensions() {
    let config = LayoutConfig {
        page_width: 816.0,
        page_height: 1056.0,
        margin_top: 96.0,
        margin_right: 96.0,
        margin_bottom: 96.0,
        margin_left: 96.0,
        ..Default::default()
    };

    assert_eq!(config.content_width(), 816.0 - 96.0 - 96.0);
    assert_eq!(config.content_height(), 1056.0 - 96.0 - 96.0);
}

/// Helper to create test display lines
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

#[test]
fn test_display_position_empty_lines() {
    let lines: Vec<DisplayLine> = vec![];
    let pos = para_to_display_pos(&lines, 0, 0);
    assert_eq!(pos.line, 0);
    assert_eq!(pos.col, 0);
}
