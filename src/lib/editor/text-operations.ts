/**
 * @fileoverview Text manipulation operations for the rich text editor.
 *
 * This module provides pure functions for text operations like insertion,
 * deletion, and paragraph manipulation. These functions operate on
 * document data structures without side effects.
 *
 * @module editor/text-operations
 */

import type { ParagraphMeta, TextPosition } from './types';
import { PAGE_BREAK_MARKER, createDefaultMeta } from './types';

/**
 * Result of a text insertion operation.
 */
export interface InsertResult {
  /** Updated paragraphs array */
  paragraphs: string[];
  /** Updated paragraph metadata array */
  paragraphMeta: ParagraphMeta[];
  /** New cursor paragraph index */
  cursorPara: number;
  /** New cursor offset within paragraph */
  cursorOffset: number;
}

/**
 * Result of a text deletion operation.
 */
export interface DeleteResult {
  /** Updated paragraphs array */
  paragraphs: string[];
  /** Updated paragraph metadata array */
  paragraphMeta: ParagraphMeta[];
  /** New cursor paragraph index */
  cursorPara: number;
  /** New cursor offset within paragraph */
  cursorOffset: number;
  /** Text that was deleted */
  deletedText: string;
}

/**
 * Inserts text at a specific position in the document.
 * Handles newlines by splitting paragraphs.
 *
 * @param paragraphs - Current paragraphs array
 * @param paragraphMeta - Current paragraph metadata
 * @param para - Target paragraph index
 * @param offset - Character offset within the paragraph
 * @param text - Text to insert
 * @returns Result with updated document and new cursor position
 *
 * @example
 * ```typescript
 * const result = insertText(
 *   ['Hello world'],
 *   [defaultMeta],
 *   0, 5,
 *   ' beautiful'
 * );
 * // result.paragraphs = ['Hello beautiful world']
 * // result.cursorOffset = 15
 * ```
 */
export function insertText(
  paragraphs: string[],
  paragraphMeta: ParagraphMeta[],
  para: number,
  offset: number,
  text: string
): InsertResult {
  const newParagraphs = [...paragraphs];
  const newMeta = [...paragraphMeta];
  let cursorPara = para;
  let cursorOffset = offset;

  // Handle newlines by splitting into multiple insertions
  const lines = text.split('\n');

  if (lines.length === 1) {
    // Simple insertion - no newlines
    const currentText = newParagraphs[para] || '';
    newParagraphs[para] =
      currentText.substring(0, offset) + text + currentText.substring(offset);
    cursorOffset = offset + text.length;
  } else {
    // Multi-line insertion
    const currentText = newParagraphs[para] || '';
    const beforeCursor = currentText.substring(0, offset);
    const afterCursor = currentText.substring(offset);

    // First line: append to current paragraph
    newParagraphs[para] = beforeCursor + lines[0];

    // Middle lines: insert as new paragraphs
    const currentMeta = newMeta[para] || createDefaultMeta();
    for (let i = 1; i < lines.length - 1; i++) {
      newParagraphs.splice(para + i, 0, lines[i]);
      newMeta.splice(para + i, 0, { ...currentMeta });
    }

    // Last line: create new paragraph with remainder
    const lastLineIndex = para + lines.length - 1;
    newParagraphs.splice(lastLineIndex, 0, lines[lines.length - 1] + afterCursor);
    newMeta.splice(lastLineIndex, 0, { ...currentMeta });

    cursorPara = lastLineIndex;
    cursorOffset = lines[lines.length - 1].length;
  }

  return {
    paragraphs: newParagraphs,
    paragraphMeta: newMeta,
    cursorPara,
    cursorOffset,
  };
}

/**
 * Inserts a page break at the current cursor position.
 *
 * @param paragraphs - Current paragraphs array
 * @param paragraphMeta - Current paragraph metadata
 * @param para - Target paragraph index
 * @param offset - Character offset within the paragraph
 * @returns Result with updated document and new cursor position
 */
export function insertPageBreak(
  paragraphs: string[],
  paragraphMeta: ParagraphMeta[],
  para: number,
  offset: number
): InsertResult {
  const newParagraphs = [...paragraphs];
  const newMeta = [...paragraphMeta];

  const currentText = newParagraphs[para] || '';
  const beforeCursor = currentText.substring(0, offset);
  const afterCursor = currentText.substring(offset);
  const currentMeta = newMeta[para] || createDefaultMeta();

  // Set current paragraph to text before cursor
  newParagraphs[para] = beforeCursor;

  // Insert page break paragraph
  newParagraphs.splice(para + 1, 0, PAGE_BREAK_MARKER);
  newMeta.splice(para + 1, 0, createDefaultMeta());

  // Insert paragraph with text after cursor
  newParagraphs.splice(para + 2, 0, afterCursor);
  newMeta.splice(para + 2, 0, { ...currentMeta });

  return {
    paragraphs: newParagraphs,
    paragraphMeta: newMeta,
    cursorPara: para + 2,
    cursorOffset: 0,
  };
}

