/**
 * @fileoverview Selection and cursor management for the rich text editor.
 *
 * This module provides utilities for managing text selection state,
 * cursor positioning, and selection-related calculations.
 *
 * @module editor/selection-manager
 */

import type { TextPosition, DisplayLine } from './types';

/**
 * Selection state containing cursor and selection range.
 */
export interface SelectionState {
  /** Current cursor paragraph index */
  cursorPara: number;
  /** Current cursor offset within paragraph */
  cursorOffset: number;
  /** Selection start position (null if no selection) */
  selectionStart: TextPosition | null;
  /** Selection end position (null if no selection) */
  selectionEnd: TextPosition | null;
}

/**
 * Normalized selection with guaranteed start before end.
 */
export interface NormalizedSelection {
  /** Start position (always before or equal to end) */
  start: TextPosition;
  /** End position (always after or equal to start) */
  end: TextPosition;
  /** Whether selection was reversed during normalization */
  reversed: boolean;
}

/**
 * Creates a default selection state with cursor at document start.
 *
 * @returns Initial selection state
 */
export function createDefaultSelectionState(): SelectionState {
  return {
    cursorPara: 0,
    cursorOffset: 0,
    selectionStart: null,
    selectionEnd: null,
  };
}

/**
 * Compares two text positions.
 *
 * @param a - First position
 * @param b - Second position
 * @returns Negative if a < b, positive if a > b, zero if equal
 *
 * @example
 * ```typescript
 * comparePositions({ para: 0, offset: 5 }, { para: 1, offset: 0 }); // -1
 * comparePositions({ para: 1, offset: 5 }, { para: 1, offset: 3 }); // 1
 * ```
 */
export function comparePositions(a: TextPosition, b: TextPosition): number {
  if (a.para !== b.para) {
    return a.para - b.para;
  }
  return a.offset - b.offset;
}

/**
 * Normalizes a selection so start is always before end.
 *
 * @param start - Selection start (may be after end)
 * @param end - Selection end (may be before start)
 * @returns Normalized selection with start before end
 *
 * @example
 * ```typescript
 * const normalized = normalizeSelection(
 *   { para: 1, offset: 5 },
 *   { para: 0, offset: 3 }
 * );
 * // normalized.start = { para: 0, offset: 3 }
 * // normalized.end = { para: 1, offset: 5 }
 * // normalized.reversed = true
 * ```
 */
export function normalizeSelection(
  start: TextPosition,
  end: TextPosition
): NormalizedSelection {
  const cmp = comparePositions(start, end);
  if (cmp <= 0) {
    return { start, end, reversed: false };
  }
  return { start: end, end: start, reversed: true };
}

/**
 * Checks if there is an active selection (not just a cursor).
 *
 * @param state - Selection state to check
 * @returns True if there is a selection range
 */
export function hasSelection(state: SelectionState): boolean {
  if (!state.selectionStart || !state.selectionEnd) {
    return false;
  }
  return comparePositions(state.selectionStart, state.selectionEnd) !== 0;
}

/**
 * Clears the selection, keeping only the cursor.
 *
 * @param state - Current selection state
 * @returns New state with no selection
 */
export function clearSelection(state: SelectionState): SelectionState {
  return {
    ...state,
    selectionStart: null,
    selectionEnd: null,
  };
}

/**
 * Sets the cursor position without changing selection.
 *
 * @param state - Current selection state
 * @param para - New paragraph index
 * @param offset - New character offset
 * @returns New state with updated cursor
 */
export function setCursor(
  state: SelectionState,
  para: number,
  offset: number
): SelectionState {
  return {
    ...state,
    cursorPara: para,
    cursorOffset: offset,
  };
}

/**
 * Sets the cursor and clears any selection.
 *
 * @param para - New paragraph index
 * @param offset - New character offset
 * @returns New selection state
 */
export function setCursorWithoutSelection(
  para: number,
  offset: number
): SelectionState {
  return {
    cursorPara: para,
    cursorOffset: offset,
    selectionStart: null,
    selectionEnd: null,
  };
}

