# Architecture Documentation

This document describes the technical architecture of the Rich Text Editor.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                          Application Layer                          │
│  ┌─────────────┐  ┌──────────────────┐  ┌────────────────────────┐  │
│  │  App.svelte │──│  Sidebar.svelte  │  │    Toolbar.svelte      │  │
│  └──────┬──────┘  └────────┬─────────┘  └───────────┬────────────┘  │
│         │                  │                        │               │
│         │    ┌─────────────┴────────────────────────┘               │
│         │    │                                                      │
│         ▼    ▼                                                      │
│  ┌───────────────────────────────────────────────────────────────┐  │
│  │                      Editor.svelte                            │  │
│  │  ┌─────────────────────────────────────────────────────────┐  │  │
│  │  │                    Canvas Layer                         │  │  │
│  │  │   ┌──────────┐  ┌──────────┐  ┌──────────┐             │  │  │
│  │  │   │  Page 1  │  │  Page 2  │  │  Page N  │  ...        │  │  │
│  │  │   └──────────┘  └──────────┘  └──────────┘             │  │  │
│  │  └─────────────────────────────────────────────────────────┘  │  │
│  └───────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          Editor Core Layer                          │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌────────────┐  │
│  │   Layout    │  │   Canvas    │  │  Keyboard   │  │  Selection │  │
│  │   Engine    │  │  Renderer   │  │  Handler    │  │  Manager   │  │
│  └─────────────┘  └─────────────┘  └─────────────┘  └────────────┘  │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                  │
│  │    Image    │  │    Text     │  │    Text     │                  │
│  │   Manager   │  │ Operations  │  │ Measurement │                  │
│  └─────────────┘  └─────────────┘  └─────────────┘                  │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          State Layer                                │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                     Svelte Stores                           │    │
│  │  pageConfig │ zoomLevel │ fontSize │ fontFamily │ headings  │    │
│  └─────────────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    Editor State                             │    │
│  │  paragraphs │ paragraphMeta │ images │ cursor │ selection   │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘
                                  │
                                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                          Type Layer                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────────────┐  │
│  │ editor/     │  │   types.ts  │  │      stores.ts              │  │
│  │ types.ts    │  │ (page cfg)  │  │   (store definitions)       │  │
│  └─────────────┘  └─────────────┘  └─────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## Data Flow

### Document Rendering Pipeline

```
┌──────────────┐     ┌───────────────┐     ┌──────────────┐     ┌────────────┐
│  Paragraphs  │────▶│ Layout Engine │────▶│ DisplayLines │────▶│  Canvas    │
│  (source)    │     │ (wrapping)    │     │  (layout)    │     │ (render)   │
└──────────────┘     └───────────────┘     └──────────────┘     └────────────┘
       ▲                                                               │
       │                                                               │
       │              ┌───────────────┐     ┌──────────────┐           │
       └──────────────│ Text/Image   │◀────│    User      │◀──────────┘
                      │ Operations    │     │   Input      │
                      └───────────────┘     └──────────────┘
```

### State Update Flow

```
User Action
     │
     ▼
┌─────────────────┐
│ Event Handler   │
│ (keyboard/mouse)│
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌─────────────────┐
│ State Update    │────▶│ Store Update    │
│ (paragraphs,    │     │ (headings,      │
│  images, etc.)  │     │  page info)     │
└────────┬────────┘     └─────────────────┘
         │
         ▼
┌─────────────────┐
│ Recompute       │
│ DisplayLines    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ Render All      │
│ Pages           │
└─────────────────┘
```

## Module Responsibilities

### Layout Engine (`layout-engine.ts`)

The layout engine transforms the document's paragraph-based structure into display lines that can be rendered on canvas.

**Key Responsibilities:**
- Wrap paragraphs into lines that fit within column width
- Calculate line positions accounting for margins and columns
- Handle floating images and text wrapping
- Track paragraph spacing and page breaks
- Compute which lines appear on which page

