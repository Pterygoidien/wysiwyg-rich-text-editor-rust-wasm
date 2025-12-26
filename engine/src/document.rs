//! Document Model
//!
//! This module defines the core data structures that represent a document's content
//! and formatting. It provides a structured, serializable representation of rich text
//! documents with support for:
//!
//! - **Paragraphs**: Text content with inline formatting (bold, italic, colors, etc.)
//! - **Block Types**: Headings (H1-H4), blockquotes, and regular paragraphs
//! - **Lists**: Bulleted and numbered lists with proper counter management
//! - **Images**: Embedded images with positioning, sizing, and text wrapping options
//! - **Page Breaks**: Explicit page break markers for document pagination
//!
//! # Architecture
//!
//! The document model follows a hierarchical structure:
//!
//! ```text
//! Document
//! ├── paragraphs: Vec<Paragraph>
//! │   ├── text: String
//! │   ├── meta: ParagraphMeta (alignment, block type, list type)
//! │   └── styles: Vec<TextStyle> (inline formatting ranges)
//! └── images: Vec<DocumentImage>
//!     └── (id, src, dimensions, wrapping options)
//! ```
//!
//! # Special Markers
//!
//! The document uses Unicode characters as markers for special content:
//! - `U+FFFD` (Replacement Character): Page break marker
//! - `U+FFFC` (Object Replacement Character): Image placeholder, followed by image ID
//!
//! # Serialization
//!
//! All types implement `Serialize` and `Deserialize` for JSON persistence,
//! enabling document save/load functionality.

use serde::{Deserialize, Serialize};

/// The root document structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// Document version for compatibility
    pub version: u32,
    /// All paragraphs in the document
    pub paragraphs: Vec<Paragraph>,
    /// All images in the document
    pub images: Vec<DocumentImage>,
    /// All tables in the document
    #[serde(default)]
    pub tables: Vec<DocumentTable>,
}

impl Document {
    pub fn new() -> Self {
        Document {
            version: 1,
            paragraphs: vec![Paragraph::new(String::new())],
            images: Vec::new(),
            tables: Vec::new(),
        }
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

/// A single paragraph in the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paragraph {
    /// The text content
    pub text: String,
    /// Paragraph metadata/formatting
    pub meta: ParagraphMeta,
    /// Inline text styles (ranges with formatting)
    #[serde(default)]
    pub styles: Vec<TextStyle>,
}

/// Inline text style for a range of characters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextStyle {
    /// Start character index (inclusive)
    pub start: usize,
    /// End character index (exclusive)
    pub end: usize,
    /// Bold formatting
    #[serde(default)]
    pub bold: bool,
    /// Italic formatting
    #[serde(default)]
    pub italic: bool,
    /// Underline formatting
    #[serde(default)]
    pub underline: bool,
    /// Strikethrough formatting
    #[serde(default)]
    pub strikethrough: bool,
    /// Text color (CSS color string)
    #[serde(default)]
    pub color: Option<String>,
    /// Background/highlight color (CSS color string)
    #[serde(default)]
    pub background: Option<String>,
}

impl TextStyle {
    pub fn new(start: usize, end: usize) -> Self {
        TextStyle {
            start,
            end,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
            color: None,
            background: None,
        }
    }

    /// Check if this style has any formatting applied
    pub fn has_formatting(&self) -> bool {
        self.bold || self.italic || self.underline || self.strikethrough
            || self.color.is_some() || self.background.is_some()
    }

    /// Check if this style overlaps with a range
    pub fn overlaps(&self, start: usize, end: usize) -> bool {
        self.start < end && self.end > start
    }

    /// Check if this style contains a range
    pub fn contains(&self, start: usize, end: usize) -> bool {
        self.start <= start && self.end >= end
    }
}

impl Paragraph {
    pub fn new(text: String) -> Self {
        Paragraph {
            text,
            meta: ParagraphMeta::default(),
            styles: Vec::new(),
        }
    }

    pub fn with_meta(text: String, meta: ParagraphMeta) -> Self {
        Paragraph { text, meta, styles: Vec::new() }
    }

