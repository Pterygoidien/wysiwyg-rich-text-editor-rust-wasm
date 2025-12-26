/**
 * @fileoverview Type definitions for the paginated rich text editor.
 *
 * This module contains all TypeScript interfaces, types, and constants used
 * throughout the editor system. Types are organized by domain:
 * - Text formatting and paragraph metadata
 * - Image handling and layout
 * - Display rendering
 * - Selection and cursor state
 *
 * @module editor/types
 */

/**
 * Horizontal text alignment options.
 * Maps to standard CSS text-align values.
 */
export type TextAlign = 'left' | 'center' | 'right' | 'justify';

/**
 * List formatting types.
 * - 'none': Regular paragraph
 * - 'bullet': Unordered list with bullet markers
 * - 'numbered': Ordered list with numeric markers
 */
export type ListType = 'none' | 'bullet' | 'numbered';

/**
 * Block-level element types.
 * Determines the semantic and visual styling of a paragraph.
 */
export type BlockType = 'p' | 'h1' | 'h2' | 'h3' | 'h4' | 'blockquote';

/**
 * Image text wrapping styles, following Microsoft Word conventions.
 *
 * - 'inline': Image flows with text as a character
 * - 'square': Text wraps in a rectangular boundary around the image
 * - 'tight': Text wraps closely following image contours (simplified to square)
 * - 'through': Text flows through transparent areas (simplified to square)
 * - 'top-bottom': Text appears only above and below the image
 * - 'behind': Image renders behind text layer
 * - 'in-front': Image renders above text layer
 */
export type ImageWrapStyle =
  | 'inline'
  | 'square'
  | 'tight'
  | 'through'
  | 'top-bottom'
  | 'behind'
  | 'in-front';

/**
 * Image positioning behavior.
 * - 'move-with-text': Image position is relative to its anchor paragraph
 * - 'fixed-position': Image maintains absolute position on the page
 */
export type ImagePositionMode = 'move-with-text' | 'fixed-position';

/**
 * Resize handle positions for image manipulation.
 * Cardinal (n, s, e, w) and ordinal (nw, ne, sw, se) directions.
 */
export type ResizeHandle = 'nw' | 'ne' | 'sw' | 'se' | 'n' | 's' | 'e' | 'w' | null;

/**
 * Metadata associated with each paragraph in the document.
 * Controls formatting, styling, and semantic meaning.
 */
export interface ParagraphMeta {
  /** Horizontal text alignment */
  align: TextAlign;
  /** List formatting type */
  listType: ListType;
  /** Block-level element type (paragraph, heading, quote) */
  blockType: BlockType;
  /** Indentation level (0 = no indent) */
  indent: number;
  /** Custom font size in pixels, overrides document default */
  fontSize?: number;
  /** Text color as CSS color string (hex, rgb, etc.) */
  textColor?: string;
}

/**
 * Represents an image embedded in the document.
 * Contains source data, dimensions, layout options, and positioning.
 */
export interface DocumentImage {
  /** Unique identifier for the image */
  id: string;
  /** Image source as data URL or external URL */
  src: string;
  /** Current display width in pixels */
  width: number;
  /** Current display height in pixels */
  height: number;
  /** Original image width before any scaling */
  naturalWidth: number;
  /** Original image height before any scaling */
  naturalHeight: number;
  /** Text wrapping behavior around the image */
  wrapStyle: ImageWrapStyle;
  /** How image position responds to text changes */
  positionMode: ImagePositionMode;
  /** Horizontal alignment within the content area */
  horizontalAlign?: 'left' | 'center' | 'right';
  /** Top crop percentage (0-100) */
  cropTop?: number;
  /** Right crop percentage (0-100) */
  cropRight?: number;
  /** Bottom crop percentage (0-100) */
  cropBottom?: number;
  /** Left crop percentage (0-100) */
  cropLeft?: number;
  /** Absolute X position from left margin (unscaled pixels) */
  x?: number;
  /** Absolute Y position from top of document (unscaled pixels) */
  y?: number;
  /** Page index where the image is anchored */
  pageIndex?: number;
}

/**
 * Represents a single rendered line in the display.
 * Created by wrapping paragraph text to fit within column widths.
 */
export interface DisplayLine {
  /** Index of the source paragraph */
  paraIndex: number;
  /** Character offset where this line starts in the paragraph */
  startOffset: number;
  /** Character offset where this line ends in the paragraph */
  endOffset: number;
  /** The text content of this line */
  text: string;
  /** Formatting metadata inherited from the paragraph */
  meta: ParagraphMeta;
  /** Sequence number for numbered lists */
  listNumber?: number;
  /** Whether this line represents an image placeholder */
  isImage?: boolean;
  /** Reference to the image if this is an image line */
  imageId?: string;
  /** Height of the image in display line units */
  imageHeight?: number;
  /** Width reduction when text wraps around a floating image */
  floatReduction?: { side: 'left' | 'right'; width: number };
  /** Whether this line represents a manual page break */
  isPageBreak?: boolean;
  /** Whether this is the last line of its paragraph (for spacing) */
  isLastLineOfParagraph?: boolean;
  /** Assigned page index (0-based), computed during layout */
  pageIndex?: number;
  /** Y position on the page (scaled pixels), computed during layout */
  yPosition?: number;
  /** Column index within the page (0-based), computed during layout */
  columnIndex?: number;
}

/**
 * Represents a floating image that affects text flow.
 * Used during layout calculations to determine text wrapping.
 */
export interface FloatImage {
  /** Image identifier */
  id: string;
  /** First display line affected by this float */
  startLine: number;
  /** Last display line affected by this float (exclusive) */
  endLine: number;
  /** Width of the float in scaled pixels */
  width: number;
  /** Which side of the content area the float occupies */
  side: 'left' | 'right';
}

/**
 * Cursor or selection position within the document.
 */
export interface TextPosition {
  /** Paragraph index */
  para: number;
  /** Character offset within the paragraph */
  offset: number;
}

/**
 * Bounding rectangle for an image on a specific page.
 */
export interface ImageBounds {
  /** Page index where the image appears */
  pageIndex: number;
  /** Left edge X coordinate */
  x: number;
  /** Top edge Y coordinate */
  y: number;
  /** Image width */
  width: number;
  /** Image height */
  height: number;
}

/**
 * Unicode replacement character used to mark image placeholder paragraphs.
 * The actual image ID follows this character in the paragraph text.
 */
export const IMAGE_MARKER = '\uFFFC';

/**
 * Unicode replacement character used to mark manual page breaks.
 * A paragraph containing only this character forces content to the next page.
 */
export const PAGE_BREAK_MARKER = '\uFFFD';

/**
 * Font size multipliers for heading block types.
 * Applied to the base font size when rendering.
 */
export const HEADING_SCALE: Record<BlockType, number> = {
  p: 1,
  h1: 2,
  h2: 1.5,
  h3: 1.17,
  h4: 1,
  blockquote: 1,
};

/**
 * Default paragraph metadata for new paragraphs.
 */
export function createDefaultMeta(): ParagraphMeta {
  return {
    align: 'left',
    listType: 'none',
    blockType: 'p',
    indent: 0,
  };
}
