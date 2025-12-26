# API Reference

This document provides a complete reference for all public APIs, types, and stores in the Rich Text Editor.

## Table of Contents

- [Svelte Stores](#svelte-stores)
- [Type Definitions](#type-definitions)
- [Editor Module](#editor-module)
- [Component APIs](#component-apis)
- [Utility Functions](#utility-functions)

---

## Svelte Stores

All stores are exported from `src/lib/stores.ts`.

### Page Configuration

#### `pageConfig`
```typescript
import { pageConfig } from './lib/stores';

type PageConfig = {
  format: PageFormat;
  margins: PageMargins;
  orientation: 'portrait' | 'landscape';
  columns: 1 | 2;
  columnGap: number; // in mm
};

// Usage
pageConfig.set({ ...newConfig });
pageConfig.update(config => ({ ...config, columns: 2 }));
$pageConfig // In Svelte component
```

#### `zoomLevel`
```typescript
import { zoomLevel } from './lib/stores';

// Default: 100 (representing 100%)
// Range: 25 to 200

zoomLevel.set(150); // Set to 150%
zoomLevel.update(z => Math.min(200, z + 10)); // Increase by 10%
```

### Typography Stores

#### `fontSize`
```typescript
import { fontSize } from './lib/stores';

// Default: 16 (pixels)
fontSize.set(14);
```

#### `fontFamily`
```typescript
import { fontFamily } from './lib/stores';

// Default: 'Arial'
fontFamily.set('Times New Roman');
```

#### `lineHeight`
```typescript
import { lineHeight } from './lib/stores';

// Default: 1.5 (multiplier)
lineHeight.set(2.0); // Double spacing
```

#### `letterSpacing`
```typescript
import { letterSpacing } from './lib/stores';

// Default: 0 (pixels)
letterSpacing.set(1); // 1px extra spacing
```

#### `paragraphSpacing`
```typescript
import { paragraphSpacing } from './lib/stores';

// Default: 12 (pixels after paragraph)
paragraphSpacing.set(24); // Double spacing after paragraphs
```

### Navigation Stores

#### `headings`
```typescript
import { headings, type HeadingItem } from './lib/stores';

interface HeadingItem {
  id: string;
  text: string;
  level: number; // 1-4 for h1-h4
  paraIndex: number;
}

// Read-only in most components, updated by Editor
const items = $headings;
```

#### `currentPage` / `totalPages`
```typescript
import { currentPage, totalPages } from './lib/stores';

// Current page (1-indexed)
currentPage.set(1);

// Total pages (read from Editor)
const total = $totalPages;
```

### Derived Stores

#### `pageDimensions`
```typescript
import { pageDimensions } from './lib/stores';

// Derived from pageConfig
// Returns: { width: number, height: number } in pixels
const dims = $pageDimensions;
```

#### `contentDimensions`
```typescript
import { contentDimensions } from './lib/stores';

// Page dimensions minus margins
const content = $contentDimensions;
```

#### `columnWidth`
```typescript
import { columnWidth } from './lib/stores';

// Width of single column in pixels
const width = $columnWidth;
```

### Constants

```typescript
import {
  FONT_SIZES,           // [8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 72]
  FONT_FAMILIES,        // ['Arial', 'Times New Roman', ...]
  LINE_HEIGHT_OPTIONS,  // [{ value: 1.0, label: 'Single' }, ...]
  LETTER_SPACING_OPTIONS,
  PARAGRAPH_SPACING_OPTIONS
} from './lib/stores';
```

---

## Type Definitions

### Page Types (`src/lib/types.ts`)

#### `PageFormat`
```typescript
interface PageFormat {
  name: string;    // Display name (e.g., 'A4', 'Letter')
  width: number;   // Width in mm
  height: number;  // Height in mm
}
```

#### `PageMargins`
```typescript
interface PageMargins {
  top: number;     // Top margin in mm
  right: number;   // Right margin in mm
  bottom: number;  // Bottom margin in mm
  left: number;    // Left margin in mm
}
```

#### `PageConfig`
```typescript
interface PageConfig {
  format: PageFormat;
  margins: PageMargins;
  orientation: 'portrait' | 'landscape';
  columns: 1 | 2;
  columnGap: number; // Gap between columns in mm
}
```

#### Preset Constants
```typescript
// Page formats
const PAGE_FORMATS: Record<string, PageFormat> = {
  A4: { name: 'A4', width: 210, height: 297 },
  A5: { name: 'A5', width: 148, height: 210 },
  LETTER: { name: 'Letter', width: 215.9, height: 279.4 },
  LEGAL: { name: 'Legal', width: 215.9, height: 355.6 },
  TEXTBOOK: { name: 'US Textbook', width: 152.4, height: 228.6 }
};

// Margin presets
const DEFAULT_MARGINS: PageMargins = { top: 25.4, right: 25.4, bottom: 25.4, left: 25.4 };
const NARROW_MARGINS: PageMargins = { top: 12.7, right: 12.7, bottom: 12.7, left: 12.7 };
const WIDE_MARGINS: PageMargins = { top: 25.4, right: 50.8, bottom: 25.4, left: 50.8 };
```

### Editor Types (`src/lib/editor/types.ts`)

#### Text Alignment
```typescript
type TextAlign = 'left' | 'center' | 'right' | 'justify';
```

#### List Types
```typescript
type ListType = 'none' | 'bullet' | 'numbered';
```

#### Block Types
```typescript
type BlockType = 'p' | 'h1' | 'h2' | 'h3' | 'h4' | 'blockquote';
```

#### Image Wrap Style
```typescript
type ImageWrapStyle =
  | 'inline'      // Flows with text
  | 'square'      // Text wraps in rectangle
  | 'tight'       // Text wraps close to edges
  | 'through'     // Text flows through transparent areas
  | 'top-bottom'  // Text only above/below
  | 'behind'      // Image behind text
  | 'in-front';   // Image in front of text
```

#### Image Position Mode
```typescript
type ImagePositionMode = 'move-with-text' | 'fixed-position';
```

#### ParagraphMeta
```typescript
interface ParagraphMeta {
  align: TextAlign;
  listType: ListType;
  blockType: BlockType;
  indent: number;          // Indentation level (0 = none)
  fontSize?: number;       // Custom font size in px
  textColor?: string;      // CSS color value
}
```

#### DocumentImage
```typescript
interface DocumentImage {
  id: string;                          // Unique identifier
  src: string;                         // Image source URL or data URL
  width: number;                       // Display width in px
  height: number;                      // Display height in px
  naturalWidth: number;                // Original width
  naturalHeight: number;               // Original height
  wrapStyle: ImageWrapStyle;
  positionMode: ImagePositionMode;
  horizontalAlign?: 'left' | 'center' | 'right';
  cropTop?: number;                    // Crop percentage (0-100)
  cropRight?: number;
  cropBottom?: number;
  cropLeft?: number;
  x?: number;                          // Absolute X position
  y?: number;                          // Absolute Y position
  pageIndex?: number;                  // Page where image is anchored
}
```

#### DisplayLine
```typescript
interface DisplayLine {
  paraIndex: number;        // Source paragraph index
  startOffset: number;      // Character offset start
  endOffset: number;        // Character offset end
  text: string;             // Line text content
  meta: ParagraphMeta;      // Formatting metadata
  listNumber?: number;      // List item number
  isImage?: boolean;        // Is this an image line?
  imageId?: string;         // Image ID if isImage
  imageHeight?: number;     // Image height in lines
  floatReduction?: {        // Width reduction for floats
    side: 'left' | 'right';
    width: number;
  };
  isPageBreak?: boolean;    // Is this a page break?
  isLastLineOfParagraph?: boolean;
}
```

#### TextPosition
```typescript
interface TextPosition {
  para: number;    // Paragraph index
  offset: number;  // Character offset
}
```

#### Special Markers
```typescript
const IMAGE_MARKER = '\uFFFC';      // Marks image placeholder
const PAGE_BREAK_MARKER = '\uFFFD'; // Marks page break
```

---

## Editor Module

### Text Measurement (`src/lib/editor/text-measurement.ts`)

#### `createTextMeasurer()`
Creates a text measurement context using an off-screen canvas.

```typescript
function createTextMeasurer(): {
  getFontStyle: (config: MeasurementConfig) => string;
  measureTextWidth: (text: string, config: MeasurementConfig) => number;
  canvas: HTMLCanvasElement;
};

// Usage
const measurer = createTextMeasurer();
const width = measurer.measureTextWidth('Hello World', {
  fontSize: 16,
  fontFamily: 'Arial',
  isBold: false,
  isItalic: false,
  letterSpacing: 0,
  zoomLevel: 100
});
```

#### `wrapParagraph()`
Wraps a paragraph into display lines.

```typescript
function wrapParagraph(
  paraIndex: number,
  text: string,
  meta: ParagraphMeta,
  measureFn: (text: string) => number,
  config: WrapConfig,
  listNumber?: number,
  floatReduction?: { side: 'left' | 'right'; width: number }
): DisplayLine[];
```

#### `getBlockFontSize()`
Returns the scaled font size for a block type.

```typescript
function getBlockFontSize(baseFontSize: number, blockType: string): number;

// Example
getBlockFontSize(16, 'h1'); // Returns 32 (16 * 2)
getBlockFontSize(16, 'h2'); // Returns 24 (16 * 1.5)
```

---

## Component APIs

### Editor Component

```svelte
<script lang="ts">
  import Editor from './lib/Editor.svelte';

  let editorRef: ReturnType<typeof Editor>;
</script>

<Editor bind:this={editorRef} />
```

#### Public Methods

```typescript
// Navigate to a specific paragraph
editorRef.navigateToParagraph(paraIndex: number): void;
```

### Toolbar Component

```svelte
<script lang="ts">
  import Toolbar from './lib/Toolbar.svelte';

  function handleFormat(command: string, value?: string) {
    // Handle formatting command
  }
</script>

<Toolbar onFormat={handleFormat} />
```

#### Props

| Prop | Type | Description |
|------|------|-------------|
| `onFormat` | `(command: string, value?: string) => void` | Callback for format commands |

#### Format Commands

| Command | Value | Description |
|---------|-------|-------------|
| `bold` | - | Toggle bold |
| `italic` | - | Toggle italic |
| `underline` | - | Toggle underline |
| `strikeThrough` | - | Toggle strikethrough |
| `justifyLeft` | - | Align left |
| `justifyCenter` | - | Align center |
| `justifyRight` | - | Align right |
| `justifyFull` | - | Justify |
| `insertUnorderedList` | - | Toggle bullet list |
| `insertOrderedList` | - | Toggle numbered list |
| `formatBlock` | `'p'`, `'h1'`, `'h2'`, `'h3'`, `'h4'`, `'blockquote'` | Set block type |
| `fontSize` | `string` (number) | Set font size |
| `foreColor` | `string` (hex) | Set text color |
| `hiliteColor` | `string` (hex) | Set highlight color |
| `insertImage` | - | Open image dialog |
| `undo` | - | Undo last action |
| `redo` | - | Redo last action |

### Sidebar Component

```svelte
<script lang="ts">
  import Sidebar from './lib/Sidebar.svelte';

  function handleNavigate(paraIndex: number) {
    // Navigate to paragraph
  }
</script>

<Sidebar onNavigate={handleNavigate} />
```

#### Props

| Prop | Type | Description |
|------|------|-------------|
| `onNavigate` | `(paraIndex: number) => void` | Callback when heading clicked |

---

## Utility Functions

### Page Dimension Functions (`src/lib/types.ts`)

#### `mmToPixels()`
```typescript
function mmToPixels(mm: number): number;

// Example
mmToPixels(25.4); // Returns 96 (1 inch at 96 DPI)
```

#### `getPageDimensions()`
```typescript
function getPageDimensions(config: PageConfig): { width: number; height: number };

// Returns page dimensions in pixels, accounting for orientation
```

#### `getContentDimensions()`
```typescript
function getContentDimensions(config: PageConfig): { width: number; height: number };

// Returns content area (page minus margins) in pixels
```

#### `getColumnWidth()`
```typescript
function getColumnWidth(config: PageConfig): number;

// Returns single column width in pixels, accounting for column gap
```

### Helper Functions (`src/lib/editor/types.ts`)

#### `createDefaultMeta()`
```typescript
function createDefaultMeta(): ParagraphMeta;

// Returns default paragraph metadata:
// { align: 'left', listType: 'none', blockType: 'p', indent: 0 }
```

---

## Events and Callbacks

### Keyboard Shortcuts

The editor handles these keyboard shortcuts internally:

| Shortcut | Action |
|----------|--------|
| `Ctrl/Cmd + B` | Toggle bold |
| `Ctrl/Cmd + I` | Toggle italic |
| `Ctrl/Cmd + U` | Toggle underline |
| `Ctrl/Cmd + A` | Select all |
| `Ctrl/Cmd + C` | Copy |
| `Ctrl/Cmd + X` | Cut |
| `Ctrl/Cmd + V` | Paste |
| `Ctrl/Cmd + Z` | Undo |
| `Ctrl/Cmd + Y` | Redo |
| `Enter` | New paragraph |
| `Alt + Enter` | Page break |
| `Arrow Keys` | Navigate |
| `Shift + Arrow` | Extend selection |
| `Home` | Go to line start |
| `End` | Go to line end |
| `Backspace` | Delete backward |
| `Delete` | Delete forward |

### Mouse Events

The editor handles mouse events for:
- Text cursor positioning
- Text selection (click and drag)
- Image selection
- Image resize handles
- Image crop handles
- Image dragging
