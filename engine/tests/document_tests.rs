//! Tests for the document module

use editor_engine::*;

#[test]
fn test_paragraph_page_break() {
    let para = Paragraph::new("\u{FFFD}".to_string());
    assert!(para.is_page_break());
}

#[test]
fn test_paragraph_not_page_break() {
    let para = Paragraph::new("regular text".to_string());
    assert!(!para.is_page_break());
}

#[test]
fn test_paragraph_image() {
    let para = Paragraph::new("\u{FFFC}image-123".to_string());
    assert!(para.is_image());
    assert_eq!(para.image_id(), Some("image-123"));
}

#[test]
fn test_paragraph_not_image() {
    let para = Paragraph::new("regular text".to_string());
    assert!(!para.is_image());
    assert_eq!(para.image_id(), None);
}

#[test]
fn test_block_type_multipliers() {
    assert_eq!(BlockType::Heading1.font_size_multiplier(), 2.0);
    assert_eq!(BlockType::Heading2.font_size_multiplier(), 1.5);
    assert_eq!(BlockType::Heading3.font_size_multiplier(), 1.17);
    assert_eq!(BlockType::Paragraph.font_size_multiplier(), 1.0);
}

#[test]
fn test_document_default() {
    let doc = Document::new();
    assert_eq!(doc.version, 1);
    assert_eq!(doc.paragraphs.len(), 1);
    assert!(doc.images.is_empty());
}

#[test]
fn test_text_style_new() {
    let style = TextStyle::new(0, 10);
    assert_eq!(style.start, 0);
    assert_eq!(style.end, 10);
    assert!(!style.bold);
    assert!(!style.italic);
    assert!(!style.has_formatting());
}

#[test]
fn test_text_style_has_formatting() {
    let mut style = TextStyle::new(0, 10);
    assert!(!style.has_formatting());

    style.bold = true;
    assert!(style.has_formatting());
}

#[test]
fn test_text_style_overlaps() {
    let style = TextStyle::new(5, 15);

    // Overlapping ranges
    assert!(style.overlaps(0, 10));
    assert!(style.overlaps(10, 20));
    assert!(style.overlaps(7, 12));

    // Non-overlapping ranges
    assert!(!style.overlaps(0, 5));
    assert!(!style.overlaps(15, 20));
}

#[test]
fn test_text_style_contains() {
    let style = TextStyle::new(5, 15);

    // Contained ranges
    assert!(style.contains(5, 15));
    assert!(style.contains(7, 12));

    // Not contained
    assert!(!style.contains(0, 10));
    assert!(!style.contains(10, 20));
}

#[test]
fn test_paragraph_apply_style() {
    let mut para = Paragraph::new("Hello World".to_string());

    para.apply_style(0, 5, |s| s.bold = true);

    assert_eq!(para.styles.len(), 1);
    assert!(para.styles[0].bold);
    assert_eq!(para.styles[0].start, 0);
    assert_eq!(para.styles[0].end, 5);
}

#[test]
fn test_paragraph_style_at() {
    let mut para = Paragraph::new("Hello World".to_string());
    para.apply_style(0, 5, |s| s.bold = true);

    assert!(para.style_at(3).is_some());
    assert!(para.style_at(3).unwrap().bold);
    assert!(para.style_at(7).is_none());
}

#[test]
fn test_image_cropped_dimensions() {
    let mut image = DocumentImage::new(
        "test".to_string(),
        "data:image/png;base64,".to_string(),
        100.0,
        200.0,
    );

    assert_eq!(image.cropped_width(), 100.0);
    assert_eq!(image.cropped_height(), 200.0);

    image.crop_left = 10.0;
    image.crop_right = 10.0;
    image.crop_top = 25.0;
    image.crop_bottom = 25.0;

    assert_eq!(image.cropped_width(), 80.0);
    assert_eq!(image.cropped_height(), 100.0);
}
