/**
 * @fileoverview Canvas rendering utilities for the rich text editor.
 *
 * This module provides utility functions for canvas-based rendering operations
 * including text drawing, selection highlighting, cursor rendering, and
 * image display. These utilities are designed to be used by the main
 * Editor component's rendering pipeline.
 *
 * @module editor/canvas-renderer
 */

import type { DocumentImage, DisplayLine, ParagraphMeta, TextAlign } from './types';

// ============================================================================
// Types
// ============================================================================

/**
 * Configuration for text rendering.
 */
export interface TextRenderConfig {
  /** Base font size in pixels */
  fontSize: number;
  /** Font family name */
  fontFamily: string;
  /** Line height in pixels */
  lineHeight: number;
  /** Letter spacing in pixels */
  letterSpacing: number;
  /** Whether bold is enabled */
  isBold: boolean;
  /** Whether italic is enabled */
  isItalic: boolean;
  /** Whether underline is enabled */
  isUnderline: boolean;
  /** Whether strikethrough is enabled */
  isStrikethrough: boolean;
}

/**
 * Page rendering configuration.
 */
export interface PageRenderConfig {
  /** Page width in pixels */
  pageWidth: number;
  /** Page height in pixels */
  pageHeight: number;
  /** Content area width in pixels */
  contentWidth: number;
  /** Content area height in pixels */
  contentHeight: number;
  /** Top margin in pixels */
  marginTop: number;
  /** Left margin in pixels */
  marginLeft: number;
  /** Number of columns */
  columnCount: number;
  /** Column width in pixels */
  columnWidth: number;
  /** Gap between columns in pixels */
  columnGap: number;
  /** Lines per column */
  linesPerColumn: number;
  /** Lines per page (total across columns) */
  linesPerPage: number;
  /** Current zoom level (percentage) */
  zoomLevel: number;
}

/**
 * Selection range for highlighting.
 */
export interface SelectionRange {
  /** Start line index */
  startLine: number;
  /** Start column (character offset) */
  startCol: number;
  /** End line index */
  endLine: number;
  /** End column (character offset) */
  endCol: number;
}

/**
 * Cursor position for rendering.
 */
export interface CursorPosition {
  /** Display line index */
  line: number;
  /** Column (character offset within line) */
  col: number;
}

/**
 * Image crop source rectangle.
 */
export interface CropRect {
  /** Source X */
  x: number;
  /** Source Y */
  y: number;
  /** Source width */
  width: number;
  /** Source height */
  height: number;
}

// ============================================================================
// Canvas Setup
// ============================================================================

/**
 * Sets up a canvas for high-DPI rendering.
 *
 * @param canvas - Canvas element to set up
 * @param width - Desired CSS width
 * @param height - Desired CSS height
 * @returns The 2D rendering context
 *
 * @example
 * ```typescript
 * const ctx = setupCanvas(canvas, 800, 600);
 * // Canvas is now ready for sharp rendering on retina displays
 * ```
 */
export function setupCanvas(
  canvas: HTMLCanvasElement,
  width: number,
  height: number
): CanvasRenderingContext2D | null {
  const ctx = canvas.getContext('2d');
  if (!ctx) return null;

  const dpr = window.devicePixelRatio || 1;
  canvas.width = width * dpr;
  canvas.height = height * dpr;
  canvas.style.width = `${width}px`;
  canvas.style.height = `${height}px`;
  ctx.scale(dpr, dpr);

  return ctx;
}

/**
 * Clears a canvas and fills with background color.
 *
 * @param ctx - Canvas rendering context
 * @param width - Canvas width
 * @param height - Canvas height
 * @param backgroundColor - Background fill color
 */
export function clearCanvas(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  backgroundColor: string = 'white'
): void {
  ctx.fillStyle = backgroundColor;
  ctx.fillRect(0, 0, width, height);
}

// ============================================================================
// Font Utilities
// ============================================================================

