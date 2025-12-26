# Rich Text Editor - Paginated (Rust/WASM)

A professional-grade, canvas-based paginated rich text editor built with **Svelte 5**, **TypeScript**, and a **Rust/WASM** layout and rendering engine. This editor provides a document editing experience similar to Google Docs or Microsoft Word, with support for multiple pages, columns, images, tables, and rich text formatting.

## Highlights

- **High-Performance Rust Engine**: Core layout and rendering logic written in Rust, compiled to WebAssembly for near-native performance
- **Canvas-Based Rendering**: Smooth, pixel-perfect rendering with proper pagination
- **Full-Featured Tables**: Create tables with row/column manipulation, cell merging/splitting, and header support
- **Advanced Image Handling**: Multiple wrap styles, cropping, resizing, and positioning

## Features

### Text Editing
- **Rich Text Formatting**: Bold, italic, underline, strikethrough
- **Text & Highlight Colors**: Full color picker support
- **Block Types**: Headings (H1-H4), paragraphs, blockquotes
- **Lists**: Bullet and numbered lists with proper indentation
- **Text Alignment**: Left, center, right, and justify

### Tables
- **Table Creation**: Insert tables with customizable rows and columns
- **Header Support**: Optional header row and header column with styling
- **Cell Operations**:
  - Add/delete rows and columns
  - Cell text editing with cursor navigation
  - Cell background colors
  - Text alignment per cell
- **Cell Merge/Split**:
  - Select multiple cells with Shift+click
  - Merge selected cells into a single cell
  - Split merged cells back into individual cells
- **Full-Width Tables**: Tables automatically span the full page/column width

### Page Layout
- **Multiple Page Formats**: A4, A5, Letter, Legal, US Textbook
- **Page Orientation**: Portrait and landscape modes
- **Configurable Margins**: Normal, narrow, and wide presets
- **Multi-Column Layout**: Single or two-column layouts with customizable gap
- **Zoom Control**: 25% to 200% zoom levels

### Typography
- **Font Selection**: 10 common font families
- **Font Sizing**: 8pt to 72pt with quick increment/decrement
- **Line Height**: Single, 1.15, 1.5, double, and custom spacing
- **Letter Spacing**: Tight to wide presets
- **Paragraph Spacing**: Configurable space after paragraphs

### Image Support
- **Multiple Wrap Styles**:
  - Inline (flows with text)
  - Square (text wraps around bounding box)
  - Tight/Through (close text wrapping)
  - Top and Bottom (text above/below only)
  - Behind/In Front of text
- **Image Manipulation**:
  - Resize with corner/edge handles
  - Crop with percentage-based controls
  - Drag to reposition
  - Horizontal alignment (left, center, right)
- **Insert Methods**: URL, file upload, drag & drop, paste

### Navigation
- **Document Outline**: Sidebar with heading hierarchy
- **Click-to-Navigate**: Jump to any heading instantly
- **Page Indicators**: Current page and total page count

## Technology Stack

