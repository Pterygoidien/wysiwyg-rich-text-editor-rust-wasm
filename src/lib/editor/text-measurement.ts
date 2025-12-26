/**
 * @fileoverview Text measurement and line wrapping utilities for the canvas-based editor.
 *
 * This module provides functions for measuring text dimensions and wrapping
 * paragraphs into display lines that fit within content columns. It handles:
 * - Font style composition
 * - Text width measurement with letter spacing
 * - Word-boundary aware line wrapping
 * - List indentation and float reduction calculations
 *
 * @module editor/text-measurement
 */

import type { ParagraphMeta, DisplayLine } from './types';

/**
 * Configuration parameters for text measurement operations.
 * These values are typically derived from editor state and zoom level.
 */
export interface MeasurementConfig {
  /** Base font size in scaled pixels */
  fontSize: number;
  /** Font family name */
  fontFamily: string;
  /** Whether bold styling is applied */
  isBold: boolean;
  /** Whether italic styling is applied */
  isItalic: boolean;
  /** Letter spacing in scaled pixels */
  letterSpacing: number;
  /** Current zoom level (percentage) */
  zoomLevel: number;
}

/**
 * Configuration for paragraph wrapping operations.
 * Extends measurement config with layout-specific parameters.
 */
export interface WrapConfig extends MeasurementConfig {
  /** Available content width for text */
  contentWidth: number;
  /** Width of the column (for multi-column layouts) */
  columnWidth: number;
}

/**
 * Creates a text measurement context using an off-screen canvas.
 *
 * The returned object provides methods for measuring text dimensions
 * consistently across the editor. Using a dedicated canvas avoids
 * interference with the visible rendering canvases.
 *
 * @returns Measurement context with getFontStyle and measureTextWidth methods
 *
 * @example
 * ```typescript
 * const measurer = createTextMeasurer();
 * const config = { fontSize: 16, fontFamily: 'Arial', ... };
 * const width = measurer.measureTextWidth('Hello', config);
 * ```
 */
export function createTextMeasurer() {
  const canvas = document.createElement('canvas');
  const ctx = canvas.getContext('2d');

  /**
   * Composes a CSS font string from configuration parameters.
   *
   * @param config - Measurement configuration
   * @returns CSS font shorthand string (e.g., "italic bold 16px Arial")
   */
  function getFontStyle(config: MeasurementConfig): string {
    const { fontSize, fontFamily, isBold, isItalic } = config;
    return `${isItalic ? 'italic ' : ''}${isBold ? 'bold ' : ''}${fontSize}px ${fontFamily}`;
  }

  /**
   * Measures the rendered width of text including letter spacing.
   *
   * This function accounts for letter spacing by adding the configured
   * spacing value between each character (but not after the last one).
   *
   * @param text - The text string to measure
   * @param config - Measurement configuration
   * @returns Width in pixels, or a fallback estimate if context unavailable
   */
  function measureTextWidth(text: string, config: MeasurementConfig): number {
    if (!ctx) {
      return text.length * config.fontSize * 0.5;
    }

    ctx.font = getFontStyle(config);
    const baseWidth = ctx.measureText(text).width;
    const spacingTotal = text.length > 0 ? (text.length - 1) * config.letterSpacing : 0;

    return baseWidth + spacingTotal;
  }

  return {
    getFontStyle,
    measureTextWidth,
    canvas,
  };
}

/**
 * Wraps a paragraph into multiple display lines that fit within the content width.
 *
 * This function implements a greedy line-breaking algorithm that:
 * 1. Checks if the remaining text fits on one line
 * 2. If not, finds the optimal break point (preferring word boundaries)
 * 3. Handles edge cases like very long words that must be force-broken
 * 4. Skips leading spaces on continuation lines
 *
 * @param paraIndex - Index of the paragraph in the document
 * @param text - The paragraph text content
 * @param meta - Paragraph formatting metadata
 * @param measureFn - Function to measure text width
 * @param config - Wrap configuration with layout parameters
 * @param listNumber - Optional sequence number for numbered lists
 * @param floatReduction - Optional width reduction for floating images
 * @returns Array of DisplayLine objects representing wrapped lines
 *
 * @example
 * ```typescript
 * const lines = wrapParagraph(
 *   0,
 *   'This is a long paragraph that needs wrapping.',
 *   { align: 'left', listType: 'none', blockType: 'p', indent: 0 },
 *   (text) => measurer.measureTextWidth(text, measureConfig),
 *   { contentWidth: 500, columnWidth: 500, ...measureConfig }
 * );
 * ```
 */