/**
 * Composes a CSS font string.
 *
 * @param config - Text render configuration
 * @returns CSS font string
 *
 * @example
 * ```typescript
 * const font = composeFontString({
 *   fontSize: 16,
 *   fontFamily: 'Arial',
 *   isBold: true,
 *   isItalic: false,
 *   // ... other config
 * });
 * // Returns: "bold 16px Arial"
 * ```
 */
export function composeFontString(config: TextRenderConfig): string {
  const style = config.isItalic ? 'italic ' : '';
  const weight = config.isBold ? 'bold ' : '';
  return `${style}${weight}${config.fontSize}px ${config.fontFamily}`;
}

/**
 * Gets the font multiplier for a block type.
 *
 * @param blockType - The block type (p, h1, h2, h3, h4, blockquote)
 * @returns Font size multiplier
 */
export function getBlockTypeFontMultiplier(
  blockType: string
): number {
  switch (blockType) {
    case 'h1':
      return 2;
    case 'h2':
      return 1.5;
    case 'h3':
      return 1.17;
    case 'h4':
    case 'blockquote':
    case 'p':
    default:
      return 1;
  }
}

/**
 * Composes a font string for a specific block type.
 *
 * @param baseFontSize - Base font size in pixels
 * @param fontFamily - Font family name
 * @param blockType - Block type
 * @param isBold - Global bold state
 * @param isItalic - Global italic state
 * @returns CSS font string
 */
export function composeBlockFont(
  baseFontSize: number,
  fontFamily: string,
  blockType: string,
  isBold: boolean,
  isItalic: boolean
): { font: string; fontSize: number } {
  const multiplier = getBlockTypeFontMultiplier(blockType);
  const fontSize = baseFontSize * multiplier;

  let fontWeight = isBold ? 'bold ' : '';
  let fontStyle = isItalic ? 'italic ' : '';

  // Headings are always bold
  if (blockType.startsWith('h')) {
    fontWeight = 'bold ';
  }

  // Blockquotes are always italic
  if (blockType === 'blockquote') {
    fontStyle = 'italic ';
  }

  return {
    font: `${fontStyle}${fontWeight}${fontSize}px ${fontFamily}`,
    fontSize,
  };
}

// ============================================================================
// Text Drawing
// ============================================================================

/**
 * Draws text with letter spacing.
 *
 * @param ctx - Canvas rendering context
 * @param text - Text to draw
 * @param x - X position
 * @param y - Y position
 * @param letterSpacing - Letter spacing in pixels
 * @returns Total width of drawn text
 */
export function drawTextWithSpacing(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  letterSpacing: number
): number {
  if (letterSpacing === 0) {
    ctx.fillText(text, x, y);
    return ctx.measureText(text).width;
  }

  let charX = x;
  for (let i = 0; i < text.length; i++) {
    ctx.fillText(text[i], charX, y);
    charX += ctx.measureText(text[i]).width + letterSpacing;
  }

  return charX - x - (text.length > 0 ? letterSpacing : 0);
}

/**
 * Draws justified text with word spacing.
 *
 * @param ctx - Canvas rendering context
 * @param text - Text to draw
 * @param x - X position
 * @param y - Y position
 * @param availableWidth - Available width for text
 * @param letterSpacing - Letter spacing in pixels
 * @param isLastLine - Whether this is the last line of paragraph
 */
export function drawJustifiedText(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  availableWidth: number,
  letterSpacing: number,
  isLastLine: boolean
): void {
  const words = text.split(' ');
  const textWidth = ctx.measureText(text).width;

  // Don't justify last line or single-word lines
  if (isLastLine || words.length <= 1 || text.trim().length === 0) {
    drawTextWithSpacing(ctx, text, x, y, letterSpacing);
    return;
  }

  const extraSpace = availableWidth - textWidth;
  const wordSpacing = extraSpace / (words.length - 1);

  let wordX = x;
  for (let w = 0; w < words.length; w++) {
    drawTextWithSpacing(ctx, words[w], wordX, y, letterSpacing);
    wordX += ctx.measureText(words[w]).width + (words[w].length * letterSpacing);
    if (w < words.length - 1) {
      wordX += ctx.measureText(' ').width + wordSpacing;
    }
  }
}