    /// Apply a style to a range of text
    /// This handles merging and splitting existing styles
    pub fn apply_style<F>(&mut self, start: usize, end: usize, modifier: F)
    where
        F: Fn(&mut TextStyle),
    {
        if start >= end {
            return;
        }

        // Collect styles that need to be modified
        let mut new_styles: Vec<TextStyle> = Vec::new();

        for style in self.styles.iter() {
            if style.overlaps(start, end) {
                // Style overlaps with the range we're modifying

                // Part before the range (unchanged)
                if style.start < start {
                    let mut before = style.clone();
                    before.end = start;
                    new_styles.push(before);
                }

                // Part after the range (unchanged)
                if style.end > end {
                    let mut after = style.clone();
                    after.start = end;
                    new_styles.push(after);
                }

                // The overlapping part (will be merged with the new style)
                let overlap_start = style.start.max(start);
                let overlap_end = style.end.min(end);
                let mut overlap = style.clone();
                overlap.start = overlap_start;
                overlap.end = overlap_end;
                modifier(&mut overlap);
                if overlap.has_formatting() {
                    new_styles.push(overlap);
                }
            } else {
                // Style doesn't overlap, keep it as is
                new_styles.push(style.clone());
            }
        }

        // Check if we need to add a new style for uncovered parts of the range
        // Find gaps in the range [start, end) that aren't covered by any style
        let mut covered: Vec<(usize, usize)> = new_styles
            .iter()
            .filter(|s| s.overlaps(start, end))
            .map(|s| (s.start.max(start), s.end.min(end)))
            .collect();
        covered.sort_by_key(|r| r.0);

        let mut pos = start;
        for (s, e) in covered {
            if pos < s {
                // Gap from pos to s
                let mut new_style = TextStyle::new(pos, s);
                modifier(&mut new_style);
                if new_style.has_formatting() {
                    new_styles.push(new_style);
                }
            }
            pos = e;
        }
        if pos < end {
            // Gap from pos to end
            let mut new_style = TextStyle::new(pos, end);
            modifier(&mut new_style);
            if new_style.has_formatting() {
                new_styles.push(new_style);
            }
        }

        // Sort by start position and merge adjacent styles with same formatting
        new_styles.sort_by_key(|s| s.start);
        self.styles = Self::merge_adjacent_styles(new_styles);
    }

    /// Merge adjacent styles that have identical formatting
    fn merge_adjacent_styles(styles: Vec<TextStyle>) -> Vec<TextStyle> {
        let mut result: Vec<TextStyle> = Vec::new();

        for style in styles {
            if let Some(last) = result.last_mut() {
                if last.end == style.start
                    && last.bold == style.bold
                    && last.italic == style.italic
                    && last.underline == style.underline
                    && last.strikethrough == style.strikethrough
                    && last.color == style.color
                    && last.background == style.background
                {
                    // Merge
                    last.end = style.end;
                    continue;
                }
            }
            result.push(style);
        }

        result
    }

    /// Get the style at a specific character position
    pub fn style_at(&self, pos: usize) -> Option<&TextStyle> {
        self.styles.iter().find(|s| s.start <= pos && s.end > pos)
    }

    /// Get all styles that overlap with a range
    pub fn styles_in_range(&self, start: usize, end: usize) -> Vec<&TextStyle> {
        self.styles.iter().filter(|s| s.overlaps(start, end)).collect()
    }

    /// Check if this paragraph is a page break marker
    /// Uses Unicode replacement character U+FFFD to match JavaScript implementation
    pub fn is_page_break(&self) -> bool {
        self.text == "\u{FFFD}"
    }

    /// Check if this paragraph is an image marker
    /// Uses Unicode object replacement character U+FFFC to match JavaScript implementation
    pub fn is_image(&self) -> bool {
        self.text.starts_with('\u{FFFC}')
    }

    /// Get image ID if this is an image paragraph
    pub fn image_id(&self) -> Option<&str> {
        if self.is_image() {
            // Skip the U+FFFC marker character (3 bytes in UTF-8)
            Some(&self.text[3..])
        } else {
            None
        }
    }

