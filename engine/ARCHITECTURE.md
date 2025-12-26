# Editor Engine Architecture

This document provides a deep dive into the technical architecture of the Editor Engine.

## Table of Contents

1. [Overview](#overview)
2. [Data Flow](#data-flow)
3. [Document Model](#document-model)
4. [Layout Pipeline](#layout-pipeline)
5. [Render Command System](#render-command-system)
6. [Position Mapping](#position-mapping)
7. [Performance Considerations](#performance-considerations)
8. [Extension Points](#extension-points)

---

## Overview

The Editor Engine is a document layout engine written in Rust and compiled to WebAssembly. It follows a **separation of concerns** principle:

- **Rust/WASM**: Document model, layout computation, render command generation
- **JavaScript**: Event handling, Canvas rendering, UI chrome

This separation provides:
- **Performance**: Layout computation runs at near-native speed
- **Testability**: Core logic can be tested without a browser
- **Portability**: Same engine could be used in different frontends

## Data Flow

```
User Input (keyboard, mouse)
         │
         ▼
┌─────────────────┐
│  Svelte/JS UI   │ ─── Event Handling
└────────┬────────┘
         │ wasm-bindgen
         ▼
┌─────────────────┐
│     Engine      │ ─── Document Mutation
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│   Layout Pass   │ ─── Line Wrapping, Pagination
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Render Commands │ ─── Generate Draw Instructions
└────────┬────────┘
         │ JSON serialization
         ▼
┌─────────────────┐
│  Canvas Draw    │ ─── Execute Commands
└─────────────────┘
```

### Lifecycle of a Keystroke

1. User presses a key
2. JavaScript captures `keydown` event
3. JS calls `engine.set_paragraph()` to update text
4. Engine marks document as `dirty`
5. JS calls `engine.recompute_layout()` with measurement function
6. Engine computes new layout, clears `dirty` flag
7. JS calls `engine.get_render_commands()` for each visible page
8. JS executes render commands on Canvas

---

## Document Model

### Structure

```rust
Document
├── version: u32                    // Schema version
├── paragraphs: Vec<Paragraph>
│   ├── text: String                // Content
│   ├── meta: ParagraphMeta
│   │   ├── align: TextAlign        // left, center, right, justify
│   │   ├── block_type: BlockType   // p, h1-h4, blockquote
│   │   ├── list_type: ListType     // none, bullet, numbered
│   │   ├── font_size: Option<f64>
│   │   └── text_color: Option<String>
│   └── styles: Vec<TextStyle>      // Inline formatting ranges
│       ├── start: usize
│       ├── end: usize
│       ├── bold, italic, underline, strikethrough: bool
│       ├── color: Option<String>
│       └── background: Option<String>
└── images: Vec<DocumentImage>
    ├── id: String
    ├── src: String                 // URL or data URL
    ├── width, height: f64
    ├── natural_width, natural_height: f64
    ├── wrap_style: ImageWrapStyle
    ├── horizontal_align: HorizontalAlign
    ├── x, y: Option<f64>           // For positioned images
    ├── page_index: Option<usize>
    └── crop_*: f64                 // Crop percentages
```

### Special Markers

The document uses Unicode characters as markers:

| Character | Code Point | Purpose |
|-----------|------------|---------|
| � | U+FFFD | Page break marker |
| ￼ | U+FFFC | Image placeholder (followed by image ID) |

### Style Application

Inline styles use a range-based system with automatic merging:

```rust
// Apply bold to characters 5-10
paragraph.apply_style(5, 10, |style| style.bold = true);
```

The `apply_style` method:
1. Finds overlapping existing styles
2. Splits styles at range boundaries
3. Applies the modification
4. Merges adjacent styles with identical formatting

---

## Layout Pipeline

### Stage 1: Paragraph Layout

For each paragraph, `layout_paragraph()`:

1. **Check special markers**: Page breaks and images are handled specially
2. **Calculate available width**: Column width minus list indent minus float reduction
3. **Word wrap**: Break text into lines that fit

```rust
fn layout_paragraph(...) -> Vec<DisplayLine> {
    // Handle page breaks
    if para.is_page_break() { return page_break_line; }

    // Handle images
    if let Some(image_id) = para.image_id() { return image_line; }

    // Word wrap text
    let mut lines = Vec::new();
    let mut current_start = 0;

    while current_start < text.len() {
        // Find break point using text measurement
        let line_end = find_line_break(text, current_start, available_width);
        lines.push(create_display_line(current_start, line_end));
        current_start = line_end;
    }

    lines
}
```

### Stage 2: Page Assignment

`assign_page_positions()` distributes lines across pages:

```rust
fn assign_page_positions(display_lines: &mut [DisplayLine], config: &LayoutConfig) {
    let mut current_y = 0.0;
    let mut current_page = 0;
    let mut current_column = 0;

    for line in display_lines {
        // Handle page breaks
        if line.is_page_break {
            current_page += 1;
            current_column = 0;
            current_y = 0.0;
            continue;
        }

        // Check for column/page overflow
        if current_y + line_height > max_height {
            if current_column < columns - 1 {
                current_column += 1;
                current_y = 0.0;
            } else {
                current_page += 1;
                current_column = 0;
                current_y = 0.0;
            }
        }

        // Assign position
        line.page_index = current_page;
        line.column_index = current_column;
        line.y_position = current_y;
        line.x_position = margin_left + column_offset;

        current_y += line_height + spacing;
    }
}
```

### DisplayLine Structure

```rust
struct DisplayLine {
    // Source tracking
    para_index: usize,
    start_offset: usize,
    end_offset: usize,
    text: String,

    // Position
    page_index: usize,
    column_index: usize,
    x_position: f64,
    y_position: f64,

    // Type flags
    is_page_break: bool,
    is_image: bool,
    is_last_line: bool,

    // Metadata
    block_type: BlockType,
    list_type: ListType,
    list_number: Option<usize>,
    float_reduction: Option<FloatReduction>,
}
```

---

## Render Command System

### Design Philosophy

Direct WASM→JS Canvas calls would be slow due to:
- Function call overhead per draw operation
- Data marshaling for each call
- Browser engine context switches

Instead, we batch all drawing operations into a JSON array:

```javascript
// Instead of many calls:
ctx.font = "16px Arial";
ctx.fillText("Hello", 100, 200);
ctx.fillText("World", 100, 224);

// One JSON transfer, then tight loop:
const commands = JSON.parse(engine.get_render_commands(0));
for (const cmd of commands) {
    executeCommand(ctx, cmd);
}
```

### Command Types

```rust
enum RenderCommand {
    SetFont { font: String, size: f64, bold: bool, italic: bool },
    SetFillColor { color: String },
    SetStrokeColor { color: String },
    DrawText { text: String, x: f64, y: f64 },
    DrawTextJustified { words: Vec<String>, x: f64, y: f64, word_spacing: f64 },
    FillRect { x: f64, y: f64, width: f64, height: f64 },
    StrokeRect { x: f64, y: f64, width: f64, height: f64 },
    DrawLine { x1: f64, y1: f64, x2: f64, y2: f64, width: f64 },
    FillCircle { x: f64, y: f64, radius: f64 },
    DrawImage { image_id: String, x: f64, y: f64, width: f64, height: f64, crop_* },
    DrawCursor { x: f64, y: f64, height: f64 },
    DrawSelection { x: f64, y: f64, width: f64, height: f64 },
    DrawPageNumber { number: usize, x: f64, y: f64 },
    DrawUnderline { x: f64, y: f64, width: f64 },
    DrawStrikethrough { x: f64, y: f64, width: f64 },
}
```

### Styled Text Rendering

For inline styles, text is split into segments:

```
Text: "Hello World"
Styles: [bold 0-5]

Segments:
  [0] "Hello" (bold)
  [1] " World" (normal)

Commands:
  SetFont { bold: true }
  DrawText { text: "Hello", x: 100 }
  SetFont { bold: false }
  DrawText { text: " World", x: 150 }
```

---

## Position Mapping

### Problem

The document model uses (paragraph index, character offset), but:
- Rendering needs (x, y) coordinates
- Mouse clicks provide (x, y) coordinates
- Cursor navigation works on display lines

### Solution: Bidirectional Mapping

```
Document Position          Display Position
(para: 0, offset: 15) ←→ (line: 2, col: 5, page: 0, x: 150, y: 48)
```

### Functions

```rust
// Document → Display
fn para_to_display_pos(lines: &[DisplayLine], para: usize, offset: usize)
    -> DisplayPosition { line, col }

// Display → Document
fn display_to_para(lines: &[DisplayLine], line: usize, col: usize)
    -> ParagraphPosition { para, offset }

// Get page for position
fn get_page_for_position(lines: &[DisplayLine], para: usize, offset: usize)
    -> usize
```

### Implementation

```rust
fn para_to_display_pos(lines: &[DisplayLine], para: usize, offset: usize) -> DisplayPosition {
    for (i, line) in lines.iter().enumerate() {
        if line.para_index == para
           && offset >= line.start_offset
           && offset <= line.end_offset {
            return DisplayPosition {
                line: i,
                col: offset - line.start_offset,
            };
        }
    }
    // Fallback to end of document
    DisplayPosition { line: lines.len() - 1, col: 0 }
}
```

---

## Performance Considerations

### Dirty Flag

The `Engine` tracks a `dirty` flag:
- Set to `true` when document changes
- `recompute_layout()` only runs if dirty
- Cleared after layout computation

This prevents unnecessary layout passes when:
- Only cursor position changes
- Scrolling between pages
- Non-content-modifying operations

### Text Measurement

Text measurement is the main bottleneck:
- Each line break check requires a measurement
- Measurement requires JS callback (WASM boundary crossing)

Mitigations:
- Measure progressively (binary search would require more calls)
- Cache could be added for repeated strings
- Future: Use rustybuzz for native text shaping

### Memory

- `Vec<DisplayLine>` is regenerated on each layout pass
- Could be optimized with incremental updates
- Currently acceptable for documents < 1000 paragraphs

---

## Extension Points

### Adding a New Block Type

1. Add variant to `BlockType` enum in `document.rs`
2. Add font size multiplier in `font_size_multiplier()`
3. Handle rendering in `generate_render_commands()` in `render.rs`
4. Update JavaScript block type selector

### Adding a New Render Command

1. Add variant to `RenderCommand` enum in `render.rs`
2. Add serialization (automatic with serde)
3. Add execution in JavaScript `executeRenderCommands()`

### Adding Text Shaping

1. Integrate rustybuzz in `text.rs`
2. Load font data (ttf-parser)
3. Shape text runs before measurement
4. Use glyph positions for rendering

### Adding Undo/Redo in Rust

Currently handled in JavaScript. To move to Rust:
1. Add `UndoManager` struct
2. Capture state snapshots
3. Expose `undo()` / `redo()` via wasm-bindgen

---

## Testing Strategy

### Unit Tests

Located in `tests/` directory:
- `document_tests.rs`: Document model operations
- `layout_tests.rs`: Layout computation
- `render_tests.rs`: Render command generation
- `text_tests.rs`: Unicode text utilities

### Running Tests

```bash
# All tests
cargo test

# Specific module
cargo test --test layout_tests

# With output
cargo test -- --nocapture
```

### Integration Tests

Full integration testing requires a browser environment:
```bash
wasm-pack test --headless --chrome
```