/**
 * Starts a new selection from the current cursor position.
 *
 * @param state - Current selection state
 * @returns New state with selection started
 */
export function startSelection(state: SelectionState): SelectionState {
  const pos: TextPosition = {
    para: state.cursorPara,
    offset: state.cursorOffset,
  };
  return {
    ...state,
    selectionStart: pos,
    selectionEnd: pos,
  };
}

/**
 * Extends the selection to a new position.
 *
 * @param state - Current selection state
 * @param para - Target paragraph index
 * @param offset - Target character offset
 * @returns New state with extended selection
 */
export function extendSelection(
  state: SelectionState,
  para: number,
  offset: number
): SelectionState {
  const newEnd: TextPosition = { para, offset };

  // If no selection exists, start one from current cursor
  if (!state.selectionStart) {
    return {
      cursorPara: para,
      cursorOffset: offset,
      selectionStart: { para: state.cursorPara, offset: state.cursorOffset },
      selectionEnd: newEnd,
    };
  }

  return {
    cursorPara: para,
    cursorOffset: offset,
    selectionStart: state.selectionStart,
    selectionEnd: newEnd,
  };
}

/**
 * Selects all text in the document.
 *
 * @param paragraphs - Document paragraphs
 * @returns Selection state covering entire document
 */
export function selectAll(paragraphs: string[]): SelectionState {
  const lastPara = paragraphs.length - 1;
  const lastOffset = paragraphs[lastPara]?.length || 0;

  return {
    cursorPara: lastPara,
    cursorOffset: lastOffset,
    selectionStart: { para: 0, offset: 0 },
    selectionEnd: { para: lastPara, offset: lastOffset },
  };
}

/**
 * Checks if a position is within the current selection.
 *
 * @param state - Selection state
 * @param para - Paragraph to check
 * @param offset - Offset to check
 * @returns True if position is within selection
 */
export function isPositionInSelection(
  state: SelectionState,
  para: number,
  offset: number
): boolean {
  if (!state.selectionStart || !state.selectionEnd) {
    return false;
  }

  const pos: TextPosition = { para, offset };
  const { start, end } = normalizeSelection(
    state.selectionStart,
    state.selectionEnd
  );

  return comparePositions(pos, start) >= 0 && comparePositions(pos, end) <= 0;
}

/**
 * Gets the selection range for a specific display line.
 * Used for rendering selection highlighting.
 *
 * @param lineIndex - Display line index
 * @param displayLine - Display line data
 * @param selStart - Normalized selection start (display coordinates)
 * @param selEnd - Normalized selection end (display coordinates)
 * @returns Start and end column within the line, or null if not selected
 */
export function getLineSelectionRange(
  lineIndex: number,
  displayLine: DisplayLine,
  selStart: { line: number; col: number },
  selEnd: { line: number; col: number }
): { startCol: number; endCol: number } | null {
  // Check if this line is within selection
  if (lineIndex < selStart.line || lineIndex > selEnd.line) {
    return null;
  }

  let startCol = 0;
  let endCol = displayLine.text.length;

  if (lineIndex === selStart.line) {
    startCol = selStart.col;
  }
  if (lineIndex === selEnd.line) {
    endCol = selEnd.col;
  }

  if (startCol >= endCol) {
    return null;
  }

  return { startCol, endCol };
}

/**
 * Calculates cursor position in pixels for a display line.
 *
 * @param text - Text content of the line up to cursor
 * @param measureFn - Function to measure text width
 * @returns X offset in pixels from line start
 */
export function getCursorXPosition(
  text: string,
  measureFn: (text: string) => number
): number {
  return measureFn(text);
}

/**
 * Finds the character offset at a given X position in a line.
 *
 * @param x - X position in pixels from line start
 * @param text - Line text content
 * @param measureFn - Function to measure text width
 * @returns Character offset (0 to text.length)
 */