/**
 * Gets the selected text from a document.
 *
 * @param paragraphs - Document paragraphs
 * @param start - Selection start position
 * @param end - Selection end position
 * @returns Selected text as a string with newlines for paragraph breaks
 *
 * @example
 * ```typescript
 * const text = getSelectedText(
 *   ['Hello', 'World'],
 *   { para: 0, offset: 3 },
 *   { para: 1, offset: 2 }
 * );
 * // Returns: 'lo\nWo'
 * ```
 */
export function getSelectedText(
  paragraphs: string[],
  start: TextPosition,
  end: TextPosition
): string {
  // Normalize selection order
  let selStart = start;
  let selEnd = end;
  if (
    start.para > end.para ||
    (start.para === end.para && start.offset > end.offset)
  ) {
    selStart = end;
    selEnd = start;
  }

  if (selStart.para === selEnd.para) {
    // Single paragraph selection
    return paragraphs[selStart.para].substring(selStart.offset, selEnd.offset);
  }

  // Multi-paragraph selection
  let result = paragraphs[selStart.para].substring(selStart.offset);

  for (let i = selStart.para + 1; i < selEnd.para; i++) {
    result += '\n' + paragraphs[i];
  }

  result += '\n' + paragraphs[selEnd.para].substring(0, selEnd.offset);

  return result;
}

/**
 * Deletes the selected text from a document.
 *
 * @param paragraphs - Document paragraphs
 * @param paragraphMeta - Paragraph metadata
 * @param start - Selection start position
 * @param end - Selection end position
 * @returns Result with updated document, new cursor position, and deleted text
 *
 * @example
 * ```typescript
 * const result = deleteSelection(
 *   ['Hello', 'World'],
 *   [meta1, meta2],
 *   { para: 0, offset: 3 },
 *   { para: 1, offset: 2 }
 * );
 * // result.paragraphs = ['Helrld']
 * // result.cursorPara = 0, result.cursorOffset = 3
 * ```
 */
export function deleteSelection(
  paragraphs: string[],
  paragraphMeta: ParagraphMeta[],
  start: TextPosition,
  end: TextPosition
): DeleteResult {
  // Normalize selection order
  let selStart = start;
  let selEnd = end;
  if (
    start.para > end.para ||
    (start.para === end.para && start.offset > end.offset)
  ) {
    selStart = end;
    selEnd = start;
  }

  const deletedText = getSelectedText(paragraphs, selStart, selEnd);
  const newParagraphs = [...paragraphs];
  const newMeta = [...paragraphMeta];

  if (selStart.para === selEnd.para) {
    // Single paragraph deletion
    const text = newParagraphs[selStart.para];
    newParagraphs[selStart.para] =
      text.substring(0, selStart.offset) + text.substring(selEnd.offset);
  } else {
    // Multi-paragraph deletion
    const startText = newParagraphs[selStart.para].substring(0, selStart.offset);
    const endText = newParagraphs[selEnd.para].substring(selEnd.offset);

    // Merge first and last paragraphs
    newParagraphs[selStart.para] = startText + endText;

    // Remove intermediate paragraphs
    const removeCount = selEnd.para - selStart.para;
    newParagraphs.splice(selStart.para + 1, removeCount);
    newMeta.splice(selStart.para + 1, removeCount);
  }

  return {
    paragraphs: newParagraphs,
    paragraphMeta: newMeta,
    cursorPara: selStart.para,
    cursorOffset: selStart.offset,
    deletedText,
  };
}

/**
 * Merges two adjacent paragraphs.
 *
 * @param paragraphs - Document paragraphs
 * @param paragraphMeta - Paragraph metadata
 * @param firstPara - Index of the first paragraph
 * @returns Result with updated document and cursor at merge point
 */
export function mergeParagraphs(
  paragraphs: string[],
  paragraphMeta: ParagraphMeta[],
  firstPara: number
): InsertResult {
  if (firstPara < 0 || firstPara >= paragraphs.length - 1) {
    return {
      paragraphs,
      paragraphMeta,
      cursorPara: firstPara,
      cursorOffset: paragraphs[firstPara]?.length || 0,
    };
  }

  const newParagraphs = [...paragraphs];
  const newMeta = [...paragraphMeta];

  const mergePoint = newParagraphs[firstPara].length;
  newParagraphs[firstPara] += newParagraphs[firstPara + 1];
  newParagraphs.splice(firstPara + 1, 1);
  newMeta.splice(firstPara + 1, 1);

  return {
    paragraphs: newParagraphs,
    paragraphMeta: newMeta,
    cursorPara: firstPara,
    cursorOffset: mergePoint,
  };
}