**Key Functions:**
```typescript
recomputeDisplayLines(config: LayoutConfig): DisplayLine[]
updateHeadings(paragraphs: string[], meta: ParagraphMeta[]): HeadingItem[]
paraToDisplayPos(para: number, offset: number): DisplayPosition
displayToPara(line: number, col: number): TextPosition
```

### Canvas Renderer (`canvas-renderer.ts`)

Handles all drawing operations to the canvas elements.

**Key Responsibilities:**
- Render text with proper formatting (font, color, alignment)
- Draw images with cropping and positioning
- Render selection highlighting
- Draw cursor and image manipulation handles
- Handle multi-column layout rendering

**Rendering Passes (in order):**
1. Clear canvas and draw background
2. Render "behind" images
3. Render floating images
4. Render inline and top-bottom images
5. Render text with selection highlighting
6. Render "in-front" images
7. Render cursor
8. Render image selection handles
9. Render page numbers

### Keyboard Handler (`keyboard-handler.ts`)

Processes all keyboard input and shortcuts.

**Key Responsibilities:**
- Handle text input from hidden textarea
- Process keyboard shortcuts (Ctrl+B, Ctrl+I, etc.)
- Handle navigation keys (arrows, Home, End)
- Process deletion keys (Backspace, Delete)
- Handle Enter for new paragraphs
- Handle special combinations (Alt+Enter for page break)

**Shortcut Categories:**
- Formatting: Bold, Italic, Underline
- Clipboard: Copy, Cut, Paste, Select All
- History: Undo, Redo
- Navigation: Arrow keys, Home, End
- Structure: Enter, Backspace, Delete

### Selection Manager (`selection-manager.ts`)

Manages cursor position and text selection state.

**Key Responsibilities:**
- Track cursor position (paragraph, offset)
- Track selection range (start, end)
- Convert between paragraph positions and display positions
- Handle selection expansion with Shift+Arrow keys
- Calculate selected text extraction
- Handle selection deletion

**Key State:**
```typescript
interface SelectionState {
  cursorPara: number;
  cursorOffset: number;
  selectionStart: TextPosition | null;
  selectionEnd: TextPosition | null;
}
```

### Image Manager (`image-manager.ts`)

Handles all image-related operations.

**Key Responsibilities:**
- Insert images from URL, file, or paste
- Delete images
- Change wrap style and position mode
- Handle image loading and caching
- Manage image selection state
- Coordinate resize, crop, and drag operations

**Image States:**
- Not selected
- Selected (shows resize handles)
- Resizing (actively changing dimensions)
- Cropping (adjusting visible area)
- Dragging (repositioning)

### Text Operations (`text-operations.ts`)

Low-level text manipulation functions.

**Key Responsibilities:**
- Insert text at cursor position
- Delete selected text
- Get selected text content
- Insert page breaks
- Apply formatting to paragraphs
- Merge/split paragraphs

### Text Measurement (`text-measurement.ts`)

Utilities for measuring text dimensions.

**Key Responsibilities:**
- Measure text width using off-screen canvas
- Account for letter spacing
- Handle different font styles (bold, italic)
- Provide consistent font style strings

## State Management

### Global Stores (Svelte Stores)

```typescript
// Page configuration
pageConfig: Writable<PageConfig>      // Format, margins, orientation, columns
zoomLevel: Writable<number>           // 25-200

// Typography
fontSize: Writable<number>            // Base font size
fontFamily: Writable<string>          // Font family name
lineHeight: Writable<number>          // Line height multiplier
letterSpacing: Writable<number>       // Letter spacing in px
paragraphSpacing: Writable<number>    // Space after paragraph in px

// Navigation
headings: Writable<HeadingItem[]>     // Document outline
currentPage: Writable<number>         // Current page number
totalPages: Writable<number>          // Total page count

// Derived
pageDimensions: Derived<Dimensions>   // Page size in pixels
contentDimensions: Derived<Dimensions> // Content area size
columnWidth: Derived<number>          // Single column width
```

