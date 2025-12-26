/**
 * @fileoverview Page configuration types and utilities.
 *
 * This module provides type definitions for page layout configuration
 * and utility functions for dimension calculations. All measurements
 * use millimeters for page dimensions and pixels for screen rendering.
 *
 * @module types
 */

// ============================================================================
// Core Types
// ============================================================================

/**
 * Represents a page format (paper size).
 * Dimensions are in millimeters.
 */
export interface PageFormat {
  /** Display name for the format (e.g., 'A4', 'Letter') */
  name: string;
  /** Page width in millimeters */
  width: number;
  /** Page height in millimeters */
  height: number;
}

/**
 * Represents page margins.
 * All values are in millimeters.
 */
export interface PageMargins {
  /** Top margin in millimeters */
  top: number;
  /** Right margin in millimeters */
  right: number;
  /** Bottom margin in millimeters */
  bottom: number;
  /** Left margin in millimeters */
  left: number;
}

/**
 * Complete page configuration.
 * Combines format, margins, orientation, and column settings.
 */
export interface PageConfig {
  /** Page format (paper size) */
  format: PageFormat;
  /** Page margins */
  margins: PageMargins;
  /** Page orientation */
  orientation: 'portrait' | 'landscape';
  /** Number of text columns (1 or 2) */
  columns: 1 | 2;
  /** Gap between columns in millimeters */
  columnGap: number;
}

/**
 * Dimensions in pixels.
 */
export interface Dimensions {
  /** Width in pixels */
  width: number;
  /** Height in pixels */
  height: number;
}

// ============================================================================
// Page Format Constants
// ============================================================================

/**
 * Standard page formats.
 * Includes common paper sizes with dimensions in millimeters.
 *
 * @example
 * ```typescript
 * const a4 = PAGE_FORMATS.A4;
 * console.log(a4.width);  // 210
 * console.log(a4.height); // 297
 * ```
 */
export const PAGE_FORMATS: Record<string, PageFormat> = {
  /** ISO A4 format (210mm x 297mm) - Standard international paper size */
  A4: { name: 'A4', width: 210, height: 297 },
  /** ISO A5 format (148mm x 210mm) - Half of A4 */
  A5: { name: 'A5', width: 148, height: 210 },
  /** US Letter format (8.5" x 11") */
  LETTER: { name: 'Letter', width: 215.9, height: 279.4 },
  /** US Legal format (8.5" x 14") */
  LEGAL: { name: 'Legal', width: 215.9, height: 355.6 },
  /** US Textbook format (9.3" x 11") */
  TEXTBOOK: { name: 'US Textbook', width: 236.2, height: 279.4 },
};

// ============================================================================
// Margin Presets
// ============================================================================

/**
 * Default margins (1 inch / 25.4mm on all sides).
 * Standard margins for most document types.
 */
export const DEFAULT_MARGINS: PageMargins = {
  top: 25.4,
  right: 25.4,
  bottom: 25.4,
  left: 25.4,
};

/**
 * Narrow margins (0.5 inch / 12.7mm on all sides).
 * Maximizes content area for dense documents.
 */
export const NARROW_MARGINS: PageMargins = {
  top: 12.7,
  right: 12.7,
  bottom: 12.7,
  left: 12.7,
};

/**
 * Wide margins (1 inch top/bottom, 2 inches left/right).
 * Useful for documents with margin annotations.
 */
export const WIDE_MARGINS: PageMargins = {
  top: 25.4,
  right: 50.8,
  bottom: 25.4,
  left: 50.8,
};

// ============================================================================
// Conversion Functions
// ============================================================================

/**
 * Converts millimeters to pixels at 96 DPI (standard screen resolution).
 *
 * The formula is: pixels = (mm / 25.4) * 96
 * where 25.4mm = 1 inch and 96 pixels = 1 inch at 96 DPI.
 *
 * @param mm - Value in millimeters
 * @returns Value in pixels
 *
 * @example
 * ```typescript
 * mmToPixels(25.4);  // Returns 96 (1 inch)
 * mmToPixels(210);   // Returns ~793.7 (A4 width)
 * ```
 */
export function mmToPixels(mm: number): number {
  return (mm / 25.4) * 96;
}

