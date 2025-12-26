# Editor Engine

A high-performance document layout engine written in Rust and compiled to WebAssembly.

## Overview

This engine handles the computationally intensive parts of a rich text editor:

- **Document Model**: Structured representation of paragraphs, formatting, and images
- **Layout Computation**: Text wrapping, pagination, and multi-column layout
- **Render Commands**: Efficient command generation for Canvas rendering
- **Position Mapping**: Cursor and selection coordinate translation

## Why Rust/WASM?

| Aspect | Benefit |
|--------|---------|
| **Performance** | Near-native speed for layout computation |
| **Predictability** | No garbage collection pauses during editing |
| **Memory Safety** | Rust prevents buffer overflows and data races |
| **Portability** | WASM runs in any modern browser |

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Svelte UI Shell                          │
│  • Event handling (keyboard, mouse)                          │
│  • Canvas rendering (executing render commands)              │
│  • UI chrome (toolbar, sidebar)                              │
└────────────────────────────┬────────────────────────────────┘
                             │ wasm-bindgen
┌────────────────────────────▼────────────────────────────────┐
│                    Rust/WASM Engine                          │
│                                                              │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐          │
│  │  Document   │→ │   Layout    │→ │   Render    │          │
│  │   Model     │  │   Engine    │  │  Commands   │          │
│  └─────────────┘  └─────────────┘  └─────────────┘          │
│                                                              │
│  ┌─────────────┐                                            │
│  │    Text     │  (Unicode handling, word boundaries)       │
│  │  Utilities  │                                            │
│  └─────────────┘                                            │
└─────────────────────────────────────────────────────────────┘
```

## Project Structure

```
engine/
├── Cargo.toml              # Dependencies and build configuration
├── README.md               # This file
├── ARCHITECTURE.md         # Detailed technical documentation
├── build.sh / build.ps1    # Build scripts
├── src/
│   ├── lib.rs              # Main entry point and Engine struct
│   ├── document.rs         # Document model
│   ├── layout.rs           # Layout computation engine
│   ├── render.rs           # Render command generation
│   └── text.rs             # Unicode text utilities
└── tests/
    ├── document_tests.rs   # Document model tests
    ├── layout_tests.rs     # Layout engine tests
    ├── render_tests.rs     # Render command tests
    └── text_tests.rs       # Text utility tests
```

## Modules

### `document`
Core data structures for document representation:
- `Document`: Root container with paragraphs and images
- `Paragraph`: Text content with inline styling
- `TextStyle`: Bold, italic, colors, etc.
- `DocumentImage`: Image metadata and positioning

### `layout`
Layout computation engine:
- `LayoutConfig`: Page dimensions, margins, columns
- `DisplayLine`: Positioned line of text for rendering
- `compute_layout()`: Main layout algorithm
- Position mapping functions for cursor handling

### `render`
Render command generation:
- `RenderCommand`: Enumeration of drawing operations
- `generate_render_commands()`: Convert layout to draw calls

### `text`
Unicode text utilities:
- Character counting and substring extraction
- Word boundary detection
- Byte/character index conversion

## Prerequisites

- Rust (install via [rustup](https://rustup.rs/))
- wasm-pack (`cargo install wasm-pack`)

## Building

### Development Build

```bash
cd engine
cargo build
```

### WASM Build

```bash
# Linux/macOS
./build.sh

# Windows PowerShell
.\build.ps1

# Via npm (from project root)
npm run build:wasm
```

### Output

The build produces a WASM module in `src/lib/engine-wasm/` containing:
- `editor_engine.js` - JavaScript bindings
- `editor_engine_bg.wasm` - WebAssembly binary
- `editor_engine.d.ts` - TypeScript type definitions

## Running Tests

```bash
# Run all tests
cargo test

# Run specific test file
cargo test --test document_tests
cargo test --test layout_tests
cargo test --test render_tests
cargo test --test text_tests

# Run tests in browser
wasm-pack test --headless --chrome
```

## API Usage

### Engine Initialization

```javascript
import init, { Engine } from './engine-wasm/editor_engine.js';

await init();
const engine = new Engine();
```

### Page Configuration

```javascript
engine.set_page_config(
  816,   // page width (px)
  1056,  // page height (px)
  96,    // margin top
  96,    // margin right
  96,    // margin bottom
  96,    // margin left
  1,     // columns
  48     // column gap
);

engine.set_font_config(
  16,    // font size
  1.5,   // line height multiplier
  0,     // letter spacing
  12     // paragraph spacing
);
```

### Content Editing

```javascript
// Paragraphs
engine.insert_paragraph(0, "Hello, World!");
engine.set_paragraph(0, "Updated text");
engine.delete_paragraph(1);

// Formatting
engine.set_block_type(0, "h1");       // h1, h2, h3, h4, p, blockquote
engine.set_alignment(0, "center");     // left, center, right, justify
engine.toggle_list(0, "bullet");       // bullet, numbered, none

// Inline styles
engine.toggle_bold(0, 0, 5);
engine.toggle_italic(0, 0, 5);
engine.set_text_color(0, 0, 5, "#ff0000");
```

### Layout and Rendering

```javascript
// Text measurement callback
const measureFn = (text, fontSize) => {
  ctx.font = `${fontSize}px Arial`;
  return ctx.measureText(text).width;
};

// Recompute layout
engine.recompute_layout(measureFn);

// Get page count
const pages = engine.page_count();

// Get render commands for a page
const commandsJson = engine.get_render_commands(0);
const commands = JSON.parse(commandsJson);
```

### Render Commands

Command types:
- `setFont` - Change current font
- `setFillColor` / `setStrokeColor` - Change colors
- `drawText` - Draw text at position
- `drawTextJustified` - Draw justified text
- `fillRect` / `strokeRect` - Draw rectangles
- `fillCircle` - Draw circles (bullets)
- `drawLine` - Draw lines
- `drawImage` - Draw an image
- `drawUnderline` / `drawStrikethrough` - Text decorations
- `drawPageNumber` - Draw page footer

### Position Mapping

```javascript
// Convert document position to display position
const displayPos = engine.para_to_display_pos(paraIndex, charOffset);
// Returns: { line, col, page, x, y }

// Convert display position to document position
const docPos = engine.display_to_para(lineIndex, column);
// Returns: { para, offset }

// Get page for a position
const page = engine.get_page_for_position(paraIndex, charOffset);
```

### Persistence

```javascript
// Save document to JSON
const json = engine.save_document();

// Load document from JSON
engine.load_document(json);
```

## Key Design Decisions

### Command Pattern for Rendering

Instead of directly calling Canvas APIs from WASM (slow due to FFI overhead), we generate render commands as JSON. JavaScript executes these in a tight loop, minimizing cross-boundary calls.

### Dirty Flag for Layout

The `Engine` tracks a `dirty` flag indicating when layout needs recomputation. This avoids unnecessary layout passes when only the cursor moves.

### Text Measurement via Callback

Since WASM cannot access browser font metrics, text measurement is delegated to JavaScript via a callback function.

## Dependencies

| Crate | Purpose |
|-------|---------|
| `wasm-bindgen` | Rust ↔ JavaScript interop |
| `serde` / `serde_json` | JSON serialization |
| `js-sys` | JavaScript type bindings |
| `web-sys` | Web API bindings |
| `rustybuzz` | Text shaping (future) |
| `ttf-parser` | Font parsing (future) |

## Future Enhancements

- [ ] Text shaping with rustybuzz (HarfBuzz port)
- [ ] Font loading and metrics
- [ ] Incremental layout updates
- [ ] Web Worker support
- [ ] Selection management in Rust

## License

Part of the Medilearn project.