    /// Check if this paragraph is a table marker
    /// Uses Unicode annotation terminator U+FFFB as marker
    pub fn is_table(&self) -> bool {
        self.text.starts_with('\u{FFFB}')
    }

    /// Get table ID if this is a table paragraph
    pub fn table_id(&self) -> Option<&str> {
        if self.is_table() {
            // Skip the U+FFFB marker character (3 bytes in UTF-8)
            Some(&self.text[3..])
        } else {
            None
        }
    }
}

/// Paragraph formatting metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphMeta {
    /// Text alignment
    pub align: TextAlign,
    /// Block type (paragraph, heading, etc.)
    pub block_type: BlockType,
    /// List type
    pub list_type: ListType,
    /// Custom font size (if different from default)
    pub font_size: Option<f64>,
    /// Text color
    pub text_color: Option<String>,
}

impl Default for ParagraphMeta {
    fn default() -> Self {
        ParagraphMeta {
            align: TextAlign::Left,
            block_type: BlockType::Paragraph,
            list_type: ListType::None,
            font_size: None,
            text_color: None,
        }
    }
}

/// Text alignment options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

/// Block-level element types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockType {
    #[serde(rename = "p")]
    Paragraph,
    #[serde(rename = "h1")]
    Heading1,
    #[serde(rename = "h2")]
    Heading2,
    #[serde(rename = "h3")]
    Heading3,
    #[serde(rename = "h4")]
    Heading4,
    #[serde(rename = "blockquote")]
    Blockquote,
}

impl BlockType {
    /// Get the font size multiplier for this block type
    pub fn font_size_multiplier(&self) -> f64 {
        match self {
            BlockType::Heading1 => 2.0,
            BlockType::Heading2 => 1.5,
            BlockType::Heading3 => 1.17,
            BlockType::Heading4 => 1.0,
            BlockType::Paragraph => 1.0,
            BlockType::Blockquote => 1.0,
        }
    }

    /// Check if this block type should be bold
    pub fn is_bold(&self) -> bool {
        matches!(
            self,
            BlockType::Heading1
                | BlockType::Heading2
                | BlockType::Heading3
                | BlockType::Heading4
        )
    }

    /// Check if this block type should be italic
    pub fn is_italic(&self) -> bool {
        matches!(self, BlockType::Blockquote)
    }
}

/// List type options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ListType {
    None,
    Bullet,
    Numbered,
}

/// Image wrap style options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ImageWrapStyle {
    Inline,
    TopBottom,
    Square,
    Tight,
    Through,
    Behind,
    InFront,
}

impl ImageWrapStyle {
    /// Check if this wrap style causes text to flow around the image
    pub fn is_float(&self) -> bool {
        matches!(
            self,
            ImageWrapStyle::Square | ImageWrapStyle::Tight | ImageWrapStyle::Through
        )
    }
}

/// Horizontal alignment for images
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HorizontalAlign {
    Left,
    Center,
    Right,
}

/// Image position mode - how the image moves with document content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ImagePositionMode {
    /// Image moves with its anchor paragraph (default)
    #[default]
    MoveWithText,
    /// Image has fixed x,y position on a specific page
    FixedPosition,
}

/// An image in the document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentImage {
    /// Unique identifier
    pub id: String,
    /// Image source URL or data URL
    pub src: String,
    /// Display width in pixels
    pub width: f64,
    /// Display height in pixels
    pub height: f64,
    /// Original/natural width
    pub natural_width: f64,
    /// Original/natural height
    pub natural_height: f64,
    /// Wrap style
    pub wrap_style: ImageWrapStyle,
    /// Horizontal alignment
    pub horizontal_align: HorizontalAlign,
    /// Position mode (move-with-text vs fixed-position)
    #[serde(default)]
    pub position_mode: ImagePositionMode,
    /// Absolute X position (if positioned)
    pub x: Option<f64>,
    /// Absolute Y position (if positioned)
    pub y: Option<f64>,
    /// Page index (if positioned)
    pub page_index: Option<usize>,
    /// Crop percentages
    pub crop_top: f64,
    pub crop_right: f64,
    pub crop_bottom: f64,
    pub crop_left: f64,
}

