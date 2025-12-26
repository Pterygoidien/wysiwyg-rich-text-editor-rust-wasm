/**
 * @fileoverview Keyboard event handling for the rich text editor.
 *
 * This module provides utilities for processing keyboard events including
 * navigation, text input, shortcuts, and special key handling. Functions
 * return action descriptors that the editor can execute.
 *
 * @module editor/keyboard-handler
 */

import type { TextPosition, DisplayLine } from './types';
import {
  type KeyboardShortcut,
  type FormatCommand,
  DEFAULT_SHORTCUTS,
  findMatchingShortcut,
} from './commands';

/**
 * Navigation direction for cursor movement.
 */
export type NavigationDirection = 'left' | 'right' | 'up' | 'down' | 'home' | 'end';

/**
 * Type of keyboard action to perform.
 */
export type KeyboardActionType =
  | 'navigate'
  | 'delete'
  | 'insert'
  | 'newline'
  | 'pageBreak'
  | 'format'
  | 'selectAll'
  | 'copy'
  | 'cut'
  | 'paste'
  | 'deleteImage'
  | 'clearImageSelection'
  | 'none';

/**
 * Result of processing a keyboard event.
 */
export interface KeyboardAction {
  /** The type of action to perform */
  type: KeyboardActionType;
  /** Navigation direction for navigate actions */
  direction?: NavigationDirection;
  /** Whether shift was held (for selection extension) */
  withShift?: boolean;
  /** Text to insert for insert actions */
  text?: string;
  /** Format command to execute */
  formatCommand?: FormatCommand | 'selectAll' | 'copy' | 'cut' | 'paste';
  /** Whether the event should be prevented */
  preventDefault: boolean;
}

/**
 * Context required for processing keyboard events.
 */
export interface KeyboardContext {
  /** Current cursor paragraph index */
  cursorPara: number;
  /** Current cursor offset within paragraph */
  cursorOffset: number;
  /** Whether there's an active selection */
  hasSelection: boolean;
  /** Whether an image is currently selected */
  hasSelectedImage: boolean;
  /** Total number of paragraphs */
  paragraphCount: number;
  /** Length of current paragraph */
  currentParagraphLength: number;
  /** Current display line index */
  currentDisplayLine: number;
  /** Total number of display lines */
  displayLineCount: number;
}

/**
 * Determines if a key event represents a navigation key.
 *
 * @param key - The key from the keyboard event
 * @returns True if the key is a navigation key
 *
 * @example
 * ```typescript
 * isNavigationKey('ArrowLeft');  // true
 * isNavigationKey('a');          // false
 * ```
 */
export function isNavigationKey(key: string): boolean {
  return ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(
    key
  );
}

/**
 * Determines if a key event represents a deletion key.
 *
 * @param key - The key from the keyboard event
 * @returns True if the key is a deletion key
 */
export function isDeletionKey(key: string): boolean {
  return key === 'Backspace' || key === 'Delete';
}

/**
 * Determines if a key event represents a printable character.
 *
 * @param key - The key from the keyboard event
 * @returns True if the key is a single printable character
 */
export function isPrintableKey(key: string): boolean {
  return key.length === 1;
}

/**
 * Converts a key name to a navigation direction.
 *
 * @param key - The key from the keyboard event
 * @returns The navigation direction, or undefined if not a navigation key
 */
export function keyToDirection(key: string): NavigationDirection | undefined {
  switch (key) {
    case 'ArrowLeft':
      return 'left';
    case 'ArrowRight':
      return 'right';
    case 'ArrowUp':
      return 'up';
    case 'ArrowDown':
      return 'down';
    case 'Home':
      return 'home';
    case 'End':
      return 'end';
    default:
      return undefined;
  }
}

/**
 * Processes a keyboard event and returns the appropriate action.
 *
 * This is the main entry point for keyboard handling. It analyzes the event
 * and context to determine what action should be taken, without actually
 * performing the action.
 *
 * @param event - The keyboard event to process
 * @param context - Current editor context
 * @param shortcuts - Optional custom keyboard shortcuts
 * @returns The action to perform
 *
 * @example
 * ```typescript
 * const action = processKeyboardEvent(event, {
 *   cursorPara: 0,
 *   cursorOffset: 5,
 *   hasSelection: false,
 *   hasSelectedImage: false,
 *   paragraphCount: 3,
 *   currentParagraphLength: 20,
 *   currentDisplayLine: 0,
 *   displayLineCount: 10,
 * });
 *
 * if (action.type === 'navigate') {
 *   moveCursor(action.direction!);
 * }
 * ```
 */
export function processKeyboardEvent(
  event: KeyboardEvent,
  context: KeyboardContext,
  shortcuts: KeyboardShortcut[] = DEFAULT_SHORTCUTS
): KeyboardAction {
  const key = event.key;
  const ctrlOrMeta = event.ctrlKey || event.metaKey;

  // Handle image deletion
  if (context.hasSelectedImage && isDeletionKey(key)) {
    return {
      type: 'deleteImage',
      preventDefault: true,
    };
  }

  // Clear image selection on any other key
  if (context.hasSelectedImage && !isDeletionKey(key)) {
    return {
      type: 'clearImageSelection',
      preventDefault: false,
    };
  }

  // Alt+Enter for page break
  if (event.altKey && key === 'Enter') {
    return {
      type: 'pageBreak',
      preventDefault: true,
    };
  }

  // Check keyboard shortcuts
  const matchedShortcut = findMatchingShortcut(event, shortcuts);
  if (matchedShortcut) {
    return {
      type: 'format',
      formatCommand: matchedShortcut.command,
      preventDefault: true,
    };
  }

  // Handle navigation keys
  if (isNavigationKey(key)) {
    const direction = keyToDirection(key);
    return {
      type: 'navigate',
      direction,
      withShift: event.shiftKey,
      preventDefault: true,
    };
  }

  // Handle deletion keys
  if (isDeletionKey(key)) {
    return {
      type: 'delete',
      direction: key === 'Backspace' ? 'left' : 'right',
      preventDefault: true,
    };
  }

  // Handle Enter key
  if (key === 'Enter') {
    return {
      type: 'newline',
      preventDefault: true,
    };
  }

  // Handle printable characters
  if (isPrintableKey(key) && !ctrlOrMeta) {
    return {
      type: 'insert',
      text: key,
      preventDefault: true,
    };
  }

  // No action needed
  return {
    type: 'none',
    preventDefault: false,
  };
}

