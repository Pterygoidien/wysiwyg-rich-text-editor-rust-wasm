/**
 * @fileoverview Layout engine for the paginated rich text editor.
 *
 * This module provides the core layout computation logic that transforms
 * the document's paragraph-based structure into renderable display lines.
 * It handles text wrapping, image positioning, page breaks, and float tracking.
 *
 * @module editor/layout-engine
 */

import type {
  ParagraphMeta,
  DocumentImage,
  DisplayLine,
  FloatImage,
  ImageWrapStyle,
} from './types';
import { IMAGE_MARKER, PAGE_BREAK_MARKER, createDefaultMeta } from './types';
import type { HeadingItem } from '../stores';

/**
 * Configuration for layout computation.
 * Contains all parameters needed to compute display lines.
 */
export interface LayoutConfig {
  /** Scaled font size in pixels (font size * zoom / 100) */
  scaledFontSize: number;
  /** Scaled line height in pixels */
  scaledLineHeight: number;
  /** Unscaled base font size */
  fontSize: number;
  /** Line height multiplier (e.g., 1.5) */
  lineHeightMultiplier: number;
  /** Content area width in pixels */
  contentWidth: number;
  /** Column width in pixels */
  columnWidth: number;
  /** Lines that fit per page */
  linesPerPage: number;
  /** Current zoom level (percentage) */
  zoomLevel: number;
}

/**
 * Result of layout computation.
 */
export interface LayoutResult {
  /** Computed display lines */
  displayLines: DisplayLine[];
  /** Active floating images affecting text flow */
  activeFloats: FloatImage[];
  /** Total number of pages */
  pageCount: number;
}

/**
 * Checks if an image wrap style causes text to flow around the image.
 *
 * @param style - The image wrap style to check
 * @returns True if text should flow around the image
 *
 * @example
 * ```typescript
 * isFloatWrapStyle('square'); // true
 * isFloatWrapStyle('inline'); // false
 * ```
 */
export function isFloatWrapStyle(style: ImageWrapStyle): boolean {
  return style === 'square' || style === 'tight' || style === 'through';
}

/**
 * Determines which side a floating image should occupy.
 *
 * @param docImage - The document image
 * @param contentWidth - Total content area width
 * @returns 'left' or 'right' based on image alignment/position
 */
export function getFloatSide(
  docImage: DocumentImage,
  contentWidth: number
): 'left' | 'right' {
  if (docImage.horizontalAlign === 'right') {
    return 'right';
  } else if (docImage.horizontalAlign === 'center') {
    // For centered, determine by actual position
    const contentCenter = contentWidth / 2;
    const imageCenterX = (docImage.x ?? 0) + docImage.width / 2;
    return imageCenterX < contentCenter ? 'left' : 'right';
  }
  return 'left';
}

/**
 * Calculates how many display lines an image occupies.
 *
 * @param imageHeight - The image height in pixels (scaled)
 * @param lineHeight - The line height in pixels (scaled)
 * @returns Number of lines the image spans
 */
export function calculateImageLines(
  imageHeight: number,
  lineHeight: number
): number {
  return Math.ceil(imageHeight / lineHeight);
}

/**
 * Collects floating images that have absolute positions.
 * These affect text wrapping based on their Y position in the document.
 *
 * @param paragraphs - Document paragraphs
 * @param images - Document images
 * @param config - Layout configuration
 * @param contentWidth - Content area width for side calculation
 * @returns Array of float image data
 */
export function collectAbsoluteFloats(
  paragraphs: string[],
  images: DocumentImage[],
  config: LayoutConfig,
  contentWidth: number
): FloatImage[] {
  const floats: FloatImage[] = [];

  for (let i = 0; i < paragraphs.length; i++) {
    const paraText = paragraphs[i];
    if (!paraText.startsWith(IMAGE_MARKER)) continue;

    const imageId = paraText.substring(1);
    const docImage = images.find(img => img.id === imageId);

    if (!docImage || !isFloatWrapStyle(docImage.wrapStyle)) continue;

    const scaledWidth = (docImage.width * config.zoomLevel) / 100;
    const scaledHeight = (docImage.height * config.zoomLevel) / 100;
    const imageLines = calculateImageLines(scaledHeight, config.scaledLineHeight);

    // Only process images with absolute position
    if (docImage.y !== undefined) {
      const unscaledLineHeight = config.fontSize * config.lineHeightMultiplier;
      const startLine = Math.floor(docImage.y / unscaledLineHeight);
      const side = getFloatSide(docImage, contentWidth);

      floats.push({
        id: imageId,
        startLine,
        endLine: startLine + imageLines,
        width: scaledWidth,
        side,
      });
    }
  }

  return floats;
}