/**
 * Calculates text X position based on alignment.
 *
 * @param align - Text alignment
 * @param startX - Starting X position
 * @param textWidth - Width of the text
 * @param availableWidth - Available width for text
 * @returns Calculated X position
 */
export function calculateTextX(
  align: TextAlign,
  startX: number,
  textWidth: number,
  availableWidth: number
): number {
  switch (align) {
    case 'center':
      return startX + (availableWidth - textWidth) / 2;
    case 'right':
      return startX + availableWidth - textWidth;
    case 'left':
    case 'justify':
    default:
      return startX;
  }
}

// ============================================================================
// Selection and Cursor
// ============================================================================

/**
 * Draws selection highlight for a line.
 *
 * @param ctx - Canvas rendering context
 * @param text - Line text
 * @param x - Text X position
 * @param y - Line Y position
 * @param lineHeight - Line height
 * @param lineIndex - Current line index
 * @param selection - Selection range
 * @param highlightColor - Selection highlight color
 */
export function drawSelectionHighlight(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  lineHeight: number,
  lineIndex: number,
  selection: SelectionRange,
  highlightColor: string = '#b4d7ff'
): void {
  if (lineIndex < selection.startLine || lineIndex > selection.endLine) {
    return;
  }

  const startCol = lineIndex === selection.startLine ? selection.startCol : 0;
  const endCol = lineIndex === selection.endLine ? selection.endCol : text.length;

  const startX = x + ctx.measureText(text.substring(0, startCol)).width;
  const width = ctx.measureText(text.substring(startCol, endCol)).width || 5;

  ctx.fillStyle = highlightColor;
  ctx.fillRect(startX, y, width, lineHeight);
}

/**
 * Draws the text cursor.
 *
 * @param ctx - Canvas rendering context
 * @param text - Line text
 * @param x - Text X position
 * @param y - Line Y position
 * @param lineHeight - Line height
 * @param cursorCol - Cursor column position
 * @param cursorColor - Cursor color
 */
export function drawCursor(
  ctx: CanvasRenderingContext2D,
  text: string,
  x: number,
  y: number,
  lineHeight: number,
  cursorCol: number,
  cursorColor: string = '#000'
): void {
  const cursorX = x + ctx.measureText(text.substring(0, cursorCol)).width;
  ctx.fillStyle = cursorColor;
  ctx.fillRect(cursorX, y + 2, 2, lineHeight - 4);
}

// ============================================================================
// List Markers
// ============================================================================

/**
 * Draws a bullet list marker.
 *
 * @param ctx - Canvas rendering context
 * @param x - X position for the bullet
 * @param y - Y position (center of line)
 * @param fontSize - Font size for sizing the bullet
 * @param color - Bullet color
 */
export function drawBulletMarker(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  fontSize: number,
  color: string = '#202124'
): void {
  ctx.fillStyle = color;
  ctx.beginPath();
  ctx.arc(x, y, fontSize * 0.15, 0, Math.PI * 2);
  ctx.fill();
}

/**
 * Draws a numbered list marker.
 *
 * @param ctx - Canvas rendering context
 * @param number - List number
 * @param x - X position (right-aligned to)
 * @param y - Y position
 * @param font - Font string
 * @param color - Text color
 */
export function drawNumberMarker(
  ctx: CanvasRenderingContext2D,
  number: number,
  x: number,
  y: number,
  font: string,
  color: string = '#202124'
): void {
  const savedAlign = ctx.textAlign;
  ctx.font = font;
  ctx.fillStyle = color;
  ctx.textAlign = 'right';
  ctx.fillText(`${number}.`, x, y);
  ctx.textAlign = savedAlign;
}

/**
 * Draws a blockquote indicator bar.
 *
 * @param ctx - Canvas rendering context
 * @param x - X position
 * @param y - Y position
 * @param height - Bar height
 * @param color - Bar color
 * @param width - Bar width
 */
