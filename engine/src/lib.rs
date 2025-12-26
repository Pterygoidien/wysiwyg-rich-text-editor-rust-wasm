//! # Editor Engine
//!
//! A high-performance document layout engine compiled to WebAssembly for use in
//! rich text editors. This crate handles the computationally intensive parts of
//! document editing: layout computation, pagination, and render command generation.
//!
//! ## Why Rust/WASM?
//!
//! - **Performance**: Layout computation is CPU-intensive; Rust provides near-native speed
//! - **Predictability**: No garbage collection pauses during editing
//! - **Correctness**: Rust's type system prevents many bugs at compile time
//! - **Portability**: WASM runs in any modern browser
//!
//! ## Architecture
//!
//! The engine follows a clear separation of concerns:
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     Svelte UI Shell                          │
//! │  - Event handling (keyboard, mouse)                          │
//! │  - Canvas rendering (executing render commands)              │
//! │  - UI chrome (toolbar, sidebar)                              │
//! └────────────────────────────┬────────────────────────────────┘
//!                              │ wasm-bindgen
//! ┌────────────────────────────▼────────────────────────────────┐
//! │                    Rust/WASM Engine                          │
//! │                                                              │
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
//! │  │  Document   │→ │   Layout    │→ │   Render    │          │
//! │  │   Model     │  │   Engine    │  │  Commands   │          │
//! │  └─────────────┘  └─────────────┘  └─────────────┘          │
//! │                                                              │
//! │  ┌─────────────┐                                            │
//! │  │    Text     │  (Unicode handling, word boundaries)       │
//! │  │  Utilities  │                                            │
//! │  └─────────────┘                                            │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Modules
//!
//! - [`document`]: Document model (paragraphs, formatting, images)
//! - [`layout`]: Layout computation (line wrapping, pagination)
//! - [`render`]: Render command generation for Canvas drawing
//! - [`text`]: Unicode-aware text manipulation utilities
//!
//! ## Quick Start
//!
//! ```ignore
//! use editor_engine::Engine;
//!
//! // Create engine instance
//! let mut engine = Engine::new();
//!
//! // Configure page layout
//! engine.set_page_config(816.0, 1056.0, 96.0, 96.0, 96.0, 96.0, 1, 0.0);
//!
//! // Add content
//! engine.set_paragraph(0, "Hello, World!".to_string());
//!
//! // Compute layout (requires JS measurement function)
//! engine.recompute_layout(&measure_fn);
//!
//! // Get render commands for page 0
//! let commands_json = engine.get_render_commands(0);
//! ```

mod document;
mod layout;
mod render;
mod text;

use wasm_bindgen::prelude::*;

pub use document::*;
pub use layout::*;
pub use render::*;
pub use text::*;

