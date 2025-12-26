/**
 * @fileoverview Svelte stores for shared application state.
 *
 * This module provides reactive stores for all shared state in the editor,
 * including page configuration, typography settings, zoom level, and navigation.
 * Stores use Svelte's writable and derived store primitives.
 *
 * @module stores
 *
 * @example
 * ```typescript
 * import { pageConfig, zoomLevel, fontSize } from './stores';
 *
 * // Subscribe to store changes
 * $: currentZoom = $zoomLevel;
 *
 * // Update store values
 * zoomLevel.set(150);
 * fontSize.update(size => size + 2);
 * ```
 */

import { writable, derived } from 'svelte/store';
import type { PageConfig } from './types';
import {
  PAGE_FORMATS,
  DEFAULT_MARGINS,
  getPageDimensions,
  getContentDimensions,
  getColumnWidth,
} from './types';

// ============================================================================
// Types
// ============================================================================

/**
 * Represents a heading in the document for navigation purposes.
 * Used by the sidebar to display the document outline.
 */
export interface HeadingItem {
  /** Unique identifier for the heading */
  id: string;
  /** The heading text content */
  text: string;
  /** Heading level (1-4 corresponding to h1-h4) */
  level: number;
  /** Index of the paragraph containing this heading */
  paraIndex: number;
}

/**
 * Preset option for spacing controls.
 */
export interface SpacingOption {
  /** Numeric value of the spacing */
  value: number;
  /** Display label for the option */
  label: string;
}

// ============================================================================
// Navigation Stores
// ============================================================================

/**
 * Store containing all headings in the document.
 * Used by the sidebar component for document outline navigation.
 * Updated by the Editor component when content changes.
 *
 * @example
 * ```typescript
 * import { headings } from './stores';
 *
 * // Read headings in a component
 * $: headingList = $headings;
 *
 * // Update headings (typically done by Editor)
 * headings.set([
 *   { id: 'h-0', text: 'Introduction', level: 1, paraIndex: 0 },
 *   { id: 'h-1', text: 'Getting Started', level: 2, paraIndex: 5 },
 * ]);
 * ```
 */
export const headings = writable<HeadingItem[]>([]);

/**
 * Current page number (1-indexed).
 * Updated when the user scrolls or clicks in the document.
 */
export const currentPage = writable<number>(1);

/**
 * Total number of pages in the document.
 * Updated by the Editor when content or layout changes.
 */
export const totalPages = writable<number>(1);

// ============================================================================
// Page Configuration Stores
// ============================================================================

/**
 * Main page configuration store.
 * Contains all page layout settings including format, margins, orientation, and columns.
 *
 * @example
 * ```typescript
 * import { pageConfig } from './stores';
 *
 * // Change page format
 * pageConfig.update(config => ({
 *   ...config,
 *   format: PAGE_FORMATS.LETTER,
 *   orientation: 'landscape',
 * }));
 *
 * // Set columns
 * pageConfig.update(config => ({
 *   ...config,
 *   columns: 2,
 *   columnGap: 15,
 * }));
 * ```
 */
export const pageConfig = writable<PageConfig>({
  format: PAGE_FORMATS.A4,
  margins: DEFAULT_MARGINS,
  orientation: 'portrait',
  columns: 1,
  columnGap: 10, // 10mm gap between columns
});

/**
 * Current zoom level as a percentage.
 * Default is 100 (100%). Valid range is 25-200.
 *
 * @example
 * ```typescript
 * import { zoomLevel } from './stores';
 *
 * // Set zoom to 150%
 * zoomLevel.set(150);
 *
 * // Increase zoom by 10%
 * zoomLevel.update(z => Math.min(200, z + 10));
 * ```
 */
export const zoomLevel = writable<number>(100);

/**
 * Derived store for page dimensions in pixels.
 * Automatically updates when pageConfig changes.
 * Accounts for page orientation.
 */
export const pageDimensions = derived(pageConfig, ($config) =>
  getPageDimensions($config)
);

/**
 * Derived store for content area dimensions in pixels.
 * This is the page size minus margins - the actual editable area.
 * Automatically updates when pageConfig changes.
 */
export const contentDimensions = derived(pageConfig, ($config) =>
  getContentDimensions($config)
);

/**
 * Derived store for single column width in pixels.
 * Accounts for multi-column layouts and column gaps.
 * Automatically updates when pageConfig changes.
 */
export const columnWidth = derived(pageConfig, ($config) =>
  getColumnWidth($config)
);

// ============================================================================
// Typography Stores
// ============================================================================

/**
 * Base font size in pixels.
 * This is the default font size for normal paragraphs.
 * Headings are scaled relative to this value.
 */
export const fontSize = writable<number>(16);

/**
 * Font family name.
 * Must be a font available on the system or loaded via CSS.
 */
export const fontFamily = writable<string>('Arial');

/**
 * Line height multiplier.
 * Applied to the font size to calculate actual line height.
 * For example, 1.5 means line height is 1.5x the font size.
 */
export const lineHeight = writable<number>(1.5);

/**
 * Letter spacing in pixels.
 * Added between each character. Can be negative for tighter text.
 */
export const letterSpacing = writable<number>(0);

/**
 * Paragraph spacing in pixels.
 * Added after each paragraph as vertical space.
 */
export const paragraphSpacing = writable<number>(12);

// ============================================================================
// Constants
// ============================================================================

/**
 * Available font sizes for the font size selector.
 * Standard typographic sizes from 8pt to 72pt.
 */
export const FONT_SIZES: readonly number[] = [
  8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 72,
] as const;

/**
 * Available font families for the font family selector.
 * Common web-safe fonts available on most systems.
 */
export const FONT_FAMILIES: readonly string[] = [
  'Arial',
  'Times New Roman',
  'Georgia',
  'Verdana',
  'Courier New',
  'Trebuchet MS',
  'Comic Sans MS',
  'Impact',
  'Palatino Linotype',
  'Garamond',
] as const;

/**
 * Preset line height options for the spacing menu.
 * Common line height values with descriptive labels.
 */
export const LINE_HEIGHT_OPTIONS: readonly SpacingOption[] = [
  { value: 1.0, label: 'Single' },
  { value: 1.15, label: '1.15' },
  { value: 1.5, label: '1.5' },
  { value: 2.0, label: 'Double' },
  { value: 2.5, label: '2.5' },
  { value: 3.0, label: 'Triple' },
] as const;

/**
 * Preset letter spacing options for the spacing menu.
 * Ranges from tight (-1px) to widest (2px).
 */
export const LETTER_SPACING_OPTIONS: readonly SpacingOption[] = [
  { value: -1, label: 'Tight' },
  { value: 0, label: 'Normal' },
  { value: 0.5, label: 'Wide' },
  { value: 1, label: 'Wider' },
  { value: 2, label: 'Widest' },
] as const;

/**
 * Preset paragraph spacing options for the spacing menu.
 * Space added after each paragraph in pixels.
 */
export const PARAGRAPH_SPACING_OPTIONS: readonly SpacingOption[] = [
  { value: 0, label: 'None' },
  { value: 6, label: 'Small' },
  { value: 12, label: 'Medium' },
  { value: 18, label: 'Large' },
  { value: 24, label: 'Extra Large' },
] as const;
