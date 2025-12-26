//! # Editor Engine
//!
//! A high-performance document layout engine compiled to WebAssembly.
//! Handles text layout, pagination, and document model management.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────┐
//! │  Svelte UI Shell (thin wrapper)             │
//! └──────────────────────┬──────────────────────┘
//!                        │ wasm-bindgen
//! ┌──────────────────────▼──────────────────────┐
//! │  Rust/WASM Engine                           │
//! │  - Document model                           │
//! │  - Layout computation                       │
//! │  - Render commands generation               │
//! └─────────────────────────────────────────────┘
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
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}
