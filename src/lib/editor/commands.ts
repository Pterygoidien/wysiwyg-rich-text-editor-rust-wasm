/**
 * @fileoverview Formatting command types and constants for the rich text editor.
 *
 * This module provides strongly-typed command definitions for all text
 * formatting operations. Using these types instead of raw strings provides
 * better type safety and autocompletion.
 *
 * @module editor/commands
 */

import type { TextAlign, ListType, BlockType } from './types';

/**
 * Text styling commands that toggle formatting on/off.
 */
export type TextStyleCommand =
  | 'bold'
  | 'italic'
  | 'underline'
  | 'strikeThrough';

/**
 * Text alignment commands.
 */
export type AlignmentCommand =
  | 'justifyLeft'
  | 'justifyCenter'
  | 'justifyRight'
  | 'justifyFull';

/**
 * List formatting commands.
 */
export type ListCommand = 'insertUnorderedList' | 'insertOrderedList';

/**
 * Block type formatting command.
 */
export type BlockCommand = 'formatBlock';

/**
 * History commands.
 */
export type HistoryCommand = 'undo' | 'redo';

/**
 * Color commands.
 */
export type ColorCommand = 'foreColor' | 'hiliteColor';

/**
 * Font commands.
 */
export type FontCommand = 'fontSize';

/**
 * Content insertion commands.
 */
export type InsertCommand = 'insertImage';

/**
 * All supported formatting commands.
 */
export type FormatCommand =
  | TextStyleCommand
  | AlignmentCommand
  | ListCommand
  | BlockCommand
  | HistoryCommand
  | ColorCommand
  | FontCommand
  | InsertCommand;

/**
 * Command with optional value.
 */
export interface FormattingAction {
  /** The command to execute */
  command: FormatCommand;
  /** Optional value for commands that require one */
  value?: string;
}

/**
 * Maps alignment commands to TextAlign values.
 */
export const ALIGNMENT_MAP: Record<AlignmentCommand, TextAlign> = {
  justifyLeft: 'left',
  justifyCenter: 'center',
  justifyRight: 'right',
  justifyFull: 'justify',
};

/**
 * Maps list commands to ListType values.
 */
export const LIST_MAP: Record<ListCommand, ListType> = {
  insertUnorderedList: 'bullet',
  insertOrderedList: 'numbered',
};

/**
 * Checks if a command is a text style toggle command.
 *
 * @param command - Command to check
 * @returns True if it's a text style command
 */
export function isTextStyleCommand(command: string): command is TextStyleCommand {
  return ['bold', 'italic', 'underline', 'strikeThrough'].includes(command);
}

/**
 * Checks if a command is an alignment command.
 *
 * @param command - Command to check
 * @returns True if it's an alignment command
 */
export function isAlignmentCommand(command: string): command is AlignmentCommand {
  return ['justifyLeft', 'justifyCenter', 'justifyRight', 'justifyFull'].includes(
    command
  );
}

/**
 * Checks if a command is a list command.
 *
 * @param command - Command to check
 * @returns True if it's a list command
 */
export function isListCommand(command: string): command is ListCommand {
  return ['insertUnorderedList', 'insertOrderedList'].includes(command);
}

/**
 * Checks if a command is a history command.
 *
 * @param command - Command to check
 * @returns True if it's a history command
 */
export function isHistoryCommand(command: string): command is HistoryCommand {
  return ['undo', 'redo'].includes(command);
}

/**
 * Checks if a command is a color command.
 *
 * @param command - Command to check
 * @returns True if it's a color command
 */
export function isColorCommand(command: string): command is ColorCommand {
  return ['foreColor', 'hiliteColor'].includes(command);
}

/**
 * Keyboard shortcut definitions.
 */
export interface KeyboardShortcut {
  /** Key code or key name */
  key: string;
  /** Whether Ctrl (or Cmd on Mac) is required */
  ctrl: boolean;
  /** Whether Shift is required */
  shift?: boolean;
  /** Whether Alt is required */
  alt?: boolean;
  /** Command to execute */
  command: FormatCommand | 'selectAll' | 'copy' | 'cut' | 'paste';
}

/**
 * Default keyboard shortcuts.
 */
export const DEFAULT_SHORTCUTS: KeyboardShortcut[] = [
  { key: 'b', ctrl: true, command: 'bold' },
  { key: 'i', ctrl: true, command: 'italic' },
  { key: 'u', ctrl: true, command: 'underline' },
  { key: 'a', ctrl: true, command: 'selectAll' },
  { key: 'c', ctrl: true, command: 'copy' },
  { key: 'x', ctrl: true, command: 'cut' },
  { key: 'v', ctrl: true, command: 'paste' },
  { key: 'z', ctrl: true, command: 'undo' },
  { key: 'y', ctrl: true, command: 'redo' },
  { key: 'z', ctrl: true, shift: true, command: 'redo' },
];

/**
 * Finds a matching shortcut for a keyboard event.
 *
 * @param event - Keyboard event
 * @param shortcuts - Array of shortcuts to check
 * @returns Matching shortcut or undefined
 */
export function findMatchingShortcut(
  event: KeyboardEvent,
  shortcuts: KeyboardShortcut[] = DEFAULT_SHORTCUTS
): KeyboardShortcut | undefined {
  const ctrlOrMeta = event.ctrlKey || event.metaKey;

  return shortcuts.find((shortcut) => {
    if (shortcut.key.toLowerCase() !== event.key.toLowerCase()) {
      return false;
    }
    if (shortcut.ctrl !== ctrlOrMeta) {
      return false;
    }
    if (shortcut.shift && !event.shiftKey) {
      return false;
    }
    if (!shortcut.shift && event.shiftKey && shortcut.ctrl) {
      // Allow shift variations for some shortcuts
      return false;
    }
    if (shortcut.alt && !event.altKey) {
      return false;
    }
    return true;
  });
}

/**
 * Parses a block type value from a format command.
 *
 * @param value - The value string from formatBlock command
 * @returns The BlockType or null if invalid
 */
export function parseBlockType(value: string | undefined): BlockType | null {
  const validTypes: BlockType[] = ['p', 'h1', 'h2', 'h3', 'h4', 'blockquote'];
  if (value && validTypes.includes(value as BlockType)) {
    return value as BlockType;
  }
  return null;
}

/**
 * Parses a font size value from a format command.
 *
 * @param value - The value string from fontSize command
 * @returns The font size number or null if invalid
 */
export function parseFontSize(value: string | undefined): number | null {
  if (!value) return null;
  const size = parseInt(value, 10);
  if (isNaN(size) || size <= 0) return null;
  return size;
}

/**
 * Validates a color value.
 *
 * @param value - Color value to validate
 * @returns True if valid color format
 */
export function isValidColor(value: string | undefined): boolean {
  if (!value) return false;

  // Check hex format
  if (/^#[0-9A-Fa-f]{3}$/.test(value) || /^#[0-9A-Fa-f]{6}$/.test(value)) {
    return true;
  }

  // Check rgb/rgba format
  if (/^rgba?\([^)]+\)$/.test(value)) {
    return true;
  }

  // Check named colors (basic set)
  const namedColors = [
    'black', 'white', 'red', 'green', 'blue', 'yellow',
    'cyan', 'magenta', 'orange', 'purple', 'pink', 'gray',
  ];
  return namedColors.includes(value.toLowerCase());
}