impl DocumentImage {
    pub fn new(id: String, src: String, width: f64, height: f64) -> Self {
        DocumentImage {
            id,
            src,
            width,
            height,
            natural_width: width,
            natural_height: height,
            wrap_style: ImageWrapStyle::Inline,
            horizontal_align: HorizontalAlign::Left,
            position_mode: ImagePositionMode::MoveWithText,
            x: None,
            y: None,
            page_index: None,
            crop_top: 0.0,
            crop_right: 0.0,
            crop_bottom: 0.0,
            crop_left: 0.0,
        }
    }

    /// Get the effective display height after cropping
    pub fn cropped_height(&self) -> f64 {
        let crop_factor = (100.0 - self.crop_top - self.crop_bottom) / 100.0;
        self.height * crop_factor
    }

    /// Get the effective display width after cropping
    pub fn cropped_width(&self) -> f64 {
        let crop_factor = (100.0 - self.crop_left - self.crop_right) / 100.0;
        self.width * crop_factor
    }
}

// ============================================================================
// Table Support
// ============================================================================

/// Table width calculation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TableWidthMode {
    /// Fixed pixel widths for columns
    #[default]
    Fixed,
    /// Column widths as percentage of available width
    Percentage,
    /// Auto-fit columns to content
    Auto,
}

/// A single table cell
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableCell {
    /// Cell text content
    pub text: String,
    /// Text styles for inline formatting
    #[serde(default)]
    pub styles: Vec<TextStyle>,
    /// Cell text alignment
    #[serde(default)]
    pub align: TextAlign,
    /// Cell background color (optional)
    #[serde(default)]
    pub background: Option<String>,
    /// Column span (default 1)
    #[serde(default = "default_span")]
    pub col_span: usize,
    /// Row span (default 1)
    #[serde(default = "default_span")]
    pub row_span: usize,
    /// Whether this cell is covered by a merged cell (not rendered)
    #[serde(default)]
    pub covered: bool,
    /// If covered, the row of the cell that covers this one
    #[serde(default)]
    pub covered_by_row: Option<usize>,
    /// If covered, the column of the cell that covers this one
    #[serde(default)]
    pub covered_by_col: Option<usize>,
}

fn default_span() -> usize {
    1
}

impl TableCell {
    pub fn new() -> Self {
        TableCell {
            text: String::new(),
            styles: Vec::new(),
            align: TextAlign::Left,
            background: None,
            col_span: 1,
            row_span: 1,
            covered: false,
            covered_by_row: None,
            covered_by_col: None,
        }
    }

    pub fn with_text(text: String) -> Self {
        TableCell {
            text,
            styles: Vec::new(),
            align: TextAlign::Left,
            background: None,
            col_span: 1,
            row_span: 1,
            covered: false,
            covered_by_row: None,
            covered_by_col: None,
        }
    }

    /// Create a covered cell (part of a merged cell region)
    pub fn covered(covered_by_row: usize, covered_by_col: usize) -> Self {
        TableCell {
            text: String::new(),
            styles: Vec::new(),
            align: TextAlign::Left,
            background: None,
            col_span: 1,
            row_span: 1,
            covered: true,
            covered_by_row: Some(covered_by_row),
            covered_by_col: Some(covered_by_col),
        }
    }

    /// Check if this cell is the origin of a merged region
    pub fn is_merge_origin(&self) -> bool {
        !self.covered && (self.col_span > 1 || self.row_span > 1)
    }
}

impl Default for TableCell {
    fn default() -> Self {
        Self::new()
    }
}

/// A table row containing cells
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TableRow {
    /// Cells in this row
    pub cells: Vec<TableCell>,
    /// Minimum row height in pixels (optional)
    #[serde(default)]
    pub min_height: Option<f64>,
}

impl TableRow {
    pub fn new(num_cols: usize) -> Self {
        TableRow {
            cells: (0..num_cols).map(|_| TableCell::new()).collect(),
            min_height: None,
        }
    }
}

