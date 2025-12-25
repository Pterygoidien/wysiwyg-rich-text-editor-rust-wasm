<script lang="ts">
  import { onMount } from 'svelte';
  import Toolbar from './Toolbar.svelte';
  import { pageConfig, zoomLevel, currentPage, totalPages } from './stores';
  import { getContentDimensions, getPageDimensions, mmToPixels } from './types';

  let editorContainer: HTMLDivElement;
  let canvasContainer: HTMLDivElement;
  let hiddenTextarea: HTMLTextAreaElement;
  let measureCanvas: HTMLCanvasElement;

  // Document state - paragraphs are logical units (separated by Enter)
  let paragraphs: string[] = $state(['']);
  let cursorPara = $state(0);
  let cursorOffset = $state(0); // offset within the paragraph
  let selectionStart: { para: number; offset: number } | null = $state(null);
  let selectionEnd: { para: number; offset: number } | null = $state(null);

  // Text styling
  let fontSize = $state(16);
  let fontFamily = $state('Arial');
  let lineHeightMultiplier = $state(1.5);
  let isBold = $state(false);
  let isItalic = $state(false);

  // Calculate dimensions
  let pageDims = $derived(getPageDimensions($pageConfig));
  let contentDims = $derived(getContentDimensions($pageConfig));

  let scaledPageHeight = $derived((pageDims.height * $zoomLevel) / 100);
  let scaledPageWidth = $derived((pageDims.width * $zoomLevel) / 100);
  let scaledContentHeight = $derived((contentDims.height * $zoomLevel) / 100);
  let scaledContentWidth = $derived((contentDims.width * $zoomLevel) / 100);

  let marginTop = $derived((mmToPixels($pageConfig.margins.top) * $zoomLevel) / 100);
  let marginLeft = $derived((mmToPixels($pageConfig.margins.left) * $zoomLevel) / 100);

  // Calculate line metrics
  let scaledFontSize = $derived((fontSize * $zoomLevel) / 100);
  let scaledLineHeight = $derived(scaledFontSize * lineHeightMultiplier);
  let linesPerPage = $derived(Math.floor(scaledContentHeight / scaledLineHeight));

  // Wrapped lines for display - computed from paragraphs
  interface DisplayLine {
    paraIndex: number;
    startOffset: number;
    endOffset: number;
    text: string;
  }

  let displayLines: DisplayLine[] = $state([]);
  let numPages = $derived(Math.max(1, Math.ceil(displayLines.length / linesPerPage)));

  // Canvas refs for each page
  let canvases: HTMLCanvasElement[] = [];

  // Get font style string
  function getFontStyle(): string {
    return `${isItalic ? 'italic ' : ''}${isBold ? 'bold ' : ''}${scaledFontSize}px ${fontFamily}`;
  }

  // Measure text width using the measurement canvas
  function measureTextWidth(text: string): number {
    if (!measureCanvas) return text.length * scaledFontSize * 0.5;
    const ctx = measureCanvas.getContext('2d');
    if (!ctx) return text.length * scaledFontSize * 0.5;
    ctx.font = getFontStyle();
    return ctx.measureText(text).width;
  }

  // Wrap a single paragraph into display lines
  function wrapParagraph(paraIndex: number, text: string): DisplayLine[] {
    if (!text) {
      return [{ paraIndex, startOffset: 0, endOffset: 0, text: '' }];
    }

    // Make sure we have a valid content width
    const contentWidth = scaledContentWidth > 0 ? scaledContentWidth : 500;

    const result: DisplayLine[] = [];
    let currentStart = 0;

    while (currentStart < text.length) {
      // First check if the entire remaining text fits
      const remainingText = text.substring(currentStart);
      if (measureTextWidth(remainingText) <= contentWidth) {
        result.push({
          paraIndex,
          startOffset: currentStart,
          endOffset: text.length,
          text: remainingText,
        });
        break;
      }

      // Find the break point by progressively measuring
      let lineEnd = currentStart;
      let lastWordBoundary = currentStart;

      for (let i = currentStart + 1; i <= text.length; i++) {
        const testText = text.substring(currentStart, i);
        const width = measureTextWidth(testText);

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
      });

      currentStart = lineEnd;

      // Skip leading spaces on new lines
      while (currentStart < text.length && text[currentStart] === ' ') {
        currentStart++;
      }
    }

    return result;
  }

  // Recompute all display lines from paragraphs
  function recomputeDisplayLines() {
    const newDisplayLines: DisplayLine[] = [];
    for (let i = 0; i < paragraphs.length; i++) {
      const wrapped = wrapParagraph(i, paragraphs[i]);
      newDisplayLines.push(...wrapped);
    }
    displayLines = newDisplayLines;
    totalPages.set(numPages);
  }

  // Convert paragraph position to display line position
  function paraToDisplayPos(para: number, offset: number): { line: number; col: number } {
    for (let i = 0; i < displayLines.length; i++) {
      const dl = displayLines[i];
      if (dl.paraIndex === para && offset >= dl.startOffset && offset <= dl.endOffset) {
        return { line: i, col: offset - dl.startOffset };
      }
    }
    // Fallback: last position
    const lastLine = displayLines.length - 1;
    return { line: lastLine, col: displayLines[lastLine]?.text.length || 0 };
  }

  // Convert display line position to paragraph position
  function displayToPara(line: number, col: number): { para: number; offset: number } {
    if (line < 0 || line >= displayLines.length) {
      return { para: paragraphs.length - 1, offset: paragraphs[paragraphs.length - 1].length };
    }
    const dl = displayLines[line];
    return { para: dl.paraIndex, offset: dl.startOffset + Math.min(col, dl.text.length) };
  }

  onMount(() => {
    hiddenTextarea?.focus();
    // Wait for next tick to ensure measureCanvas is bound
    requestAnimationFrame(() => {
      recomputeDisplayLines();
      renderAllPages();
    });
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

      // Calculate which display lines belong to this page
      const startLine = pageIndex * linesPerPage;
      const endLine = Math.min(startLine + linesPerPage, displayLines.length);

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

      // Render display lines for this page
      for (let i = startLine; i < endLine; i++) {
        const lineIndex = i - startLine;
        const y = marginTop + lineIndex * scaledLineHeight;
        const x = marginLeft;
        const dl = displayLines[i];
        const text = dl.text;

        // Draw selection highlight
        if (selStartDisplay && selEndDisplay) {
          if (i >= selStartDisplay.line && i <= selEndDisplay.line) {
            ctx.fillStyle = '#b4d7ff';
            const startCol = i === selStartDisplay.line ? selStartDisplay.col : 0;
            const endCol = i === selEndDisplay.line ? selEndDisplay.col : text.length;

            const startX = x + measureTextWidth(text.substring(0, startCol));
            const width = measureTextWidth(text.substring(startCol, endCol)) || 5;

            ctx.fillRect(startX, y, width, scaledLineHeight);
            ctx.fillStyle = '#202124';
          }
        }

        // Draw text
        ctx.font = getFontStyle();
        ctx.fillText(text, x, y + (scaledLineHeight - scaledFontSize) / 2);

        // Draw cursor
        if (i === cursorDisplay.line && !selectionStart) {
          const cursorX = x + measureTextWidth(text.substring(0, cursorDisplay.col));
          ctx.fillStyle = '#000';
          ctx.fillRect(cursorX, y + 2, 2, scaledLineHeight - 4);
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

  function handleKeyDown(event: KeyboardEvent) {
    const key = event.key;

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
              deleteSelection();
            }
          }
          event.preventDefault();
          return;
        case 'v':
          event.preventDefault();
          navigator.clipboard.readText().then(text => {
            if (selectionStart) {
              deleteSelection();
            }
            insertText(text);
          });
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
        if (selectionStart && selectionEnd) {
          deleteSelection();
        } else if (cursorOffset > 0) {
          paragraphs[cursorPara] = paragraphs[cursorPara].substring(0, cursorOffset - 1) + paragraphs[cursorPara].substring(cursorOffset);
          cursorOffset--;
          recomputeDisplayLines();
        } else if (cursorPara > 0) {
          const prevLen = paragraphs[cursorPara - 1].length;
          paragraphs[cursorPara - 1] += paragraphs[cursorPara];
          paragraphs = [...paragraphs.slice(0, cursorPara), ...paragraphs.slice(cursorPara + 1)];
          cursorPara--;
          cursorOffset = prevLen;
          recomputeDisplayLines();
        }
        break;

      case 'Delete':
        event.preventDefault();
        if (selectionStart && selectionEnd) {
          deleteSelection();
        } else if (cursorOffset < paragraphs[cursorPara].length) {
          paragraphs[cursorPara] = paragraphs[cursorPara].substring(0, cursorOffset) + paragraphs[cursorPara].substring(cursorOffset + 1);
          recomputeDisplayLines();
        } else if (cursorPara < paragraphs.length - 1) {
          paragraphs[cursorPara] += paragraphs[cursorPara + 1];
          paragraphs = [...paragraphs.slice(0, cursorPara + 1), ...paragraphs.slice(cursorPara + 2)];
          recomputeDisplayLines();
        }
        break;

      case 'Enter':
        event.preventDefault();
        if (selectionStart && selectionEnd) {
          deleteSelection();
        }
        {
          const before = paragraphs[cursorPara].substring(0, cursorOffset);
          const after = paragraphs[cursorPara].substring(cursorOffset);
          paragraphs[cursorPara] = before;
          paragraphs = [...paragraphs.slice(0, cursorPara + 1), after, ...paragraphs.slice(cursorPara + 1)];
          cursorPara++;
          cursorOffset = 0;
          recomputeDisplayLines();
        }
        break;

      default:
        if (key.length === 1) {
          event.preventDefault();
          if (selectionStart && selectionEnd) {
            deleteSelection();
          }
          insertText(key);
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

      paragraphs[cursorPara] = before + lines[0];

      const newParas = lines.slice(1, -1);
      const lastPara = lines[lines.length - 1] + after;

      paragraphs = [
        ...paragraphs.slice(0, cursorPara + 1),
        ...newParas,
        lastPara,
        ...paragraphs.slice(cursorPara + 1)
      ];

      cursorPara += lines.length - 1;
      cursorOffset = lines[lines.length - 1].length;
    }

    recomputeDisplayLines();
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

    // Calculate which display line was clicked
    const lineInPage = Math.floor((y - marginTop) / scaledLineHeight);
    const displayLineIndex = pageIndex * linesPerPage + lineInPage;

    if (displayLineIndex >= 0 && displayLineIndex < displayLines.length) {
      const dl = displayLines[displayLineIndex];

      // Calculate character position
      let col = 0;
      let textWidth = marginLeft;

      for (let i = 0; i <= dl.text.length; i++) {
        const charWidth = i < dl.text.length ? measureTextWidth(dl.text[i]) : 0;
        if (textWidth + charWidth / 2 >= x) {
          col = i;
          break;
        }
        textWidth += charWidth;
        col = i + 1;
      }

      col = Math.min(col, dl.text.length);

      cursorPara = dl.paraIndex;
      cursorOffset = dl.startOffset + col;
    }

    selectionStart = { para: cursorPara, offset: cursorOffset };
    selectionEnd = null;

    const handleMouseMove = (e: MouseEvent) => {
      for (let i = 0; i < canvases.length; i++) {
        const cvs = canvases[i];
        if (!cvs) continue;

        const r = cvs.getBoundingClientRect();
        if (e.clientY >= r.top && e.clientY <= r.bottom) {
          const mx = e.clientX - r.left;
          const my = e.clientY - r.top;

          const lineInPg = Math.floor((my - marginTop) / scaledLineHeight);
          const dlIdx = i * linesPerPage + lineInPg;

          if (dlIdx >= 0 && dlIdx < displayLines.length) {
            const dl = displayLines[dlIdx];

            let col = 0;
            let tw = marginLeft;
            for (let j = 0; j <= dl.text.length; j++) {
              const cw = j < dl.text.length ? measureTextWidth(dl.text[j]) : 0;
              if (tw + cw / 2 >= mx) {
                col = j;
                break;
              }
              tw += cw;
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

      // Clear selection if start equals end
      if (selectionStart && selectionEnd &&
          selectionStart.para === selectionEnd.para &&
          selectionStart.offset === selectionEnd.offset) {
        selectionStart = null;
        selectionEnd = null;
      }
    };

    window.addEventListener('mousemove', handleMouseMove);
    window.addEventListener('mouseup', handleMouseUp);

    hiddenTextarea?.focus();
  }

  function handleFormat(command: string) {
    switch (command) {
      case 'bold':
        isBold = !isBold;
        break;
      case 'italic':
        isItalic = !isItalic;
        break;
    }
    recomputeDisplayLines();
    hiddenTextarea?.focus();
  }

  // Generate page array for rendering
  let pageArray = $derived(Array.from({ length: numPages }, (_, i) => i));
</script>

<div class="editor-wrapper">
  <Toolbar onFormat={handleFormat} />

  <!-- Hidden canvas for text measurement -->
  <canvas bind:this={measureCanvas} class="measure-canvas"></canvas>

  <!-- Hidden textarea for keyboard input -->
  <textarea
    bind:this={hiddenTextarea}
    class="hidden-input"
    onkeydown={handleKeyDown}
    autocomplete="off"
    autocorrect="off"
    autocapitalize="off"
    spellcheck="false"
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
</style>