export function drawBlockquoteIndicator(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  height: number,
  color: string = '#ccc',
  width: number = 3
): void {
  ctx.fillStyle = color;
  ctx.fillRect(x, y, width, height);
}

// ============================================================================
// Text Decorations
// ============================================================================

/**
 * Draws an underline.
 *
 * @param ctx - Canvas rendering context
 * @param x - X start position
 * @param y - Y position (bottom of text)
 * @param width - Underline width
 * @param color - Underline color
 */
export function drawUnderline(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  color: string = '#202124'
): void {
  ctx.strokeStyle = color;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(x + width, y);
  ctx.stroke();
}

/**
 * Draws a strikethrough line.
 *
 * @param ctx - Canvas rendering context
 * @param x - X start position
 * @param y - Y position (middle of text)
 * @param width - Line width
 * @param color - Line color
 */
export function drawStrikethrough(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  color: string = '#202124'
): void {
  ctx.strokeStyle = color;
  ctx.lineWidth = 1;
  ctx.beginPath();
  ctx.moveTo(x, y);
  ctx.lineTo(x + width, y);
  ctx.stroke();
}

// ============================================================================
// Image Rendering
// ============================================================================

/**
 * Calculates crop source rectangle for an image.
 *
 * @param naturalWidth - Original image width
 * @param naturalHeight - Original image height
 * @param cropTop - Top crop percentage
 * @param cropRight - Right crop percentage
 * @param cropBottom - Bottom crop percentage
 * @param cropLeft - Left crop percentage
 * @returns Source rectangle for drawing
 */
export function calculateCropRect(
  naturalWidth: number,
  naturalHeight: number,
  cropTop: number,
  cropRight: number,
  cropBottom: number,
  cropLeft: number
): CropRect {
  return {
    x: (cropLeft / 100) * naturalWidth,
    y: (cropTop / 100) * naturalHeight,
    width: ((100 - cropLeft - cropRight) / 100) * naturalWidth,
    height: ((100 - cropTop - cropBottom) / 100) * naturalHeight,
  };
}

/**
 * Draws an image with crop applied.
 *
 * @param ctx - Canvas rendering context
 * @param img - Image element to draw
 * @param cropRect - Source crop rectangle
 * @param destX - Destination X
 * @param destY - Destination Y
 * @param destWidth - Destination width
 * @param destHeight - Destination height
 * @param opacity - Opacity (0-1)
 */
export function drawCroppedImage(
  ctx: CanvasRenderingContext2D,
  img: HTMLImageElement,
  cropRect: CropRect,
  destX: number,
  destY: number,
  destWidth: number,
  destHeight: number,
  opacity: number = 1
): void {
  if (opacity !== 1) {
    ctx.globalAlpha = opacity;
  }

  ctx.drawImage(
    img,
    cropRect.x,
    cropRect.y,
    cropRect.width,
    cropRect.height,
    destX,
    destY,
    destWidth,
    destHeight
  );

  if (opacity !== 1) {
    ctx.globalAlpha = 1;
  }
}

/**
 * Draws image selection border and handles.
 *
 * @param ctx - Canvas rendering context
 * @param x - Image X position
 * @param y - Image Y position
 * @param width - Image width
 * @param height - Image height
 * @param isDragging - Whether image is being dragged
 * @param isCropping - Whether image is in crop mode
 */
