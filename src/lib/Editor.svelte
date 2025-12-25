<script lang="ts">
  import { onMount } from 'svelte';
  import Toolbar from './Toolbar.svelte';
  import { pageConfig, zoomLevel, currentPage, totalPages } from './stores';
  import { getContentDimensions, getPageDimensions, mmToPixels } from './types';

  let editorContainer: HTMLDivElement;
  let canvasContainer: HTMLDivElement;
  let hiddenTextarea: HTMLTextAreaElement;
  let measureCanvas: HTMLCanvasElement;

  // Paragraph metadata type
  type TextAlign = 'left' | 'center' | 'right' | 'justify';
  type ListType = 'none' | 'bullet' | 'numbered';
  type BlockType = 'p' | 'h1' | 'h2' | 'h3' | 'h4' | 'blockquote';
  type ImageDisplay = 'inline' | 'block' | 'float-left' | 'float-right';

  interface ParagraphMeta {
    align: TextAlign;
    listType: ListType;
    blockType: BlockType;
    indent: number;
  }

  interface DocumentImage {
    id: string;
    src: string;  // data URL or external URL
    width: number;
    height: number;
    naturalWidth: number;  // Original image width
    naturalHeight: number; // Original image height
    display: ImageDisplay;
    // Crop settings (percentages 0-100)
    cropTop?: number;
    cropRight?: number;
    cropBottom?: number;
    cropLeft?: number;
  }

  // Special marker for image paragraphs
  const IMAGE_MARKER = '\uFFFC'; // Object replacement character

  // Document state - paragraphs are logical units (separated by Enter)
  let paragraphs: string[] = $state(['']);
  let paragraphMeta: ParagraphMeta[] = $state([{ align: 'left', listType: 'none', blockType: 'p', indent: 0 }]);
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

  // Text styling
  let fontSize = $state(16);
  let fontFamily = $state('Arial');
  let lineHeightMultiplier = $state(1.5);
  let isBold = $state(false);
  let isItalic = $state(false);
  let isUnderline = $state(false);
  let isStrikethrough = $state(false);

  // Default paragraph meta
  function getDefaultMeta(): ParagraphMeta {
    return { align: 'left', listType: 'none', blockType: 'p', indent: 0 };
  }

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
    meta: ParagraphMeta;
    listNumber?: number; // For numbered lists
    isImage?: boolean; // Is this an image line?
    imageId?: string; // Reference to image
    imageHeight?: number; // Height in display lines
    floatReduction?: { side: 'left' | 'right'; width: number }; // Text width reduction for floats
  }

  // Track selected image
  let selectedImageId: string | null = $state(null);

  // Image options popup state
  let showImageOptionsPopup = $state(false);
  let imageOptionsPosition = $state({ x: 0, y: 0 });

  // Image resize state
  type ResizeHandle = 'nw' | 'ne' | 'sw' | 'se' | 'n' | 's' | 'e' | 'w' | null;
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

  // Float image tracking for text wrapping
  interface FloatImage {
    id: string;
    startLine: number; // Display line where float starts
    endLine: number;   // Display line where float ends
    width: number;     // Scaled width
    side: 'left' | 'right';
  }
  let activeFloats: FloatImage[] = $state([]);

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
  function wrapParagraph(
    paraIndex: number,
    text: string,
    listNumber?: number,
    floatReduction?: { side: 'left' | 'right'; width: number }
  ): DisplayLine[] {
    const meta = paragraphMeta[paraIndex] || getDefaultMeta();

    // Calculate effective content width (account for list indent and float)
    const listIndent = meta.listType !== 'none' ? scaledFontSize * 1.5 : 0;
    const floatWidth = floatReduction ? floatReduction.width + 10 : 0; // 10px gap
    const baseContentWidth = scaledContentWidth > 0 ? scaledContentWidth : 500;
    const contentWidth = baseContentWidth - listIndent - floatWidth;

    if (!text) {
      return [{ paraIndex, startOffset: 0, endOffset: 0, text: '', meta, listNumber, floatReduction }];
    }

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
          meta,
          listNumber: result.length === 0 ? listNumber : undefined,
          floatReduction,
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

  // Recompute all display lines from paragraphs
  function recomputeDisplayLines() {
    const newDisplayLines: DisplayLine[] = [];
    const newActiveFloats: FloatImage[] = [];
    let numberedListCounter = 0;

    for (let i = 0; i < paragraphs.length; i++) {
      const meta = paragraphMeta[i] || getDefaultMeta();
      const paraText = paragraphs[i];

      // Check if this is an image paragraph
      if (paraText.startsWith(IMAGE_MARKER)) {
        const imageId = paraText.substring(1); // Remove the marker
        const docImage = images.find(img => img.id === imageId);

        if (docImage) {
          const scaledWidth = (docImage.width * $zoomLevel) / 100;
          const scaledHeight = (docImage.height * $zoomLevel) / 100;
          const imageLines = Math.ceil(scaledHeight / scaledLineHeight);

          if (docImage.display === 'float-left' || docImage.display === 'float-right') {
            // For float images, track the float for text wrapping
            // The float starts at current line position and extends for imageLines
            const startLine = newDisplayLines.length;
            newActiveFloats.push({
              id: imageId,
              startLine,
              endLine: startLine + imageLines,
              width: scaledWidth,
              side: docImage.display === 'float-left' ? 'left' : 'right',
            });

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

      // Wrap the paragraph with float consideration
      const wrapped = wrapParagraph(i, paraText, listNumber, floatReduction);

      // For each wrapped line, check if float still applies
      for (let j = 0; j < wrapped.length; j++) {
        const lineIdx = newDisplayLines.length;
        let lineFloatReduction: { side: 'left' | 'right'; width: number } | undefined;

        for (const float of newActiveFloats) {
          if (lineIdx >= float.startLine && lineIdx < float.endLine) {
            lineFloatReduction = { side: float.side, width: float.width };
            break;
          }
        }

        // If float changed, we need to re-wrap from this point
        // For simplicity, we'll just tag lines with their float reduction
        wrapped[j].floatReduction = lineFloatReduction;
        newDisplayLines.push(wrapped[j]);
      }
    }

    activeFloats = newActiveFloats;
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

    // Add global paste handler for when textarea might lose focus
    const handleGlobalPaste = (event: ClipboardEvent) => {
      // Only handle if focus is on the editor area
      if (!editorContainer?.contains(document.activeElement) && document.activeElement !== hiddenTextarea) {
        return;
      }
      handlePaste(event);
    };

    // Global mouse handlers for resize and crop
    const handleGlobalMouseMove = (event: MouseEvent) => {
      if (isResizing) {
        handleResizeMove(event.clientX, event.clientY);
      } else if (isCropping && cropHandle) {
        handleCropMove(event.clientX, event.clientY);
      }
    };

    const handleGlobalMouseUp = () => {
      if (isResizing) {
        endResize();
      }
      if (cropHandle) {
        endCropDrag();
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

      // First pass: render float images for this page
      for (const float of activeFloats) {
        // Check if this float is visible on this page
        const floatStartLineOnPage = float.startLine - startLine;
        if (floatStartLineOnPage >= 0 && floatStartLineOnPage < linesPerPage) {
          const docImage = images.find(img => img.id === float.id);
          const img = docImage ? loadedImages.get(docImage.id) : null;

          if (docImage && img) {
            const scaledWidth = (docImage.width * $zoomLevel) / 100;
            const scaledHeight = (docImage.height * $zoomLevel) / 100;
            const floatY = marginTop + floatStartLineOnPage * scaledLineHeight;
            const floatX = float.side === 'left' ? marginLeft : marginLeft + scaledContentWidth - scaledWidth;

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
            let destW = scaledWidth;
            let destH = scaledWidth / cropAspect;

            ctx.drawImage(img, srcX, srcY, srcW, srcH, floatX, floatY, destW, destH);

            // Draw selection border if selected
            if (selectedImageId === docImage.id) {
              ctx.strokeStyle = '#1a73e8';
              ctx.lineWidth = 2;
              ctx.setLineDash([]);
              ctx.strokeRect(floatX - 2, floatY - 2, destW + 4, destH + 4);

              // Draw resize handles (8 handles)
              const handleSize = 8;
              ctx.fillStyle = isCropping ? '#ff9800' : '#1a73e8';

              // Corners
              ctx.fillRect(floatX - handleSize/2, floatY - handleSize/2, handleSize, handleSize);
              ctx.fillRect(floatX + destW - handleSize/2, floatY - handleSize/2, handleSize, handleSize);
              ctx.fillRect(floatX - handleSize/2, floatY + destH - handleSize/2, handleSize, handleSize);
              ctx.fillRect(floatX + destW - handleSize/2, floatY + destH - handleSize/2, handleSize, handleSize);

              // Edge midpoints
              ctx.fillRect(floatX + destW/2 - handleSize/2, floatY - handleSize/2, handleSize, handleSize);
              ctx.fillRect(floatX + destW/2 - handleSize/2, floatY + destH - handleSize/2, handleSize, handleSize);
              ctx.fillRect(floatX - handleSize/2, floatY + destH/2 - handleSize/2, handleSize, handleSize);
              ctx.fillRect(floatX + destW - handleSize/2, floatY + destH/2 - handleSize/2, handleSize, handleSize);

              // Draw crop mode indicator
              if (isCropping) {
                ctx.strokeStyle = '#ff9800';
                ctx.lineWidth = 2;
                ctx.setLineDash([5, 5]);
                ctx.strokeRect(floatX, floatY, destW, destH);
                ctx.setLineDash([]);
              }
            } else {
              ctx.strokeStyle = '#e0e0e0';
              ctx.lineWidth = 1;
              ctx.setLineDash([]);
              ctx.strokeRect(floatX, floatY, destW, destH);
            }
          }
        }
      }

      // Render display lines for this page
      for (let i = startLine; i < endLine; i++) {
        const lineIndex = i - startLine;
        const y = marginTop + lineIndex * scaledLineHeight;
        const dl = displayLines[i];

        // Handle block image rendering (non-float)
        if (dl.isImage && dl.imageId) {
          const docImage = images.find(img => img.id === dl.imageId);

          // Skip float images (they're rendered separately in the first pass)
          if (docImage && (docImage.display === 'float-left' || docImage.display === 'float-right')) {
            continue;
          }

          // Only render block images on their first line (imageHeight > 0)
          if (!dl.imageHeight || dl.imageHeight <= 0) {
            continue;
          }

          const img = docImage ? loadedImages.get(docImage.id) : null;

          if (docImage && img) {
            const scaledWidth = (docImage.width * $zoomLevel) / 100;
            const scaledHeight = (docImage.height * $zoomLevel) / 100;

            // Center the image
            const imageX = marginLeft + (scaledContentWidth - scaledWidth) / 2;

            // Draw the image with crop applied
            const cropTop = docImage.cropTop || 0;
            const cropRight = docImage.cropRight || 0;
            const cropBottom = docImage.cropBottom || 0;
            const cropLeft = docImage.cropLeft || 0;

            // Calculate source rect from crop percentages
            const srcX = (cropLeft / 100) * img.naturalWidth;
            const srcY = (cropTop / 100) * img.naturalHeight;
            const srcW = ((100 - cropLeft - cropRight) / 100) * img.naturalWidth;
            const srcH = ((100 - cropTop - cropBottom) / 100) * img.naturalHeight;

            // Calculate destination dimensions maintaining aspect ratio
            const cropAspect = srcW / srcH;
            let destW = scaledWidth;
            let destH = scaledWidth / cropAspect;

            ctx.drawImage(img, srcX, srcY, srcW, srcH, imageX, y, destW, destH);

            // Draw selection border if selected
            if (selectedImageId === docImage.id) {
              ctx.strokeStyle = '#1a73e8';
              ctx.lineWidth = 2;
              ctx.setLineDash([]);
              ctx.strokeRect(imageX - 2, y - 2, destW + 4, destH + 4);

              // Draw resize handles (8 handles: 4 corners + 4 edges)
              const handleSize = 8;
              ctx.fillStyle = isCropping ? '#ff9800' : '#1a73e8';

              // Corners
              ctx.fillRect(imageX - handleSize/2, y - handleSize/2, handleSize, handleSize);
              ctx.fillRect(imageX + destW - handleSize/2, y - handleSize/2, handleSize, handleSize);
              ctx.fillRect(imageX - handleSize/2, y + destH - handleSize/2, handleSize, handleSize);
              ctx.fillRect(imageX + destW - handleSize/2, y + destH - handleSize/2, handleSize, handleSize);

              // Edge midpoints
              ctx.fillRect(imageX + destW/2 - handleSize/2, y - handleSize/2, handleSize, handleSize);
              ctx.fillRect(imageX + destW/2 - handleSize/2, y + destH - handleSize/2, handleSize, handleSize);
              ctx.fillRect(imageX - handleSize/2, y + destH/2 - handleSize/2, handleSize, handleSize);
              ctx.fillRect(imageX + destW - handleSize/2, y + destH/2 - handleSize/2, handleSize, handleSize);

              // Draw crop mode indicator
              if (isCropping) {
                ctx.strokeStyle = '#ff9800';
                ctx.lineWidth = 2;
                ctx.setLineDash([5, 5]);
                ctx.strokeRect(imageX, y, destW, destH);
                ctx.setLineDash([]);
              }
            } else {
              // Draw subtle border
              ctx.strokeStyle = '#e0e0e0';
              ctx.lineWidth = 1;
              ctx.setLineDash([]);
              ctx.strokeRect(imageX, y, destW, destH);
            }
          }
          continue; // Skip normal text rendering for image lines
        }

        // Skip placeholder lines for images (they don't render anything)
        if (dl.isImage) {
          continue;
        }

        const text = dl.text;
        const meta = dl.meta;

        // Calculate list indent and float offset
        const listIndent = meta.listType !== 'none' ? scaledFontSize * 1.5 : 0;
        const floatOffset = dl.floatReduction && dl.floatReduction.side === 'left' ? dl.floatReduction.width + 10 : 0;
        const textStartX = marginLeft + listIndent + floatOffset;

        // Get font style based on block type
        let blockFontSize = scaledFontSize;
        let blockFontWeight = isBold ? 'bold ' : '';
        let blockFontStyle = isItalic ? 'italic ' : '';

        switch (meta.blockType) {
          case 'h1':
            blockFontSize = scaledFontSize * 2;
            blockFontWeight = 'bold ';
            break;
          case 'h2':
            blockFontSize = scaledFontSize * 1.5;
            blockFontWeight = 'bold ';
            break;
          case 'h3':
            blockFontSize = scaledFontSize * 1.17;
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

        // Measure text width for alignment
        const textWidth = ctx.measureText(text).width;
        const floatWidthReduction = dl.floatReduction ? dl.floatReduction.width + 10 : 0;
        const availableWidth = scaledContentWidth - listIndent - floatWidthReduction;

        // Calculate x position and word spacing based on alignment
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
            // Only justify if not the last line of the paragraph and has multiple words
            if (!isLastLineOfPara && words.length > 1 && text.trim().length > 0) {
              const extraSpace = availableWidth - textWidth;
              wordSpacing = extraSpace / (words.length - 1);
            }
            break;
        }

        // Draw selection highlight
        if (selStartDisplay && selEndDisplay) {
          if (i >= selStartDisplay.line && i <= selEndDisplay.line) {
            ctx.fillStyle = '#b4d7ff';
            const startCol = i === selStartDisplay.line ? selStartDisplay.col : 0;
            const endCol = i === selEndDisplay.line ? selEndDisplay.col : text.length;

            const selStartX = x + ctx.measureText(text.substring(0, startCol)).width;
            const selWidth = ctx.measureText(text.substring(startCol, endCol)).width || 5;

            ctx.fillRect(selStartX, y, selWidth, scaledLineHeight);
          }
        }

        // Draw list marker
        if (dl.startOffset === 0 && meta.listType !== 'none') {
          ctx.fillStyle = '#202124';
          if (meta.listType === 'bullet') {
            // Draw bullet point
            const bulletX = marginLeft + scaledFontSize * 0.5;
            const bulletY = y + scaledLineHeight / 2;
            ctx.beginPath();
            ctx.arc(bulletX, bulletY, scaledFontSize * 0.15, 0, Math.PI * 2);
            ctx.fill();
          } else if (meta.listType === 'numbered' && dl.listNumber) {
            // Draw number
            ctx.font = `${scaledFontSize}px ${fontFamily}`;
            ctx.textAlign = 'right';
            ctx.fillText(`${dl.listNumber}.`, marginLeft + scaledFontSize * 1.2, y + (scaledLineHeight - scaledFontSize) / 2);
            ctx.textAlign = 'left';
            ctx.font = lineFont;
          }
        }

        // Draw blockquote indicator
        if (meta.blockType === 'blockquote' && dl.startOffset === 0) {
          ctx.fillStyle = '#ccc';
          ctx.fillRect(marginLeft, y, 3, scaledLineHeight);
        }

        // Draw text (with word spacing for justify)
        ctx.fillStyle = '#202124';
        ctx.font = lineFont;
        const textY = y + (scaledLineHeight - blockFontSize) / 2;

        if (wordSpacing > 0 && meta.align === 'justify') {
          // Draw words individually with extra spacing
          let wordX = x;
          for (let w = 0; w < words.length; w++) {
            ctx.fillText(words[w], wordX, textY);
            wordX += ctx.measureText(words[w]).width;
            if (w < words.length - 1) {
              wordX += ctx.measureText(' ').width + wordSpacing;
            }
          }
        } else {
          ctx.fillText(text, x, textY);
        }

        // Calculate actual rendered width for decorations
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

        // Draw cursor - account for word spacing in justify mode
        if (i === cursorDisplay.line && !selectionStart) {
          let cursorX = x;
          if (wordSpacing > 0 && meta.align === 'justify') {
            // Calculate cursor position with word spacing
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

    // Handle image deletion
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
        if (selectionStart && selectionEnd) {
          deleteSelection();
        }
        {
          const before = paragraphs[cursorPara].substring(0, cursorOffset);
          const after = paragraphs[cursorPara].substring(cursorOffset);
          paragraphs[cursorPara] = before;
          paragraphs = [...paragraphs.slice(0, cursorPara + 1), after, ...paragraphs.slice(cursorPara + 1)];
          // Copy current paragraph's metadata to new paragraph (inherit formatting)
          const currentMeta = paragraphMeta[cursorPara] || getDefaultMeta();
          paragraphMeta = [
            ...paragraphMeta.slice(0, cursorPara + 1),
            { ...currentMeta },
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
      const currentMeta = paragraphMeta[cursorPara] || getDefaultMeta();

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

    // Check if clicking on a resize handle of a selected image
    if (selectedImageId && !isCropping) {
      const bounds = getSelectedImageBounds();
      if (bounds && bounds.pageIndex === pageIndex) {
        const handle = getResizeHandleAtPoint(x, y, bounds);
        if (handle) {
          event.preventDefault();
          startResize(handle, event.clientX, event.clientY);
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

    // Calculate which display line was clicked
    const lineInPage = Math.floor((y - marginTop) / scaledLineHeight);
    const displayLineIndex = pageIndex * linesPerPage + lineInPage;

    // Check if clicked on a float image first
    for (const float of activeFloats) {
      const floatStartLineOnPage = float.startLine - (pageIndex * linesPerPage);
      if (floatStartLineOnPage >= 0 && floatStartLineOnPage < linesPerPage) {
        const docImage = images.find(img => img.id === float.id);
        if (docImage) {
          const scaledWidth = (docImage.width * $zoomLevel) / 100;
          const scaledHeight = (docImage.height * $zoomLevel) / 100;
          const floatY = marginTop + floatStartLineOnPage * scaledLineHeight;
          const floatX = float.side === 'left' ? marginLeft : marginLeft + scaledContentWidth - scaledWidth;

          if (x >= floatX && x <= floatX + scaledWidth && y >= floatY && y <= floatY + scaledHeight) {
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

    // Check if clicked on a block image
    if (displayLineIndex >= 0 && displayLineIndex < displayLines.length) {
      const dl = displayLines[displayLineIndex];

      if (dl.isImage && dl.imageId) {
        // Find the image and check if click is within its bounds
        const docImage = images.find(img => img.id === dl.imageId);
        if (docImage && docImage.display !== 'float-left' && docImage.display !== 'float-right') {
          const scaledWidth = (docImage.width * $zoomLevel) / 100;
          const scaledHeight = (docImage.height * $zoomLevel) / 100;
          const imageX = marginLeft + (scaledContentWidth - scaledWidth) / 2;

          // Find the first line of this image
          let firstImageLine = displayLineIndex;
          while (firstImageLine > 0 && displayLines[firstImageLine - 1].imageId === dl.imageId) {
            firstImageLine--;
          }
          const imageY = marginTop + (firstImageLine - pageIndex * linesPerPage) * scaledLineHeight;

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
    } else if (displayLineIndex < 0) {
      // Clicked above content - go to start of first line on this page
      const firstLineOnPage = pageIndex * linesPerPage;
      if (firstLineOnPage < displayLines.length) {
        const dl = displayLines[firstLineOnPage];
        cursorPara = dl.paraIndex;
        cursorOffset = dl.startOffset;
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

    // Focus immediately on mousedown
    hiddenTextarea?.focus();
    // Render to show cursor position change
    renderAllPages();
  }

  function handleFormat(command: string, value?: string) {
    // Get selected paragraph range
    let startPara = cursorPara;
    let endPara = cursorPara;
    if (selectionStart && selectionEnd) {
      startPara = Math.min(selectionStart.para, selectionEnd.para);
      endPara = Math.max(selectionStart.para, selectionEnd.para);
    }

    switch (command) {
      // Text styling
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

      // Undo/Redo (placeholder - would need history implementation)
      case 'undo':
      case 'redo':
        // TODO: implement undo/redo history
        break;

      // Image insertion
      case 'insertImage':
        showImagePopup = true;
        return; // Don't refocus textarea
    }

    // Trigger reactivity
    paragraphMeta = [...paragraphMeta];
    recomputeDisplayLines();
    hiddenTextarea?.focus();
  }

  // Image handling functions
  function generateImageId(): string {
    return 'img-' + Date.now() + '-' + Math.random().toString(36).substr(2, 9);
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

  async function insertImage(src: string, display: ImageDisplay = 'block') {
    try {
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
        display,
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
      const currentMeta = paragraphMeta[cursorPara] || getDefaultMeta();
      paragraphMeta = [
        ...paragraphMeta.slice(0, cursorPara + 1),
        { ...getDefaultMeta(), align: 'center' }, // Image meta
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
      paragraphMeta = [getDefaultMeta()];
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

  function changeImageDisplay(display: ImageDisplay) {
    if (!selectedImageId) return;

    const imageIndex = images.findIndex(img => img.id === selectedImageId);
    if (imageIndex === -1) return;

    images[imageIndex] = { ...images[imageIndex], display };
    images = [...images]; // Trigger reactivity

    showImageOptionsPopup = false;
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

  // Get image bounds for a selected image (accounts for crop)
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

    // Find the display line for this image
    const displayLineIdx = displayLines.findIndex(dl => dl.imageId === selectedImageId && dl.isImage);
    if (displayLineIdx === -1) return null;

    const pageIndex = Math.floor(displayLineIdx / linesPerPage);
    const lineInPage = displayLineIdx - pageIndex * linesPerPage;

    // Check if it's a float
    const float = activeFloats.find(f => f.id === selectedImageId);
    let imageX: number;
    if (float) {
      imageX = float.side === 'left' ? marginLeft : marginLeft + scaledContentWidth - destW;
    } else {
      imageX = marginLeft + (scaledContentWidth - destW) / 2;
    }

    const imageY = marginTop + lineInPage * scaledLineHeight;

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
    onpaste={handlePaste}
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
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="image-options-overlay" onclick={closeImageOptionsPopup}>
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="image-options-popup"
        style="left: {imageOptionsPosition.x}px; top: {imageOptionsPosition.y}px;"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="image-options-header">
          <span>Image Options</span>
          <button class="popup-close" onclick={closeImageOptionsPopup}>&times;</button>
        </div>
        <div class="image-options-section">
          <div class="section-label">Layout</div>
          <div class="image-options-buttons">
            <button
              class="layout-btn"
              class:active={images.find(img => img.id === selectedImageId)?.display === 'inline'}
              onclick={() => changeImageDisplay('inline')}
              title="Inline with text"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <line x1="2" y1="12" x2="8" y2="12" stroke="currentColor" stroke-width="1.5"/>
                <rect x="9" y="8" width="6" height="8" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
                <line x1="16" y1="12" x2="22" y2="12" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>Inline</span>
            </button>
            <button
              class="layout-btn"
              class:active={images.find(img => img.id === selectedImageId)?.display === 'block'}
              onclick={() => changeImageDisplay('block')}
              title="Block (centered)"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <rect x="4" y="6" width="16" height="12" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
              </svg>
              <span>Block</span>
            </button>
            <button
              class="layout-btn"
              class:active={images.find(img => img.id === selectedImageId)?.display === 'float-left'}
              onclick={() => changeImageDisplay('float-left')}
              title="Float left (text wraps right)"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <rect x="2" y="4" width="8" height="8" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
                <line x1="12" y1="6" x2="22" y2="6" stroke="currentColor" stroke-width="1.5"/>
                <line x1="12" y1="10" x2="22" y2="10" stroke="currentColor" stroke-width="1.5"/>
                <line x1="2" y1="16" x2="22" y2="16" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>Left</span>
            </button>
            <button
              class="layout-btn"
              class:active={images.find(img => img.id === selectedImageId)?.display === 'float-right'}
              onclick={() => changeImageDisplay('float-right')}
              title="Float right (text wraps left)"
            >
              <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
                <rect x="14" y="4" width="8" height="8" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
                <line x1="2" y1="6" x2="12" y2="6" stroke="currentColor" stroke-width="1.5"/>
                <line x1="2" y1="10" x2="12" y2="10" stroke="currentColor" stroke-width="1.5"/>
                <line x1="2" y1="16" x2="22" y2="16" stroke="currentColor" stroke-width="1.5"/>
              </svg>
              <span>Right</span>
            </button>
          </div>
        </div>
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