/**
 * Calculates the new cursor position after left arrow navigation.
 *
 * @param cursorPara - Current paragraph index
 * @param cursorOffset - Current offset within paragraph
 * @param paragraphs - Array of paragraph texts
 * @returns New cursor position
 */
export function navigateLeft(
  cursorPara: number,
  cursorOffset: number,
  paragraphs: string[]
): TextPosition {
  if (cursorOffset > 0) {
    return { para: cursorPara, offset: cursorOffset - 1 };
  } else if (cursorPara > 0) {
    return { para: cursorPara - 1, offset: paragraphs[cursorPara - 1].length };
  }
  return { para: cursorPara, offset: cursorOffset };
}

/**
 * Calculates the new cursor position after right arrow navigation.
 *
 * @param cursorPara - Current paragraph index
 * @param cursorOffset - Current offset within paragraph
 * @param paragraphs - Array of paragraph texts
 * @returns New cursor position
 */
export function navigateRight(
  cursorPara: number,
  cursorOffset: number,
  paragraphs: string[]
): TextPosition {
  if (cursorOffset < paragraphs[cursorPara].length) {
    return { para: cursorPara, offset: cursorOffset + 1 };
  } else if (cursorPara < paragraphs.length - 1) {
    return { para: cursorPara + 1, offset: 0 };
  }
  return { para: cursorPara, offset: cursorOffset };
}

/**
 * Calculates the new cursor position for home key navigation.
 *
 * @param displayLine - The current display line
 * @param cursorPara - Current paragraph index
 * @returns New cursor position at line start
 */
export function navigateHome(
  displayLine: DisplayLine,
  cursorPara: number
): TextPosition {
  return { para: cursorPara, offset: displayLine.startOffset };
}

/**
 * Calculates the new cursor position for end key navigation.
 *
 * @param displayLine - The current display line
 * @param cursorPara - Current paragraph index
 * @returns New cursor position at line end
 */
export function navigateEnd(
  displayLine: DisplayLine,
  cursorPara: number
): TextPosition {
  return { para: cursorPara, offset: displayLine.endOffset };
}

/**
 * Handles the result of a format command shortcut.
 *
 * @param command - The format command from the shortcut
 * @returns Object describing the formatting action to take
 */
export function handleFormatCommand(
  command: FormatCommand | 'selectAll' | 'copy' | 'cut' | 'paste'
): {
  action: 'toggle' | 'clipboard' | 'selection';
  style?: 'bold' | 'italic' | 'underline';
  clipboard?: 'copy' | 'cut' | 'paste';
} {
  switch (command) {
    case 'bold':
      return { action: 'toggle', style: 'bold' };
    case 'italic':
      return { action: 'toggle', style: 'italic' };
    case 'underline':
      return { action: 'toggle', style: 'underline' };
    case 'copy':
      return { action: 'clipboard', clipboard: 'copy' };
    case 'cut':
      return { action: 'clipboard', clipboard: 'cut' };
    case 'paste':
      return { action: 'clipboard', clipboard: 'paste' };
    case 'selectAll':
      return { action: 'selection' };
    default:
      return { action: 'toggle' };
  }
}

/**
 * Determines if selection should be started or extended based on key state.
 *
 * @param event - The keyboard event
 * @param hasExistingSelection - Whether a selection already exists
 * @returns Object describing selection behavior
 */
export function getSelectionBehavior(
  event: KeyboardEvent,
  hasExistingSelection: boolean
): {
  shouldStartSelection: boolean;
  shouldExtendSelection: boolean;
  shouldClearSelection: boolean;
} {
  const isNav = isNavigationKey(event.key);

  if (!isNav) {
    return {
      shouldStartSelection: false,
      shouldExtendSelection: false,
      shouldClearSelection: false,
    };
  }

  if (event.shiftKey) {
    return {
      shouldStartSelection: !hasExistingSelection,
      shouldExtendSelection: true,
      shouldClearSelection: false,
    };
  }

  return {
    shouldStartSelection: false,
    shouldExtendSelection: false,
    shouldClearSelection: hasExistingSelection,
  };
}

/**
 * Creates a keyboard context from editor state.
 *
 * @param state - Current editor state
 * @returns Keyboard context for event processing
 */
export function createKeyboardContext(state: {
  cursorPara: number;
  cursorOffset: number;
  selectionStart: TextPosition | null;
  selectionEnd: TextPosition | null;
  selectedImageId: string | null;
  paragraphs: string[];
  displayLines: DisplayLine[];
  currentDisplayLine: number;
}): KeyboardContext {
  return {
    cursorPara: state.cursorPara,
    cursorOffset: state.cursorOffset,
    hasSelection: state.selectionStart !== null && state.selectionEnd !== null,
    hasSelectedImage: state.selectedImageId !== null,
    paragraphCount: state.paragraphs.length,
    currentParagraphLength: state.paragraphs[state.cursorPara]?.length ?? 0,
    currentDisplayLine: state.currentDisplayLine,
    displayLineCount: state.displayLines.length,
  };
}
