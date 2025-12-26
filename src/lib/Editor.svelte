<script lang="ts">
  import { onMount } from 'svelte';
  import Toolbar from './Toolbar.svelte';
  import { pageConfig, zoomLevel, currentPage, totalPages, fontSize as fontSizeStore, fontFamily as fontFamilyStore, headings, type HeadingItem, lineHeight as lineHeightStore, letterSpacing as letterSpacingStore, paragraphSpacing as paragraphSpacingStore } from './stores';
  import { getContentDimensions, getPageDimensions, getColumnWidth, mmToPixels } from './types';
  import {
    type TextAlign,
    type ListType,
    type BlockType,
    type ImageWrapStyle,
    type ImagePositionMode,
    type ParagraphMeta,
    type DocumentImage,
    type DisplayLine,
    type FloatImage,
    type ResizeHandle,
    type EditorSnapshot,
    type HistoryManager,
    IMAGE_MARKER,
    PAGE_BREAK_MARKER,
    createDefaultMeta,
    createHistoryManager,
    createDebouncedPush
  } from './editor';

  let editorContainer: HTMLDivElement;
  let canvasContainer: HTMLDivElement;
  let hiddenTextarea: HTMLTextAreaElement;
  let measureCanvas: HTMLCanvasElement;

  let paragraphs: string[] = $state(['']);
  let paragraphMeta: ParagraphMeta[] = $state([createDefaultMeta()]);
  let images: DocumentImage[] = $state([]);
  let loadedImages: Map<string, HTMLImageElement> = new Map();

  // Image popup state
  let showImagePopup = $state(false);
  let imageUrl = $state('');
  let dragOver = $state(false);
  let cursorPara = $state(0);
  let cursorOffset = $state(0); // offset within the paragraph
  let selectionStart: { para: number; offset: number } | null = $state(null);
  let selectionEnd: { para: number; offset: number } | null = $state(null);

  // Undo/Redo history
  const historyManager = createHistoryManager(100);
  const { schedulePush: scheduleHistoryPush, cancelPush: cancelHistoryPush, flushPush: flushHistoryPush } = createDebouncedPush(historyManager, 500);
  let canUndo = $state(false);
  let canRedo = $state(false);

  /** Creates a snapshot of the current editor state for history */
  function getEditorSnapshot(): EditorSnapshot {
    return {
      paragraphs: [...paragraphs],
      paragraphMeta: paragraphMeta.map(m => ({ ...m })),
      images: images.map(img => ({ ...img })),
      cursorPara,
      cursorOffset,
      selectionStart: selectionStart ? { ...selectionStart } : null,
      selectionEnd: selectionEnd ? { ...selectionEnd } : null,
    };
  }

  /** Restores editor state from a snapshot */
  function restoreSnapshot(snapshot: EditorSnapshot): void {
    paragraphs = [...snapshot.paragraphs];
    paragraphMeta = snapshot.paragraphMeta.map(m => ({ ...m }));
    images = snapshot.images.map(img => ({ ...img }));
    cursorPara = snapshot.cursorPara;
    cursorOffset = snapshot.cursorOffset;
    selectionStart = snapshot.selectionStart ? { ...snapshot.selectionStart } : null;
    selectionEnd = snapshot.selectionEnd ? { ...snapshot.selectionEnd } : null;

    recomputeDisplayLines();
    renderAllPages();
  }

  /** Saves current state to history (immediate, for significant actions) */
  function saveToHistory(): void {
    flushHistoryPush();
    historyManager.push(getEditorSnapshot());
    updateHistoryState();
  }

  /** Schedules a history save (debounced, for typing) */
  function scheduleHistorySave(): void {
    scheduleHistoryPush(getEditorSnapshot());
    updateHistoryState();
  }

  /** Updates the canUndo/canRedo reactive state */
  function updateHistoryState(): void {
    canUndo = historyManager.canUndo();
    canRedo = historyManager.canRedo();
  }

  /** Performs an undo operation */
  function performUndo(): void {
    cancelHistoryPush();
    const snapshot = historyManager.undo();
    if (snapshot) {
      restoreSnapshot(snapshot);
      updateHistoryState();
    }
  }

  /** Performs a redo operation */
  function performRedo(): void {
    cancelHistoryPush();
    const snapshot = historyManager.redo();
    if (snapshot) {
      restoreSnapshot(snapshot);
      updateHistoryState();
    }
  }

  // Text styling - using stores for font size and family
  let fontSize = $derived($fontSizeStore);
  let fontFamily = $derived($fontFamilyStore);
  let lineHeightMultiplier = $derived($lineHeightStore);
  let letterSpacingValue = $derived($letterSpacingStore);
  let paragraphSpacingValue = $derived($paragraphSpacingStore);
  let isBold = $state(false);
  let isItalic = $state(false);
  let isUnderline = $state(false);
  let isStrikethrough = $state(false);

  // Calculate dimensions
  let pageDims = $derived(getPageDimensions($pageConfig));
  let contentDims = $derived(getContentDimensions($pageConfig));

  let scaledPageHeight = $derived((pageDims.height * $zoomLevel) / 100);
  let scaledPageWidth = $derived((pageDims.width * $zoomLevel) / 100);
  let scaledContentHeight = $derived((contentDims.height * $zoomLevel) / 100);
  let scaledContentWidth = $derived((contentDims.width * $zoomLevel) / 100);

  let marginTop = $derived((mmToPixels($pageConfig.margins.top) * $zoomLevel) / 100);
  let marginLeft = $derived((mmToPixels($pageConfig.margins.left) * $zoomLevel) / 100);

  // Column configuration
  let columnCount = $derived($pageConfig.columns || 1);
  let columnGap = $derived((mmToPixels($pageConfig.columnGap || 10) * $zoomLevel) / 100);
  let columnWidth = $derived((getColumnWidth($pageConfig) * $zoomLevel) / 100);

  // Calculate line metrics
  let scaledFontSize = $derived((fontSize * $zoomLevel) / 100);
  let scaledLineHeight = $derived(scaledFontSize * lineHeightMultiplier);
  let linesPerColumn = $derived(Math.floor(scaledContentHeight / scaledLineHeight));
  // Total lines per page (across all columns)
  let linesPerPage = $derived(linesPerColumn * columnCount);

  // Track selected image
  let selectedImageId: string | null = $state(null);

  // Image options popup state
  let showImageOptionsPopup = $state(false);
  let imageOptionsPosition = $state({ x: 0, y: 0 });

  // Image resize state
  let isResizing = $state(false);
  let resizeHandle: ResizeHandle = $state(null);
  let resizeStartX = $state(0);
  let resizeStartY = $state(0);
  let resizeStartWidth = $state(0);
  let resizeStartHeight = $state(0);

  // Image crop state
  let isCropping = $state(false);
  let cropHandle: ResizeHandle = $state(null);
  let cropStartX = $state(0);
  let cropStartY = $state(0);
  let cropStartValues = $state({ top: 0, right: 0, bottom: 0, left: 0 });
  let cropOriginalValues = $state({ top: 0, right: 0, bottom: 0, left: 0 }); // For cancel

  // Image dragging state
  let isDragging = $state(false);
  let dragStartX = $state(0);
  let dragStartY = $state(0);
  let dragStartImageX = $state(0);
  let dragStartImageY = $state(0);

  let activeFloats: FloatImage[] = $state([]);

  let displayLines: DisplayLine[] = $state([]);
  let numPages = $derived(Math.max(1, Math.ceil(displayLines.length / linesPerPage)));

  let canvases: HTMLCanvasElement[] = [];

  /**
   * Composes a CSS font shorthand string from current styling state.
   * Combines italic, bold, font size, and font family into a single string
   * suitable for setting the canvas context font property.
   *
   * @returns CSS font string (e.g., "italic bold 16px Arial")
   */
  function getFontStyle(): string {
    return `${isItalic ? 'italic ' : ''}${isBold ? 'bold ' : ''}${scaledFontSize}px ${fontFamily}`;
  }

  /**
   * Measures the rendered width of text using the off-screen measurement canvas.
   * Accounts for letter spacing by adding the configured spacing between characters.
   * Falls back to an estimated width if the canvas is unavailable.
   *
   * @param text - The text string to measure
   * @param meta - Optional paragraph metadata for font size/style overrides
   * @returns Width in pixels
   */
  function measureTextWidth(text: string, meta?: ParagraphMeta): number {
    if (!measureCanvas) return text.length * scaledFontSize * 0.5;
    const ctx = measureCanvas.getContext('2d');
    if (!ctx) return text.length * scaledFontSize * 0.5;

    let effectiveFontSize = scaledFontSize;
    let fontWeight = isBold ? 'bold ' : '';
    let fontStyle = isItalic ? 'italic ' : '';

    if (meta) {
      const baseFontSize = meta.fontSize ? (meta.fontSize * $zoomLevel) / 100 : scaledFontSize;
      switch (meta.blockType) {
        case 'h1':
          effectiveFontSize = baseFontSize * 2;
          fontWeight = 'bold ';
          break;
        case 'h2':
          effectiveFontSize = baseFontSize * 1.5;
          fontWeight = 'bold ';
          break;
        case 'h3':
          effectiveFontSize = baseFontSize * 1.17;
          fontWeight = 'bold ';
          break;
        case 'h4':
          effectiveFontSize = baseFontSize;
          fontWeight = 'bold ';
          break;
        case 'blockquote':
          effectiveFontSize = baseFontSize;
          fontStyle = 'italic ';
          break;
        default:
          effectiveFontSize = baseFontSize;
      }
    }

    ctx.font = `${fontStyle}${fontWeight}${effectiveFontSize}px ${fontFamily}`;
    const baseWidth = ctx.measureText(text).width;
    const scaledLetterSpacing = (letterSpacingValue * $zoomLevel) / 100;
    return baseWidth + (text.length > 0 ? (text.length - 1) * scaledLetterSpacing : 0);
  }

  /**
   * Wraps a paragraph into multiple display lines that fit within the content width.
   * Implements a greedy line-breaking algorithm that prefers word boundaries.
   * Handles list indentation and float width reductions for text wrapping around images.
   *
   * @param paraIndex - Index of the paragraph in the document
   * @param text - The paragraph text content
   * @param listNumber - Optional sequence number for numbered lists
   * @param floatReduction - Optional width reduction for floating images
   * @returns Array of DisplayLine objects representing wrapped lines
   */
  function wrapParagraph(
    paraIndex: number,
    text: string,
    listNumber?: number,
    floatReduction?: { side: 'left' | 'right'; width: number }
  ): DisplayLine[] {
    const meta = paragraphMeta[paraIndex] || createDefaultMeta();

    const baseFontSize = meta.fontSize ? (meta.fontSize * $zoomLevel) / 100 : scaledFontSize;
    let effectiveFontSize = baseFontSize;
    switch (meta.blockType) {
      case 'h1': effectiveFontSize = baseFontSize * 2; break;
      case 'h2': effectiveFontSize = baseFontSize * 1.5; break;
      case 'h3': effectiveFontSize = baseFontSize * 1.17; break;
    }

    const listIndent = meta.listType !== 'none' ? effectiveFontSize * 1.5 : 0;
    const floatWidth = floatReduction ? floatReduction.width + 10 : 0;
    const baseContentWidth = columnWidth > 0 ? columnWidth : (scaledContentWidth > 0 ? scaledContentWidth : 500);
    const contentWidth = baseContentWidth - listIndent - floatWidth;

    if (!text) {
      return [{ paraIndex, startOffset: 0, endOffset: 0, text: '', meta, listNumber, floatReduction }];
    }

    const result: DisplayLine[] = [];
    let currentStart = 0;

    while (currentStart < text.length) {
      const remainingText = text.substring(currentStart);
      if (measureTextWidth(remainingText, meta) <= contentWidth) {
        result.push({
          paraIndex,
          startOffset: currentStart,
          endOffset: text.length,
          text: remainingText,
          meta,
          listNumber: result.length === 0 ? listNumber : undefined,
          floatReduction,
        });
        break;
      }

      let lineEnd = currentStart;
      let lastWordBoundary = currentStart;

      for (let i = currentStart + 1; i <= text.length; i++) {
        const testText = text.substring(currentStart, i);
        const width = measureTextWidth(testText, meta);

        // Track word boundaries (position after space)
        if (i > currentStart && text[i - 1] === ' ') {
          lastWordBoundary = i;
        }

        if (width > contentWidth) {
          // We've exceeded the width, use the position before this
          if (lastWordBoundary > currentStart) {
            // Break at word boundary
            lineEnd = lastWordBoundary;
          } else {
            // No word boundary found, force break at previous char
            lineEnd = Math.max(currentStart + 1, i - 1);
          }
          break;
        }

        lineEnd = i;
      }

      // Safety check - ensure we make progress
      if (lineEnd <= currentStart) {
        lineEnd = currentStart + 1;
      }

      result.push({
        paraIndex,
        startOffset: currentStart,
        endOffset: lineEnd,
        text: text.substring(currentStart, lineEnd),
        meta,
        listNumber: result.length === 0 ? listNumber : undefined,
        floatReduction,
      });

      currentStart = lineEnd;

      // Skip leading spaces on new lines
      while (currentStart < text.length && text[currentStart] === ' ') {
        currentStart++;
      }
    }

    return result;
  }

  /**
   * Recomputes all display lines from the document paragraphs.
   *
   * This is the core layout function that transforms the logical paragraph structure
   * into renderable display lines. It handles:
   * - Text wrapping within column widths
   * - Floating image positioning and text flow
   * - Page break markers
   * - Numbered list sequencing
   * - Paragraph spacing markers
   *
   * Called whenever content or layout parameters change.
   */
  function recomputeDisplayLines() {
    const newDisplayLines: DisplayLine[] = [];
    const newActiveFloats: FloatImage[] = [];
    let numberedListCounter = 0;

    // Helper to check if wrap style causes text to flow around image
    const isFloatWrapStyle = (style: ImageWrapStyle) =>
      style === 'square' || style === 'tight' || style === 'through';

    // First, collect all floating images with absolute positions
    // These will affect text wrapping based on their Y position
    for (let i = 0; i < paragraphs.length; i++) {
      const paraText = paragraphs[i];
      if (paraText.startsWith(IMAGE_MARKER)) {
        const imageId = paraText.substring(1);
        const docImage = images.find(img => img.id === imageId);

        if (docImage && isFloatWrapStyle(docImage.wrapStyle)) {
          const scaledWidth = (docImage.width * $zoomLevel) / 100;
          const scaledHeight = (docImage.height * $zoomLevel) / 100;
          const imageLines = Math.ceil(scaledHeight / scaledLineHeight);

          // If image has absolute position, calculate which lines it affects
          if (docImage.y !== undefined) {
            // Convert Y position to line number
            const unscaledLineHeight = fontSize * lineHeightMultiplier;
            const startLine = Math.floor(docImage.y / unscaledLineHeight);
            // Determine side based on horizontal alignment or position
            let side: 'left' | 'right';
            if (docImage.horizontalAlign === 'right') {
              side = 'right';
            } else if (docImage.horizontalAlign === 'center') {
              // For centered, determine by actual position
              const contentCenter = contentDims.width / 2;
              const imageCenterX = (docImage.x ?? 0) + docImage.width / 2;
              side = imageCenterX < contentCenter ? 'left' : 'right';
            } else {
              side = 'left';
            }
            newActiveFloats.push({
              id: imageId,
              startLine,
              endLine: startLine + imageLines,
              width: scaledWidth,
              side,
            });
          }
        }
      }
    }

    for (let i = 0; i < paragraphs.length; i++) {
      const meta = paragraphMeta[i] || createDefaultMeta();
      const paraText = paragraphs[i];

      // Check if this is a page break
      if (paraText === PAGE_BREAK_MARKER) {
        // Calculate how many lines to add to fill the current page
        const currentLine = newDisplayLines.length;
        const linesOnCurrentPage = currentLine % linesPerPage;
        const linesToAdd = linesOnCurrentPage === 0 ? 0 : linesPerPage - linesOnCurrentPage;

        // Add the page break marker line
        newDisplayLines.push({
          paraIndex: i,
          startOffset: 0,
          endOffset: 1,
          text: '',
          meta,
          isPageBreak: true,
        });

        // Fill with empty lines to complete the page
        for (let j = 1; j < linesToAdd; j++) {
          newDisplayLines.push({
            paraIndex: i,
            startOffset: 0,
            endOffset: 1,
            text: '',
            meta,
            isPageBreak: true,
          });
        }
        continue;
      }

      // Check if this is an image paragraph
      if (paraText.startsWith(IMAGE_MARKER)) {
        const imageId = paraText.substring(1); // Remove the marker
        const docImage = images.find(img => img.id === imageId);

        if (docImage) {
          const scaledWidth = (docImage.width * $zoomLevel) / 100;
          const scaledHeight = (docImage.height * $zoomLevel) / 100;
          const imageLines = Math.ceil(scaledHeight / scaledLineHeight);

          if (isFloatWrapStyle(docImage.wrapStyle)) {
            // For float images without absolute position, track the float for text wrapping
            if (docImage.y === undefined) {
              const startLine = newDisplayLines.length;
              // Check if this float was already added (has absolute position)
              const existingFloat = newActiveFloats.find(f => f.id === imageId);
              if (!existingFloat) {
                // Determine side based on horizontal alignment
                let side: 'left' | 'right';
                if (docImage.horizontalAlign === 'right') {
                  side = 'right';
                } else if (docImage.horizontalAlign === 'center') {
                  const contentCenter = contentDims.width / 2;
                  const imageCenterX = (docImage.x ?? 0) + docImage.width / 2;
                  side = imageCenterX < contentCenter ? 'left' : 'right';
                } else {
                  side = 'left';
                }
                newActiveFloats.push({
                  id: imageId,
                  startLine,
                  endLine: startLine + imageLines,
                  width: scaledWidth,
                  side,
                });
              }
            }

            // Float images don't add display lines - they float alongside text
            // We still need to track the paragraph though for deletion purposes
            // Add a zero-height marker
            newDisplayLines.push({
              paraIndex: i,
              startOffset: 0,
              endOffset: paraText.length,
              text: '',
              meta,
              isImage: true,
              imageId: imageId,
              imageHeight: imageLines, // Store for rendering purposes
            });
          } else if (docImage.wrapStyle === 'inline') {
            // Inline images are treated as part of text - but for simplicity,
            // we still give them their own paragraph line
            newDisplayLines.push({
              paraIndex: i,
              startOffset: 0,
              endOffset: paraText.length,
              text: '',
              meta,
              isImage: true,
              imageId: imageId,
              imageHeight: imageLines,
            });

            // Add placeholder lines for the image height
            for (let j = 1; j < imageLines; j++) {
              newDisplayLines.push({
                paraIndex: i,
                startOffset: 0,
                endOffset: paraText.length,
                text: '',
                meta,
                isImage: true,
                imageId: imageId,
                imageHeight: 0, // Not the first line
              });
            }
          } else if (docImage.wrapStyle === 'top-bottom') {
            // Top and bottom - image takes up space, text flows above and below only
            newDisplayLines.push({
              paraIndex: i,
              startOffset: 0,
              endOffset: paraText.length,
              text: '',
              meta,
              isImage: true,
              imageId: imageId,
              imageHeight: imageLines,
            });

            // Add placeholder lines for the image height
            for (let j = 1; j < imageLines; j++) {
              newDisplayLines.push({
                paraIndex: i,
                startOffset: 0,
                endOffset: paraText.length,
                text: '',
                meta,
                isImage: true,
                imageId: imageId,
                imageHeight: 0,
              });
            }
          } else if (docImage.wrapStyle === 'behind' || docImage.wrapStyle === 'in-front') {
            // Behind/in-front - image doesn't affect text flow at all
            // Just add a marker for tracking purposes
            newDisplayLines.push({
              paraIndex: i,
              startOffset: 0,
              endOffset: paraText.length,
              text: '',
              meta,
              isImage: true,
              imageId: imageId,
              imageHeight: 0, // No height impact on text
            });
          } else {
            // Block image - takes up full width and vertical space
            newDisplayLines.push({
              paraIndex: i,
              startOffset: 0,
              endOffset: paraText.length,
              text: '',
              meta,
              isImage: true,
              imageId: imageId,
              imageHeight: imageLines,
            });

            // Add placeholder lines for the image height
            for (let j = 1; j < imageLines; j++) {
              newDisplayLines.push({
                paraIndex: i,
                startOffset: 0,
                endOffset: paraText.length,
                text: '',
                meta,
                isImage: true,
                imageId: imageId,
              });
            }
          }
        }
        continue;
      }

      // Handle list numbering
      let listNumber: number | undefined;
      if (meta.listType === 'numbered') {
        numberedListCounter++;
        listNumber = numberedListCounter;
      } else if (meta.listType === 'bullet') {
        listNumber = undefined;
        numberedListCounter = 0;
      } else {
        numberedListCounter = 0;
      }

      // Check if there's an active float affecting this line
      const currentLineIndex = newDisplayLines.length;
      let floatReduction: { side: 'left' | 'right'; width: number } | undefined;

      for (const float of newActiveFloats) {
        if (currentLineIndex >= float.startLine && currentLineIndex < float.endLine) {
          floatReduction = { side: float.side, width: float.width };
          break;
        }
      }

      const wrapped = wrapParagraph(i, paraText, listNumber, floatReduction);

      for (let j = 0; j < wrapped.length; j++) {
        const lineIdx = newDisplayLines.length;
        let lineFloatReduction: { side: 'left' | 'right'; width: number } | undefined;

        for (const float of newActiveFloats) {
          if (lineIdx >= float.startLine && lineIdx < float.endLine) {
            lineFloatReduction = { side: float.side, width: float.width };
            break;
          }
        }

        wrapped[j].floatReduction = lineFloatReduction;
        if (j === wrapped.length - 1) {
          wrapped[j].isLastLineOfParagraph = true;
        }
        newDisplayLines.push(wrapped[j]);
      }
    }

    // Second pass: Assign page indices based on actual Y positions including paragraph spacing
    // This ensures text doesn't overflow past the bottom margin
    const scaledParagraphSpacing = (paragraphSpacingValue * $zoomLevel) / 100;
    let currentY = 0;
    let currentPage = 0;
    let currentColumn = 0;
    const maxColumnHeight = scaledContentHeight;

    for (let i = 0; i < newDisplayLines.length; i++) {
      const dl = newDisplayLines[i];

      // Calculate line height for this line
      let lineHeight = scaledLineHeight;
      if (dl.isImage && dl.imageHeight) {
        lineHeight = dl.imageHeight * scaledLineHeight;
      }

      // Add paragraph spacing after last line of paragraph
      const spacingAfter = dl.isLastLineOfParagraph ? scaledParagraphSpacing : 0;

      // Check if this line would overflow the current column/page
      if (currentY + lineHeight > maxColumnHeight && !dl.isPageBreak) {
        // Move to next column or page
        if (columnCount > 1 && currentColumn < columnCount - 1) {
          currentColumn++;
          currentY = 0;
        } else {
          currentPage++;
          currentColumn = 0;
          currentY = 0;
        }
      }

      // Assign position to this line
      dl.pageIndex = currentPage;
      dl.yPosition = currentY;
      dl.columnIndex = currentColumn;

      // Advance Y position
      currentY += lineHeight + spacingAfter;

      // Handle page breaks - move to next page
      if (dl.isPageBreak) {
        currentPage++;
        currentColumn = 0;
        currentY = 0;
      }
    }

    activeFloats = newActiveFloats;
    displayLines = newDisplayLines;

    // Calculate total pages based on assigned page indices
    const maxPageIndex = newDisplayLines.reduce((max, dl) => Math.max(max, dl.pageIndex ?? 0), 0);
    totalPages.set(maxPageIndex + 1);

    // Update headings store for navigation sidebar
    updateHeadings();
  }

  /**
   * Extracts heading paragraphs and updates the navigation store.
   * Scans all paragraphs for heading block types (h1-h4) and builds
   * a hierarchical structure for the document outline sidebar.
   */
  function updateHeadings() {
    const newHeadings: HeadingItem[] = [];
    for (let i = 0; i < paragraphs.length; i++) {
      const meta = paragraphMeta[i] || createDefaultMeta();
      const blockType = meta.blockType;
      if (blockType.startsWith('h')) {
        const level = parseInt(blockType[1]);
        if (level >= 1 && level <= 4) {
          newHeadings.push({
            id: `heading-${i}`,
            text: paragraphs[i],
            level,
            paraIndex: i,
          });
        }
      }
    }
    headings.set(newHeadings);
  }

  /**
   * Converts a paragraph position to a display line position.
   * Used for mapping cursor/selection positions to rendered coordinates.
   *
   * @param para - Paragraph index in the document
   * @param offset - Character offset within the paragraph
   * @returns Display line index and column offset
   */
  function paraToDisplayPos(para: number, offset: number): { line: number; col: number } {
    for (let i = 0; i < displayLines.length; i++) {
      const dl = displayLines[i];
      if (dl.paraIndex === para && offset >= dl.startOffset && offset <= dl.endOffset) {
        return { line: i, col: offset - dl.startOffset };
      }
    }
    const lastLine = displayLines.length - 1;
    return { line: lastLine, col: displayLines[lastLine]?.text.length || 0 };
  }

  /**
   * Converts a display line position to a paragraph position.
   * Used for mapping click coordinates back to document positions.
   *
   * @param line - Display line index
   * @param col - Column offset within the display line
   * @returns Paragraph index and character offset
   */
  function displayToPara(line: number, col: number): { para: number; offset: number } {
    if (line < 0 || line >= displayLines.length) {
      return { para: paragraphs.length - 1, offset: paragraphs[paragraphs.length - 1].length };
    }
    const dl = displayLines[line];
    return { para: dl.paraIndex, offset: dl.startOffset + Math.min(col, dl.text.length) };
  }

  /**
   * Navigates the cursor to a specific paragraph.
   * Used by the document outline sidebar for heading navigation.
   * Scrolls the view to show the target paragraph and focuses the editor.
   *
   * @param paraIndex - Target paragraph index
   */
  export function navigateToParagraph(paraIndex: number) {
    if (paraIndex < 0 || paraIndex >= paragraphs.length) return;

    // Move cursor to the beginning of the paragraph
    cursorPara = paraIndex;
    cursorOffset = 0;
    selectionStart = null;
    selectionEnd = null;

    // Update current page
    const displayPos = paraToDisplayPos(cursorPara, cursorOffset);
    const pageNum = Math.floor(displayPos.line / linesPerPage) + 1;
    currentPage.set(pageNum);

    // Scroll to show the paragraph
    scrollToCursor();
    hiddenTextarea?.focus();
    renderAllPages();
  }

  onMount(() => {
    hiddenTextarea?.focus();
    // Wait for next tick to ensure measureCanvas is bound
    requestAnimationFrame(() => {
      recomputeDisplayLines();
      renderAllPages();
      // Initialize history with initial state
      historyManager.push(getEditorSnapshot());
      updateHistoryState();
    });

    // Add global paste handler for when textarea might lose focus
    const handleGlobalPaste = (event: ClipboardEvent) => {
      // Only handle if focus is on the editor area
      if (!editorContainer?.contains(document.activeElement) && document.activeElement !== hiddenTextarea) {
        return;
      }
      handlePaste(event);
    };

    // Global mouse handlers for resize, crop, and drag
    const handleGlobalMouseMove = (event: MouseEvent) => {
      if (isResizing) {
        handleResizeMove(event.clientX, event.clientY);
      } else if (isCropping && cropHandle) {
        handleCropMove(event.clientX, event.clientY);
      } else if (isDragging) {
        handleDragMove(event.clientX, event.clientY);
      }
    };

    const handleGlobalMouseUp = () => {
      if (isResizing) {
        endResize();
      }
      if (cropHandle) {
        endCropDrag();
      }
      if (isDragging) {
        endDrag();
      }
    };

    document.addEventListener('paste', handleGlobalPaste);
    document.addEventListener('mousemove', handleGlobalMouseMove);
    document.addEventListener('mouseup', handleGlobalMouseUp);

    return () => {
      document.removeEventListener('paste', handleGlobalPaste);
      document.removeEventListener('mousemove', handleGlobalMouseMove);
      document.removeEventListener('mouseup', handleGlobalMouseUp);
    };
  });

  $effect(() => {
    if (paragraphs && $zoomLevel && $pageConfig && scaledContentWidth > 0) {
      recomputeDisplayLines();
    }
  });

  $effect(() => {
    if (displayLines.length > 0 && cursorPara !== undefined) {
      renderAllPages();
    }
  });

  /**
   * Renders a single image onto the canvas.
   * Handles cropping, opacity for behind-text images, selection highlighting,
   * resize handles, and drag/crop mode indicators.
   *
   * @param ctx - Canvas 2D rendering context
   * @param docImage - Document image metadata
   * @param img - Loaded HTMLImageElement
   * @param imageX - X coordinate for image placement
   * @param imageY - Y coordinate for image placement
   * @param pageIndex - Current page being rendered
   */
  function renderImage(
    ctx: CanvasRenderingContext2D,
    docImage: DocumentImage,
    img: HTMLImageElement,
    imageX: number,
    imageY: number,
    pageIndex: number
  ) {
    const scaledWidth = (docImage.width * $zoomLevel) / 100;

    // Draw the image with crop applied
    const cropTop = docImage.cropTop || 0;
    const cropRight = docImage.cropRight || 0;
    const cropBottom = docImage.cropBottom || 0;
    const cropLeft = docImage.cropLeft || 0;

    const srcX = (cropLeft / 100) * img.naturalWidth;
    const srcY = (cropTop / 100) * img.naturalHeight;
    const srcW = ((100 - cropLeft - cropRight) / 100) * img.naturalWidth;
    const srcH = ((100 - cropTop - cropBottom) / 100) * img.naturalHeight;

    const cropAspect = srcW / srcH;
    const destW = scaledWidth;
    const destH = scaledWidth / cropAspect;

    // Apply opacity for behind images
    if (docImage.wrapStyle === 'behind') {
      ctx.globalAlpha = 0.5;
    }

    ctx.drawImage(img, srcX, srcY, srcW, srcH, imageX, imageY, destW, destH);

    // Reset opacity
    ctx.globalAlpha = 1;

    // Draw selection border if selected
    if (selectedImageId === docImage.id) {
      ctx.strokeStyle = isDragging ? '#4caf50' : '#1a73e8';
      ctx.lineWidth = 2;
      ctx.setLineDash([]);
      ctx.strokeRect(imageX - 2, imageY - 2, destW + 4, destH + 4);

      // Draw resize handles (8 handles)
      const handleSize = 8;
      ctx.fillStyle = isCropping ? '#ff9800' : (isDragging ? '#4caf50' : '#1a73e8');

      // Corners
      ctx.fillRect(imageX - handleSize/2, imageY - handleSize/2, handleSize, handleSize);
      ctx.fillRect(imageX + destW - handleSize/2, imageY - handleSize/2, handleSize, handleSize);
      ctx.fillRect(imageX - handleSize/2, imageY + destH - handleSize/2, handleSize, handleSize);
      ctx.fillRect(imageX + destW - handleSize/2, imageY + destH - handleSize/2, handleSize, handleSize);

      // Edge midpoints
      ctx.fillRect(imageX + destW/2 - handleSize/2, imageY - handleSize/2, handleSize, handleSize);
      ctx.fillRect(imageX + destW/2 - handleSize/2, imageY + destH - handleSize/2, handleSize, handleSize);
      ctx.fillRect(imageX - handleSize/2, imageY + destH/2 - handleSize/2, handleSize, handleSize);
      ctx.fillRect(imageX + destW - handleSize/2, imageY + destH/2 - handleSize/2, handleSize, handleSize);

      // Draw move cursor indicator when dragging
      if (isDragging) {
        ctx.fillStyle = 'rgba(76, 175, 80, 0.2)';
        ctx.fillRect(imageX, imageY, destW, destH);
      }

      // Draw crop mode indicator
      if (isCropping) {
        ctx.strokeStyle = '#ff9800';
        ctx.lineWidth = 2;
        ctx.setLineDash([5, 5]);
        ctx.strokeRect(imageX, imageY, destW, destH);
        ctx.setLineDash([]);
      }
    } else {
      // Draw subtle border
      ctx.strokeStyle = '#e0e0e0';
      ctx.lineWidth = 1;
      ctx.setLineDash([]);
      ctx.strokeRect(imageX, imageY, destW, destH);
    }
  }

  /**
   * Calculates the position of an image on a specific page.
   * Handles both absolute positioning (dragged images) and line-based positioning.
   * Accounts for multi-column layouts and horizontal alignment settings.
   *
   * @param docImage - Document image metadata
   * @param displayLineIdx - Display line where the image anchor is located
   * @param pageIndex - Page being rendered
   * @param startLine - First display line on the current page
   * @returns Position coordinates and visibility flag
   */
  function getImagePosition(
    docImage: DocumentImage,
    displayLineIdx: number,
    pageIndex: number,
    startLine: number
  ): { x: number; y: number; visible: boolean } {
    const scaledWidth = (docImage.width * $zoomLevel) / 100;
    const dl = displayLines[displayLineIdx];
    const isInline = docImage.wrapStyle === 'inline';

    // For inline images, ignore absolute positioning - they follow text alignment
    // For other wrap styles, check if image has absolute position (has been dragged)
    if (!isInline && docImage.x !== undefined && docImage.y !== undefined) {
      const pageHeight = contentDims.height;
      const imagePageIndex = Math.floor(docImage.y / pageHeight);

      if (imagePageIndex === pageIndex) {
        const yOnPage = docImage.y - (imagePageIndex * pageHeight);
        const imageX = marginLeft + (docImage.x * $zoomLevel) / 100;
        const imageY = marginTop + (yOnPage * $zoomLevel) / 100;
        return { x: imageX, y: imageY, visible: true };
      }
      return { x: 0, y: 0, visible: false };
    }

    // Use line-based positioning
    const lineOnPage = displayLineIdx - startLine;
    if (lineOnPage < 0 || lineOnPage >= linesPerPage) {
      return { x: 0, y: 0, visible: false };
    }

    // Calculate column and line within column for multi-column layout
    const colIndex = columnCount > 1 ? Math.floor(lineOnPage / linesPerColumn) : 0;
    const lineInCol = columnCount > 1 ? lineOnPage % linesPerColumn : lineOnPage;
    const columnOffsetX = colIndex * (columnWidth + columnGap);

    const imageY = marginTop + lineInCol * scaledLineHeight;

    // Calculate X position based on alignment
    // For inline images, use paragraph's text alignment
    // For other images, use image's horizontalAlign property
    let imageX: number;
    let alignment: 'left' | 'center' | 'right';

    if (isInline && dl?.meta) {
      // Inline images follow the paragraph's text alignment
      const textAlign = dl.meta.align;
      if (textAlign === 'center') {
        alignment = 'center';
      } else if (textAlign === 'right') {
        alignment = 'right';
      } else {
        alignment = 'left';
      }
    } else {
      // Other image types use their own horizontalAlign property
      alignment = docImage.horizontalAlign || 'left';
    }

    if (alignment === 'center') {
      imageX = marginLeft + columnOffsetX + (columnWidth - scaledWidth) / 2;
    } else if (alignment === 'right') {
      imageX = marginLeft + columnOffsetX + columnWidth - scaledWidth;
    } else {
      imageX = marginLeft + columnOffsetX;
    }

    return { x: imageX, y: imageY, visible: true };
  }

  /**
   * Calculates image position using pre-computed line positions.
   * Uses the yPosition stored in the display line for accurate positioning.
   *
   * @param docImage - Document image metadata
   * @param dl - Display line with pre-computed position
   * @param pageIndex - Page being rendered
   * @returns Position coordinates and visibility flag
   */
  function getImagePositionFromLine(
    docImage: DocumentImage,
    dl: DisplayLine,
    pageIndex: number
  ): { x: number; y: number; visible: boolean } {
    const scaledWidth = (docImage.width * $zoomLevel) / 100;
    const isInline = docImage.wrapStyle === 'inline';

    // Check if image is on this page
    if (dl.pageIndex !== pageIndex) {
      return { x: 0, y: 0, visible: false };
    }

    // For non-inline images with absolute position
    if (!isInline && docImage.x !== undefined && docImage.y !== undefined) {
      const pageHeight = contentDims.height;
      const imagePageIndex = Math.floor(docImage.y / pageHeight);

      if (imagePageIndex === pageIndex) {
        const yOnPage = docImage.y - (imagePageIndex * pageHeight);
        const imageX = marginLeft + (docImage.x * $zoomLevel) / 100;
        const imageY = marginTop + (yOnPage * $zoomLevel) / 100;
        return { x: imageX, y: imageY, visible: true };
      }
      return { x: 0, y: 0, visible: false };
    }

    // Use pre-computed Y position from layout
    const imageY = marginTop + (dl.yPosition ?? 0);

    // Calculate column offset for multi-column layout
    const colIndex = dl.columnIndex ?? 0;
    const columnOffsetX = colIndex * (columnWidth + columnGap);

    // Calculate X position based on alignment
    let alignment: 'left' | 'center' | 'right';
    if (isInline && dl.meta) {
      const textAlign = dl.meta.align;
      alignment = textAlign === 'center' ? 'center' : textAlign === 'right' ? 'right' : 'left';
    } else {
      alignment = docImage.horizontalAlign || 'left';
    }

    let imageX: number;
    if (alignment === 'center') {
      imageX = marginLeft + columnOffsetX + (columnWidth - scaledWidth) / 2;
    } else if (alignment === 'right') {
      imageX = marginLeft + columnOffsetX + columnWidth - scaledWidth;
    } else {
      imageX = marginLeft + columnOffsetX;
    }

    return { x: imageX, y: imageY, visible: true };
  }

  /**
   * Renders all pages of the document.
   *
   * This is the main rendering function that draws the complete document.
   * It processes each page in sequence, rendering in multiple passes:
   * 1. Behind-text images (wrapStyle: 'behind')
   * 2. Floating images with text wrapping
   * 3. Inline and top-bottom images
   * 4. Text content with formatting, selection, and cursor
   * 5. In-front images (wrapStyle: 'in-front')
   * 6. Page numbers
   *
   * The function handles paragraph spacing, multi-column layouts,
   * text alignment, list markers, and block-level styling.
   */
  function renderAllPages() {
    const pageCount = numPages;

    for (let pageIndex = 0; pageIndex < pageCount; pageIndex++) {
      const canvas = canvases[pageIndex];
      if (!canvas) continue;

      const ctx = canvas.getContext('2d');
      if (!ctx) continue;

      // Set canvas size with device pixel ratio for sharp text
      const dpr = window.devicePixelRatio || 1;
      canvas.width = scaledPageWidth * dpr;
      canvas.height = scaledPageHeight * dpr;
      canvas.style.width = `${scaledPageWidth}px`;
      canvas.style.height = `${scaledPageHeight}px`;
      ctx.scale(dpr, dpr);

      // Clear and draw page background
      ctx.fillStyle = 'white';
      ctx.fillRect(0, 0, scaledPageWidth, scaledPageHeight);

      // Set up text rendering
      ctx.font = getFontStyle();
      ctx.fillStyle = '#202124';
      ctx.textBaseline = 'top';

      // Get lines belonging to this page using pre-computed pageIndex
      const pageLines = displayLines.filter(dl => dl.pageIndex === pageIndex);

      // Get cursor display position
      const cursorDisplay = paraToDisplayPos(cursorPara, cursorOffset);

      // Normalize selection
      let selStartDisplay: { line: number; col: number } | null = null;
      let selEndDisplay: { line: number; col: number } | null = null;
      if (selectionStart && selectionEnd) {
        const s1 = paraToDisplayPos(selectionStart.para, selectionStart.offset);
        const s2 = paraToDisplayPos(selectionEnd.para, selectionEnd.offset);
        if (s1.line < s2.line || (s1.line === s2.line && s1.col <= s2.col)) {
          selStartDisplay = s1;
          selEndDisplay = s2;
        } else {
          selStartDisplay = s2;
          selEndDisplay = s1;
        }
      }

      // === PASS 1: Render "behind" images first (before text) ===
      for (const dl of pageLines) {
        if (!dl.isImage || !dl.imageId || !dl.imageHeight) continue;

        const docImage = images.find(img => img.id === dl.imageId);
        if (!docImage || docImage.wrapStyle !== 'behind') continue;

        const img = loadedImages.get(docImage.id);
        if (!img) continue;

        const displayLineIdx = displayLines.indexOf(dl);
        const pos = getImagePositionFromLine(docImage, dl, pageIndex);
        if (pos.visible) {
          renderImage(ctx, docImage, img, pos.x, pos.y, pageIndex);
        }
      }

      // === PASS 2: Render float images (square, tight, through) ===
      for (const float of activeFloats) {
        const docImage = images.find(img => img.id === float.id);
        if (!docImage) continue;

        const img = loadedImages.get(docImage.id);
        if (!img) continue;

        const scaledWidth = (docImage.width * $zoomLevel) / 100;

        let floatX: number;
        let floatY: number;
        let isVisibleOnPage = false;

        // Check if image has absolute position
        if (docImage.x !== undefined && docImage.y !== undefined) {
          const pageHeight = contentDims.height;
          const imagePageIndex = Math.floor(docImage.y / pageHeight);

          if (imagePageIndex === pageIndex) {
            const yOnPage = docImage.y - (imagePageIndex * pageHeight);
            floatX = marginLeft + (docImage.x * $zoomLevel) / 100;
            floatY = marginTop + (yOnPage * $zoomLevel) / 100;
            isVisibleOnPage = true;
          }
        } else {
          // Find the display line for this float and use pre-computed page assignment
          const floatLine = displayLines[float.startLine];
          if (floatLine && floatLine.pageIndex === pageIndex) {
            // Use pre-computed Y position and column
            floatY = marginTop + (floatLine.yPosition ?? 0);
            const floatColIndex = floatLine.columnIndex ?? 0;
            const floatColumnOffsetX = floatColIndex * (columnWidth + columnGap);
            floatX = float.side === 'left'
              ? marginLeft + floatColumnOffsetX
              : marginLeft + floatColumnOffsetX + columnWidth - scaledWidth;
            isVisibleOnPage = true;
          }
        }

        if (isVisibleOnPage) {
          renderImage(ctx, docImage, img, floatX!, floatY!, pageIndex);
        }
      }

      // === PASS 3: Render inline and top-bottom images ===
      for (const dl of pageLines) {
        if (!dl.isImage || !dl.imageId || !dl.imageHeight) continue;

        const docImage = images.find(img => img.id === dl.imageId);
        if (!docImage) continue;

        // Only render inline and top-bottom here
        if (docImage.wrapStyle !== 'inline' && docImage.wrapStyle !== 'top-bottom') continue;

        const img = loadedImages.get(docImage.id);
        if (!img) continue;

        const pos = getImagePositionFromLine(docImage, dl, pageIndex);
        if (pos.visible) {
          renderImage(ctx, docImage, img, pos.x, pos.y, pageIndex);
        }
      }

      // === PASS 4: Render text ===
      for (const dl of pageLines) {
        if (dl.isImage || dl.isPageBreak) continue;

        // Use pre-computed Y position
        const y = marginTop + (dl.yPosition ?? 0);

        // Calculate column offset for multi-column layout
        const colIndex = dl.columnIndex ?? 0;
        const columnOffsetX = colIndex * (columnWidth + columnGap);

        const text = dl.text;
        const meta = dl.meta;
        const lineIndex = displayLines.indexOf(dl);

        // Calculate list indent and float offset
        const listIndent = meta.listType !== 'none' ? scaledFontSize * 1.5 : 0;
        const floatOffset = dl.floatReduction && dl.floatReduction.side === 'left' ? dl.floatReduction.width + 10 : 0;
        const textStartX = marginLeft + columnOffsetX + listIndent + floatOffset;

        // Get font style based on block type
        // Use paragraph-specific font size if set, otherwise use global
        const baseFontSize = meta.fontSize ? (meta.fontSize * $zoomLevel) / 100 : scaledFontSize;
        let blockFontSize = baseFontSize;
        let blockFontWeight = isBold ? 'bold ' : '';
        let blockFontStyle = isItalic ? 'italic ' : '';

        switch (meta.blockType) {
          case 'h1':
            blockFontSize = baseFontSize * 2;
            blockFontWeight = 'bold ';
            break;
          case 'h2':
            blockFontSize = baseFontSize * 1.5;
            blockFontWeight = 'bold ';
            break;
          case 'h3':
            blockFontSize = baseFontSize * 1.17;
            blockFontWeight = 'bold ';
            break;
          case 'h4':
            blockFontWeight = 'bold ';
            break;
          case 'blockquote':
            blockFontStyle = 'italic ';
            break;
        }

        const lineFont = `${blockFontStyle}${blockFontWeight}${blockFontSize}px ${fontFamily}`;
        ctx.font = lineFont;

        const textWidth = ctx.measureText(text).width;
        const floatWidthReduction = dl.floatReduction ? dl.floatReduction.width + 10 : 0;
        const availableWidth = columnWidth - listIndent - floatWidthReduction;

        let x = textStartX;
        let wordSpacing = 0;
        const words = text.split(' ');
        const isLastLineOfPara = dl.endOffset >= paragraphs[dl.paraIndex].length;

        switch (meta.align) {
          case 'center':
            x = textStartX + (availableWidth - textWidth) / 2;
            break;
          case 'right':
            x = textStartX + availableWidth - textWidth;
            break;
          case 'justify':
            if (!isLastLineOfPara && words.length > 1 && text.trim().length > 0) {
              const extraSpace = availableWidth - textWidth;
              wordSpacing = extraSpace / (words.length - 1);
            }
            break;
        }

        // Draw selection highlight
        if (selStartDisplay && selEndDisplay) {
          if (lineIndex >= selStartDisplay.line && lineIndex <= selEndDisplay.line) {
            ctx.fillStyle = '#b4d7ff';
            const startCol = lineIndex === selStartDisplay.line ? selStartDisplay.col : 0;
            const endCol = lineIndex === selEndDisplay.line ? selEndDisplay.col : text.length;
            const selStartX = x + ctx.measureText(text.substring(0, startCol)).width;
            const selWidth = ctx.measureText(text.substring(startCol, endCol)).width || 5;
            ctx.fillRect(selStartX, y, selWidth, scaledLineHeight);
          }
        }

        // Draw list marker
        if (dl.startOffset === 0 && meta.listType !== 'none') {
          ctx.fillStyle = '#202124';
          if (meta.listType === 'bullet') {
            const bulletX = marginLeft + columnOffsetX + scaledFontSize * 0.5;
            const bulletY = y + scaledLineHeight / 2;
            ctx.beginPath();
            ctx.arc(bulletX, bulletY, scaledFontSize * 0.15, 0, Math.PI * 2);
            ctx.fill();
          } else if (meta.listType === 'numbered' && dl.listNumber) {
            ctx.font = `${scaledFontSize}px ${fontFamily}`;
            ctx.textAlign = 'right';
            ctx.fillText(`${dl.listNumber}.`, marginLeft + columnOffsetX + scaledFontSize * 1.2, y + (scaledLineHeight - scaledFontSize) / 2);
            ctx.textAlign = 'left';
            ctx.font = lineFont;
          }
        }

        // Draw blockquote indicator
        if (meta.blockType === 'blockquote' && dl.startOffset === 0) {
          ctx.fillStyle = '#ccc';
          ctx.fillRect(marginLeft + columnOffsetX, y, 3, scaledLineHeight);
        }

        // Draw text
        ctx.fillStyle = meta.textColor || '#202124';
        ctx.font = lineFont;
        const textY = y + (scaledLineHeight - blockFontSize) / 2;
        const scaledLetterSpacing = (letterSpacingValue * $zoomLevel) / 100;

        // Function to draw text with letter spacing
        const drawTextWithSpacing = (str: string, startX: number) => {
          if (scaledLetterSpacing === 0) {
            ctx.fillText(str, startX, textY);
            return ctx.measureText(str).width;
          } else {
            let charX = startX;
            for (let c = 0; c < str.length; c++) {
              ctx.fillText(str[c], charX, textY);
              charX += ctx.measureText(str[c]).width + scaledLetterSpacing;
            }
            return charX - startX - scaledLetterSpacing; // subtract last spacing
          }
        };

        if (wordSpacing > 0 && meta.align === 'justify') {
          let wordX = x;
          for (let w = 0; w < words.length; w++) {
            drawTextWithSpacing(words[w], wordX);
            wordX += ctx.measureText(words[w]).width + (words[w].length * scaledLetterSpacing);
            if (w < words.length - 1) {
              wordX += ctx.measureText(' ').width + wordSpacing;
            }
          }
        } else {
          drawTextWithSpacing(text, x);
        }

        const renderedWidth = wordSpacing > 0 ? availableWidth : textWidth;

        // Draw underline
        if (isUnderline && text.length > 0) {
          const underlineY = y + scaledLineHeight - 4;
          ctx.strokeStyle = '#202124';
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(x, underlineY);
          ctx.lineTo(x + renderedWidth, underlineY);
          ctx.stroke();
        }

        // Draw strikethrough
        if (isStrikethrough && text.length > 0) {
          const strikeY = y + scaledLineHeight / 2;
          ctx.strokeStyle = '#202124';
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(x, strikeY);
          ctx.lineTo(x + renderedWidth, strikeY);
          ctx.stroke();
        }

        // Draw cursor
        if (lineIndex === cursorDisplay.line && !selectionStart) {
          let cursorX = x;
          if (wordSpacing > 0 && meta.align === 'justify') {
            const textBeforeCursor = text.substring(0, cursorDisplay.col);
            const wordsBeforeCursor = textBeforeCursor.split(' ');
            const spacesBeforeCursor = wordsBeforeCursor.length - 1;
            cursorX = x + ctx.measureText(textBeforeCursor).width + (spacesBeforeCursor * wordSpacing);
          } else {
            cursorX = x + ctx.measureText(text.substring(0, cursorDisplay.col)).width;
          }
          ctx.fillStyle = '#000';
          ctx.fillRect(cursorX, y + 2, 2, scaledLineHeight - 4);
        }

      }

      // === PASS 5: Render "in-front" images last (after text) ===
      for (const dl of pageLines) {
        if (!dl.isImage || !dl.imageId || !dl.imageHeight) continue;

        const docImage = images.find(img => img.id === dl.imageId);
        if (!docImage || docImage.wrapStyle !== 'in-front') continue;

        const img = loadedImages.get(docImage.id);
        if (!img) continue;

        const pos = getImagePositionFromLine(docImage, dl, pageIndex);
        if (pos.visible) {
          renderImage(ctx, docImage, img, pos.x, pos.y, pageIndex);
        }
      }

      // Draw page number
      ctx.fillStyle = '#999';
      ctx.font = `10px ${fontFamily}`;
      ctx.textAlign = 'center';
      ctx.fillText(`${pageIndex + 1}`, scaledPageWidth / 2, scaledPageHeight - 20);
      ctx.textAlign = 'left';
    }
  }

  /**
   * Handles keyboard events for the editor.
   *
   * Processes all keyboard input including:
   * - Text input and deletion
   * - Navigation (arrows, Home, End, Page Up/Down)
   * - Selection (Shift+arrows, Ctrl+A)
   * - Clipboard operations (Ctrl+C/X/V)
   * - Formatting shortcuts (Ctrl+B/I/U)
   * - Special characters (Tab for indent)
   * - Page breaks (Alt+Enter)
   * - Image operations (Delete/Backspace when image selected)
   *
   * @param event - The keyboard event to process
   */
  function handleKeyDown(event: KeyboardEvent) {
    const key = event.key;

    if (selectedImageId && (key === 'Delete' || key === 'Backspace')) {
      event.preventDefault();
      deleteSelectedImage();
      return;
    }

    // Clear image selection on any other key
    if (selectedImageId && key !== 'Delete' && key !== 'Backspace') {
      selectedImageId = null;
      renderAllPages();
    }

    // Alt+Enter for page break
    if (event.altKey && key === 'Enter') {
      event.preventDefault();
      insertPageBreak();
      return;
    }

    if (event.ctrlKey || event.metaKey) {
      switch (key.toLowerCase()) {
        case 'b':
          event.preventDefault();
          isBold = !isBold;
          recomputeDisplayLines();
          return;
        case 'i':
          event.preventDefault();
          isItalic = !isItalic;
          recomputeDisplayLines();
          return;
        case 'a':
          event.preventDefault();
          selectionStart = { para: 0, offset: 0 };
          selectionEnd = { para: paragraphs.length - 1, offset: paragraphs[paragraphs.length - 1].length };
          return;
        case 'c':
        case 'x':
          if (selectionStart && selectionEnd) {
            const text = getSelectedText();
            navigator.clipboard.writeText(text);
            if (key.toLowerCase() === 'x') {
              saveToHistory(); // Save before cut
              deleteSelection();
            }
          }
          event.preventDefault();
          return;
        case 'v':
          event.preventDefault();
          saveToHistory(); // Save before paste
          navigator.clipboard.readText().then(text => {
            if (selectionStart) {
              deleteSelection();
            }
            insertText(text);
          });
          return;
        case 'z':
          event.preventDefault();
          if (event.shiftKey) {
            // Ctrl+Shift+Z = Redo
            performRedo();
          } else {
            // Ctrl+Z = Undo
            performUndo();
          }
          return;
        case 'y':
          // Ctrl+Y = Redo
          event.preventDefault();
          performRedo();
          return;
      }
    }

    // Get current display position for navigation
    const cursorDisplay = paraToDisplayPos(cursorPara, cursorOffset);

    // Handle selection with shift
    if (event.shiftKey && ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(key)) {
      if (!selectionStart) {
        selectionStart = { para: cursorPara, offset: cursorOffset };
      }
    } else if (!event.shiftKey && ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(key)) {
      selectionStart = null;
      selectionEnd = null;
    }

    switch (key) {
      case 'ArrowLeft':
        event.preventDefault();
        if (cursorOffset > 0) {
          cursorOffset--;
        } else if (cursorPara > 0) {
          cursorPara--;
          cursorOffset = paragraphs[cursorPara].length;
        }
        break;

      case 'ArrowRight':
        event.preventDefault();
        if (cursorOffset < paragraphs[cursorPara].length) {
          cursorOffset++;
        } else if (cursorPara < paragraphs.length - 1) {
          cursorPara++;
          cursorOffset = 0;
        }
        break;

      case 'ArrowUp':
        event.preventDefault();
        if (cursorDisplay.line > 0) {
          const newPos = displayToPara(cursorDisplay.line - 1, cursorDisplay.col);
          cursorPara = newPos.para;
          cursorOffset = newPos.offset;
        }
        break;

      case 'ArrowDown':
        event.preventDefault();
        if (cursorDisplay.line < displayLines.length - 1) {
          const newPos = displayToPara(cursorDisplay.line + 1, cursorDisplay.col);
          cursorPara = newPos.para;
          cursorOffset = newPos.offset;
        }
        break;

      case 'Home':
        event.preventDefault();
        // Go to start of current display line
        {
          const dl = displayLines[cursorDisplay.line];
          cursorOffset = dl.startOffset;
        }
        break;

      case 'End':
        event.preventDefault();
        // Go to end of current display line
        {
          const dl = displayLines[cursorDisplay.line];
          cursorOffset = dl.endOffset;
        }
        break;

      case 'Backspace':
        event.preventDefault();
        saveToHistory(); // Save state before deletion
        if (selectionStart && selectionEnd) {
          deleteSelection();
        } else if (cursorOffset > 0) {
          paragraphs[cursorPara] = paragraphs[cursorPara].substring(0, cursorOffset - 1) + paragraphs[cursorPara].substring(cursorOffset);
          cursorOffset--;
          recomputeDisplayLines();
        } else if (cursorPara > 0) {
          // Check if previous paragraph is an image
          if (paragraphs[cursorPara - 1].startsWith(IMAGE_MARKER)) {
            // Select the image for deletion
            const imageId = paragraphs[cursorPara - 1].substring(1);
            selectedImageId = imageId;
            renderAllPages();
          } else {
            const prevLen = paragraphs[cursorPara - 1].length;
            paragraphs[cursorPara - 1] += paragraphs[cursorPara];
            paragraphs = [...paragraphs.slice(0, cursorPara), ...paragraphs.slice(cursorPara + 1)];
            // Remove the merged paragraph's metadata
            paragraphMeta = [...paragraphMeta.slice(0, cursorPara), ...paragraphMeta.slice(cursorPara + 1)];
            cursorPara--;
            cursorOffset = prevLen;
            recomputeDisplayLines();
          }
        }
        break;

      case 'Delete':
        event.preventDefault();
        saveToHistory(); // Save state before deletion
        if (selectionStart && selectionEnd) {
          deleteSelection();
        } else if (cursorOffset < paragraphs[cursorPara].length) {
          paragraphs[cursorPara] = paragraphs[cursorPara].substring(0, cursorOffset) + paragraphs[cursorPara].substring(cursorOffset + 1);
          recomputeDisplayLines();
        } else if (cursorPara < paragraphs.length - 1) {
          // Check if next paragraph is an image
          if (paragraphs[cursorPara + 1].startsWith(IMAGE_MARKER)) {
            // Select the image for deletion
            const imageId = paragraphs[cursorPara + 1].substring(1);
            selectedImageId = imageId;
            renderAllPages();
          } else {
            paragraphs[cursorPara] += paragraphs[cursorPara + 1];
            paragraphs = [...paragraphs.slice(0, cursorPara + 1), ...paragraphs.slice(cursorPara + 2)];
            // Remove the merged paragraph's metadata
            paragraphMeta = [...paragraphMeta.slice(0, cursorPara + 1), ...paragraphMeta.slice(cursorPara + 2)];
            recomputeDisplayLines();
          }
        }
        break;

      case 'Enter':
        event.preventDefault();
        saveToHistory(); // Save state before new paragraph
        if (selectionStart && selectionEnd) {
          deleteSelection();
        }
        {
          const before = paragraphs[cursorPara].substring(0, cursorOffset);
          const after = paragraphs[cursorPara].substring(cursorOffset);
          paragraphs[cursorPara] = before;
          paragraphs = [...paragraphs.slice(0, cursorPara + 1), after, ...paragraphs.slice(cursorPara + 1)];
          // Copy current paragraph's metadata to new paragraph
          const currentMeta = paragraphMeta[cursorPara] || createDefaultMeta();
          // Reset block type to normal 'p' for new line after headings
          const newBlockType = currentMeta.blockType.startsWith('h') ? 'p' : currentMeta.blockType;
          paragraphMeta = [
            ...paragraphMeta.slice(0, cursorPara + 1),
            { ...currentMeta, blockType: newBlockType as BlockType },
            ...paragraphMeta.slice(cursorPara + 1)
          ];
          cursorPara++;
          cursorOffset = 0;
          recomputeDisplayLines();
        }
        break;

      default:
        if (key.length === 1) {
          event.preventDefault();
          if (selectionStart && selectionEnd) {
            saveToHistory(); // Save before replacing selection
            deleteSelection();
          }
          insertText(key);
          scheduleHistorySave(); // Debounced save for typing
        }
    }

    if (event.shiftKey && ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(key)) {
      selectionEnd = { para: cursorPara, offset: cursorOffset };
    }

    // Update current page
    const newCursorDisplay = paraToDisplayPos(cursorPara, cursorOffset);
    const cursorPage = Math.floor(newCursorDisplay.line / linesPerPage) + 1;
    currentPage.set(cursorPage);

    scrollToCursor();
  }

  function insertText(text: string) {
    const lines = text.split(/\r?\n/);

    if (lines.length === 1) {
      paragraphs[cursorPara] = paragraphs[cursorPara].substring(0, cursorOffset) + text + paragraphs[cursorPara].substring(cursorOffset);
      cursorOffset += text.length;
    } else {
      const before = paragraphs[cursorPara].substring(0, cursorOffset);
      const after = paragraphs[cursorPara].substring(cursorOffset);
      const currentMeta = paragraphMeta[cursorPara] || createDefaultMeta();

      paragraphs[cursorPara] = before + lines[0];

      const newParas = lines.slice(1, -1);
      const lastPara = lines[lines.length - 1] + after;

      paragraphs = [
        ...paragraphs.slice(0, cursorPara + 1),
        ...newParas,
        lastPara,
        ...paragraphs.slice(cursorPara + 1)
      ];

      // Create metadata for new paragraphs (inherit from current)
      const newMetas = lines.slice(1).map(() => ({ ...currentMeta }));
      paragraphMeta = [
        ...paragraphMeta.slice(0, cursorPara + 1),
        ...newMetas,
        ...paragraphMeta.slice(cursorPara + 1)
      ];

      cursorPara += lines.length - 1;
      cursorOffset = lines[lines.length - 1].length;
    }

    recomputeDisplayLines();
  }

  function insertPageBreak() {
    saveToHistory(); // Save before inserting page break
    if (selectionStart && selectionEnd) {
      deleteSelection();
    }

    // Split current paragraph at cursor
    const before = paragraphs[cursorPara].substring(0, cursorOffset);
    const after = paragraphs[cursorPara].substring(cursorOffset);

    // Current paragraph becomes text before cursor
    paragraphs[cursorPara] = before;

    // Insert page break paragraph and text after cursor
    paragraphs = [
      ...paragraphs.slice(0, cursorPara + 1),
      PAGE_BREAK_MARKER,
      after,
      ...paragraphs.slice(cursorPara + 1)
    ];

    // Add metadata for new paragraphs
    const currentMeta = paragraphMeta[cursorPara] || createDefaultMeta();
    paragraphMeta = [
      ...paragraphMeta.slice(0, cursorPara + 1),
      { ...createDefaultMeta() }, // Page break meta (not really used)
      { ...currentMeta }, // After text meta
      ...paragraphMeta.slice(cursorPara + 1)
    ];

    // Move cursor to after the page break
    cursorPara += 2;
    cursorOffset = 0;

    recomputeDisplayLines();
    renderAllPages();
  }

  function getSelectedText(): string {
    if (!selectionStart || !selectionEnd) return '';

    let start = selectionStart;
    let end = selectionEnd;

    if (start.para > end.para || (start.para === end.para && start.offset > end.offset)) {
      [start, end] = [end, start];
    }

    if (start.para === end.para) {
      return paragraphs[start.para].substring(start.offset, end.offset);
    }

    let text = paragraphs[start.para].substring(start.offset);
    for (let i = start.para + 1; i < end.para; i++) {
      text += '\n' + paragraphs[i];
    }
    text += '\n' + paragraphs[end.para].substring(0, end.offset);

    return text;
  }

  function deleteSelection() {
    if (!selectionStart || !selectionEnd) return;

    let start = selectionStart;
    let end = selectionEnd;

    if (start.para > end.para || (start.para === end.para && start.offset > end.offset)) {
      [start, end] = [end, start];
    }

    if (start.para === end.para) {
      paragraphs[start.para] = paragraphs[start.para].substring(0, start.offset) + paragraphs[start.para].substring(end.offset);
    } else {
      const newPara = paragraphs[start.para].substring(0, start.offset) + paragraphs[end.para].substring(end.offset);
      paragraphs = [...paragraphs.slice(0, start.para), newPara, ...paragraphs.slice(end.para + 1)];
      // Remove deleted paragraphs' metadata (keep start.para's metadata)
      paragraphMeta = [...paragraphMeta.slice(0, start.para + 1), ...paragraphMeta.slice(end.para + 1)];
    }

    cursorPara = start.para;
    cursorOffset = start.offset;
    selectionStart = null;
    selectionEnd = null;

    recomputeDisplayLines();
  }

  function scrollToCursor() {
    const cursorDisplay = paraToDisplayPos(cursorPara, cursorOffset);
    const pageIndex = Math.floor(cursorDisplay.line / linesPerPage);
    const pageElement = canvasContainer?.children[pageIndex] as HTMLElement;
    pageElement?.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  }

  function handleCanvasMouseDown(event: MouseEvent, pageIndex: number) {
    const canvas = canvases[pageIndex];
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    // Check if clicking on a resize handle or inside a selected image
    if (selectedImageId && !isCropping) {
      const bounds = getSelectedImageBounds();
      if (bounds && bounds.pageIndex === pageIndex) {
        const handle = getResizeHandleAtPoint(x, y, bounds);
        if (handle) {
          event.preventDefault();
          startResize(handle, event.clientX, event.clientY);
          return;
        }
        // Check if clicking inside the image (for dragging)
        if (x >= bounds.x && x <= bounds.x + bounds.width && y >= bounds.y && y <= bounds.y + bounds.height) {
          event.preventDefault();
          startDrag(event.clientX, event.clientY, pageIndex);
          return;
        }
      }
    }

    // Check if clicking on a crop handle
    if (selectedImageId && isCropping) {
      const bounds = getSelectedImageBounds();
      if (bounds && bounds.pageIndex === pageIndex) {
        const handle = getResizeHandleAtPoint(x, y, bounds);
        if (handle) {
          event.preventDefault();
          startCropDrag(handle, event.clientX, event.clientY);
          return;
        }
      }
    }

    // Determine which column was clicked
    let clickedColumn = 0;
    if (columnCount > 1) {
      const xInContent = x - marginLeft;
      if (xInContent >= columnWidth + columnGap) {
        clickedColumn = 1;
      }
    }

    // Find display line index using pre-computed page assignments
    const yInContent = y - marginTop;
    const pageLines = displayLines.filter(dl => dl.pageIndex === pageIndex);

    let displayLineIndex = -1;

    // Find the line at the clicked Y position using pre-computed yPosition
    for (const dl of pageLines) {
      if (dl.yPosition === undefined) continue;

      // Calculate line height for this line
      let lineHeight = scaledLineHeight;
      if (dl.isImage && dl.imageHeight) {
        lineHeight = dl.imageHeight * scaledLineHeight;
      }

      const lineY = dl.yPosition;
      const lineBottom = lineY + lineHeight;

      if (yInContent >= lineY && yInContent < lineBottom) {
        displayLineIndex = displayLines.indexOf(dl);
        break;
      }
    }

    // If click is below all lines, select the last line on the page
    if (displayLineIndex === -1 && yInContent >= 0 && pageLines.length > 0) {
      displayLineIndex = displayLines.indexOf(pageLines[pageLines.length - 1]);
    }

    // Check if clicked on a float image first
    for (const float of activeFloats) {
      const docImage = images.find(img => img.id === float.id);
      if (docImage) {
        const scaledWidth = (docImage.width * $zoomLevel) / 100;
        const scaledHeight = (docImage.height * $zoomLevel) / 100;

        let floatX: number;
        let floatY: number;
        let isVisibleOnPage = false;

        // Check if image has absolute position
        if (docImage.x !== undefined && docImage.y !== undefined) {
          const pageHeight = contentDims.height;
          const imagePageIndex = Math.floor(docImage.y / pageHeight);

          if (imagePageIndex === pageIndex) {
            const yOnPage = docImage.y - (imagePageIndex * pageHeight);
            floatX = marginLeft + (docImage.x * $zoomLevel) / 100;
            floatY = marginTop + (yOnPage * $zoomLevel) / 100;
            isVisibleOnPage = true;
          }
        } else {
          // Use pre-computed page assignment
          const floatLine = displayLines[float.startLine];
          if (floatLine && floatLine.pageIndex === pageIndex) {
            floatY = marginTop + (floatLine.yPosition ?? 0);
            const floatColIndex = floatLine.columnIndex ?? 0;
            const floatColumnOffsetX = floatColIndex * (columnWidth + columnGap);
            floatX = float.side === 'left'
              ? marginLeft + floatColumnOffsetX
              : marginLeft + floatColumnOffsetX + columnWidth - scaledWidth;
            isVisibleOnPage = true;
          }
        }

        if (isVisibleOnPage && x >= floatX! && x <= floatX! + scaledWidth && y >= floatY! && y <= floatY! + scaledHeight) {
          selectedImageId = docImage.id;
          selectionStart = null;
          selectionEnd = null;
          // Show options popup near the image
          showImageOptions(event.clientX, event.clientY);
          hiddenTextarea?.focus();
          renderAllPages();
          return;
        }
      }
    }

    // Check if clicked on a non-float image (inline, top-bottom, behind, in-front)
    if (displayLineIndex >= 0 && displayLineIndex < displayLines.length) {
      const dl = displayLines[displayLineIndex];

      if (dl.isImage && dl.imageId) {
        // Find the image and check if click is within its bounds
        const docImage = images.find(img => img.id === dl.imageId);
        const isNonFloatImage = docImage && (
          docImage.wrapStyle === 'inline' ||
          docImage.wrapStyle === 'top-bottom' ||
          docImage.wrapStyle === 'behind' ||
          docImage.wrapStyle === 'in-front'
        );
        if (isNonFloatImage) {
          const scaledWidth = (docImage.width * $zoomLevel) / 100;
          const scaledHeight = (docImage.height * $zoomLevel) / 100;

          // Calculate X based on alignment
          let imageX: number;
          const columnOffsetX = clickedColumn * (columnWidth + columnGap);
          if (docImage.horizontalAlign === 'center') {
            imageX = marginLeft + columnOffsetX + (columnWidth - scaledWidth) / 2;
          } else if (docImage.horizontalAlign === 'right') {
            imageX = marginLeft + columnOffsetX + columnWidth - scaledWidth;
          } else {
            imageX = marginLeft + columnOffsetX;
          }

          // Find the first line of this image and use its pre-computed Y position
          let firstImageLine = displayLineIndex;
          while (firstImageLine > 0 && displayLines[firstImageLine - 1].imageId === dl.imageId) {
            firstImageLine--;
          }
          const firstLine = displayLines[firstImageLine];
          const imageY = marginTop + (firstLine.yPosition ?? 0);

          // Check if click is within image bounds
          if (x >= imageX && x <= imageX + scaledWidth && y >= imageY && y <= imageY + scaledHeight) {
            selectedImageId = docImage.id;
            selectionStart = null;
            selectionEnd = null;
            // Show options popup near the image
            showImageOptions(event.clientX, event.clientY);
            hiddenTextarea?.focus();
            renderAllPages();
            return;
          }
        }
      }
    }

    // Clear image selection and popup if clicking elsewhere
    if (isCropping) {
      endCropMode();
    }
    selectedImageId = null;
    showImageOptionsPopup = false;

    if (displayLineIndex >= 0 && displayLineIndex < displayLines.length) {
      // Clicked on an existing line
      const dl = displayLines[displayLineIndex];

      // Skip if it's an image line (but not on the image itself)
      if (dl.isImage) {
        // Move cursor to after the image paragraph
        cursorPara = dl.paraIndex + 1;
        cursorOffset = 0;
        if (cursorPara >= paragraphs.length) {
          cursorPara = paragraphs.length - 1;
          cursorOffset = paragraphs[cursorPara].length;
        }
      } else {
        const columnOffsetX = clickedColumn * (columnWidth + columnGap);
        const meta = dl.meta;

        const listIndent = meta.listType !== 'none' ? scaledFontSize * 1.5 : 0;
        const floatOffset = dl.floatReduction && dl.floatReduction.side === 'left' ? dl.floatReduction.width + 10 : 0;
        const textStartX = marginLeft + columnOffsetX + listIndent + floatOffset;

        const baseFontSize = meta.fontSize ? (meta.fontSize * $zoomLevel) / 100 : scaledFontSize;
        let blockFontSize = baseFontSize;
        switch (meta.blockType) {
          case 'h1': blockFontSize = baseFontSize * 2; break;
          case 'h2': blockFontSize = baseFontSize * 1.5; break;
          case 'h3': blockFontSize = baseFontSize * 1.17; break;
        }

        const floatWidthReduction = dl.floatReduction ? dl.floatReduction.width + 10 : 0;
        const availableWidth = columnWidth - listIndent - floatWidthReduction;

        const measureCtx = measureCanvas?.getContext('2d');
        if (measureCtx) {
          const blockFontWeight = ['h1', 'h2', 'h3', 'h4'].includes(meta.blockType) ? 'bold ' : (isBold ? 'bold ' : '');
          const blockFontStyle = meta.blockType === 'blockquote' ? 'italic ' : (isItalic ? 'italic ' : '');
          measureCtx.font = `${blockFontStyle}${blockFontWeight}${blockFontSize}px ${fontFamily}`;
        }

        const textWidth = measureCtx ? measureCtx.measureText(dl.text).width : dl.text.length * blockFontSize * 0.5;

        let lineStartX = textStartX;
        if (meta.align === 'center') {
          lineStartX = textStartX + (availableWidth - textWidth) / 2;
        } else if (meta.align === 'right') {
          lineStartX = textStartX + availableWidth - textWidth;
        }

        let col = 0;
        let currentX = lineStartX;

        for (let i = 0; i <= dl.text.length; i++) {
          const charWidth = i < dl.text.length && measureCtx
            ? measureCtx.measureText(dl.text[i]).width
            : 0;
          if (currentX + charWidth / 2 >= x) {
            col = i;
            break;
          }
          currentX += charWidth;
          col = i + 1;
        }

        col = Math.min(col, dl.text.length);

        cursorPara = dl.paraIndex;
        cursorOffset = dl.startOffset + col;
      }
    } else if (displayLineIndex < 0) {
      // Clicked above content - go to start of first line on this page
      const firstLineOnPage = pageLines[0];
      if (firstLineOnPage) {
        cursorPara = firstLineOnPage.paraIndex;
        cursorOffset = firstLineOnPage.startOffset;
      }
    } else {
      // Clicked below content - go to end of last line
      const lastLine = displayLines.length - 1;
      if (lastLine >= 0) {
        const dl = displayLines[lastLine];
        cursorPara = dl.paraIndex;
        cursorOffset = dl.endOffset;
      }
    }

    // Clear any existing selection when clicking
    selectionStart = null;
    selectionEnd = null;

    // Track if we're starting a drag selection
    let isDragging = false;
    const startPara = cursorPara;
    const startOffset = cursorOffset;

    const handleMouseMove = (e: MouseEvent) => {
      isDragging = true;
      if (!selectionStart) {
        selectionStart = { para: startPara, offset: startOffset };
      }
      for (let i = 0; i < canvases.length; i++) {
        const cvs = canvases[i];
        if (!cvs) continue;

        const r = cvs.getBoundingClientRect();
        if (e.clientY >= r.top && e.clientY <= r.bottom) {
          const mx = e.clientX - r.left;
          const my = e.clientY - r.top;

          let dragColumn = 0;
          if (columnCount > 1) {
            const xInContent = mx - marginLeft;
            if (xInContent >= columnWidth + columnGap) {
              dragColumn = 1;
            }
          }

          const dragStartLine = i * linesPerPage;
          const dragEndLine = Math.min(dragStartLine + linesPerPage, displayLines.length);
          const dragScaledParagraphSpacing = (paragraphSpacingValue * $zoomLevel) / 100;
          const yInContent = my - marginTop;

          let dlIdx = -1;
          let dragCumulativeSpacing = 0;

          for (let li = dragStartLine; li < dragEndLine; li++) {
            const lineIndexOnPage = li - dragStartLine;
            const colIndex = columnCount > 1 ? Math.floor(lineIndexOnPage / linesPerColumn) : 0;

            if (colIndex !== dragColumn) {
              const dl = displayLines[li];
              if (dl && dl.isLastLineOfParagraph) {
                dragCumulativeSpacing += dragScaledParagraphSpacing;
              }
              continue;
            }

            const lineInColumn = columnCount > 1 ? lineIndexOnPage % linesPerColumn : lineIndexOnPage;
            const lineY = lineInColumn * scaledLineHeight + dragCumulativeSpacing;
            const lineBottom = lineY + scaledLineHeight;

            if (yInContent >= lineY && yInContent < lineBottom) {
              dlIdx = li;
              break;
            }

            const dl = displayLines[li];
            if (dl && dl.isLastLineOfParagraph) {
              dragCumulativeSpacing += dragScaledParagraphSpacing;
            }
          }

          if (dlIdx === -1 && yInContent >= 0) {
            dlIdx = dragEndLine - 1;
          }

          const dragColumnOffsetX = dragColumn * (columnWidth + columnGap);

          if (dlIdx >= 0 && dlIdx < displayLines.length) {
            const dl = displayLines[dlIdx];
            const meta = dl.meta;

            const listIndent = meta.listType !== 'none' ? scaledFontSize * 1.5 : 0;
            const floatOffset = dl.floatReduction && dl.floatReduction.side === 'left' ? dl.floatReduction.width + 10 : 0;
            const textStartX = marginLeft + dragColumnOffsetX + listIndent + floatOffset;

            const baseFontSize = meta.fontSize ? (meta.fontSize * $zoomLevel) / 100 : scaledFontSize;
            let blockFontSize = baseFontSize;
            switch (meta.blockType) {
              case 'h1': blockFontSize = baseFontSize * 2; break;
              case 'h2': blockFontSize = baseFontSize * 1.5; break;
              case 'h3': blockFontSize = baseFontSize * 1.17; break;
            }

            const floatWidthReduction = dl.floatReduction ? dl.floatReduction.width + 10 : 0;
            const availableWidth = columnWidth - listIndent - floatWidthReduction;

            const measureCtx = measureCanvas?.getContext('2d');
            if (measureCtx) {
              const blockFontWeight = ['h1', 'h2', 'h3', 'h4'].includes(meta.blockType) ? 'bold ' : (isBold ? 'bold ' : '');
              const blockFontStyle = meta.blockType === 'blockquote' ? 'italic ' : (isItalic ? 'italic ' : '');
              measureCtx.font = `${blockFontStyle}${blockFontWeight}${blockFontSize}px ${fontFamily}`;
            }

            const textWidth = measureCtx ? measureCtx.measureText(dl.text).width : dl.text.length * blockFontSize * 0.5;

            let lineStartX = textStartX;
            if (meta.align === 'center') {
              lineStartX = textStartX + (availableWidth - textWidth) / 2;
            } else if (meta.align === 'right') {
              lineStartX = textStartX + availableWidth - textWidth;
            }

            let col = 0;
            let currentX = lineStartX;

            for (let j = 0; j <= dl.text.length; j++) {
              const cw = j < dl.text.length && measureCtx
                ? measureCtx.measureText(dl.text[j]).width
                : 0;
              if (currentX + cw / 2 >= mx) {
                col = j;
                break;
              }
              currentX += cw;
              col = j + 1;
            }
            col = Math.min(col, dl.text.length);

            cursorPara = dl.paraIndex;
            cursorOffset = dl.startOffset + col;
            selectionEnd = { para: cursorPara, offset: cursorOffset };
          }
          break;
        }
      }
    };

    const handleMouseUp = () => {
      window.removeEventListener('mousemove', handleMouseMove);
      window.removeEventListener('mouseup', handleMouseUp);

      // Clear selection if start equals end or no drag occurred
      if (!isDragging || (selectionStart && selectionEnd &&
          selectionStart.para === selectionEnd.para &&
          selectionStart.offset === selectionEnd.offset)) {
        selectionStart = null;
        selectionEnd = null;
      }

      // Always focus the hidden textarea after click
      hiddenTextarea?.focus();
      // Force re-render to show cursor
      renderAllPages();
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    // Update current page
    currentPage.set(pageIndex + 1);

    hiddenTextarea?.focus();
    renderAllPages();
  }

  /**
   * Handles formatting commands from the toolbar.
   *
   * Applies formatting to the current selection or cursor position.
   * Supports multiple command types:
   * - Text styling: bold, italic, underline, strikeThrough
   * - Alignment: justifyLeft, justifyCenter, justifyRight, justifyFull
   * - Lists: insertUnorderedList, insertOrderedList
   * - Block types: heading (h1-h4), blockquote
   * - Indentation: indent, outdent
   * - Font: fontSize, foreColor
   *
   * @param command - The formatting command to execute
   * @param value - Optional value for commands that require it (e.g., fontSize)
   */
  function handleFormat(command: string, value?: string) {
    let startPara = cursorPara;
    let endPara = cursorPara;
    if (selectionStart && selectionEnd) {
      startPara = Math.min(selectionStart.para, selectionEnd.para);
      endPara = Math.max(selectionStart.para, selectionEnd.para);
    }

    // Save history for commands that modify the document
    const documentModifyingCommands = [
      'justifyLeft', 'justifyCenter', 'justifyRight', 'justifyFull',
      'insertUnorderedList', 'insertOrderedList',
      'formatBlock', 'fontSize', 'foreColor'
    ];
    if (documentModifyingCommands.includes(command)) {
      saveToHistory();
    }

    switch (command) {
      // Text styling (these don't modify document state, they affect future input)
      case 'bold':
        isBold = !isBold;
        break;
      case 'italic':
        isItalic = !isItalic;
        break;
      case 'underline':
        isUnderline = !isUnderline;
        break;
      case 'strikeThrough':
        isStrikethrough = !isStrikethrough;
        break;

      // Text alignment
      case 'justifyLeft':
        for (let i = startPara; i <= endPara; i++) {
          paragraphMeta[i] = { ...paragraphMeta[i], align: 'left' };
        }
        break;
      case 'justifyCenter':
        for (let i = startPara; i <= endPara; i++) {
          paragraphMeta[i] = { ...paragraphMeta[i], align: 'center' };
        }
        break;
      case 'justifyRight':
        for (let i = startPara; i <= endPara; i++) {
          paragraphMeta[i] = { ...paragraphMeta[i], align: 'right' };
        }
        break;
      case 'justifyFull':
        for (let i = startPara; i <= endPara; i++) {
          paragraphMeta[i] = { ...paragraphMeta[i], align: 'justify' };
        }
        break;

      // Lists
      case 'insertUnorderedList':
        for (let i = startPara; i <= endPara; i++) {
          const currentType = paragraphMeta[i].listType;
          paragraphMeta[i] = {
            ...paragraphMeta[i],
            listType: currentType === 'bullet' ? 'none' : 'bullet'
          };
        }
        break;
      case 'insertOrderedList':
        for (let i = startPara; i <= endPara; i++) {
          const currentType = paragraphMeta[i].listType;
          paragraphMeta[i] = {
            ...paragraphMeta[i],
            listType: currentType === 'numbered' ? 'none' : 'numbered'
          };
        }
        break;

      // Block formatting
      case 'formatBlock':
        if (value) {
          const blockType = value as BlockType;
          for (let i = startPara; i <= endPara; i++) {
            paragraphMeta[i] = { ...paragraphMeta[i], blockType };
          }
        }
        break;

      // Font size for selected paragraphs
      case 'fontSize':
        if (value) {
          const newFontSize = parseInt(value);
          for (let i = startPara; i <= endPara; i++) {
            paragraphMeta[i] = { ...paragraphMeta[i], fontSize: newFontSize };
          }
        }
        break;

      // Text color for selected paragraphs
      case 'foreColor':
        if (value) {
          for (let i = startPara; i <= endPara; i++) {
            paragraphMeta[i] = { ...paragraphMeta[i], textColor: value };
          }
        }
        break;

      // Undo/Redo
      case 'undo':
        performUndo();
        return;
      case 'redo':
        performRedo();
        return;

      // Image insertion
      case 'insertImage':
        showImagePopup = true;
        return; // Don't refocus textarea
    }

    // Trigger reactivity
    paragraphMeta = [...paragraphMeta];
    recomputeDisplayLines();
    renderAllPages();
    hiddenTextarea?.focus();
  }

  // Image handling functions
  function generateImageId(): string {
    return 'img-' + Date.now() + '-' + Math.random().toString(36).substring(2, 11);
  }

  function loadImage(src: string): Promise<HTMLImageElement> {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.crossOrigin = 'anonymous';
      img.onload = () => resolve(img);
      img.onerror = reject;
      img.src = src;
    });
  }

  async function insertImage(src: string, wrapStyle: ImageWrapStyle = 'inline') {
    try {
      saveToHistory(); // Save before inserting image
      const img = await loadImage(src);
      const id = generateImageId();

      // Calculate appropriate size (max width = content width)
      const maxWidth = (contentDims.width * 0.8 * $zoomLevel) / 100;
      let width = img.naturalWidth;
      let height = img.naturalHeight;

      if (width > maxWidth) {
        const ratio = maxWidth / width;
        width = maxWidth;
        height = height * ratio;
      }

      const docImage: DocumentImage = {
        id,
        src,
        width,
        height,
        naturalWidth: img.naturalWidth,
        naturalHeight: img.naturalHeight,
        wrapStyle,
        positionMode: 'move-with-text',
        horizontalAlign: 'left',
        cropTop: 0,
        cropRight: 0,
        cropBottom: 0,
        cropLeft: 0,
      };

      images = [...images, docImage];
      loadedImages.set(id, img);

      // Insert image as a new paragraph with special marker
      // Split current paragraph at cursor
      const before = paragraphs[cursorPara].substring(0, cursorOffset);
      const after = paragraphs[cursorPara].substring(cursorOffset);

      // Current paragraph becomes text before cursor
      paragraphs[cursorPara] = before;

      // Insert image paragraph (with marker + id) and text after cursor
      const imagePara = IMAGE_MARKER + id;
      paragraphs = [
        ...paragraphs.slice(0, cursorPara + 1),
        imagePara,
        after,
        ...paragraphs.slice(cursorPara + 1)
      ];

      // Add metadata for new paragraphs
      const currentMeta = paragraphMeta[cursorPara] || createDefaultMeta();
      paragraphMeta = [
        ...paragraphMeta.slice(0, cursorPara + 1),
        { ...createDefaultMeta(), align: 'center' }, // Image meta
        { ...currentMeta }, // After text meta
        ...paragraphMeta.slice(cursorPara + 1)
      ];

      // Move cursor to after the image
      cursorPara += 2;
      cursorOffset = 0;

      recomputeDisplayLines();
      renderAllPages();
    } catch (error) {
      console.error('Failed to load image:', error);
      alert('Failed to load image. Please check the URL.');
    }
  }

  function handleImageUrlSubmit() {
    if (imageUrl.trim()) {
      insertImage(imageUrl.trim());
      imageUrl = '';
      showImagePopup = false;
      hiddenTextarea?.focus();
    }
  }

  function handleFileDrop(event: DragEvent) {
    event.preventDefault();
    dragOver = false;

    const files = event.dataTransfer?.files;
    if (files && files.length > 0) {
      handleImageFile(files[0]);
    }
  }

  function handleFileSelect(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files.length > 0) {
      handleImageFile(input.files[0]);
    }
  }

  function handleImageFile(file: File) {
    if (!file.type.startsWith('image/')) {
      alert('Please select an image file');
      return;
    }

    const reader = new FileReader();
    reader.onload = (e) => {
      const dataUrl = e.target?.result as string;
      insertImage(dataUrl);
      showImagePopup = false;
      hiddenTextarea?.focus();
    };
    reader.readAsDataURL(file);
  }

  function handlePaste(event: ClipboardEvent) {
    const items = event.clipboardData?.items;
    if (!items) return;

    for (const item of items) {
      if (item.type.startsWith('image/')) {
        event.preventDefault();
        const file = item.getAsFile();
        if (file) {
          handleImageFile(file);
        }
        return;
      }
    }
  }

  function closeImagePopup() {
    showImagePopup = false;
    imageUrl = '';
    dragOver = false;
    hiddenTextarea?.focus();
  }

  function deleteSelectedImage() {
    if (!selectedImageId) return;

    saveToHistory(); // Save before deleting image

    // Find the paragraph containing this image
    const imageParaIndex = paragraphs.findIndex(p => p === IMAGE_MARKER + selectedImageId);
    if (imageParaIndex === -1) return;

    // Remove the image from the images array
    images = images.filter(img => img.id !== selectedImageId);
    loadedImages.delete(selectedImageId);

    // Remove the image paragraph
    paragraphs = [...paragraphs.slice(0, imageParaIndex), ...paragraphs.slice(imageParaIndex + 1)];
    paragraphMeta = [...paragraphMeta.slice(0, imageParaIndex), ...paragraphMeta.slice(imageParaIndex + 1)];

    // Ensure we have at least one paragraph
    if (paragraphs.length === 0) {
      paragraphs = [''];
      paragraphMeta = [createDefaultMeta()];
    }

    // Adjust cursor position if needed
    if (cursorPara >= paragraphs.length) {
      cursorPara = paragraphs.length - 1;
      cursorOffset = paragraphs[cursorPara].length;
    } else if (cursorPara > imageParaIndex) {
      cursorPara--;
    }

    selectedImageId = null;
    showImageOptionsPopup = false;
    recomputeDisplayLines();
    renderAllPages();
  }

  function changeImageWrapStyle(wrapStyle: ImageWrapStyle) {
    if (!selectedImageId) return;

    const imageIndex = images.findIndex(img => img.id === selectedImageId);
    if (imageIndex === -1) return;

    const updatedImage = { ...images[imageIndex], wrapStyle };

    // Clear absolute position when switching to move-with-text mode
    // or when switching to inline mode (inline images follow paragraph alignment)
    if (updatedImage.positionMode === 'move-with-text' || wrapStyle === 'inline') {
      updatedImage.x = undefined;
      updatedImage.y = undefined;
      updatedImage.pageIndex = undefined;
    }

    images[imageIndex] = updatedImage;
    images = [...images];

    recomputeDisplayLines();
    renderAllPages();
  }

  function changeImagePositionMode(positionMode: ImagePositionMode) {
    if (!selectedImageId) return;

    const imageIndex = images.findIndex(img => img.id === selectedImageId);
    if (imageIndex === -1) return;

    const updatedImage = { ...images[imageIndex], positionMode };

    // Clear absolute position when switching to move-with-text
    if (positionMode === 'move-with-text') {
      updatedImage.x = undefined;
      updatedImage.y = undefined;
      updatedImage.pageIndex = undefined;
    }

    images[imageIndex] = updatedImage;
    images = [...images];

    recomputeDisplayLines();
    renderAllPages();
  }

  function changeImageHorizontalAlign(align: 'left' | 'center' | 'right') {
    if (!selectedImageId) return;

    const imageIndex = images.findIndex(img => img.id === selectedImageId);
    if (imageIndex === -1) return;

    images[imageIndex] = { ...images[imageIndex], horizontalAlign: align };
    images = [...images];

    recomputeDisplayLines();
    renderAllPages();
  }

  function showImageOptions(x: number, y: number) {
    imageOptionsPosition = { x, y };
    showImageOptionsPopup = true;
  }

  function closeImageOptionsPopup() {
    showImageOptionsPopup = false;
  }

  // Get image bounds for a selected image (accounts for crop and absolute positioning)
  function getSelectedImageBounds(): { x: number; y: number; width: number; height: number; pageIndex: number } | null {
    if (!selectedImageId) return null;

    const docImage = images.find(img => img.id === selectedImageId);
    if (!docImage) return null;

    const img = loadedImages.get(docImage.id);
    if (!img) return null;

    const scaledWidth = (docImage.width * $zoomLevel) / 100;

    // Account for crop in dimensions
    const cropTop = docImage.cropTop || 0;
    const cropRight = docImage.cropRight || 0;
    const cropBottom = docImage.cropBottom || 0;
    const cropLeft = docImage.cropLeft || 0;

    const srcW = ((100 - cropLeft - cropRight) / 100) * img.naturalWidth;
    const srcH = ((100 - cropTop - cropBottom) / 100) * img.naturalHeight;
    const cropAspect = srcW / srcH;
    const destW = scaledWidth;
    const destH = scaledWidth / cropAspect;

    // Check if image has absolute position
    if (docImage.x !== undefined && docImage.y !== undefined) {
      const pageHeight = contentDims.height;
      const imagePageIndex = Math.floor(docImage.y / pageHeight);
      const yOnPage = docImage.y - (imagePageIndex * pageHeight);

      const imageX = marginLeft + (docImage.x * $zoomLevel) / 100;
      const imageY = marginTop + (yOnPage * $zoomLevel) / 100;

      return { x: imageX, y: imageY, width: destW, height: destH, pageIndex: imagePageIndex };
    }

    // Find the display line for this image
    const displayLine = displayLines.find(dl => dl.imageId === selectedImageId && dl.isImage);
    if (!displayLine || displayLine.pageIndex === undefined) return null;

    const pageIndex = displayLine.pageIndex;

    // Check if it's a float
    const float = activeFloats.find(f => f.id === selectedImageId);
    let imageX: number;
    if (float) {
      imageX = float.side === 'left' ? marginLeft : marginLeft + scaledContentWidth - destW;
    } else {
      imageX = marginLeft + (scaledContentWidth - destW) / 2;
    }

    // Use pre-computed Y position
    const imageY = marginTop + (displayLine.yPosition ?? 0);

    return { x: imageX, y: imageY, width: destW, height: destH, pageIndex };
  }

  // Check if mouse is on a resize handle
  function getResizeHandleAtPoint(x: number, y: number, bounds: { x: number; y: number; width: number; height: number }): ResizeHandle {
    const handleSize = 10;
    const { x: bx, y: by, width: bw, height: bh } = bounds;

    // Check corners
    if (x >= bx - handleSize && x <= bx + handleSize && y >= by - handleSize && y <= by + handleSize) return 'nw';
    if (x >= bx + bw - handleSize && x <= bx + bw + handleSize && y >= by - handleSize && y <= by + handleSize) return 'ne';
    if (x >= bx - handleSize && x <= bx + handleSize && y >= by + bh - handleSize && y <= by + bh + handleSize) return 'sw';
    if (x >= bx + bw - handleSize && x <= bx + bw + handleSize && y >= by + bh - handleSize && y <= by + bh + handleSize) return 'se';

    // Check edges (middle of each side)
    const midX = bx + bw / 2;
    const midY = by + bh / 2;
    if (x >= midX - handleSize && x <= midX + handleSize && y >= by - handleSize && y <= by + handleSize) return 'n';
    if (x >= midX - handleSize && x <= midX + handleSize && y >= by + bh - handleSize && y <= by + bh + handleSize) return 's';
    if (x >= bx - handleSize && x <= bx + handleSize && y >= midY - handleSize && y <= midY + handleSize) return 'w';
    if (x >= bx + bw - handleSize && x <= bx + bw + handleSize && y >= midY - handleSize && y <= midY + handleSize) return 'e';

    return null;
  }

  // Start resizing
  function startResize(handle: ResizeHandle, clientX: number, clientY: number) {
    if (!selectedImageId || !handle) return;

    const docImage = images.find(img => img.id === selectedImageId);
    if (!docImage) return;

    isResizing = true;
    resizeHandle = handle;
    resizeStartX = clientX;
    resizeStartY = clientY;
    resizeStartWidth = docImage.width;
    resizeStartHeight = docImage.height;

    showImageOptionsPopup = false;
  }

  // Handle resize move
  function handleResizeMove(clientX: number, clientY: number) {
    if (!isResizing || !selectedImageId || !resizeHandle) return;

    const imageIndex = images.findIndex(img => img.id === selectedImageId);
    if (imageIndex === -1) return;

    const docImage = images[imageIndex];
    const aspectRatio = docImage.naturalWidth / docImage.naturalHeight;

    const deltaX = (clientX - resizeStartX) * (100 / $zoomLevel);
    const deltaY = (clientY - resizeStartY) * (100 / $zoomLevel);

    let newWidth = resizeStartWidth;
    let newHeight = resizeStartHeight;

    // Calculate new dimensions based on handle
    switch (resizeHandle) {
      case 'se':
        newWidth = Math.max(50, resizeStartWidth + deltaX);
        newHeight = newWidth / aspectRatio;
        break;
      case 'sw':
        newWidth = Math.max(50, resizeStartWidth - deltaX);
        newHeight = newWidth / aspectRatio;
        break;
      case 'ne':
        newWidth = Math.max(50, resizeStartWidth + deltaX);
        newHeight = newWidth / aspectRatio;
        break;
      case 'nw':
        newWidth = Math.max(50, resizeStartWidth - deltaX);
        newHeight = newWidth / aspectRatio;
        break;
      case 'e':
        newWidth = Math.max(50, resizeStartWidth + deltaX);
        newHeight = newWidth / aspectRatio;
        break;
      case 'w':
        newWidth = Math.max(50, resizeStartWidth - deltaX);
        newHeight = newWidth / aspectRatio;
        break;
      case 'n':
      case 's':
        newHeight = Math.max(50, resizeStartHeight + (resizeHandle === 's' ? deltaY : -deltaY));
        newWidth = newHeight * aspectRatio;
        break;
    }

    // Constrain to max content width
    const maxWidth = contentDims.width * 0.95;
    if (newWidth > maxWidth) {
      newWidth = maxWidth;
      newHeight = newWidth / aspectRatio;
    }

    images[imageIndex] = { ...docImage, width: newWidth, height: newHeight };
    images = [...images];
    recomputeDisplayLines();
    renderAllPages();
  }

  // End resize
  function endResize() {
    isResizing = false;
    resizeHandle = null;
  }

  // Start cropping mode
  function startCropMode() {
    if (!selectedImageId) return;

    // Save original crop values for cancel
    const docImage = images.find(img => img.id === selectedImageId);
    if (docImage) {
      cropOriginalValues = {
        top: docImage.cropTop || 0,
        right: docImage.cropRight || 0,
        bottom: docImage.cropBottom || 0,
        left: docImage.cropLeft || 0,
      };
    }

    isCropping = true;
    showImageOptionsPopup = false;
    renderAllPages();
  }

  // End cropping mode
  function endCropMode() {
    isCropping = false;
    cropHandle = null;
    renderAllPages();
  }

  // Cancel crop and restore original values
  function cancelCrop() {
    if (!selectedImageId) return;

    const imageIndex = images.findIndex(img => img.id === selectedImageId);
    if (imageIndex !== -1) {
      images[imageIndex] = {
        ...images[imageIndex],
        cropTop: cropOriginalValues.top,
        cropRight: cropOriginalValues.right,
        cropBottom: cropOriginalValues.bottom,
        cropLeft: cropOriginalValues.left,
      };
      images = [...images];
    }

    endCropMode();
  }

  // Start crop drag
  function startCropDrag(handle: ResizeHandle, clientX: number, clientY: number) {
    if (!selectedImageId || !handle) return;

    const docImage = images.find(img => img.id === selectedImageId);
    if (!docImage) return;

    cropHandle = handle;
    cropStartX = clientX;
    cropStartY = clientY;
    cropStartValues = {
      top: docImage.cropTop || 0,
      right: docImage.cropRight || 0,
      bottom: docImage.cropBottom || 0,
      left: docImage.cropLeft || 0,
    };
  }

  // Handle crop move
  function handleCropMove(clientX: number, clientY: number) {
    if (!isCropping || !selectedImageId || !cropHandle) return;

    const imageIndex = images.findIndex(img => img.id === selectedImageId);
    if (imageIndex === -1) return;

    const docImage = images[imageIndex];
    const bounds = getSelectedImageBounds();
    if (!bounds) return;

    // Calculate delta as percentage of image dimensions
    const deltaX = ((clientX - cropStartX) / bounds.width) * 100;
    const deltaY = ((clientY - cropStartY) / bounds.height) * 100;

    let newCrop = { ...cropStartValues };

    // Update crop based on handle
    switch (cropHandle) {
      case 'n':
        newCrop.top = Math.max(0, Math.min(100 - newCrop.bottom - 10, cropStartValues.top + deltaY));
        break;
      case 's':
        newCrop.bottom = Math.max(0, Math.min(100 - newCrop.top - 10, cropStartValues.bottom - deltaY));
        break;
      case 'w':
        newCrop.left = Math.max(0, Math.min(100 - newCrop.right - 10, cropStartValues.left + deltaX));
        break;
      case 'e':
        newCrop.right = Math.max(0, Math.min(100 - newCrop.left - 10, cropStartValues.right - deltaX));
        break;
      case 'nw':
        newCrop.top = Math.max(0, Math.min(100 - newCrop.bottom - 10, cropStartValues.top + deltaY));
        newCrop.left = Math.max(0, Math.min(100 - newCrop.right - 10, cropStartValues.left + deltaX));
        break;
      case 'ne':
        newCrop.top = Math.max(0, Math.min(100 - newCrop.bottom - 10, cropStartValues.top + deltaY));
        newCrop.right = Math.max(0, Math.min(100 - newCrop.left - 10, cropStartValues.right - deltaX));
        break;
      case 'sw':
        newCrop.bottom = Math.max(0, Math.min(100 - newCrop.top - 10, cropStartValues.bottom - deltaY));
        newCrop.left = Math.max(0, Math.min(100 - newCrop.right - 10, cropStartValues.left + deltaX));
        break;
      case 'se':
        newCrop.bottom = Math.max(0, Math.min(100 - newCrop.top - 10, cropStartValues.bottom - deltaY));
        newCrop.right = Math.max(0, Math.min(100 - newCrop.left - 10, cropStartValues.right - deltaX));
        break;
    }

    images[imageIndex] = {
      ...docImage,
      cropTop: newCrop.top,
      cropRight: newCrop.right,
      cropBottom: newCrop.bottom,
      cropLeft: newCrop.left,
    };
    images = [...images];
    renderAllPages();
  }

  // End crop drag
  function endCropDrag() {
    cropHandle = null;
  }

  // Reset crop to full image
  function resetCrop() {
    if (!selectedImageId) return;

    const imageIndex = images.findIndex(img => img.id === selectedImageId);
    if (imageIndex === -1) return;

    images[imageIndex] = {
      ...images[imageIndex],
      cropTop: 0,
      cropRight: 0,
      cropBottom: 0,
      cropLeft: 0,
    };
    images = [...images];
    renderAllPages();
  }

  // Start dragging an image - inline images cannot be dragged (they follow text alignment)
  function startDrag(clientX: number, clientY: number, _pageIndex: number) {
    if (!selectedImageId) return;

    const docImage = images.find(img => img.id === selectedImageId);
    if (!docImage) return;

    // Inline images cannot be freely positioned - they follow paragraph alignment
    if (docImage.wrapStyle === 'inline') return;

    isDragging = true;
    dragStartX = clientX;
    dragStartY = clientY;

    // Initialize position if not set
    if (docImage.x === undefined || docImage.y === undefined) {
      const bounds = getSelectedImageBounds();
      if (bounds) {
        // Convert to unscaled coordinates relative to content area
        dragStartImageX = (bounds.x - marginLeft) * (100 / $zoomLevel);
        dragStartImageY = bounds.pageIndex * (contentDims.height) + ((bounds.y - marginTop) * (100 / $zoomLevel));
      } else {
        dragStartImageX = 0;
        dragStartImageY = 0;
      }
    } else {
      dragStartImageX = docImage.x;
      dragStartImageY = docImage.y;
    }

    showImageOptionsPopup = false;
  }

  // Handle drag move
  function handleDragMove(clientX: number, clientY: number) {
    if (!isDragging || !selectedImageId) return;

    const imageIndex = images.findIndex(img => img.id === selectedImageId);
    if (imageIndex === -1) return;

    const docImage = images[imageIndex];

    // Calculate delta in unscaled coordinates
    const deltaX = (clientX - dragStartX) * (100 / $zoomLevel);
    const deltaY = (clientY - dragStartY) * (100 / $zoomLevel);

    let newX = dragStartImageX + deltaX;
    let newY = dragStartImageY + deltaY;

    // Constrain to content area
    const maxX = contentDims.width - docImage.width;
    newX = Math.max(0, Math.min(maxX, newX));

    // Calculate which page the image is on based on Y position
    const pageHeight = contentDims.height;
    const totalY = Math.max(0, newY);
    const newPageIndex = Math.floor(totalY / pageHeight);

    // Update position - works for all images
    images[imageIndex] = {
      ...docImage,
      x: newX,
      y: totalY,
      pageIndex: newPageIndex,
    };
    images = [...images];
    recomputeDisplayLines();
    renderAllPages();
  }

  // End dragging
  function endDrag() {
    isDragging = false;
  }

  // Generate page array for rendering
  let pageArray = $derived(Array.from({ length: numPages }, (_, i) => i));