/**
 * Creates display lines for an image paragraph.
 *
 * @param paraIndex - Paragraph index
 * @param imageId - Image ID
 * @param docImage - Document image data
 * @param meta - Paragraph metadata
 * @param config - Layout configuration
 * @returns Array of display lines for the image
 */
export function createImageDisplayLines(
  paraIndex: number,
  imageId: string,
  docImage: DocumentImage,
  meta: ParagraphMeta,
  config: LayoutConfig
): DisplayLine[] {
  const scaledHeight = (docImage.height * config.zoomLevel) / 100;
  const imageLines = calculateImageLines(scaledHeight, config.scaledLineHeight);
  const lines: DisplayLine[] = [];
  const paraText = IMAGE_MARKER + imageId;

  if (isFloatWrapStyle(docImage.wrapStyle)) {
    // Float images add a zero-height marker
    lines.push({
      paraIndex,
      startOffset: 0,
      endOffset: paraText.length,
      text: '',
      meta,
      isImage: true,
      imageId,
      imageHeight: imageLines,
    });
  } else if (
    docImage.wrapStyle === 'inline' ||
    docImage.wrapStyle === 'top-bottom'
  ) {
    // Inline/top-bottom images take up vertical space
    lines.push({
      paraIndex,
      startOffset: 0,
      endOffset: paraText.length,
      text: '',
      meta,
      isImage: true,
      imageId,
      imageHeight: imageLines,
    });

    // Add placeholder lines for the image height
    for (let j = 1; j < imageLines; j++) {
      lines.push({
        paraIndex,
        startOffset: 0,
        endOffset: paraText.length,
        text: '',
        meta,
        isImage: true,
        imageId,
        imageHeight: 0, // Not the first line
      });
    }
  } else if (
    docImage.wrapStyle === 'behind' ||
    docImage.wrapStyle === 'in-front'
  ) {
    // Behind/in-front images don't affect text flow
    lines.push({
      paraIndex,
      startOffset: 0,
      endOffset: paraText.length,
      text: '',
      meta,
      isImage: true,
      imageId,
      imageHeight: 0, // No height impact on text
    });
  } else {
    // Block image - takes up full width and vertical space
    lines.push({
      paraIndex,
      startOffset: 0,
      endOffset: paraText.length,
      text: '',
      meta,
      isImage: true,
      imageId,
      imageHeight: imageLines,
    });

    for (let j = 1; j < imageLines; j++) {
      lines.push({
        paraIndex,
        startOffset: 0,
        endOffset: paraText.length,
        text: '',
        meta,
        isImage: true,
        imageId,
      });
    }
  }

  return lines;
}

/**
 * Creates display lines for a page break.
 *
 * @param paraIndex - Paragraph index
 * @param meta - Paragraph metadata
 * @param currentLineCount - Current number of display lines
 * @param linesPerPage - Lines that fit per page
 * @returns Array of display lines for the page break
 */
export function createPageBreakLines(
  paraIndex: number,
  meta: ParagraphMeta,
  currentLineCount: number,
  linesPerPage: number
): DisplayLine[] {
  const lines: DisplayLine[] = [];
  const linesOnCurrentPage = currentLineCount % linesPerPage;
  const linesToAdd = linesOnCurrentPage === 0 ? 0 : linesPerPage - linesOnCurrentPage;

  // Add the page break marker line
  lines.push({
    paraIndex,
    startOffset: 0,
    endOffset: 1,
    text: '',
    meta,
    isPageBreak: true,
  });

  // Fill with empty lines to complete the page
  for (let j = 1; j < linesToAdd; j++) {
    lines.push({
      paraIndex,
      startOffset: 0,
      endOffset: 1,
      text: '',
      meta,
      isPageBreak: true,
    });
  }

  return lines;
}

/**
 * Finds active float reduction for a given line index.
 *
 * @param lineIndex - Current line index
 * @param floats - Array of active floats
 * @returns Float reduction or undefined if no float affects this line
 */
