import { writable, derived } from 'svelte/store';
import type { PageConfig } from './types';
import { PAGE_FORMATS, DEFAULT_MARGINS, getPageDimensions, getContentDimensions, getColumnWidth } from './types';

// Heading structure for navigation
export interface HeadingItem {
  id: string;
  text: string;
  level: number; // 1-4 for h1-h4
  paraIndex: number;
}

// Headings store for navigation sidebar
export const headings = writable<HeadingItem[]>([]);

// Page configuration store
export const pageConfig = writable<PageConfig>({
  format: PAGE_FORMATS.A4,
  margins: DEFAULT_MARGINS,
  orientation: 'portrait',
  columns: 1,
  columnGap: 10, // 10mm gap between columns
});

// Zoom level (percentage)
export const zoomLevel = writable<number>(100);

// Derived store for page dimensions in pixels
export const pageDimensions = derived(pageConfig, ($config) => getPageDimensions($config));

// Derived store for content area dimensions
export const contentDimensions = derived(pageConfig, ($config) => getContentDimensions($config));

// Derived store for column width
export const columnWidth = derived(pageConfig, ($config) => getColumnWidth($config));

// Current page number (1-indexed)
export const currentPage = writable<number>(1);

// Total page count
export const totalPages = writable<number>(1);

// Font settings
export const fontSize = writable<number>(16);
export const fontFamily = writable<string>('Arial');

// Common font sizes
export const FONT_SIZES = [8, 9, 10, 11, 12, 14, 16, 18, 20, 24, 28, 32, 36, 48, 72];

// Common font families
export const FONT_FAMILIES = [
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
];

// Spacing settings
export const lineHeight = writable<number>(1.5); // multiplier
export const letterSpacing = writable<number>(0); // in px
export const paragraphSpacing = writable<number>(12); // in px (space after paragraph)

// Preset line height options
export const LINE_HEIGHT_OPTIONS = [
  { value: 1.0, label: 'Single' },
  { value: 1.15, label: '1.15' },
  { value: 1.5, label: '1.5' },
  { value: 2.0, label: 'Double' },
  { value: 2.5, label: '2.5' },
  { value: 3.0, label: 'Triple' },
];

// Preset letter spacing options
export const LETTER_SPACING_OPTIONS = [
  { value: -1, label: 'Tight' },
  { value: 0, label: 'Normal' },
  { value: 0.5, label: 'Wide' },
  { value: 1, label: 'Wider' },
  { value: 2, label: 'Widest' },
];

// Preset paragraph spacing options (after paragraph)
export const PARAGRAPH_SPACING_OPTIONS = [
  { value: 0, label: 'None' },
  { value: 6, label: 'Small' },
  { value: 12, label: 'Medium' },
  { value: 18, label: 'Large' },
  { value: 24, label: 'Extra Large' },
];