/**
 * Gets effective page dimensions in pixels based on configuration.
 * Accounts for page orientation by swapping width/height if landscape.
 *
 * @param config - Page configuration
 * @returns Page dimensions in pixels
 *
 * @example
 * ```typescript
 * const dims = getPageDimensions({
 *   format: PAGE_FORMATS.A4,
 *   orientation: 'portrait',
 *   // ... other config
 * });
 * console.log(dims.width);  // ~793.7
 * console.log(dims.height); // ~1122.5
 * ```
 */
export function getPageDimensions(config: PageConfig): Dimensions {
  const { format, orientation } = config;

  if (orientation === 'landscape') {
    return {
      width: mmToPixels(format.height),
      height: mmToPixels(format.width),
    };
  }

  return {
    width: mmToPixels(format.width),
    height: mmToPixels(format.height),
  };
}

/**
 * Gets content area dimensions (page minus margins) in pixels.
 * This is the actual area available for text and content.
 *
 * @param config - Page configuration
 * @returns Content area dimensions in pixels
 *
 * @example
 * ```typescript
 * const content = getContentDimensions(config);
 * console.log(`Content area: ${content.width}px x ${content.height}px`);
 * ```
 */
export function getContentDimensions(config: PageConfig): Dimensions {
  const pageDims = getPageDimensions(config);
  const { margins } = config;

  return {
    width: pageDims.width - mmToPixels(margins.left) - mmToPixels(margins.right),
    height: pageDims.height - mmToPixels(margins.top) - mmToPixels(margins.bottom),
  };
}

/**
 * Gets single column width in pixels based on configuration.
 * For single-column layouts, returns full content width.
 * For two-column layouts, returns (content width - gap) / 2.
 *
 * @param config - Page configuration
 * @returns Column width in pixels
 *
 * @example
 * ```typescript
 * const singleColumn = getColumnWidth({ ...config, columns: 1 });
 * const twoColumns = getColumnWidth({ ...config, columns: 2, columnGap: 10 });
 * ```
 */
export function getColumnWidth(config: PageConfig): number {
  const contentDims = getContentDimensions(config);
  const columns = config.columns || 1;
  const columnGap = mmToPixels(config.columnGap || 0);

  if (columns === 1) {
    return contentDims.width;
  }

  // For 2 columns: (contentWidth - gap) / 2
  return (contentDims.width - columnGap) / 2;
}

// ============================================================================
// Text Configuration
// ============================================================================

/**
 * Text metrics configuration for layout calculations.
 */
export interface TextConfig {
  /** Base font size in pixels */
  baseFontSize: number;
  /** Line height multiplier (e.g., 1.5 for 150% line height) */
  lineHeight: number;
  /** Paragraph spacing in em units */
  paragraphSpacing: number;
}

/**
 * Default text configuration.
 * 16px font, 1.5 line height, 1em paragraph spacing.
 */
export const DEFAULT_TEXT_CONFIG: TextConfig = {
  baseFontSize: 16,
  lineHeight: 1.5,
  paragraphSpacing: 1,
};

// ============================================================================
// Element Height Calculations
// ============================================================================

/**
 * Pre-calculated heights for different element types.
 * Used for layout estimation.
 */
export interface ElementHeights {
  /** Single line height in pixels */
  line: number;
  /** Paragraph height including spacing */
  paragraph: number;
  /** H1 heading height including margin */
  h1: number;
  /** H2 heading height including margin */
  h2: number;
  /** H3 heading height including margin */
  h3: number;
  /** H4 heading height including margin */
  h4: number;
  /** Blockquote height including margins */
  blockquote: number;
  /** List item height including margin */
  listItem: number;
}

/**
 * Calculates heights for various element types based on text configuration.
 * All heights are in pixels and account for zoom level.
 *
 * @param textConfig - Text configuration
 * @param zoomLevel - Zoom level as percentage (default: 100)
 * @returns Object containing heights for each element type
 *
 * @example
 * ```typescript
 * const heights = calculateElementHeights(DEFAULT_TEXT_CONFIG, 150);
 * console.log(heights.line);     // Line height at 150% zoom
 * console.log(heights.h1);       // H1 height at 150% zoom
 * ```
 */
