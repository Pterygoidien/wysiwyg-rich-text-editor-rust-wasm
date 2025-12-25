import { writable, derived } from 'svelte/store';
import type { PageConfig } from './types';
import { PAGE_FORMATS, DEFAULT_MARGINS, getPageDimensions, getContentDimensions } from './types';

// Page configuration store
export const pageConfig = writable<PageConfig>({
  format: PAGE_FORMATS.A4,
  margins: DEFAULT_MARGINS,
  orientation: 'portrait',
});

// Zoom level (percentage)
export const zoomLevel = writable<number>(100);

// Derived store for page dimensions in pixels
export const pageDimensions = derived(pageConfig, ($config) => getPageDimensions($config));

// Derived store for content area dimensions
export const contentDimensions = derived(pageConfig, ($config) => getContentDimensions($config));

// Current page number (1-indexed)
export const currentPage = writable<number>(1);

// Total page count
export const totalPages = writable<number>(1);