export function getFloatReductionForLine(
  lineIndex: number,
  floats: FloatImage[]
): { side: 'left' | 'right'; width: number } | undefined {
  for (const float of floats) {
    if (lineIndex >= float.startLine && lineIndex < float.endLine) {
      return { side: float.side, width: float.width };
    }
  }
  return undefined;
}

/**
 * Converts a paragraph position to a display line position.
 * Used for mapping cursor/selection positions to rendered coordinates.
 *
 * @param para - Paragraph index in the document
 * @param offset - Character offset within the paragraph
 * @param displayLines - Array of computed display lines
 * @returns Display line index and column offset
 *
 * @example
 * ```typescript
 * const pos = paraToDisplayPos(0, 5, displayLines);
 * console.log(`Line ${pos.line}, column ${pos.col}`);
 * ```
 */
export function paraToDisplayPos(
  para: number,
  offset: number,
  displayLines: DisplayLine[]
): { line: number; col: number } {
  for (let i = 0; i < displayLines.length; i++) {
    const dl = displayLines[i];
    if (dl.paraIndex === para && offset >= dl.startOffset && offset <= dl.endOffset) {
      return { line: i, col: offset - dl.startOffset };
    }
  }
  const lastLine = displayLines.length - 1;
  return { line: lastLine, col: displayLines[lastLine]?.text.length || 0 };
}

/**
 * Converts a display line position to a paragraph position.
 * Used for mapping click coordinates back to document positions.
 *
 * @param line - Display line index
 * @param col - Column offset within the display line
 * @param displayLines - Array of computed display lines
 * @param paragraphs - Document paragraphs for bounds checking
 * @returns Paragraph index and character offset
 *
 * @example
 * ```typescript
 * const pos = displayToPara(5, 10, displayLines, paragraphs);
 * console.log(`Paragraph ${pos.para}, offset ${pos.offset}`);
 * ```
 */
export function displayToPara(
  line: number,
  col: number,
  displayLines: DisplayLine[],
  paragraphs: string[]
): { para: number; offset: number } {
  if (line < 0 || line >= displayLines.length) {
    return {
      para: paragraphs.length - 1,
      offset: paragraphs[paragraphs.length - 1]?.length || 0,
    };
  }
  const dl = displayLines[line];
  return {
    para: dl.paraIndex,
    offset: dl.startOffset + Math.min(col, dl.text.length),
  };
}

/**
 * Extracts heading paragraphs for document navigation.
 * Scans all paragraphs for heading block types (h1-h4) and builds
 * a list for the document outline sidebar.
 *
 * @param paragraphs - Document paragraphs
 * @param paragraphMeta - Paragraph metadata array
 * @returns Array of heading items for navigation
 *
 * @example
 * ```typescript
 * const headings = extractHeadings(paragraphs, meta);
 * // Returns: [{ id: 'heading-0', text: 'Introduction', level: 1, paraIndex: 0 }, ...]
 * ```
 */
export function extractHeadings(
  paragraphs: string[],
  paragraphMeta: ParagraphMeta[]
): HeadingItem[] {
  const headings: HeadingItem[] = [];

  for (let i = 0; i < paragraphs.length; i++) {
    const meta = paragraphMeta[i] || createDefaultMeta();
    const blockType = meta.blockType;

    if (blockType.startsWith('h')) {
      const level = parseInt(blockType[1]);
      if (level >= 1 && level <= 4) {
        headings.push({
          id: `heading-${i}`,
          text: paragraphs[i],
          level,
          paraIndex: i,
        });
      }
    }
  }

  return headings;
}

/**
 * Calculates which page a display line appears on.
 *
 * @param lineIndex - The display line index
 * @param linesPerPage - Lines that fit per page
 * @returns Page number (0-indexed)
 */
export function getPageForLine(lineIndex: number, linesPerPage: number): number {
  return Math.floor(lineIndex / linesPerPage);
}

/**
 * Calculates the line range for a specific page.
 *
 * @param pageIndex - Page index (0-based)
 * @param linesPerPage - Lines per page
 * @param totalLines - Total display lines
 * @returns Start and end line indices for the page
 */
export function getPageLineRange(
  pageIndex: number,
  linesPerPage: number,
  totalLines: number
): { startLine: number; endLine: number } {
  const startLine = pageIndex * linesPerPage;
  const endLine = Math.min(startLine + linesPerPage, totalLines);
  return { startLine, endLine };
}