export function wrapParagraph(
  paraIndex: number,
  text: string,
  meta: ParagraphMeta,
  measureFn: (text: string) => number,
  config: WrapConfig,
  listNumber?: number,
  floatReduction?: { side: 'left' | 'right'; width: number }
): DisplayLine[] {
  const listIndent = meta.listType !== 'none' ? config.fontSize * 1.5 : 0;
  const floatWidth = floatReduction ? floatReduction.width + 10 : 0;
  const baseContentWidth = config.columnWidth > 0 ? config.columnWidth : (config.contentWidth > 0 ? config.contentWidth : 500);
  const effectiveWidth = baseContentWidth - listIndent - floatWidth;

  if (!text) {
    return [{
      paraIndex,
      startOffset: 0,
      endOffset: 0,
      text: '',
      meta,
      listNumber,
      floatReduction,
    }];
  }

  const result: DisplayLine[] = [];
  let currentStart = 0;

  while (currentStart < text.length) {
    const remainingText = text.substring(currentStart);

    if (measureFn(remainingText) <= effectiveWidth) {
      result.push({
        paraIndex,
        startOffset: currentStart,
        endOffset: text.length,
        text: remainingText,
        meta,
        listNumber: result.length === 0 ? listNumber : undefined,
        floatReduction,
      });
      break;
    }

    let lineEnd = currentStart;
    let lastWordBoundary = currentStart;

    for (let i = currentStart + 1; i <= text.length; i++) {
      const testText = text.substring(currentStart, i);
      const width = measureFn(testText);

      if (i > currentStart && text[i - 1] === ' ') {
        lastWordBoundary = i;
      }

      if (width > effectiveWidth) {
        if (lastWordBoundary > currentStart) {
          lineEnd = lastWordBoundary;
        } else {
          lineEnd = Math.max(currentStart + 1, i - 1);
        }
        break;
      }

      lineEnd = i;
    }

    if (lineEnd <= currentStart) {
      lineEnd = currentStart + 1;
    }

    result.push({
      paraIndex,
      startOffset: currentStart,
      endOffset: lineEnd,
      text: text.substring(currentStart, lineEnd),
      meta,
      listNumber: result.length === 0 ? listNumber : undefined,
      floatReduction,
    });

    currentStart = lineEnd;

    while (currentStart < text.length && text[currentStart] === ' ') {
      currentStart++;
    }
  }

  return result;
}

/**
 * Calculates the font size for a specific block type.
 *
 * Heading block types (h1-h4) have scaling multipliers applied
 * to the base font size. Other block types use the base size.
 *
 * @param baseFontSize - The base font size in pixels
 * @param blockType - The block type (p, h1, h2, h3, h4, blockquote)
 * @returns Scaled font size in pixels
 */
export function getBlockFontSize(baseFontSize: number, blockType: string): number {
  switch (blockType) {
    case 'h1':
      return baseFontSize * 2;
    case 'h2':
      return baseFontSize * 1.5;
    case 'h3':
      return baseFontSize * 1.17;
    default:
      return baseFontSize;
  }
}

/**
 * Determines font weight for a block type.
 *
 * @param blockType - The block type
 * @param baseIsBold - Whether the base text is bold
 * @returns CSS font-weight value ('bold ' with trailing space, or '')
 */
export function getBlockFontWeight(blockType: string, baseIsBold: boolean): string {
  switch (blockType) {
    case 'h1':
    case 'h2':
    case 'h3':
    case 'h4':
      return 'bold ';
    default:
      return baseIsBold ? 'bold ' : '';
  }
}

/**
 * Determines font style for a block type.
 *
 * @param blockType - The block type
 * @param baseIsItalic - Whether the base text is italic
 * @returns CSS font-style value ('italic ' with trailing space, or '')
 */
export function getBlockFontStyle(blockType: string, baseIsItalic: boolean): string {
  if (blockType === 'blockquote') {
    return 'italic ';
  }
  return baseIsItalic ? 'italic ' : '';
}
