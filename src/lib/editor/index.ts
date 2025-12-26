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
 *   type ParagraphMeta,
 *   type DisplayLine,
 *   IMAGE_MARKER,
 *   createTextMeasurer,
 *   wrapParagraph
 * } from './editor';
 * ```
 */

export * from './types';
export * from './text-measurement';
