/**
 * @fileoverview Undo/Redo history manager for the rich text editor.
 *
 * Provides state snapshot management for implementing undo and redo functionality.
 * Uses a simple snapshot-based approach that captures the full document state
 * at each change point.
 *
 * @module editor/history-manager
 *
 * @example
 * ```typescript
 * import { createHistoryManager } from './history-manager';
 *
 * const history = createHistoryManager<EditorState>(50);
 *
 * // Save state after changes
 * history.push(currentState);
 *
 * // Undo
 * const previousState = history.undo();
 * if (previousState) applyState(previousState);
 *
 * // Redo
 * const nextState = history.redo();
 * if (nextState) applyState(nextState);
 * ```
 */

import type { ParagraphMeta, DocumentImage } from './types';

/**
 * Represents the complete editor state at a point in time.
 * This is what gets saved to the history stack.
 */
export interface EditorSnapshot {
  /** Array of paragraph text content */
  paragraphs: string[];
  /** Formatting metadata for each paragraph */
  paragraphMeta: ParagraphMeta[];
  /** Document images */
  images: DocumentImage[];
  /** Current cursor paragraph index */
  cursorPara: number;
  /** Current cursor offset within paragraph */
  cursorOffset: number;
  /** Selection start position (if any) */
  selectionStart: { para: number; offset: number } | null;
  /** Selection end position (if any) */
  selectionEnd: { para: number; offset: number } | null;
}

/**
 * History manager interface for undo/redo operations.
 */
export interface HistoryManager {
  /**
   * Push a new state onto the history stack.
   * This clears any redo history.
   * @param state - The current editor state to save
   */
  push(state: EditorSnapshot): void;

  /**
   * Undo the last change, returning the previous state.
   * @returns The previous state, or null if at the beginning of history
   */
  undo(): EditorSnapshot | null;

  /**
   * Redo a previously undone change.
   * @returns The next state, or null if at the end of history
   */
  redo(): EditorSnapshot | null;

  /**
   * Check if undo is available.
   * @returns True if there's history to undo
   */
  canUndo(): boolean;

  /**
   * Check if redo is available.
   * @returns True if there's history to redo
   */
  canRedo(): boolean;

  /**
   * Clear all history.
   */
  clear(): void;

  /**
   * Get the current history size.
   * @returns Number of states in the undo stack
   */
  size(): number;
}

/**
 * Creates a deep clone of an editor snapshot.
 * Ensures modifications to the returned state don't affect stored history.
 */
function cloneSnapshot(snapshot: EditorSnapshot): EditorSnapshot {
  return {
    paragraphs: [...snapshot.paragraphs],
    paragraphMeta: snapshot.paragraphMeta.map(meta => ({ ...meta })),
    images: snapshot.images.map(img => ({ ...img })),
    cursorPara: snapshot.cursorPara,
    cursorOffset: snapshot.cursorOffset,
    selectionStart: snapshot.selectionStart ? { ...snapshot.selectionStart } : null,
    selectionEnd: snapshot.selectionEnd ? { ...snapshot.selectionEnd } : null,
  };
}

/**
 * Compares two snapshots to check if they represent the same state.
 * Used to avoid pushing duplicate states.
 */
function snapshotsEqual(a: EditorSnapshot, b: EditorSnapshot): boolean {
  // Quick check: paragraph count and image count
  if (a.paragraphs.length !== b.paragraphs.length) return false;
  if (a.images.length !== b.images.length) return false;

  // Check paragraph content
  for (let i = 0; i < a.paragraphs.length; i++) {
    if (a.paragraphs[i] !== b.paragraphs[i]) return false;
  }

  // Check paragraph metadata
  for (let i = 0; i < a.paragraphMeta.length; i++) {
    const metaA = a.paragraphMeta[i];
    const metaB = b.paragraphMeta[i];
    if (
      metaA.align !== metaB.align ||
      metaA.listType !== metaB.listType ||
      metaA.blockType !== metaB.blockType ||
      metaA.indent !== metaB.indent ||
      metaA.fontSize !== metaB.fontSize ||
      metaA.textColor !== metaB.textColor
    ) {
      return false;
    }
  }

  // Check images (by ID, position, and key properties)
  for (let i = 0; i < a.images.length; i++) {
    const imgA = a.images[i];
    const imgB = b.images[i];
    if (
      imgA.id !== imgB.id ||
      imgA.width !== imgB.width ||
      imgA.height !== imgB.height ||
      imgA.wrapStyle !== imgB.wrapStyle ||
      imgA.x !== imgB.x ||
      imgA.y !== imgB.y
    ) {
      return false;
    }
  }

  return true;
}