| Technology | Version | Purpose |
|------------|---------|---------|
| [Rust](https://www.rust-lang.org/) | 1.70+ | Layout engine, document model |
| [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/) | 0.2+ | Rust/WASM interop |
| [Svelte](https://svelte.dev/) | 5.46+ | UI Framework with runes |
| [TypeScript](https://www.typescriptlang.org/) | 5.9+ | Type safety |
| [Vite](https://vitejs.dev/) | 7.3+ | Build tool and dev server |
| Canvas API | - | High-performance rendering |

## Getting Started

### Prerequisites

- Node.js 18+
- npm 9+
- Rust toolchain (for building the WASM engine)
- wasm-pack (`cargo install wasm-pack`)

### Installation

```bash
# Clone the repository
git clone https://github.com/Pterygoidien/wysiwyg-rich-text-editor-rust-wasm.git
cd wysiwyg-rich-text-editor-rust-wasm

# Install dependencies
npm install

# Build the WASM engine
npm run build:wasm

# Start development server
npm run dev
```

### Development Commands

| Command | Description |
|---------|-------------|
| `npm run dev` | Start development server with HMR |
| `npm run build:wasm` | Build the Rust/WASM engine |
| `npm run build` | Build everything for production |
| `npm run preview` | Preview production build locally |

## Project Structure

```
src/
├── App.svelte                 # Root application component
├── main.ts                    # Application entry point
└── lib/
    ├── EditorWasm.svelte      # Main WASM-powered editor component
    ├── Editor.svelte          # Pure JS editor component (fallback)
    ├── Toolbar.svelte         # Formatting toolbar with all controls
    ├── Sidebar.svelte         # Document outline navigation
    ├── engine-bridge.ts       # TypeScript/WASM bridge
    ├── engine-wasm/           # Generated WASM bindings
    ├── stores.ts              # Svelte stores for shared state
    └── types.ts               # Page configuration types

engine/
├── Cargo.toml                 # Rust project configuration
└── src/
    ├── lib.rs                 # WASM entry point and API
    ├── document.rs            # Document model (paragraphs, images, tables)
    ├── layout.rs              # Layout engine (line wrapping, pagination)
    └── render.rs              # Render command generation
```

## Architecture Overview

The editor uses a **hybrid architecture** with Rust handling the compute-intensive layout and document operations, while Svelte manages the UI and user interactions.

### Rendering Pipeline
```
Document Model (Rust)
        ↓
Layout Engine (Rust) → Display Lines
        ↓
Render Commands (Rust→JS via WASM)
        ↓
Canvas Renderer (TypeScript)
        ↓
Pixels on Screen
```

### WASM Engine Modules

| Module | Responsibility |
|--------|---------------|
| `document.rs` | Document model: paragraphs, styles, images, tables |
| `layout.rs` | Converts document to display lines with text wrapping |
| `render.rs` | Generates canvas drawing commands |
| `lib.rs` | WASM-bindgen API for JavaScript interop |

### Table Data Model

Tables use a special marker paragraph (`U+FFFB` + table_id) and store cell data separately:

```rust
pub struct TableCell {
    pub text: String,
    pub align: TextAlign,
    pub background: Option<String>,
    pub col_span: usize,
    pub row_span: usize,
    pub covered: bool,  // Part of a merged cell
}
```

## Usage Examples

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl/Cmd + B` | Bold |
| `Ctrl/Cmd + I` | Italic |
| `Ctrl/Cmd + U` | Underline |
| `Ctrl/Cmd + A` | Select all |
| `Ctrl/Cmd + C` | Copy |
| `Ctrl/Cmd + V` | Paste |
| `Ctrl/Cmd + Z` | Undo |
| `Ctrl/Cmd + Y` | Redo |
| `Enter` | New paragraph |
| `Alt + Enter` | Page break |
| `Tab` (in table) | Next cell |
| `Shift + Tab` (in table) | Previous cell |

### Working with Tables

1. Click the table icon in the toolbar
2. Set the number of rows and columns
3. Optionally enable header row/column
4. Click "Insert Table"

**Cell Operations:**
- Click a cell to edit its content
- Right-click for context menu (add/delete rows/columns)
- Shift+click to select multiple cells for merging
- Use Tab to navigate between cells

### Merging Cells

1. Click on a cell to select it
2. Hold Shift and click on another cell to extend selection
3. Right-click to open context menu
4. Click "Merge Cells"

### Splitting Cells

1. Click on a merged cell
2. Right-click to open context menu
3. Click "Split Cell"

## Browser Support

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

Requires WebAssembly support and modern JavaScript features.

## Performance

The Rust/WASM engine provides significant performance benefits:

- **Layout Computation**: ~10x faster than equivalent JavaScript
- **Memory Efficiency**: Rust's ownership model minimizes allocations
- **Batch Rendering**: Commands are batched and sent to JavaScript in one call
- **Incremental Updates**: Only dirty regions are re-computed

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run `cargo test` for Rust tests
5. Run `npm run build` to ensure everything compiles
6. Submit a pull request

## License

MIT License - see LICENSE file for details.

## Acknowledgments

- Inspired by Google Docs, Microsoft Word, and LibreOffice Writer
- Built with [Svelte 5](https://svelte.dev/) and [Rust](https://www.rust-lang.org/)
- Table merge/split approach inspired by ProseMirror