export function calculateElementHeights(
  textConfig: TextConfig,
  zoomLevel: number = 100
): ElementHeights {
  const scaledFontSize = (textConfig.baseFontSize * zoomLevel) / 100;
  const lineHeight = scaledFontSize * textConfig.lineHeight;
  const paragraphSpacing = scaledFontSize * textConfig.paragraphSpacing;

  return {
    line: lineHeight,
    paragraph: lineHeight + paragraphSpacing,
    // H1: 2em font + 0.5em margin
    h1: scaledFontSize * 2 * textConfig.lineHeight + scaledFontSize * 0.5,
    // H2: 1.5em font + 0.5em margin
    h2: scaledFontSize * 1.5 * textConfig.lineHeight + scaledFontSize * 0.5,
    // H3: 1.17em font + 0.5em margin
    h3: scaledFontSize * 1.17 * textConfig.lineHeight + scaledFontSize * 0.5,
    // H4: 1em font + 0.5em margin
    h4: scaledFontSize * textConfig.lineHeight + scaledFontSize * 0.5,
    // Blockquote: line + 1em top/bottom margin
    blockquote: lineHeight + scaledFontSize * 2,
    // List item: line + 0.25em margin
    listItem: lineHeight + scaledFontSize * 0.25,
  };
}

/**
 * Calculates how many lines fit in the content area.
 *
 * @param config - Page configuration
 * @param textConfig - Text configuration
 * @param zoomLevel - Zoom level as percentage (default: 100)
 * @returns Maximum number of lines per page
 */
export function getMaxLinesPerPage(
  config: PageConfig,
  textConfig: TextConfig,
  zoomLevel: number = 100
): number {
  const contentDims = getContentDimensions(config);
  const scaledContentHeight = (contentDims.height * zoomLevel) / 100;
  const heights = calculateElementHeights(textConfig, zoomLevel);

  return Math.floor(scaledContentHeight / heights.line);
}

/**
 * Estimates the height of an HTML element based on its type and content.
 * Used for pre-layout calculations before canvas rendering.
 *
 * @param element - The HTML element to measure
 * @param textConfig - Text configuration
 * @param contentWidth - Available content width in pixels
 * @param zoomLevel - Zoom level as percentage (default: 100)
 * @returns Estimated height in pixels
 */
export function estimateElementHeight(
  element: HTMLElement,
  textConfig: TextConfig,
  contentWidth: number,
  zoomLevel: number = 100
): number {
  const heights = calculateElementHeights(textConfig, zoomLevel);
  const tagName = element.tagName.toLowerCase();
  const scaledFontSize = (textConfig.baseFontSize * zoomLevel) / 100;

  // Get font size multiplier based on element type
  let fontSizeMultiplier = 1;
  let marginBottom = textConfig.paragraphSpacing * scaledFontSize;

  switch (tagName) {
    case 'h1':
      fontSizeMultiplier = 2;
      marginBottom = 0.5 * scaledFontSize;
      break;
    case 'h2':
      fontSizeMultiplier = 1.5;
      marginBottom = 0.5 * scaledFontSize;
      break;
    case 'h3':
      fontSizeMultiplier = 1.17;
      marginBottom = 0.5 * scaledFontSize;
      break;
    case 'h4':
      fontSizeMultiplier = 1;
      marginBottom = 0.5 * scaledFontSize;
      break;
    case 'blockquote':
      marginBottom = scaledFontSize;
      break;
    case 'ul':
    case 'ol':
      // Count list items
      const items = element.querySelectorAll('li');
      return items.length * heights.listItem + marginBottom;
    case 'li':
      return heights.listItem;
  }

  // Calculate number of lines based on text content and width
  const text = element.textContent || '';
  const effectiveFontSize = scaledFontSize * fontSizeMultiplier;
  const avgCharWidth = effectiveFontSize * 0.5; // Approximate average character width
  const charsPerLine = Math.floor(contentWidth / avgCharWidth);
  const numLines = Math.max(1, Math.ceil(text.length / charsPerLine));

  const lineHeight = effectiveFontSize * textConfig.lineHeight;
  return numLines * lineHeight + marginBottom;
}