/**
 * Creates a new history manager with the specified maximum size.
 *
 * @param maxSize - Maximum number of states to keep in history (default: 100)
 * @returns A HistoryManager instance
 *
 * @example
 * ```typescript
 * const history = createHistoryManager(50);
 *
 * // Initial state
 * history.push(getEditorState());
 *
 * // After user types
 * history.push(getEditorState());
 *
 * // Undo
 * const prev = history.undo();
 * ```
 */
export function createHistoryManager(maxSize: number = 100): HistoryManager {
  /** Stack of previous states (for undo) */
  let undoStack: EditorSnapshot[] = [];

  /** Stack of future states (for redo) */
  let redoStack: EditorSnapshot[] = [];

  return {
    push(state: EditorSnapshot): void {
      // Clone to prevent external modifications
      const snapshot = cloneSnapshot(state);

      // Don't push if identical to current state
      if (undoStack.length > 0 && snapshotsEqual(undoStack[undoStack.length - 1], snapshot)) {
        return;
      }

      // Add to undo stack
      undoStack.push(snapshot);

      // Clear redo stack on new action
      redoStack = [];

      // Limit stack size
      if (undoStack.length > maxSize) {
        undoStack.shift();
      }
    },

    undo(): EditorSnapshot | null {
      // Need at least 2 states: current and one to go back to
      if (undoStack.length < 2) {
        return null;
      }

      // Move current state to redo stack
      const current = undoStack.pop()!;
      redoStack.push(current);

      // Return the previous state (clone it)
      const previous = undoStack[undoStack.length - 1];
      return cloneSnapshot(previous);
    },

    redo(): EditorSnapshot | null {
      if (redoStack.length === 0) {
        return null;
      }

      // Pop from redo stack and push to undo stack
      const next = redoStack.pop()!;
      undoStack.push(next);

      // Return the restored state (clone it)
      return cloneSnapshot(next);
    },

    canUndo(): boolean {
      return undoStack.length >= 2;
    },

    canRedo(): boolean {
      return redoStack.length > 0;
    },

    clear(): void {
      undoStack = [];
      redoStack = [];
    },

    size(): number {
      return undoStack.length;
    },
  };
}

/**
 * Debounce helper for batching rapid changes into single history entries.
 * Useful for text input where each keystroke shouldn't be a separate undo step.
 *
 * @param historyManager - The history manager to push to
 * @param delay - Delay in milliseconds before pushing (default: 300ms)
 * @returns A function that schedules a push, and a cancel function
 *
 * @example
 * ```typescript
 * const { schedulePush, cancelPush } = createDebouncedPush(history, 300);
 *
 * // On each keystroke
 * schedulePush(getEditorState());
 *
 * // On explicit action (like Enter, Delete)
 * cancelPush();
 * history.push(getEditorState());
 * ```
 */
export function createDebouncedPush(
  historyManager: HistoryManager,
  delay: number = 300
): {
  schedulePush: (state: EditorSnapshot) => void;
  cancelPush: () => void;
  flushPush: () => void;
} {
  let timeoutId: ReturnType<typeof setTimeout> | null = null;
  let pendingState: EditorSnapshot | null = null;

  return {
    schedulePush(state: EditorSnapshot): void {
      pendingState = state;

      if (timeoutId !== null) {
        clearTimeout(timeoutId);
      }

      timeoutId = setTimeout(() => {
        if (pendingState) {
          historyManager.push(pendingState);
          pendingState = null;
        }
        timeoutId = null;
      }, delay);
    },

    cancelPush(): void {
      if (timeoutId !== null) {
        clearTimeout(timeoutId);
        timeoutId = null;
      }
      pendingState = null;
    },

    flushPush(): void {
      if (timeoutId !== null) {
        clearTimeout(timeoutId);
        timeoutId = null;
      }
      if (pendingState) {
        historyManager.push(pendingState);
        pendingState = null;
      }
    },
  };
}
