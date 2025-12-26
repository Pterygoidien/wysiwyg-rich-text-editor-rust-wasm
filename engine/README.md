# Editor Engine (Rust/WASM)

High-performance document layout engine compiled to WebAssembly.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  Svelte UI Shell                                            │
│  - Toolbar, dialogs, keyboard/mouse events                  │
│  - Canvas element hosting                                   │
│  - Calls into WASM module via engine-bridge.ts              │
└──────────────────────────┬──────────────────────────────────┘
                           │ wasm-bindgen
┌──────────────────────────▼──────────────────────────────────┐
│  Rust/WASM Engine                                           │
├─────────────────────────────────────────────────────────────┤
│  document.rs           │  layout.rs                         │
│  - Paragraphs          │  - Line wrapping                   │
│  - Formatting          │  - Pagination                      │
│  - Images              │  - Column layout                   │
│                        │  - Float positioning               │
├─────────────────────────────────────────────────────────────┤
│  text.rs               │  render.rs                         │
│  - Unicode handling    │  - Render command generation       │
│  - Word boundaries     │  - Command serialization           │
└─────────────────────────────────────────────────────────────┘
```

## Prerequisites

- Rust (install via [rustup](https://rustup.rs/))
- wasm-pack (`cargo install wasm-pack`)

## Building

### Linux/macOS
```bash
./build.sh
```

### Windows (PowerShell)
```powershell
.\build.ps1
```

### Via npm
```bash
npm run build:wasm
```

## Output

The build produces a WASM module in `src/lib/engine-wasm/` containing:
- `editor_engine.js` - JavaScript bindings
- `editor_engine_bg.wasm` - WebAssembly binary
- `editor_engine.d.ts` - TypeScript type definitions

## API

### Engine

The main entry point is the `Engine` struct:

```javascript
import init, { Engine } from './engine-wasm/editor_engine.js';

await init();
const engine = new Engine();

// Configure page layout
engine.set_page_config(
  816,  // page width (px)
  1056, // page height (px)
  96,   // margin top
  96,   // margin right
  96,   // margin bottom
  96,   // margin left
  1,    // columns
  48    // column gap
);

// Configure fonts
engine.set_font_config(
  16,   // font size
  1.5,  // line height multiplier
  0,    // letter spacing
  12    // paragraph spacing
);

// Edit content
engine.insert_paragraph(0, "Hello, World!");
engine.set_paragraph(0, "Updated text");

// Compute layout (requires text measurement callback)
const measureFn = (text, fontSize) => {
  ctx.font = `${fontSize}px Arial`;
  return ctx.measureText(text).width;
};
engine.recompute_layout(measureFn);

// Get render commands for a page
const commandsJson = engine.get_render_commands(0);
const commands = JSON.parse(commandsJson);

// Save/load document
const json = engine.save_document();
engine.load_document(json);
```

### Render Commands

The engine generates render commands instead of directly drawing to canvas.
This allows the JavaScript side to handle actual rendering while keeping
all layout logic in Rust.

Command types:
- `setFont` - Change current font
- `setFillColor` - Change fill color
- `drawText` - Draw text at position
- `drawTextJustified` - Draw justified text with word spacing
- `fillRect` - Fill a rectangle
- `fillCircle` - Fill a circle (for bullets)
- `drawImage` - Draw an image
- `drawCursor` - Draw the text cursor
- `drawSelection` - Draw selection highlight
- `drawPageNumber` - Draw page number

## Development

### Running tests
```bash
cargo test
```

### Running tests in browser
```bash
wasm-pack test --headless --chrome
```

## Future Enhancements

- [ ] Text shaping with rustybuzz (HarfBuzz port)
- [ ] Font loading and metrics
- [ ] Undo/redo in Rust
- [ ] Cursor and selection management
- [ ] Incremental layout updates
- [ ] Web Worker support