/// A table in the document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTable {
    /// Unique identifier
    pub id: String,
    /// Table rows
    pub rows: Vec<TableRow>,
    /// Column widths (pixels or percentages depending on width_mode)
    pub column_widths: Vec<f64>,
    /// Border width in pixels
    #[serde(default = "default_border_width")]
    pub border_width: f64,
    /// Border color
    #[serde(default = "default_border_color")]
    pub border_color: String,
    /// Width calculation mode
    #[serde(default)]
    pub width_mode: TableWidthMode,
}

fn default_border_width() -> f64 {
    1.0
}

fn default_border_color() -> String {
    "#000000".to_string()
}

impl DocumentTable {
    /// Create a new table with the specified dimensions
    pub fn new(id: String, num_rows: usize, num_cols: usize, _column_width: f64) -> Self {
        // Default column widths: evenly distributed percentages
        let col_width = 100.0 / num_cols as f64;
        let column_widths: Vec<f64> = (0..num_cols).map(|_| col_width).collect();

        DocumentTable {
            id,
            rows: (0..num_rows).map(|_| TableRow::new(num_cols)).collect(),
            column_widths,
            border_width: 1.0,
            border_color: "#000000".to_string(),
            width_mode: TableWidthMode::Percentage,
        }
    }

    /// Get the number of columns
    pub fn num_cols(&self) -> usize {
        self.column_widths.len()
    }

    /// Get the number of rows
    pub fn num_rows(&self) -> usize {
        self.rows.len()
    }

    /// Get a cell at the specified position
    pub fn get_cell(&self, row: usize, col: usize) -> Option<&TableCell> {
        self.rows.get(row).and_then(|r| r.cells.get(col))
    }

    /// Get a mutable cell at the specified position
    pub fn get_cell_mut(&mut self, row: usize, col: usize) -> Option<&mut TableCell> {
        self.rows.get_mut(row).and_then(|r| r.cells.get_mut(col))
    }

    /// Add a row at the specified index
    pub fn add_row(&mut self, at_index: usize) {
        let num_cols = self.num_cols();
        let index = at_index.min(self.rows.len());
        self.rows.insert(index, TableRow::new(num_cols));
    }

    /// Add a column at the specified index
    pub fn add_column(&mut self, at_index: usize) {
        let index = at_index.min(self.num_cols());

        // Add cell to each row
        for row in &mut self.rows {
            row.cells.insert(index, TableCell::new());
        }

        // Redistribute column widths
        let new_width = 100.0 / (self.num_cols() + 1) as f64;
        self.column_widths.insert(index, new_width);

        // Normalize widths to 100%
        let total: f64 = self.column_widths.iter().sum();
        for w in &mut self.column_widths {
            *w = *w / total * 100.0;
        }
    }

    /// Delete a row at the specified index
    pub fn delete_row(&mut self, row: usize) -> bool {
        if row < self.rows.len() && self.rows.len() > 1 {
            self.rows.remove(row);
            true
        } else {
            false
        }
    }

    /// Delete a column at the specified index
    pub fn delete_column(&mut self, col: usize) -> bool {
        if col < self.num_cols() && self.num_cols() > 1 {
            for row in &mut self.rows {
                if col < row.cells.len() {
                    row.cells.remove(col);
                }
            }
            self.column_widths.remove(col);

            // Normalize widths to 100%
            let total: f64 = self.column_widths.iter().sum();
            for w in &mut self.column_widths {
                *w = *w / total * 100.0;
            }
            true
        } else {
            false
        }
    }