### Component State (Editor.svelte)

```typescript
// Document content
paragraphs: string[]                  // Paragraph text content
paragraphMeta: ParagraphMeta[]        // Paragraph formatting
images: DocumentImage[]               // Image data
loadedImages: Map<string, HTMLImageElement>  // Image cache

// Cursor & Selection
cursorPara: number                    // Current paragraph index
cursorOffset: number                  // Character offset in paragraph
selectionStart: TextPosition | null   // Selection start
selectionEnd: TextPosition | null     // Selection end

// Display
displayLines: DisplayLine[]           // Computed layout
activeFloats: FloatImage[]            // Active floating images

// Image manipulation
selectedImageId: string | null        // Selected image
isResizing: boolean                   // Resize mode active
isCropping: boolean                   // Crop mode active
isDragging: boolean                   // Drag mode active
```

## Coordinate Systems

### Document Coordinates

- **Paragraph-based**: `(paraIndex, charOffset)` - Position in source text
- **Display-based**: `(lineIndex, columnOffset)` - Position in rendered output

### Canvas Coordinates

- **Unscaled**: Raw coordinates in points (1pt = 1px at 100% zoom)
- **Scaled**: Coordinates after zoom transformation
- **Page-relative**: Coordinates relative to page origin
- **Content-relative**: Coordinates relative to content area (after margins)

### Conversion Functions

```typescript
// Paragraph → Display
paraToDisplayPos(para: number, offset: number): { line: number, col: number }

// Display → Paragraph
displayToPara(line: number, col: number): { para: number, offset: number }

// Screen → Document
screenToDocument(x: number, y: number, pageIndex: number): DocumentCoord

// Document → Screen
documentToScreen(coord: DocumentCoord, pageIndex: number): ScreenCoord
```

## Image Layout System

### Wrap Styles

| Style | Text Behavior | Image Layer |
|-------|--------------|-------------|
| `inline` | Flows around like character | With text |
| `square` | Wraps in rectangle | With text |
| `tight` | Wraps close to edges | With text |
| `through` | Wraps through transparent areas | With text |
| `top-bottom` | Above and below only | With text |
| `behind` | No text displacement | Behind text |
| `in-front` | No text displacement | Above text |

### Position Modes

| Mode | Behavior |
|------|----------|
| `move-with-text` | Anchored to paragraph, moves when text changes |
| `fixed-position` | Absolute position on page |

## Performance Optimizations

### Current Optimizations

1. **Canvas Rendering**: Direct pixel manipulation avoids DOM overhead
2. **Pre-computed Layout**: Display lines computed once, reused for rendering
3. **Image Caching**: Loaded images stored in Map for fast access
4. **Lazy Page Creation**: Canvas elements created on demand

### Future Optimization Opportunities

1. **Dirty Rectangles**: Only redraw changed regions
2. **Virtual Scrolling**: Only render visible pages
3. **Layout Caching**: Skip layout recomputation when only styling changes
4. **Web Workers**: Offload layout computation to background thread
5. **Debounced Rendering**: Batch rapid changes into single render

## File Dependencies

```
Editor.svelte
├── stores.ts (global state)
├── types.ts (page types)
└── editor/
    ├── index.ts (re-exports)
    ├── types.ts (editor types)
    ├── text-measurement.ts
    ├── layout-engine.ts
    ├── canvas-renderer.ts
    ├── keyboard-handler.ts
    ├── selection-manager.ts
    ├── image-manager.ts
    └── text-operations.ts

Toolbar.svelte
├── stores.ts
└── types.ts

Sidebar.svelte
└── stores.ts (headings)

App.svelte
├── Editor.svelte
├── Sidebar.svelte
└── Toolbar.svelte
```