/**
 * Splits a paragraph at the cursor position.
 *
 * @param paragraphs - Document paragraphs
 * @param paragraphMeta - Paragraph metadata
 * @param para - Paragraph to split
 * @param offset - Character offset where to split
 * @param inheritMeta - Whether to copy metadata to new paragraph
 * @returns Result with updated document and cursor at start of new paragraph
 */
export function splitParagraph(
  paragraphs: string[],
  paragraphMeta: ParagraphMeta[],
  para: number,
  offset: number,
  inheritMeta: boolean = true
): InsertResult {
  const newParagraphs = [...paragraphs];
  const newMeta = [...paragraphMeta];

  const currentText = newParagraphs[para] || '';
  const currentMeta = newMeta[para] || createDefaultMeta();

  const beforeCursor = currentText.substring(0, offset);
  const afterCursor = currentText.substring(offset);

  newParagraphs[para] = beforeCursor;
  newParagraphs.splice(para + 1, 0, afterCursor);

  if (inheritMeta) {
    newMeta.splice(para + 1, 0, { ...currentMeta });
  } else {
    newMeta.splice(para + 1, 0, createDefaultMeta());
  }

  return {
    paragraphs: newParagraphs,
    paragraphMeta: newMeta,
    cursorPara: para + 1,
    cursorOffset: 0,
  };
}

/**
 * Deletes a single character before the cursor (backspace).
 *
 * @param paragraphs - Document paragraphs
 * @param paragraphMeta - Paragraph metadata
 * @param para - Current paragraph index
 * @param offset - Current cursor offset
 * @returns Result with updated document and new cursor position
 */
export function deleteCharBefore(
  paragraphs: string[],
  paragraphMeta: ParagraphMeta[],
  para: number,
  offset: number
): DeleteResult {
  const newParagraphs = [...paragraphs];
  const newMeta = [...paragraphMeta];

  if (offset > 0) {
    // Delete character in current paragraph
    const text = newParagraphs[para];
    const deletedChar = text[offset - 1];
    newParagraphs[para] = text.substring(0, offset - 1) + text.substring(offset);

    return {
      paragraphs: newParagraphs,
      paragraphMeta: newMeta,
      cursorPara: para,
      cursorOffset: offset - 1,
      deletedText: deletedChar,
    };
  } else if (para > 0) {
    // Merge with previous paragraph
    const mergeResult = mergeParagraphs(newParagraphs, newMeta, para - 1);
    return {
      ...mergeResult,
      deletedText: '\n',
    };
  }

  // At start of document - nothing to delete
  return {
    paragraphs: newParagraphs,
    paragraphMeta: newMeta,
    cursorPara: para,
    cursorOffset: offset,
    deletedText: '',
  };
}

/**
 * Deletes a single character after the cursor (delete key).
 *
 * @param paragraphs - Document paragraphs
 * @param paragraphMeta - Paragraph metadata
 * @param para - Current paragraph index
 * @param offset - Current cursor offset
 * @returns Result with updated document and new cursor position
 */
export function deleteCharAfter(
  paragraphs: string[],
  paragraphMeta: ParagraphMeta[],
  para: number,
  offset: number
): DeleteResult {
  const newParagraphs = [...paragraphs];
  const newMeta = [...paragraphMeta];
  const text = newParagraphs[para] || '';

  if (offset < text.length) {
    // Delete character in current paragraph
    const deletedChar = text[offset];
    newParagraphs[para] = text.substring(0, offset) + text.substring(offset + 1);

    return {
      paragraphs: newParagraphs,
      paragraphMeta: newMeta,
      cursorPara: para,
      cursorOffset: offset,
      deletedText: deletedChar,
    };
  } else if (para < paragraphs.length - 1) {
    // Merge with next paragraph
    const mergeResult = mergeParagraphs(newParagraphs, newMeta, para);
    return {
      paragraphs: mergeResult.paragraphs,
      paragraphMeta: mergeResult.paragraphMeta,
      cursorPara: para,
      cursorOffset: offset,
      deletedText: '\n',
    };
  }

  // At end of document - nothing to delete
  return {
    paragraphs: newParagraphs,
    paragraphMeta: newMeta,
    cursorPara: para,
    cursorOffset: offset,
    deletedText: '',
  };
}

/**
 * Applies formatting metadata to a range of paragraphs.
 *
 * @param paragraphMeta - Current paragraph metadata
 * @param startPara - First paragraph to format
 * @param endPara - Last paragraph to format (inclusive)
 * @param updates - Partial metadata to apply
 * @returns Updated paragraph metadata array
 */
export function applyFormatting(
  paragraphMeta: ParagraphMeta[],
  startPara: number,
  endPara: number,
  updates: Partial<ParagraphMeta>
): ParagraphMeta[] {
  const newMeta = [...paragraphMeta];

  for (let i = startPara; i <= endPara; i++) {
    newMeta[i] = {
      ...(newMeta[i] || createDefaultMeta()),
      ...updates,
    };
  }

  return newMeta;
}
