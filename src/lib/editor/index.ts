/**
 * @fileoverview Editor module entry point.
 *
 * Re-exports all public types, constants, and utilities from the editor module.
 * Import from this file for cleaner imports in consuming code.
 *
 * @module editor
 *
 * @example
 * ```typescript
 * import {
 *   // Types
 *   type ParagraphMeta,
 *   type DisplayLine,
 *   type DocumentImage,
 *   type SelectionState,
 *   type FormatCommand,
 *
 *   // Constants
 *   IMAGE_MARKER,
 *   PAGE_BREAK_MARKER,
 *
 *   // Text measurement
 *   createTextMeasurer,
 *   wrapParagraph,
 *
 *   // Layout
 *   paraToDisplayPos,
 *   displayToPara,
 *   extractHeadings,
 *
 *   // Text operations
 *   insertText,
 *   deleteSelection,
 *   getSelectedText,
 *
 *   // Selection
 *   normalizeSelection,
 *   hasSelection,
 *
 *   // Commands
 *   isTextStyleCommand,
 *   isAlignmentCommand,
 *   ALIGNMENT_MAP,
 * } from './editor';
 * ```
 */

// Core types and constants
export * from './types';

// Text measurement utilities
export * from './text-measurement';

// Layout engine utilities
export * from './layout-engine';

// Text manipulation operations
export * from './text-operations';

// Selection management
export * from './selection-manager';

// Command types and utilities
export * from './commands';

// Keyboard event handling
export * from './keyboard-handler';

// Image management
export * from './image-manager';

// Canvas rendering utilities
export * from './canvas-renderer';