/// Initialize the engine (call once at startup)
#[wasm_bindgen(start)]
pub fn init() {
    // Set up better panic messages in console
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

/// Engine instance that holds the document state and provides the API
#[wasm_bindgen]
pub struct Engine {
    document: Document,
    layout_config: LayoutConfig,
    display_lines: Vec<DisplayLine>,
    dirty: bool,
}

#[wasm_bindgen]
impl Engine {
    /// Create a new engine instance
    #[wasm_bindgen(constructor)]
    pub fn new() -> Engine {
        Engine {
            document: Document::new(),
            layout_config: LayoutConfig::default(),
            display_lines: Vec::new(),
            dirty: true,
        }
    }

    /// Set page configuration
    #[wasm_bindgen]
    pub fn set_page_config(
        &mut self,
        page_width: f64,
        page_height: f64,
        margin_top: f64,
        margin_right: f64,
        margin_bottom: f64,
        margin_left: f64,
        columns: u8,
        column_gap: f64,
    ) {
        self.layout_config = LayoutConfig {
            page_width,
            page_height,
            margin_top,
            margin_right,
            margin_bottom,
            margin_left,
            columns,
            column_gap,
            // Preserve existing font settings
            font_size: self.layout_config.font_size,
            line_height: self.layout_config.line_height,
            letter_spacing: self.layout_config.letter_spacing,
            paragraph_spacing: self.layout_config.paragraph_spacing,
        };
        self.dirty = true;
    }

    /// Set font configuration
    #[wasm_bindgen]
    pub fn set_font_config(
        &mut self,
        font_size: f64,
        line_height: f64,
        letter_spacing: f64,
        paragraph_spacing: f64,
    ) {
        self.layout_config.font_size = font_size;
        self.layout_config.line_height = line_height;
        self.layout_config.letter_spacing = letter_spacing;
        self.layout_config.paragraph_spacing = paragraph_spacing;
        self.dirty = true;
    }

    /// Get the number of paragraphs
    #[wasm_bindgen]
    pub fn paragraph_count(&self) -> usize {
        self.document.paragraphs.len()
    }

    /// Get paragraph text by index
    #[wasm_bindgen]
    pub fn get_paragraph(&self, index: usize) -> Option<String> {
        self.document.paragraphs.get(index).map(|p| p.text.clone())
    }

    /// Set paragraph text
    #[wasm_bindgen]
    pub fn set_paragraph(&mut self, index: usize, text: String) {
        if let Some(para) = self.document.paragraphs.get_mut(index) {
            para.text = text;
            self.dirty = true;
        }
    }

    /// Insert a new paragraph at index
    #[wasm_bindgen]
    pub fn insert_paragraph(&mut self, index: usize, text: String) {
        let para = Paragraph::new(text);
        if index >= self.document.paragraphs.len() {
            self.document.paragraphs.push(para);
        } else {
            self.document.paragraphs.insert(index, para);
        }
        self.dirty = true;
    }

    /// Delete paragraph at index
    #[wasm_bindgen]
    pub fn delete_paragraph(&mut self, index: usize) {
        if index < self.document.paragraphs.len() {
            self.document.paragraphs.remove(index);
            self.dirty = true;
        }
    }

    /// Recompute layout if dirty, returns true if layout was recomputed
    #[wasm_bindgen]
    pub fn recompute_layout(&mut self, measure_fn: &js_sys::Function) -> bool {
        if !self.dirty {
            return false;
        }

        self.display_lines = layout::compute_layout(
            &self.document,
            &self.layout_config,
            measure_fn,
        );
        self.dirty = false;
        true
    }

    /// Get the total number of pages after layout
    #[wasm_bindgen]
    pub fn page_count(&self) -> usize {
        self.display_lines
            .iter()
            .map(|dl| dl.page_index)
            .max()
            .map(|max| max + 1)
            .unwrap_or(1)
    }

    /// Get render commands for a specific page as JSON
    #[wasm_bindgen]
    pub fn get_render_commands(&self, page_index: usize) -> String {
        let commands = render::generate_render_commands(
            &self.display_lines,
            &self.document,
            &self.layout_config,
            page_index,
        );
        serde_json::to_string(&commands).unwrap_or_else(|_| "[]".to_string())
    }

    /// Get all display lines as JSON (for debugging)
    #[wasm_bindgen]
    pub fn get_display_lines_json(&self) -> String {
        serde_json::to_string(&self.display_lines).unwrap_or_else(|_| "[]".to_string())
    }

    /// Convert paragraph position to display line position
    /// Returns JSON: { line, col, page, x, y }
    #[wasm_bindgen]
    pub fn para_to_display_pos(&self, para_index: usize, char_offset: usize) -> JsValue {
        let pos = layout::para_to_display_pos(&self.display_lines, para_index, char_offset);

        if let Some(dl) = self.display_lines.get(pos.line) {
            let result = serde_json::json!({
                "line": pos.line,
                "col": pos.col,
                "page": dl.page_index,
                "x": dl.x_position,
                "y": dl.y_position,
            });
            JsValue::from_str(&result.to_string())
        } else {
            JsValue::NULL
        }
    }

    /// Convert display line position to paragraph position
    /// Returns JSON: { para, offset }
    #[wasm_bindgen]
    pub fn display_to_para(&self, line: usize, col: usize) -> JsValue {
        let pos = layout::display_to_para(&self.display_lines, line, col);
        let result = serde_json::json!({
            "para": pos.para,
            "offset": pos.offset,
        });
        JsValue::from_str(&result.to_string())
    }

    /// Get the page index for a given paragraph and offset
    #[wasm_bindgen]
    pub fn get_page_for_position(&self, para_index: usize, char_offset: usize) -> usize {
        layout::get_page_for_position(&self.display_lines, para_index, char_offset)
    }

    /// Get total number of display lines
    #[wasm_bindgen]
    pub fn display_line_count(&self) -> usize {
        self.display_lines.len()
    }

    /// Load document from JSON
    #[wasm_bindgen]
    pub fn load_document(&mut self, json: &str) -> Result<(), JsValue> {
        match serde_json::from_str::<Document>(json) {
            Ok(doc) => {
                self.document = doc;
                self.dirty = true;
                Ok(())
            }
            Err(e) => Err(JsValue::from_str(&e.to_string())),
        }
    }

    /// Save document to JSON
    #[wasm_bindgen]
    pub fn save_document(&self) -> String {
        serde_json::to_string_pretty(&self.document).unwrap_or_else(|_| "{}".to_string())
    }

    /// Get paragraph metadata as JSON
    /// Returns: { align, blockType, listType }
    #[wasm_bindgen]
    pub fn get_paragraph_meta(&self, index: usize) -> JsValue {
        if let Some(para) = self.document.paragraphs.get(index) {
            let result = serde_json::json!({
                "align": para.meta.align,
                "blockType": para.meta.block_type,
                "listType": para.meta.list_type,
            });
            JsValue::from_str(&result.to_string())
        } else {
            JsValue::NULL
        }
    }

    /// Get the list type of a paragraph as a string
    /// Returns: "none", "bullet", or "numbered"
    #[wasm_bindgen]
    pub fn get_list_type(&self, index: usize) -> String {
        if let Some(para) = self.document.paragraphs.get(index) {
            match para.meta.list_type {
                ListType::None => "none".to_string(),
                ListType::Bullet => "bullet".to_string(),
                ListType::Numbered => "numbered".to_string(),
            }
        } else {
            "none".to_string()
        }
    }

    /// Insert a new paragraph that inherits list type from source paragraph
    #[wasm_bindgen]
    pub fn insert_paragraph_with_list(&mut self, index: usize, text: String, source_para: usize) {
        let list_type = if let Some(para) = self.document.paragraphs.get(source_para) {
            para.meta.list_type
        } else {
            ListType::None
        };

        let mut para = Paragraph::new(text);
        para.meta.list_type = list_type;

        if index >= self.document.paragraphs.len() {
            self.document.paragraphs.push(para);
        } else {
            self.document.paragraphs.insert(index, para);
        }
        self.dirty = true;
    }

    /// Set paragraph block type (p, h1, h2, h3, h4, blockquote)
    #[wasm_bindgen]
    pub fn set_block_type(&mut self, index: usize, block_type: &str) {
        if let Some(para) = self.document.paragraphs.get_mut(index) {
            para.meta.block_type = match block_type {
                "h1" => BlockType::Heading1,
                "h2" => BlockType::Heading2,
                "h3" => BlockType::Heading3,
                "h4" => BlockType::Heading4,
                "blockquote" => BlockType::Blockquote,
                _ => BlockType::Paragraph,
            };
            self.dirty = true;
        }
    }

    /// Set paragraph alignment (left, center, right, justify)
    #[wasm_bindgen]
    pub fn set_alignment(&mut self, index: usize, align: &str) {
        if let Some(para) = self.document.paragraphs.get_mut(index) {
            para.meta.align = match align {
                "center" => TextAlign::Center,
                "right" => TextAlign::Right,
                "justify" => TextAlign::Justify,
                _ => TextAlign::Left,
            };
            self.dirty = true;
        }
    }

    /// Set paragraph list type (none, bullet, numbered)
    #[wasm_bindgen]
    pub fn set_list_type(&mut self, index: usize, list_type: &str) {
        if let Some(para) = self.document.paragraphs.get_mut(index) {
            para.meta.list_type = match list_type {
                "bullet" => ListType::Bullet,
                "numbered" => ListType::Numbered,
                _ => ListType::None,
            };
            self.dirty = true;
        }
    }

    /// Toggle list type for a paragraph
    #[wasm_bindgen]
    pub fn toggle_list(&mut self, index: usize, list_type: &str) {
        if let Some(para) = self.document.paragraphs.get_mut(index) {
            let target = match list_type {
                "bullet" => ListType::Bullet,
                "numbered" => ListType::Numbered,
                _ => ListType::None,
            };

            // Toggle: if already this type, remove it; otherwise set it
            para.meta.list_type = if para.meta.list_type == target {
                ListType::None
            } else {
                target
            };
            self.dirty = true;
        }
    }

    /// Toggle bold for a range of text in a paragraph
    #[wasm_bindgen]
    pub fn toggle_bold(&mut self, para_index: usize, start: usize, end: usize) {
        if let Some(para) = self.document.paragraphs.get_mut(para_index) {
            // Check if the range is already bold
            let is_bold = para.styles_in_range(start, end)
                .iter()
                .all(|s| s.bold);

            para.apply_style(start, end, |style| {
                style.bold = !is_bold;
            });
            self.dirty = true;
        }
    }

    /// Toggle italic for a range of text in a paragraph
    #[wasm_bindgen]
    pub fn toggle_italic(&mut self, para_index: usize, start: usize, end: usize) {
        if let Some(para) = self.document.paragraphs.get_mut(para_index) {
            let is_italic = para.styles_in_range(start, end)
                .iter()
                .all(|s| s.italic);

            para.apply_style(start, end, |style| {
                style.italic = !is_italic;
            });
            self.dirty = true;
        }
    }

    /// Toggle underline for a range of text in a paragraph
    #[wasm_bindgen]
    pub fn toggle_underline(&mut self, para_index: usize, start: usize, end: usize) {
        if let Some(para) = self.document.paragraphs.get_mut(para_index) {
            let is_underline = para.styles_in_range(start, end)
                .iter()
                .all(|s| s.underline);

            para.apply_style(start, end, |style| {
                style.underline = !is_underline;
            });
            self.dirty = true;
        }
    }

    /// Toggle strikethrough for a range of text in a paragraph
    #[wasm_bindgen]
    pub fn toggle_strikethrough(&mut self, para_index: usize, start: usize, end: usize) {
        if let Some(para) = self.document.paragraphs.get_mut(para_index) {
            let is_strike = para.styles_in_range(start, end)
                .iter()
                .all(|s| s.strikethrough);

            para.apply_style(start, end, |style| {
                style.strikethrough = !is_strike;
            });
            self.dirty = true;
        }
    }

    /// Set text color for a range
    #[wasm_bindgen]
    pub fn set_text_color(&mut self, para_index: usize, start: usize, end: usize, color: &str) {
        if let Some(para) = self.document.paragraphs.get_mut(para_index) {
            let color_opt = if color.is_empty() { None } else { Some(color.to_string()) };
            para.apply_style(start, end, |style| {
                style.color = color_opt.clone();
            });
            self.dirty = true;
        }
    }

    /// Set background/highlight color for a range
    #[wasm_bindgen]
    pub fn set_highlight_color(&mut self, para_index: usize, start: usize, end: usize, color: &str) {
        if let Some(para) = self.document.paragraphs.get_mut(para_index) {
            let color_opt = if color.is_empty() { None } else { Some(color.to_string()) };
            para.apply_style(start, end, |style| {
                style.background = color_opt.clone();
            });
            self.dirty = true;
        }
    }

    /// Get styles for a paragraph as JSON array
    #[wasm_bindgen]
    pub fn get_paragraph_styles(&self, index: usize) -> JsValue {
        if let Some(para) = self.document.paragraphs.get(index) {
            let result = serde_json::to_string(&para.styles).unwrap_or_else(|_| "[]".to_string());
            JsValue::from_str(&result)
        } else {
            JsValue::from_str("[]")
        }
    }

    /// Add an image to the document
    #[wasm_bindgen]
    pub fn add_image(
        &mut self,
        id: &str,
        src: &str,
        width: f64,
        height: f64,
        natural_width: f64,
        natural_height: f64,
    ) {
        let mut image = DocumentImage::new(id.to_string(), src.to_string(), width, height);
        image.natural_width = natural_width;
        image.natural_height = natural_height;
        self.document.images.push(image);
    }

    /// Insert an image paragraph at the given index
    #[wasm_bindgen]
    pub fn insert_image_paragraph(&mut self, index: usize, image_id: &str) {
        // Create the image marker paragraph (U+FFFC + image ID)
        let text = format!("\u{FFFC}{}", image_id);
        let para = Paragraph::new(text);
        if index >= self.document.paragraphs.len() {
            self.document.paragraphs.push(para);
        } else {
            self.document.paragraphs.insert(index, para);
        }
        self.dirty = true;
    }

    /// Get image info by ID
    #[wasm_bindgen]
    pub fn get_image(&self, id: &str) -> JsValue {
        if let Some(image) = self.document.images.iter().find(|img| img.id == id) {
            let result = serde_json::to_string(image).unwrap_or_else(|_| "null".to_string());
            JsValue::from_str(&result)
        } else {
            JsValue::NULL
        }
    }

    /// Update image dimensions
    #[wasm_bindgen]
    pub fn update_image_size(&mut self, id: &str, width: f64, height: f64) {
        if let Some(image) = self.document.images.iter_mut().find(|img| img.id == id) {
            image.width = width;
            image.height = height;
            self.dirty = true;
        }
    }

    /// Delete an image from the document
    #[wasm_bindgen]
    pub fn delete_image(&mut self, id: &str) {
        self.document.images.retain(|img| img.id != id);
        // Also remove any image paragraphs referencing this image
        self.document.paragraphs.retain(|p| {
            if let Some(img_id) = p.image_id() {
                img_id != id
            } else {
                true
            }
        });
        self.dirty = true;
    }

    /// Insert a page break at the given paragraph index
    #[wasm_bindgen]
    pub fn insert_page_break(&mut self, index: usize) {
        // Create the page break marker paragraph (U+FFFD)
        let para = Paragraph::new("\u{FFFD}".to_string());
        if index >= self.document.paragraphs.len() {
            self.document.paragraphs.push(para);
        } else {
            self.document.paragraphs.insert(index, para);
        }
        self.dirty = true;
    }

    /// Set image wrap style
    #[wasm_bindgen]
    pub fn set_image_wrap_style(&mut self, id: &str, wrap_style: &str) {
        if let Some(image) = self.document.images.iter_mut().find(|img| img.id == id) {
            image.wrap_style = match wrap_style {
                "inline" => ImageWrapStyle::Inline,
                "top-bottom" => ImageWrapStyle::TopBottom,
                "square" => ImageWrapStyle::Square,
                "tight" => ImageWrapStyle::Tight,
                "through" => ImageWrapStyle::Through,
                "behind" => ImageWrapStyle::Behind,
                "in-front" => ImageWrapStyle::InFront,
                _ => ImageWrapStyle::Inline,
            };
            self.dirty = true;
        }
    }

    /// Set image to fixed position (for dragging)
    #[wasm_bindgen]
    pub fn set_image_position(&mut self, id: &str, x: f64, y: f64, page_index: usize) {
        if let Some(image) = self.document.images.iter_mut().find(|img| img.id == id) {
            image.x = Some(x);
            image.y = Some(y);
            image.page_index = Some(page_index);
            image.position_mode = ImagePositionMode::FixedPosition;
            self.dirty = true;
        }
    }

    /// Clear image position (reset to move-with-text mode)
    #[wasm_bindgen]
    pub fn clear_image_position(&mut self, id: &str) {
        if let Some(image) = self.document.images.iter_mut().find(|img| img.id == id) {
            image.x = None;
            image.y = None;
            image.page_index = None;
            image.position_mode = ImagePositionMode::MoveWithText;
            self.dirty = true;
        }
    }

    /// Set image horizontal alignment
    #[wasm_bindgen]
    pub fn set_image_horizontal_align(&mut self, id: &str, align: &str) {
        if let Some(image) = self.document.images.iter_mut().find(|img| img.id == id) {
            image.horizontal_align = match align {
                "left" => HorizontalAlign::Left,
                "center" => HorizontalAlign::Center,
                "right" => HorizontalAlign::Right,
                _ => HorizontalAlign::Left,
            };
            self.dirty = true;
        }
    }

    // =========================================================================
    // Table API
    // =========================================================================

    /// Create a new table with the specified dimensions and return its ID
    #[wasm_bindgen]
    pub fn create_table(&mut self, rows: usize, cols: usize) -> String {
        let id = format!("table_{}", self.document.tables.len());
        let table = DocumentTable::new(id.clone(), rows, cols, self.layout_config.column_width());
        self.document.tables.push(table);
        self.dirty = true;
        id
    }

    /// Insert a table paragraph at the given index
    #[wasm_bindgen]
    pub fn insert_table_paragraph(&mut self, index: usize, table_id: &str) {
        let text = format!("\u{FFFB}{}", table_id);
        let para = Paragraph::new(text);
        let idx = index.min(self.document.paragraphs.len());
        self.document.paragraphs.insert(idx, para);
        self.dirty = true;
    }

    /// Get table info by ID as JSON
    #[wasm_bindgen]
    pub fn get_table(&self, id: &str) -> JsValue {
        if let Some(table) = self.document.tables.iter().find(|t| t.id == id) {
            let result = serde_json::to_string(table).unwrap_or_else(|_| "null".to_string());
            JsValue::from_str(&result)
        } else {
            JsValue::NULL
        }
    }

    /// Get cell text
    #[wasm_bindgen]
    pub fn get_cell_text(&self, table_id: &str, row: usize, col: usize) -> Option<String> {
        self.document.tables
            .iter()
            .find(|t| t.id == table_id)
            .and_then(|t| t.get_cell(row, col))
            .map(|c| c.text.clone())
    }

    /// Set cell text
    #[wasm_bindgen]
    pub fn set_cell_text(&mut self, table_id: &str, row: usize, col: usize, text: &str) {
        if let Some(table) = self.document.tables.iter_mut().find(|t| t.id == table_id) {
            if let Some(cell) = table.get_cell_mut(row, col) {
                cell.text = text.to_string();
                self.dirty = true;
            }
        }
    }

    /// Set cell background color
    #[wasm_bindgen]
    pub fn set_cell_background(&mut self, table_id: &str, row: usize, col: usize, color: &str) {
        if let Some(table) = self.document.tables.iter_mut().find(|t| t.id == table_id) {
            if let Some(cell) = table.get_cell_mut(row, col) {
                cell.background = if color.is_empty() { None } else { Some(color.to_string()) };
                self.dirty = true;
            }
        }
    }

    /// Set cell alignment
    #[wasm_bindgen]
    pub fn set_cell_align(&mut self, table_id: &str, row: usize, col: usize, align: &str) {
        if let Some(table) = self.document.tables.iter_mut().find(|t| t.id == table_id) {
            if let Some(cell) = table.get_cell_mut(row, col) {
                cell.align = match align {
                    "center" => TextAlign::Center,
                    "right" => TextAlign::Right,
                    "justify" => TextAlign::Justify,
                    _ => TextAlign::Left,
                };
                self.dirty = true;
            }
        }
    }

    /// Add a row at the specified index
    #[wasm_bindgen]
    pub fn add_table_row(&mut self, table_id: &str, at_index: usize) {
        if let Some(table) = self.document.tables.iter_mut().find(|t| t.id == table_id) {
            table.add_row(at_index);
            self.dirty = true;
        }
    }

    /// Add a column at the specified index
    #[wasm_bindgen]
    pub fn add_table_column(&mut self, table_id: &str, at_index: usize) {
        if let Some(table) = self.document.tables.iter_mut().find(|t| t.id == table_id) {
            table.add_column(at_index);
            self.dirty = true;
        }
    }

    /// Delete a row at the specified index
    #[wasm_bindgen]
    pub fn delete_table_row(&mut self, table_id: &str, row: usize) -> bool {
        if let Some(table) = self.document.tables.iter_mut().find(|t| t.id == table_id) {
            let result = table.delete_row(row);
            if result {
                self.dirty = true;
            }
            result
        } else {
            false
        }
    }

    /// Delete a column at the specified index
    #[wasm_bindgen]
    pub fn delete_table_column(&mut self, table_id: &str, col: usize) -> bool {
        if let Some(table) = self.document.tables.iter_mut().find(|t| t.id == table_id) {
            let result = table.delete_column(col);
            if result {
                self.dirty = true;
            }
            result
        } else {
            false
        }
    }

    /// Delete entire table
    #[wasm_bindgen]
    pub fn delete_table(&mut self, id: &str) {
        // Remove table from tables list
        self.document.tables.retain(|t| t.id != id);

        // Remove any table paragraphs referencing this table
        self.document.paragraphs.retain(|p| {
            if let Some(table_id) = p.table_id() {
                table_id != id
            } else {
                true
            }
        });
        self.dirty = true;
    }

    /// Set column width
    #[wasm_bindgen]
    pub fn set_column_width(&mut self, table_id: &str, col: usize, width: f64) {
        if let Some(table) = self.document.tables.iter_mut().find(|t| t.id == table_id) {
            if col < table.column_widths.len() {
                table.column_widths[col] = width;
                self.dirty = true;
            }
        }
    }

    /// Set table border style
    #[wasm_bindgen]
    pub fn set_table_border(&mut self, table_id: &str, width: f64, color: &str) {
        if let Some(table) = self.document.tables.iter_mut().find(|t| t.id == table_id) {
            table.border_width = width;
            table.border_color = color.to_string();
            self.dirty = true;
        }
    }

    /// Get table dimensions as JSON { rows, cols }
    #[wasm_bindgen]
    pub fn get_table_dimensions(&self, table_id: &str) -> JsValue {
        if let Some(table) = self.document.tables.iter().find(|t| t.id == table_id) {
            let result = serde_json::json!({
                "rows": table.num_rows(),
                "cols": table.num_cols(),
            });
            JsValue::from_str(&result.to_string())
        } else {
            JsValue::NULL
        }
    }

    /// Get cell at click position within a table
    /// Returns JSON { row, col } or null if not found
    #[wasm_bindgen]
    pub fn get_cell_at_position(&self, table_id: &str, rel_x: f64, rel_y: f64) -> JsValue {
        if let Some(table) = self.document.tables.iter().find(|t| t.id == table_id) {
            let border = table.border_width;
            let available_width = self.layout_config.column_width();
            let line_height = self.layout_config.line_height_px();
            let cell_padding = 8.0;

            // Compute column widths (percentage mode)
            let column_widths: Vec<f64> = table.column_widths
                .iter()
                .map(|w| available_width * w / 100.0)
                .collect();

            // Compute row heights based on cell content
            let mut row_heights: Vec<f64> = Vec::new();
            for row in &table.rows {
                let mut max_lines = 1usize;
                for (col_idx, cell) in row.cells.iter().enumerate() {
                    let cell_width = column_widths.get(col_idx).copied().unwrap_or(100.0) - cell_padding;
                    // Estimate lines needed for this cell's text
                    let text_width = self.measure_text_width(&cell.text);
                    let lines = if cell_width > 0.0 && text_width > 0.0 {
                        ((text_width / cell_width).ceil() as usize).max(1)
                    } else {
                        // Count explicit newlines too
                        cell.text.matches('\n').count() + 1
                    };
                    max_lines = max_lines.max(lines);
                }
                let row_height = (max_lines as f64 * line_height + cell_padding)
                    .max(row.min_height.unwrap_or(line_height + cell_padding));
                row_heights.push(row_height);
            }

            // Find row by Y position
            let mut y = border;
            let mut found_row = None;
            for (row_idx, &row_height) in row_heights.iter().enumerate() {
                if rel_y >= y && rel_y < y + row_height {
                    found_row = Some(row_idx);
                    break;
                }
                y += row_height + border;
            }

            // Find column by X position
            let mut x = border;
            let mut found_col = None;
            for (col_idx, &col_width) in column_widths.iter().enumerate() {
                if rel_x >= x && rel_x < x + col_width {
                    found_col = Some(col_idx);
                    break;
                }
                x += col_width + border;
            }

            if let (Some(row), Some(col)) = (found_row, found_col) {
                let result = serde_json::json!({
                    "row": row,
                    "col": col,
                });
                return JsValue::from_str(&result.to_string());
            }
        }
        JsValue::NULL
    }

    /// Measure text width using layout config
    fn measure_text_width(&self, text: &str) -> f64 {
        // Simple estimation: character count * average character width
        // This is a rough approximation; actual width depends on font
        let avg_char_width = self.layout_config.font_size * 0.6;
        text.chars().count() as f64 * avg_char_width
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