export function getCharOffsetAtX(
  x: number,
  text: string,
  measureFn: (text: string) => number
): number {
  if (x <= 0 || text.length === 0) {
    return 0;
  }

  // Binary search for efficiency with long lines
  let low = 0;
  let high = text.length;

  while (low < high) {
    const mid = Math.floor((low + high) / 2);
    const width = measureFn(text.substring(0, mid));

    if (width < x) {
      low = mid + 1;
    } else {
      high = mid;
    }
  }

  // Check if we should round to the closer character
  if (low > 0) {
    const prevWidth = measureFn(text.substring(0, low - 1));
    const currWidth = measureFn(text.substring(0, low));
    const prevDist = Math.abs(x - prevWidth);
    const currDist = Math.abs(x - currWidth);

    if (prevDist < currDist) {
      return low - 1;
    }
  }

  return Math.min(low, text.length);
}

/**
 * Moves cursor left by one character, respecting paragraph boundaries.
 *
 * @param state - Current selection state
 * @param paragraphs - Document paragraphs
 * @param extendSel - Whether to extend selection
 * @returns New selection state
 */
export function moveCursorLeft(
  state: SelectionState,
  paragraphs: string[],
  extendSel: boolean = false
): SelectionState {
  let newPara = state.cursorPara;
  let newOffset = state.cursorOffset;

  if (newOffset > 0) {
    newOffset--;
  } else if (newPara > 0) {
    newPara--;
    newOffset = paragraphs[newPara].length;
  }

  if (extendSel) {
    return extendSelection(state, newPara, newOffset);
  }

  return setCursorWithoutSelection(newPara, newOffset);
}

/**
 * Moves cursor right by one character, respecting paragraph boundaries.
 *
 * @param state - Current selection state
 * @param paragraphs - Document paragraphs
 * @param extendSel - Whether to extend selection
 * @returns New selection state
 */
export function moveCursorRight(
  state: SelectionState,
  paragraphs: string[],
  extendSel: boolean = false
): SelectionState {
  let newPara = state.cursorPara;
  let newOffset = state.cursorOffset;

  if (newOffset < paragraphs[newPara].length) {
    newOffset++;
  } else if (newPara < paragraphs.length - 1) {
    newPara++;
    newOffset = 0;
  }

  if (extendSel) {
    return extendSelection(state, newPara, newOffset);
  }

  return setCursorWithoutSelection(newPara, newOffset);
}

/**
 * Moves cursor to the start of the current line.
 *
 * @param state - Current selection state
 * @param displayLines - Display lines array
 * @param extendSel - Whether to extend selection
 * @returns New selection state
 */
export function moveCursorToLineStart(
  state: SelectionState,
  displayLines: DisplayLine[],
  extendSel: boolean = false
): SelectionState {
  // Find current display line
  for (const dl of displayLines) {
    if (
      dl.paraIndex === state.cursorPara &&
      state.cursorOffset >= dl.startOffset &&
      state.cursorOffset <= dl.endOffset
    ) {
      const newOffset = dl.startOffset;

      if (extendSel) {
        return extendSelection(state, state.cursorPara, newOffset);
      }
      return setCursorWithoutSelection(state.cursorPara, newOffset);
    }
  }

  return state;
}

/**
 * Moves cursor to the end of the current line.
 *
 * @param state - Current selection state
 * @param displayLines - Display lines array
 * @param extendSel - Whether to extend selection
 * @returns New selection state
 */
export function moveCursorToLineEnd(
  state: SelectionState,
  displayLines: DisplayLine[],
  extendSel: boolean = false
): SelectionState {
  // Find current display line
  for (const dl of displayLines) {
    if (
      dl.paraIndex === state.cursorPara &&
      state.cursorOffset >= dl.startOffset &&
      state.cursorOffset <= dl.endOffset
    ) {
      const newOffset = dl.endOffset;

      if (extendSel) {
        return extendSelection(state, state.cursorPara, newOffset);
      }
      return setCursorWithoutSelection(state.cursorPara, newOffset);
    }
  }

  return state;
}