export function drawImageSelection(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  isDragging: boolean = false,
  isCropping: boolean = false
): void {
  const color = isDragging ? '#4caf50' : '#1a73e8';
  const handleSize = 8;

  // Draw border
  ctx.strokeStyle = color;
  ctx.lineWidth = 2;
  ctx.setLineDash([]);
  ctx.strokeRect(x - 2, y - 2, width + 4, height + 4);

  // Draw handles
  ctx.fillStyle = isCropping ? '#ff9800' : color;

  // Corners
  ctx.fillRect(x - handleSize / 2, y - handleSize / 2, handleSize, handleSize);
  ctx.fillRect(x + width - handleSize / 2, y - handleSize / 2, handleSize, handleSize);
  ctx.fillRect(x - handleSize / 2, y + height - handleSize / 2, handleSize, handleSize);
  ctx.fillRect(x + width - handleSize / 2, y + height - handleSize / 2, handleSize, handleSize);

  // Edge midpoints
  ctx.fillRect(x + width / 2 - handleSize / 2, y - handleSize / 2, handleSize, handleSize);
  ctx.fillRect(x + width / 2 - handleSize / 2, y + height - handleSize / 2, handleSize, handleSize);
  ctx.fillRect(x - handleSize / 2, y + height / 2 - handleSize / 2, handleSize, handleSize);
  ctx.fillRect(x + width - handleSize / 2, y + height / 2 - handleSize / 2, handleSize, handleSize);

  // Draw drag overlay
  if (isDragging) {
    ctx.fillStyle = 'rgba(76, 175, 80, 0.2)';
    ctx.fillRect(x, y, width, height);
  }

  // Draw crop mode indicator
  if (isCropping) {
    ctx.strokeStyle = '#ff9800';
    ctx.lineWidth = 2;
    ctx.setLineDash([5, 5]);
    ctx.strokeRect(x, y, width, height);
    ctx.setLineDash([]);
  }
}

/**
 * Draws a subtle border around an unselected image.
 *
 * @param ctx - Canvas rendering context
 * @param x - Image X position
 * @param y - Image Y position
 * @param width - Image width
 * @param height - Image height
 * @param color - Border color
 */
export function drawImageBorder(
  ctx: CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number,
  color: string = '#e0e0e0'
): void {
  ctx.strokeStyle = color;
  ctx.lineWidth = 1;
  ctx.setLineDash([]);
  ctx.strokeRect(x, y, width, height);
}

// ============================================================================
// Page Elements
// ============================================================================

/**
 * Draws a page number.
 *
 * @param ctx - Canvas rendering context
 * @param pageNumber - Page number to display
 * @param x - X position (center)
 * @param y - Y position
 * @param fontFamily - Font family
 * @param color - Text color
 */
export function drawPageNumber(
  ctx: CanvasRenderingContext2D,
  pageNumber: number,
  x: number,
  y: number,
  fontFamily: string,
  color: string = '#999'
): void {
  ctx.fillStyle = color;
  ctx.font = `10px ${fontFamily}`;
  ctx.textAlign = 'center';
  ctx.fillText(`${pageNumber}`, x, y);
  ctx.textAlign = 'left';
}

// ============================================================================
// Position Utilities
// ============================================================================

/**
 * Calculates line position within a page for multi-column layout.
 *
 * @param lineIndexOnPage - Line index within the page
 * @param config - Page render configuration
 * @returns Column index and line within column
 */
export function getLineColumnPosition(
  lineIndexOnPage: number,
  config: PageRenderConfig
): { columnIndex: number; lineInColumn: number; columnOffsetX: number } {
  const columnIndex =
    config.columnCount > 1
      ? Math.floor(lineIndexOnPage / config.linesPerColumn)
      : 0;
  const lineInColumn =
    config.columnCount > 1
      ? lineIndexOnPage % config.linesPerColumn
      : lineIndexOnPage;
  const columnOffsetX = columnIndex * (config.columnWidth + config.columnGap);

  return { columnIndex, lineInColumn, columnOffsetX };
}

/**
 * Calculates Y position for a line.
 *
 * @param lineInColumn - Line index within the column
 * @param marginTop - Top margin
 * @param lineHeight - Line height
 * @param cumulativeSpacing - Accumulated paragraph spacing
 * @returns Y position in pixels
 */
export function calculateLineY(
  lineInColumn: number,
  marginTop: number,
  lineHeight: number,
  cumulativeSpacing: number = 0
): number {
  return marginTop + lineInColumn * lineHeight + cumulativeSpacing;
}