</script>

<div class="editor-wrapper">
  <Toolbar onFormat={handleFormat} {canUndo} {canRedo} />

  <!-- Hidden canvas for text measurement -->
  <canvas bind:this={measureCanvas} class="measure-canvas"></canvas>

  <!-- Hidden textarea for keyboard input -->
  <!-- svelte-ignore a11y_autocomplete_valid -->
  <textarea
    bind:this={hiddenTextarea}
    class="hidden-input"
    onkeydown={handleKeyDown}
    onpaste={handlePaste}
    autocomplete="off"
    spellcheck={false}
  ></textarea>

  <div class="editor-container" bind:this={editorContainer}>
    <div class="canvas-container" bind:this={canvasContainer}>
      {#each pageArray as pageIndex (pageIndex)}
        <canvas
          bind:this={canvases[pageIndex]}
          class="page-canvas"
          class:active={$currentPage === pageIndex + 1}
          onmousedown={(e) => handleCanvasMouseDown(e, pageIndex)}
        ></canvas>
      {/each}
    </div>
  </div>

  <div class="page-indicator">
    Page {$currentPage} of {numPages}
  </div>

  <!-- Image popup dialog -->
  {#if showImagePopup}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="popup-overlay" onclick={closeImagePopup}>
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="popup-dialog" onclick={(e) => e.stopPropagation()}>
        <div class="popup-header">
          <h3>Insert Image</h3>
          <button class="popup-close" onclick={closeImagePopup}>&times;</button>
        </div>

        <div class="popup-content">
          <!-- URL input -->
          <div class="input-section">
            <label for="image-url">Image URL</label>
            <div class="url-input-row">
              <input
                id="image-url"
                type="text"
                placeholder="https://example.com/image.jpg"
                bind:value={imageUrl}
                onkeydown={(e) => e.key === 'Enter' && handleImageUrlSubmit()}
              />
              <button onclick={handleImageUrlSubmit}>Insert</button>
            </div>
          </div>

          <div class="divider">
            <span>or</span>
          </div>

          <!-- Drag and drop zone -->
          <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
          <div
            class="drop-zone"
            class:drag-over={dragOver}
            ondragover={(e) => { e.preventDefault(); dragOver = true; }}
            ondragleave={() => dragOver = false}
            ondrop={handleFileDrop}
            onclick={() => document.getElementById('file-input')?.click()}
          >
            <svg width="48" height="48" viewBox="0 0 24 24" fill="#9aa0a6">
              <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
            </svg>
            <p>Drag and drop an image here</p>
            <p class="small">or click to select from your computer</p>
          </div>
          <input
            id="file-input"
            type="file"
            accept="image/*"
            class="file-input"
            onchange={handleFileSelect}
          />

          <p class="tip">Tip: You can also paste an image directly in the editor (Ctrl+V)</p>
        </div>
      </div>
    </div>
  {/if}

  <!-- Image options popup -->
  {#if showImageOptionsPopup && selectedImageId && !isCropping}
    {@const selectedImage = images.find(img => img.id === selectedImageId)}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="image-options-overlay" onclick={closeImageOptionsPopup}>
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="image-options-popup"
        style="left: {imageOptionsPosition.x}px; top: {imageOptionsPosition.y}px;"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="image-options-header">
          <span>Layout Options</span>
          <button class="popup-close" onclick={closeImageOptionsPopup}>&times;</button>
        </div>

        <!-- In Line with Text -->
        <div class="image-options-section">
          <div class="section-label">In Line with Text</div>
          <div class="image-options-buttons">
            <button
              class="layout-btn"
              class:active={selectedImage?.wrapStyle === 'inline'}
              onclick={() => changeImageWrapStyle('inline')}
              title="In line with text"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <line x1="2" y1="8" x2="8" y2="8" stroke="currentColor" stroke-width="1.5"/>
                <rect x="9" y="5" width="6" height="6" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
                <line x1="16" y1="8" x2="22" y2="8" stroke="currentColor" stroke-width="1.5"/>
                <line x1="2" y1="14" x2="22" y2="14" stroke="currentColor" stroke-width="1.5"/>
                <line x1="2" y1="18" x2="22" y2="18" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>In Line</span>
            </button>
          </div>
        </div>

        <!-- With Text Wrapping -->
        <div class="image-options-section">
          <div class="section-label">With Text Wrapping</div>
          <div class="image-options-buttons wrap-buttons">
            <button
              class="layout-btn"
              class:active={selectedImage?.wrapStyle === 'square'}
              onclick={() => changeImageWrapStyle('square')}
              title="Square - text wraps in a square around the image"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <rect x="2" y="4" width="8" height="8" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
                <line x1="12" y1="5" x2="22" y2="5" stroke="currentColor" stroke-width="1.5"/>
                <line x1="12" y1="8" x2="22" y2="8" stroke="currentColor" stroke-width="1.5"/>
                <line x1="12" y1="11" x2="22" y2="11" stroke="currentColor" stroke-width="1.5"/>
                <line x1="2" y1="15" x2="22" y2="15" stroke="currentColor" stroke-width="1.5"/>
                <line x1="2" y1="19" x2="22" y2="19" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>Square</span>
            </button>
            <button
              class="layout-btn"
              class:active={selectedImage?.wrapStyle === 'tight'}
              onclick={() => changeImageWrapStyle('tight')}
              title="Tight - text wraps tightly around the image"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <path d="M3 5 L9 5 L9 11 L3 11 Z" stroke="currentColor" stroke-width="1.5" fill="none"/>
                <line x1="11" y1="5" x2="21" y2="5" stroke="currentColor" stroke-width="1.5"/>
                <line x1="11" y1="8" x2="21" y2="8" stroke="currentColor" stroke-width="1.5"/>
                <line x1="11" y1="11" x2="21" y2="11" stroke="currentColor" stroke-width="1.5"/>
                <line x1="3" y1="14" x2="21" y2="14" stroke="currentColor" stroke-width="1.5"/>
                <line x1="3" y1="18" x2="21" y2="18" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>Tight</span>
            </button>
            <button
              class="layout-btn"
              class:active={selectedImage?.wrapStyle === 'through'}
              onclick={() => changeImageWrapStyle('through')}
              title="Through - text flows through the image"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <path d="M4 4 L10 4 L10 10 L4 10 Z" stroke="currentColor" stroke-width="1.5" fill="none" stroke-dasharray="2,1"/>
                <line x1="12" y1="5" x2="20" y2="5" stroke="currentColor" stroke-width="1.5"/>
                <line x1="12" y1="9" x2="20" y2="9" stroke="currentColor" stroke-width="1.5"/>
                <line x1="4" y1="14" x2="20" y2="14" stroke="currentColor" stroke-width="1.5"/>
                <line x1="4" y1="18" x2="20" y2="18" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>Through</span>
            </button>
            <button
              class="layout-btn"
              class:active={selectedImage?.wrapStyle === 'top-bottom'}
              onclick={() => changeImageWrapStyle('top-bottom')}
              title="Top and Bottom - text above and below only"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <line x1="2" y1="4" x2="22" y2="4" stroke="currentColor" stroke-width="1.5"/>
                <rect x="6" y="8" width="12" height="8" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
                <line x1="2" y1="20" x2="22" y2="20" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>Top/Bottom</span>
            </button>
            <button
              class="layout-btn"
              class:active={selectedImage?.wrapStyle === 'behind'}
              onclick={() => changeImageWrapStyle('behind')}
              title="Behind Text - image appears behind text"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <rect x="6" y="6" width="12" height="12" rx="1" stroke="currentColor" stroke-width="1.5" fill="none" opacity="0.5"/>
                <line x1="2" y1="8" x2="22" y2="8" stroke="currentColor" stroke-width="1.5"/>
                <line x1="2" y1="12" x2="22" y2="12" stroke="currentColor" stroke-width="1.5"/>
                <line x1="2" y1="16" x2="22" y2="16" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>Behind</span>
            </button>
            <button
              class="layout-btn"
              class:active={selectedImage?.wrapStyle === 'in-front'}
              onclick={() => changeImageWrapStyle('in-front')}
              title="In Front of Text - image appears in front of text"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <line x1="2" y1="8" x2="22" y2="8" stroke="currentColor" stroke-width="1.5" opacity="0.5"/>
                <line x1="2" y1="12" x2="22" y2="12" stroke="currentColor" stroke-width="1.5" opacity="0.5"/>
                <line x1="2" y1="16" x2="22" y2="16" stroke="currentColor" stroke-width="1.5" opacity="0.5"/>
                <rect x="6" y="6" width="12" height="12" rx="1" stroke="currentColor" stroke-width="1.5" fill="white"/>
              </svg>
              <span>In Front</span>
            </button>
          </div>
        </div>

        <!-- Horizontal Alignment (for non-inline modes) -->
        {#if selectedImage?.wrapStyle !== 'inline'}
          <div class="image-options-section">
            <div class="section-label">Horizontal Position</div>
            <div class="image-options-buttons">
              <button
                class="layout-btn small"
                class:active={selectedImage?.horizontalAlign === 'left'}
                onclick={() => changeImageHorizontalAlign('left')}
                title="Align left"
              >
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="2" y="6" width="10" height="12" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
                </svg>
              </button>
              <button
                class="layout-btn small"
                class:active={selectedImage?.horizontalAlign === 'center'}
                onclick={() => changeImageHorizontalAlign('center')}
                title="Center"
              >
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="7" y="6" width="10" height="12" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
                </svg>
              </button>
              <button
                class="layout-btn small"
                class:active={selectedImage?.horizontalAlign === 'right'}
                onclick={() => changeImageHorizontalAlign('right')}
                title="Align right"
              >
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="12" y="6" width="10" height="12" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
                </svg>
              </button>
            </div>
          </div>
        {/if}

        <!-- Position Mode -->
        <div class="image-options-section">
          <div class="section-label">Position</div>
          <div class="position-radio-group">
            <label class="radio-option">
              <input
                type="radio"
                name="positionMode"
                checked={selectedImage?.positionMode === 'move-with-text'}
                onchange={() => changeImagePositionMode('move-with-text')}
              />
              <span>Move with text</span>
            </label>
            <label class="radio-option">
              <input
                type="radio"
                name="positionMode"
                checked={selectedImage?.positionMode === 'fixed-position'}
                onchange={() => changeImagePositionMode('fixed-position')}
              />
              <span>Fix position on page</span>
            </label>
          </div>
        </div>

        <!-- Edit Section -->
        <div class="image-options-section">
          <div class="section-label">Edit</div>
          <div class="image-options-buttons">
            <button
              class="layout-btn"
              onclick={startCropMode}
              title="Crop image"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <path d="M17 15h2V7c0-1.1-.9-2-2-2H9v2h8v8zM7 17V1H5v4H1v2h4v10c0 1.1.9 2 2 2h10v4h2v-4h4v-2H7z"/>
              </svg>
              <span>Crop</span>
            </button>
            <button
              class="layout-btn"
              onclick={resetCrop}
              title="Reset crop"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <path d="M12 5V1L7 6l5 5V7c3.31 0 6 2.69 6 6s-2.69 6-6 6-6-2.69-6-6H4c0 4.42 3.58 8 8 8s8-3.58 8-8-3.58-8-8-8z"/>
              </svg>
              <span>Reset</span>
            </button>
          </div>
        </div>

        <button class="delete-image-btn" onclick={deleteSelectedImage}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
          </svg>
          Delete Image
        </button>
      </div>
    </div>
  {/if}

  <!-- Crop mode overlay -->
  {#if isCropping && selectedImageId}
    <div class="crop-mode-bar">
      <span>Crop Mode - Drag handles to adjust crop area</span>
      <div class="crop-mode-buttons">
        <button class="crop-btn done" onclick={endCropMode}>Done</button>
        <button class="crop-btn reset" onclick={resetCrop}>Reset</button>
        <button class="crop-btn cancel" onclick={cancelCrop}>Cancel</button>
      </div>
    </div>
  {/if}
</div>

<style>
  .editor-wrapper {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #f8f9fa;
  }

  .measure-canvas {
    position: absolute;
    left: -9999px;
    top: 0;
    visibility: hidden;
  }

  .hidden-input {
    position: absolute;
    left: -9999px;
    top: 0;
    width: 1px;
    height: 1px;
    opacity: 0;
  }

  .editor-container {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .canvas-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 40px;
  }

  .page-canvas {
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12), 0 1px 2px rgba(0, 0, 0, 0.24);
    cursor: text;
  }

  .page-canvas.active {
    box-shadow: 0 0 0 2px #1a73e8, 0 1px 3px rgba(0, 0, 0, 0.12), 0 1px 2px rgba(0, 0, 0, 0.24);
  }

  .page-indicator {
    position: fixed;
    bottom: 20px;
    right: 20px;
    background: white;
    padding: 8px 16px;
    border-radius: 4px;
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
    font-size: 12px;
    color: #5f6368;
    z-index: 100;
  }

  /* Image popup styles */
  .popup-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .popup-dialog {
    background: white;
    border-radius: 8px;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
    width: 480px;
    max-width: 90vw;
  }

  .popup-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid #e0e0e0;
  }

  .popup-header h3 {
    margin: 0;
    font-size: 18px;
    font-weight: 500;
    color: #202124;
  }

  .popup-close {
    background: none;
    border: none;
    font-size: 24px;
    color: #5f6368;
    cursor: pointer;
    padding: 0;
    line-height: 1;
  }

  .popup-close:hover {
    color: #202124;
  }

  .popup-content {
    padding: 20px;
  }

  .input-section {
    margin-bottom: 16px;
  }

  .input-section label {
    display: block;
    margin-bottom: 8px;
    font-size: 14px;
    color: #5f6368;
  }

  .url-input-row {
    display: flex;
    gap: 8px;
  }

  .url-input-row input {
    flex: 1;
    padding: 10px 12px;
    border: 1px solid #dadce0;
    border-radius: 4px;
    font-size: 14px;
  }

  .url-input-row input:focus {
    outline: none;
    border-color: #1a73e8;
  }

  .url-input-row button {
    padding: 10px 20px;
    background: #1a73e8;
    color: white;
    border: none;
    border-radius: 4px;
    font-size: 14px;
    cursor: pointer;
  }

  .url-input-row button:hover {
    background: #1557b0;
  }

  .divider {
    display: flex;
    align-items: center;
    margin: 20px 0;
  }

  .divider::before,
  .divider::after {
    content: '';
    flex: 1;
    height: 1px;
    background: #e0e0e0;
  }

  .divider span {
    padding: 0 16px;
    color: #5f6368;
    font-size: 14px;
  }

  .drop-zone {
    border: 2px dashed #dadce0;
    border-radius: 8px;
    padding: 40px 20px;
    text-align: center;
    cursor: pointer;
    transition: all 0.2s;
  }

  .drop-zone:hover {
    border-color: #1a73e8;
    background: #f8f9ff;
  }

  .drop-zone.drag-over {
    border-color: #1a73e8;
    background: #e8f0fe;
  }

  .drop-zone p {
    margin: 8px 0 0;
    color: #5f6368;
  }

  .drop-zone p.small {
    font-size: 12px;
    color: #9aa0a6;
  }

  .file-input {
    display: none;
  }

  .tip {
    margin-top: 16px;
    padding: 12px;
    background: #f8f9fa;
    border-radius: 4px;
    font-size: 13px;
    color: #5f6368;
    text-align: center;
  }

  /* Image options popup styles */
  .image-options-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 1000;
  }

  .image-options-popup {
    position: fixed;
    background: white;
    border-radius: 8px;
    box-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
    min-width: 200px;
    transform: translate(-50%, 10px);
    z-index: 1001;
  }

  .image-options-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 10px 12px;
    border-bottom: 1px solid #e0e0e0;
    font-size: 13px;
    font-weight: 500;
    color: #202124;
  }

  .image-options-buttons {
    display: flex;
    padding: 8px;
    gap: 4px;
  }

  .layout-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px 12px;
    border: 1px solid transparent;
    border-radius: 4px;
    background: none;
    cursor: pointer;
    color: #5f6368;
    font-size: 11px;
    transition: all 0.15s;
  }

  .layout-btn:hover {
    background: #f1f3f4;
  }

  .layout-btn.active {
    background: #e8f0fe;
    border-color: #1a73e8;
    color: #1a73e8;
  }

  .layout-btn svg {
    width: 28px;
    height: 28px;
  }

  .delete-image-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    width: calc(100% - 16px);
    margin: 0 8px 8px;
    padding: 8px 12px;
    border: none;
    border-radius: 4px;
    background: none;
    color: #d93025;
    font-size: 13px;
    cursor: pointer;
    transition: background 0.15s;
  }

  .delete-image-btn:hover {
    background: #fce8e6;
  }

  .image-options-section {
    padding: 8px;
    border-bottom: 1px solid #e0e0e0;
  }

  .image-options-section:last-of-type {
    border-bottom: none;
  }

  .section-label {
    font-size: 11px;
    color: #5f6368;
    text-transform: uppercase;
    margin-bottom: 6px;
    padding-left: 4px;
  }

  .wrap-buttons {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 4px;
  }

  .layout-btn.small {
    padding: 6px 10px;
  }

  .layout-btn.small svg {
    width: 20px;
    height: 20px;
  }

  .position-radio-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 4px 8px;
  }

  .radio-option {
    display: flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
    font-size: 13px;
    color: #202124;
  }

  .radio-option input[type="radio"] {
    margin: 0;
    cursor: pointer;
  }

  /* Crop mode bar */
  .crop-mode-bar {
    position: fixed;
    top: 60px;
    left: 50%;
    transform: translateX(-50%);
    background: #ff9800;
    color: white;
    padding: 10px 20px;
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.2);
    display: flex;
    align-items: center;
    gap: 20px;
    z-index: 1001;
    font-size: 14px;
  }

  .crop-mode-buttons {
    display: flex;
    gap: 8px;
  }

  .crop-btn {
    padding: 6px 14px;
    border: none;
    border-radius: 4px;
    font-size: 13px;
    cursor: pointer;
    font-weight: 500;
    transition: background 0.15s;
  }

  .crop-btn.done {
    background: white;
    color: #ff9800;
  }

  .crop-btn.done:hover {
    background: #fff3e0;
  }

  .crop-btn.reset {
    background: rgba(255, 255, 255, 0.2);
    color: white;
  }

  .crop-btn.reset:hover {
    background: rgba(255, 255, 255, 0.3);
  }

  .crop-btn.cancel {
    background: transparent;
    color: white;
    border: 1px solid rgba(255, 255, 255, 0.5);
  }

  .crop-btn.cancel:hover {
    background: rgba(255, 255, 255, 0.1);
  }
</style>
