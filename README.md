# Rich Text Editor - Paginated

A professional-grade, canvas-based paginated rich text editor built with **Svelte 5** and **TypeScript**. This editor provides a document editing experience similar to Google Docs or Microsoft Word, with support for multiple pages, columns, images, and rich text formatting.

## Features

### Text Editing
- **Rich Text Formatting**: Bold, italic, underline, strikethrough
- **Text & Highlight Colors**: Full color picker support
- **Block Types**: Headings (H1-H4), paragraphs, blockquotes
- **Lists**: Bullet and numbered lists with proper indentation
- **Text Alignment**: Left, center, right, and justify

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
| [Svelte](https://svelte.dev/) | 5.46+ | UI Framework with runes |
| [TypeScript](https://www.typescriptlang.org/) | 5.9+ | Type safety |
| [Vite](https://vitejs.dev/) | 7.3+ | Build tool and dev server |
| Canvas API | - | High-performance rendering |

## Getting Started

### Prerequisites

- Node.js 18+
- npm 9+

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd rich-text-editor-paginated

# Install dependencies
npm install

# Start development server
npm run dev
```

### Development Commands

| Command | Description |
|---------|-------------|
| `npm run dev` | Start development server with HMR |
| `npm run build` | Build for production |
| `npm run preview` | Preview production build locally |

## Project Structure

```
src/
├── App.svelte                 # Root application component
├── main.ts                    # Application entry point
└── lib/
    ├── Editor.svelte          # Main canvas-based editor component
    ├── Toolbar.svelte         # Formatting toolbar with all controls
    ├── Sidebar.svelte         # Document outline navigation
    ├── Page.svelte            # Page display wrapper
    ├── stores.ts              # Svelte stores for shared state
    ├── types.ts               # Page configuration types
    ├── components/            # Reusable UI components
    │   ├── ImageOptionsPopup.svelte
    │   └── ImageInsertDialog.svelte
    └── editor/                # Editor core modules
        ├── index.ts           # Module exports
        ├── types.ts           # Editor type definitions
        ├── text-measurement.ts # Text wrapping utilities
        ├── layout-engine.ts   # Display line computation
        ├── canvas-renderer.ts # Canvas rendering logic
        ├── keyboard-handler.ts # Keyboard event handling
        ├── selection-manager.ts # Selection state management
        ├── image-manager.ts   # Image operations
        ├── text-operations.ts # Text manipulation
        └── commands.ts        # Formatting command types
docs/
├── ARCHITECTURE.md            # Technical architecture
├── API.md                     # API reference
└── CONTRIBUTING.md            # Contribution guidelines
```

## Architecture Overview

The editor uses a **canvas-based rendering approach** for optimal performance with large documents. Key architectural decisions:

### Rendering Pipeline
```
Paragraphs → Layout Engine → Display Lines → Canvas Renderer → Pixels
     ↑                              ↓
     └──── User Input ←──── Event Handlers
```

### State Management
- **Global State**: Svelte stores for page config, zoom, fonts, spacing
- **Local State**: Editor component manages document content, cursor, selection
- **Derived Values**: Computed dimensions, display lines, page count

### Module Responsibilities

| Module | Responsibility |
|--------|---------------|
| `layout-engine` | Converts paragraphs to display lines with wrapping |
| `canvas-renderer` | Renders text, images, selection, cursor to canvas |
| `keyboard-handler` | Processes keyboard input and shortcuts |
| `selection-manager` | Manages cursor position and text selection |
| `image-manager` | Handles image insertion, manipulation, positioning |
| `text-operations` | Text insertion, deletion, formatting |

## Usage Examples

### Basic Text Editing

The editor supports standard keyboard shortcuts:

| Shortcut | Action |
|----------|--------|
| `Ctrl/Cmd + B` | Bold |
| `Ctrl/Cmd + I` | Italic |
| `Ctrl/Cmd + U` | Underline |
| `Ctrl/Cmd + A` | Select all |
| `Ctrl/Cmd + C` | Copy |
| `Ctrl/Cmd + X` | Cut |
| `Ctrl/Cmd + V` | Paste |
| `Ctrl/Cmd + Z` | Undo |
| `Ctrl/Cmd + Y` | Redo |
| `Enter` | New paragraph |
| `Alt + Enter` | Page break |

### Inserting Images

1. Click the image icon in the toolbar
2. Choose one of:
   - Enter a URL
   - Drag and drop an image file
   - Click to select a file
3. The image appears at the cursor position
4. Click the image to show manipulation handles
5. Right-click for wrap style options

### Page Configuration

1. Click the page icon in the toolbar
2. Select page format (A4, Letter, etc.)
3. Choose orientation (Portrait/Landscape)
4. Set margin preset (Normal/Narrow/Wide)
5. Configure column layout (1 or 2 columns)

## Type Definitions

### Core Types

```typescript
// Paragraph formatting metadata
interface ParagraphMeta {
  align: 'left' | 'center' | 'right' | 'justify';
  listType: 'none' | 'bullet' | 'numbered';
  blockType: 'p' | 'h1' | 'h2' | 'h3' | 'h4' | 'blockquote';
  indent: number;
  fontSize?: number;
  textColor?: string;
}

// Image configuration
interface DocumentImage {
  id: string;
  src: string;
  width: number;
  height: number;
  wrapStyle: ImageWrapStyle;
  positionMode: ImagePositionMode;
  horizontalAlign?: 'left' | 'center' | 'right';
  // ... crop and position properties
}

// Page configuration
interface PageConfig {
  format: PageFormat;
  margins: PageMargins;
  orientation: 'portrait' | 'landscape';
  columns: 1 | 2;
  columnGap: number;
}
```

## Browser Support

- Chrome 90+
- Firefox 88+
- Safari 14+
- Edge 90+

The editor requires modern browser features including:
- Canvas 2D API
- ES2020+ JavaScript
- CSS Flexbox and Grid

## Performance Considerations

- **Canvas Rendering**: Direct pixel manipulation avoids DOM overhead
- **Display Lines**: Pre-computed line breaks minimize layout calculations
- **Efficient Redraws**: Only affected pages are re-rendered
- **Image Caching**: Loaded images are cached for fast redraw

## Contributing

See [CONTRIBUTING.md](docs/CONTRIBUTING.md) for development setup and guidelines.

## License

MIT License - see LICENSE file for details.

## Acknowledgments

- Inspired by Google Docs and Microsoft Word
- Built with the excellent [Svelte 5](https://svelte.dev/) framework
- Canvas rendering techniques from various open-source editors