    /// Merge cells in a rectangular region
    /// Returns true if merge was successful
    pub fn merge_cells(
        &mut self,
        start_row: usize,
        start_col: usize,
        end_row: usize,
        end_col: usize,
    ) -> bool {
        // Validate bounds
        if end_row >= self.num_rows() || end_col >= self.num_cols() {
            return false;
        }
        if start_row > end_row || start_col > end_col {
            return false;
        }

        // Check if this is a valid merge (no cells already covered by other merges)
        for row_idx in start_row..=end_row {
            for col_idx in start_col..=end_col {
                if let Some(cell) = self.get_cell(row_idx, col_idx) {
                    // If cell is covered by a merge outside our range, invalid
                    if cell.covered {
                        if let (Some(by_row), Some(by_col)) = (cell.covered_by_row, cell.covered_by_col) {
                            if by_row < start_row || by_col < start_col {
                                return false;
                            }
                        }
                    }
                    // If cell is a merge origin that extends outside our range, invalid
                    if cell.is_merge_origin() {
                        let cell_end_row = row_idx + cell.row_span - 1;
                        let cell_end_col = col_idx + cell.col_span - 1;
                        if cell_end_row > end_row || cell_end_col > end_col {
                            return false;
                        }
                    }
                }
            }
        }

        // Collect text from all cells being merged (only non-covered cells)
        let mut combined_text = String::new();
        for row_idx in start_row..=end_row {
            for col_idx in start_col..=end_col {
                if let Some(cell) = self.get_cell(row_idx, col_idx) {
                    if !cell.covered && !cell.text.is_empty() {
                        if !combined_text.is_empty() {
                            combined_text.push('\n');
                        }
                        combined_text.push_str(&cell.text);
                    }
                }
            }
        }

        // Calculate span sizes
        let row_span = end_row - start_row + 1;
        let col_span = end_col - start_col + 1;

        // Update the origin cell (top-left)
        if let Some(origin) = self.get_cell_mut(start_row, start_col) {
            origin.text = combined_text;
            origin.row_span = row_span;
            origin.col_span = col_span;
            origin.covered = false;
            origin.covered_by_row = None;
            origin.covered_by_col = None;
        }

        // Mark all other cells in the region as covered
        for row_idx in start_row..=end_row {
            for col_idx in start_col..=end_col {
                if row_idx == start_row && col_idx == start_col {
                    continue; // Skip origin cell
                }
                if let Some(cell) = self.get_cell_mut(row_idx, col_idx) {
                    cell.text.clear();
                    cell.covered = true;
                    cell.covered_by_row = Some(start_row);
                    cell.covered_by_col = Some(start_col);
                    cell.row_span = 1;
                    cell.col_span = 1;
                }
            }
        }

        true
    }

    /// Split a merged cell back into individual cells
    /// Returns true if split was successful
    pub fn split_cell(&mut self, row: usize, col: usize) -> bool {
        // Get the origin cell
        let (row_span, col_span, background) = {
            if let Some(cell) = self.get_cell(row, col) {
                if !cell.is_merge_origin() {
                    return false; // Not a merged cell
                }
                (cell.row_span, cell.col_span, cell.background.clone())
            } else {
                return false;
            }
        };

        // Reset the origin cell
        if let Some(origin) = self.get_cell_mut(row, col) {
            origin.row_span = 1;
            origin.col_span = 1;
        }

        // Uncover all cells in the region
        for row_idx in row..(row + row_span) {
            for col_idx in col..(col + col_span) {
                if row_idx == row && col_idx == col {
                    continue; // Skip origin
                }
                if let Some(cell) = self.get_cell_mut(row_idx, col_idx) {
                    cell.covered = false;
                    cell.covered_by_row = None;
                    cell.covered_by_col = None;
                    // Optionally preserve background from the merged cell
                    cell.background = background.clone();
                }
            }
        }

        true
    }

    /// Get the actual cell that should be rendered at a position
    /// (follows covered_by references to find the origin cell)
    pub fn get_visible_cell(&self, row: usize, col: usize) -> Option<(usize, usize, &TableCell)> {
        if let Some(cell) = self.get_cell(row, col) {
            if cell.covered {
                if let (Some(origin_row), Some(origin_col)) = (cell.covered_by_row, cell.covered_by_col) {
                    if let Some(origin) = self.get_cell(origin_row, origin_col) {
                        return Some((origin_row, origin_col, origin));
                    }
                }
                None
            } else {
                Some((row, col, cell))
            }
        } else {
            None
        }
    }

    /// Check if a cell position should render content (is origin or not covered)
    pub fn should_render_cell(&self, row: usize, col: usize) -> bool {
        if let Some(cell) = self.get_cell(row, col) {
            !cell.covered
        } else {
            false
        }
    }
}

