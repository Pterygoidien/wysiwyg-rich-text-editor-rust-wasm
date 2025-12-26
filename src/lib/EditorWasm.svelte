<script lang="ts">
  import { onMount, tick, onDestroy } from 'svelte';
  import init, { Engine } from './engine-wasm/editor_engine.js';
  import { parseRenderCommands, executeRenderCommands, type RenderCommand } from './engine-bridge';
  import Toolbar from './Toolbar.svelte';
  import { pageConfig, fontSize, lineHeight, letterSpacing, paragraphSpacing, fontFamily, zoomLevel } from './stores';
  import { getPageDimensions, getContentDimensions, mmToPixels } from './types';

  // Special markers used by the engine
  const IMAGE_MARKER = '\u{FFFC}';  // Object replacement character
  const PAGE_BREAK_MARKER = '\u{FFFD}';  // Replacement character
  const TABLE_MARKER = '\u{FFFB}';  // Annotation terminator

  /**
   * Check if a paragraph is a special paragraph (image, page break, or table)
   * that shouldn't be directly edited
   */
  function isSpecialParagraph(text: string): boolean {
    return text.startsWith(IMAGE_MARKER) || text === PAGE_BREAK_MARKER || text.startsWith(TABLE_MARKER);
  }

  let pageCanvases: HTMLCanvasElement[] = [];
  let pagesContainer: HTMLDivElement;
  let measureCanvas: HTMLCanvasElement;
  let hiddenTextarea: HTMLTextAreaElement;
  let engine: Engine | null = null;
  let isReady = $state(false);
  let error = $state<string | null>(null);
  let currentPage = $state(0); // Page where cursor is located

  // Editor state
  let cursorPara = $state(0);
  let cursorOffset = $state(0);
  let pageCount = $state(1);

  // Selection state
  let selectionStart: { para: number; offset: number } | null = $state(null);
  let selectionEnd: { para: number; offset: number } | null = $state(null);

  // Undo/Redo state
  interface EditorSnapshot {
    document: string;  // JSON serialized document
    cursorPara: number;
    cursorOffset: number;
  }

  let undoStack: EditorSnapshot[] = $state([]);
  let redoStack: EditorSnapshot[] = $state([]);
  const MAX_UNDO_STACK = 100;

  // Derived state for toolbar
  let canUndo = $derived(undoStack.length > 0);
  let canRedo = $derived(redoStack.length > 0);

  // Page configuration - reactive from stores
  let PAGE_WIDTH = $state(816);
  let PAGE_HEIGHT = $state(1056);
  const PAGE_GAP = 40; // Gap between pages
  let MARGIN_TOP = $state(96);
  let MARGIN_RIGHT = $state(96);
  let MARGIN_BOTTOM = $state(96);
  let MARGIN_LEFT = $state(96);
  let COLUMNS = $state(1);
  let COLUMN_GAP = $state(48);
  let FONT_SIZE = $state(16);
  let LINE_HEIGHT = $state(1.5);
  let FONT_FAMILY = $state('Arial');
  let LETTER_SPACING = $state(0);
  let PARAGRAPH_SPACING = $state(12);
  let ZOOM = $state(100); // Zoom level as percentage

  // Cursor blinking state
  let cursorVisible = $state(true);
  let cursorBlinkInterval: ReturnType<typeof setInterval> | null = null;
  let isFocused = $state(false);

  // Loaded images cache
  const loadedImages = new Map<string, HTMLImageElement>();

  // Image insertion dialog state
  let showImageDialog = $state(false);
  let imageUrlInput = $state('');
  let dragOver = $state(false);

  // Table insertion dialog state
  let showTableDialog = $state(false);
  let tableRows = $state(3);
  let tableCols = $state(3);
  let tableHasHeaderRow = $state(false);
  let tableHasHeaderCol = $state(false);

  // Table editing state
  let activeTableId = $state<string | null>(null);
  let activeTablePara = $state<number | null>(null);
  let activeCell = $state<{ row: number; col: number } | null>(null);
  let cellCursorOffset = $state(0);
  let cellText = $state('');
  let showTableContextMenu = $state(false);
  let tableContextMenuPos = $state({ x: 0, y: 0 });

  // Cell selection for merge operations
  let cellSelectionStart = $state<{ row: number; col: number } | null>(null);
  let cellSelectionEnd = $state<{ row: number; col: number } | null>(null);

  // Mouse drag selection state
  let isMouseDown = $state(false);
  let mouseDownPage = $state(0);

  // Image selection and dragging state
  let selectedImageId = $state<string | null>(null);
  let showImageOptions = $state(false);
  let imageOptionsPosition = $state({ x: 0, y: 0 });
  let isDraggingImage = $state(false);
  let dragStartPos = $state({ x: 0, y: 0 });
  let dragImageStartPos = $state({ x: 0, y: 0 });
  let dragImagePage = $state(0);

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
  let cropOriginalValues = $state({ top: 0, right: 0, bottom: 0, left: 0 });

  // Get resize cursor based on handle
  function getResizeCursor(handle: ResizeHandle): string {
    switch (handle) {
      case 'nw':
      case 'se':
        return 'nwse-resize';
      case 'ne':
      case 'sw':
        return 'nesw-resize';
      case 'n':
      case 's':
        return 'ns-resize';
      case 'e':
      case 'w':
        return 'ew-resize';
      default:
        return 'default';
    }
  }

  // Cursor style based on current operation
  let canvasCursor = $derived.by(() => {
    if (isResizing && resizeHandle) {
      return getResizeCursor(resizeHandle);
    }
    if (isCropping && cropHandle) {
      return getResizeCursor(cropHandle);
    }
    if (isDraggingImage) {
      return 'move';
    }
    if (selectedImageId) {
      return 'default';
    }
    return 'text';
  });

  /**
   * Save current state to undo stack
   */
  function saveUndoState() {
    if (!engine) return;

    const snapshot: EditorSnapshot = {
      document: engine.save_document(),
      cursorPara,
      cursorOffset,
    };

    undoStack.push(snapshot);

    // Limit stack size
    if (undoStack.length > MAX_UNDO_STACK) {
      undoStack.shift();
    }

    // Clear redo stack on new action
    redoStack = [];
  }

  /**
   * Undo the last action
   */
  function undo() {
    if (!engine || undoStack.length === 0) return;

    // Save current state to redo stack
    const currentSnapshot: EditorSnapshot = {
      document: engine.save_document(),
      cursorPara,
      cursorOffset,
    };
    redoStack.push(currentSnapshot);

    // Pop and restore from undo stack
    const snapshot = undoStack.pop()!;
    engine.load_document(snapshot.document);
    cursorPara = snapshot.cursorPara;
    cursorOffset = snapshot.cursorOffset;

    clearSelection();
    recomputeAndRender();
  }

  /**
   * Redo the last undone action
   */
  function redo() {
    if (!engine || redoStack.length === 0) return;

    // Save current state to undo stack
    const currentSnapshot: EditorSnapshot = {
      document: engine.save_document(),
      cursorPara,
      cursorOffset,
    };
    undoStack.push(currentSnapshot);

    // Pop and restore from redo stack
    const snapshot = redoStack.pop()!;
    engine.load_document(snapshot.document);
    cursorPara = snapshot.cursorPara;
    cursorOffset = snapshot.cursorOffset;

    clearSelection();
    recomputeAndRender();
  }

  /**
   * Set block type for current paragraph (or selected paragraphs)
   */
  function setBlockType(blockType: string) {
    if (!engine) return;

    saveUndoState();

    if (hasSelection()) {
      // Apply to all selected paragraphs
      let startPara = selectionStart!.para;
      let endPara = selectionEnd!.para;
      if (startPara > endPara) [startPara, endPara] = [endPara, startPara];

      for (let i = startPara; i <= endPara; i++) {
        engine.set_block_type(i, blockType);
      }
    } else {
      // Apply to current paragraph
      engine.set_block_type(cursorPara, blockType);
    }

    recomputeAndRender();
  }

  /**
   * Set alignment for current paragraph (or selected paragraphs)
   */
  function setAlignment(align: string) {
    if (!engine) return;

    saveUndoState();

    if (hasSelection()) {
      // Apply to all selected paragraphs
      let startPara = selectionStart!.para;
      let endPara = selectionEnd!.para;
      if (startPara > endPara) [startPara, endPara] = [endPara, startPara];

      for (let i = startPara; i <= endPara; i++) {
        engine.set_alignment(i, align);
      }
    } else {
      // Apply to current paragraph
      engine.set_alignment(cursorPara, align);
    }

    recomputeAndRender();
  }

  /**
   * Toggle list type for current paragraph (or selected paragraphs)
   */
  function toggleList(listType: string) {
    if (!engine) return;

    saveUndoState();

    if (hasSelection()) {
      // Apply to all selected paragraphs
      let startPara = selectionStart!.para;
      let endPara = selectionEnd!.para;
      if (startPara > endPara) [startPara, endPara] = [endPara, startPara];

      for (let i = startPara; i <= endPara; i++) {
        engine.toggle_list(i, listType);
      }
    } else {
      // Apply to current paragraph
      engine.toggle_list(cursorPara, listType);
    }

    recomputeAndRender();
  }

  /**
   * Apply inline text styling (bold, italic, underline, strikethrough)
   */
  function applyInlineStyle(style: 'bold' | 'italic' | 'underline' | 'strikethrough') {
    if (!engine) return;

    saveUndoState();

    if (hasSelection()) {
      // Apply to selection
      let startPos = selectionStart!;
      let endPos = selectionEnd!;
      if (startPos.para > endPos.para ||
          (startPos.para === endPos.para && startPos.offset > endPos.offset)) {
        [startPos, endPos] = [endPos, startPos];
      }

      // Apply style to each paragraph in the selection
      for (let para = startPos.para; para <= endPos.para; para++) {
        const paraText = engine.get_paragraph(para) || '';
        const start = para === startPos.para ? startPos.offset : 0;
        const end = para === endPos.para ? endPos.offset : paraText.length;

        if (start < end) {
          switch (style) {
            case 'bold':
              engine.toggle_bold(para, start, end);
              break;
            case 'italic':
              engine.toggle_italic(para, start, end);
              break;
            case 'underline':
              engine.toggle_underline(para, start, end);
              break;
            case 'strikethrough':
              engine.toggle_strikethrough(para, start, end);
              break;
          }
        }
      }

      recomputeAndRender();
    }
    // If no selection, do nothing (need selection to apply inline styles)
  }

  /**
   * Apply text color to selected text
   */
  function applyTextColor(color: string) {
    if (!engine) return;

    saveUndoState();

    if (hasSelection()) {
      let startPos = selectionStart!;
      let endPos = selectionEnd!;
      if (startPos.para > endPos.para ||
          (startPos.para === endPos.para && startPos.offset > endPos.offset)) {
        [startPos, endPos] = [endPos, startPos];
      }

      for (let para = startPos.para; para <= endPos.para; para++) {
        const paraText = engine.get_paragraph(para) || '';
        const start = para === startPos.para ? startPos.offset : 0;
        const end = para === endPos.para ? endPos.offset : paraText.length;

        if (start < end) {
          engine.set_text_color(para, start, end, color);
        }
      }

      recomputeAndRender();
    }
  }

  /**
   * Apply highlight/background color to selected text
   */
  function applyHighlightColor(color: string) {
    if (!engine) return;

    saveUndoState();

    if (hasSelection()) {
      let startPos = selectionStart!;
      let endPos = selectionEnd!;
      if (startPos.para > endPos.para ||
          (startPos.para === endPos.para && startPos.offset > endPos.offset)) {
        [startPos, endPos] = [endPos, startPos];
      }

      for (let para = startPos.para; para <= endPos.para; para++) {
        const paraText = engine.get_paragraph(para) || '';
        const start = para === startPos.para ? startPos.offset : 0;
        const end = para === endPos.para ? endPos.offset : paraText.length;

        if (start < end) {
          engine.set_highlight_color(para, start, end, color);
        }
      }

      recomputeAndRender();
    }
  }

  /**
   * Show the image insertion dialog
   */
  function showImageInsertDialog() {
    showImageDialog = true;
    imageUrlInput = '';
  }

  /**
   * Close the image insertion dialog
   */
  function closeImageDialog() {
    showImageDialog = false;
    imageUrlInput = '';
    hiddenTextarea?.focus();
  }

  /**
   * Generate a unique image ID
   */
  function generateImageId(): string {
    return `img-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
  }

  /**
   * Insert an image from a URL
   */
  function insertImageFromUrl() {
    if (!imageUrlInput.trim()) return;

    const img = new Image();
    img.crossOrigin = 'anonymous'; // Allow cross-origin images
    img.onload = () => {
      insertLoadedImage(img, imageUrlInput);
    };
    img.onerror = () => {
      console.error('Failed to load image:', imageUrlInput);
    };
    img.src = imageUrlInput;
    closeImageDialog();
  }

  /**
   * Handle file input for image insertion
   */
  function handleImageFileSelect(event: Event) {
    const input = event.target as HTMLInputElement;
    if (!input.files || input.files.length === 0) return;

    const file = input.files[0];
    if (!file.type.startsWith('image/')) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      const dataUrl = e.target?.result as string;
      const img = new Image();
      img.onload = () => {
        insertLoadedImage(img, dataUrl);
      };
      img.src = dataUrl;
    };
    reader.readAsDataURL(file);
    closeImageDialog();
  }

  /**
   * Handle drag and drop for image insertion
   */
  function handleImageDrop(event: DragEvent) {
    event.preventDefault();
    dragOver = false;

    const files = event.dataTransfer?.files;
    if (!files || files.length === 0) return;

    const file = files[0];
    if (!file.type.startsWith('image/')) return;

    const reader = new FileReader();
    reader.onload = (e) => {
      const dataUrl = e.target?.result as string;
      const img = new Image();
      img.onload = () => {
        insertLoadedImage(img, dataUrl);
      };
      img.src = dataUrl;
    };
    reader.readAsDataURL(file);
    closeImageDialog();
  }

  /**
   * Insert a loaded image into the document
   */
  function insertLoadedImage(img: HTMLImageElement, src: string) {
    if (!engine) return;

    saveUndoState();

    const id = generateImageId();

    // Calculate max width based on column layout
    const contentWidth = PAGE_WIDTH - MARGIN_LEFT - MARGIN_RIGHT;
    const columnWidth = COLUMNS > 1
      ? (contentWidth - (COLUMNS - 1) * COLUMN_GAP) / COLUMNS
      : contentWidth;
    const maxWidth = columnWidth;

    // Calculate display size (max width within column, maintain aspect ratio)
    let width = img.naturalWidth;
    let height = img.naturalHeight;

    if (width > maxWidth) {
      height = (maxWidth / width) * height;
      width = maxWidth;
    }

    console.log(`Inserting image ${id}: ${width}x${height} (natural: ${img.naturalWidth}x${img.naturalHeight})`);

    // Add image to the engine
    engine.add_image(id, src, width, height, img.naturalWidth, img.naturalHeight);

    // Store the already-loaded image in cache for rendering
    // The img passed to this function is already loaded (via onload callback)
    loadedImages.set(id, img);

    // Split current paragraph at cursor position
    const currentText = engine.get_paragraph(cursorPara) || '';
    const beforeCursor = currentText.slice(0, cursorOffset);
    const afterCursor = currentText.slice(cursorOffset);

    // Update current paragraph with text before cursor
    engine.set_paragraph(cursorPara, beforeCursor);

    // Insert image paragraph after current paragraph
    engine.insert_image_paragraph(cursorPara + 1, id);

    // Insert a new paragraph with text after cursor (after the image)
    engine.insert_paragraph(cursorPara + 2, afterCursor);

    console.log(`Image paragraph inserted at index ${cursorPara + 1}, loadedImages has ${loadedImages.size} entries`);

    // Move cursor to the paragraph AFTER the image (not on the image itself)
    cursorPara += 2;
    cursorOffset = 0;

    recomputeAndRender();
  }

  /**
   * Get image info at a click position
   */
  function getImageAtPosition(pageIdx: number, x: number, y: number): { id: string; x: number; y: number; width: number; height: number } | null {
    if (!engine) return null;

    const displayLinesJson = engine.get_display_lines_json();
    try {
      const displayLines = JSON.parse(displayLinesJson);
      // Fields are now camelCase from Rust
      const linesOnPage = displayLines.filter((dl: { pageIndex: number }) => dl.pageIndex === pageIdx);

      for (const line of linesOnPage) {
        if (line.isImage && line.imageId) {
          const imageJson = engine.get_image(line.imageId);
          if (imageJson) {
            const image = JSON.parse(imageJson);
            const imgY = MARGIN_TOP + line.yPosition;
            const imgX = line.xPosition || MARGIN_LEFT;
            const imgWidth = Math.min(image.width, PAGE_WIDTH - MARGIN_LEFT - MARGIN_RIGHT);
            const imgHeight = image.height * ((100 - image.cropTop - image.cropBottom) / 100);

            // Check if click is within image bounds
            if (x >= imgX && x <= imgX + imgWidth && y >= imgY && y <= imgY + imgHeight) {
              return {
                id: line.imageId,
                x: imgX,
                y: imgY,
                width: imgWidth,
                height: imgHeight
              };
            }
          }
        }
      }
    } catch (e) {
      console.error('Failed to get image at position:', e);
    }
    return null;
  }

  /**
   * Select an image and show options popup
   */
  function selectImage(imageId: string, pageIdx: number, x: number, y: number) {
    selectedImageId = imageId;
    showImageOptions = true;
    // Position popup near the image
    const canvas = pageCanvases[pageIdx];
    if (canvas) {
      const rect = canvas.getBoundingClientRect();
      const zoomFactor = ZOOM / 100;
      imageOptionsPosition = {
        x: rect.left + x * zoomFactor,
        y: rect.top + y * zoomFactor + 10
      };
    }
  }

  /**
   * Deselect image and hide options
   */
  function deselectImage() {
    selectedImageId = null;
    showImageOptions = false;
    renderAllPages();
  }

  /**
   * Set wrap style for selected image
   */
  function setImageWrapStyle(wrapStyle: string) {
    if (!engine || !selectedImageId) return;
    saveUndoState();
    engine.set_image_wrap_style(selectedImageId, wrapStyle);
    recomputeAndRender();
  }

  /**
   * Set horizontal alignment for selected image
   */
  function setImageAlign(align: string) {
    if (!engine || !selectedImageId) return;
    saveUndoState();
    engine.set_image_horizontal_align(selectedImageId, align);
    recomputeAndRender();
  }

  /**
   * Clear image position (reset to move-with-text)
   */
  function resetImagePosition() {
    if (!engine || !selectedImageId) return;
    saveUndoState();
    engine.clear_image_position(selectedImageId);
    recomputeAndRender();
  }

  /**
   * Delete the selected image
   */
  function deleteSelectedImage() {
    if (!engine || !selectedImageId) return;
    saveUndoState();

    // Find and delete the paragraph containing this image
    const paraCount = engine.paragraph_count();
    for (let i = 0; i < paraCount; i++) {
      const text = engine.get_paragraph(i) || '';
      if (text.startsWith('\uFFFC') && text.slice(1) === selectedImageId) {
        engine.delete_paragraph(i);
        break;
      }
    }

    // Delete the image itself
    engine.delete_image(selectedImageId);
    loadedImages.delete(selectedImageId);

    deselectImage();
    recomputeAndRender();
  }

  /**
   * Handle image drag start
   */
  function startImageDrag(pageIdx: number, mouseX: number, mouseY: number, imgX: number, imgY: number) {
    isDraggingImage = true;
    dragStartPos = { x: mouseX, y: mouseY };
    dragImageStartPos = { x: imgX, y: imgY };
    dragImagePage = pageIdx;
  }

  /**
   * Handle image drag move
   */
  function handleImageDrag(pageIdx: number, mouseX: number, mouseY: number) {
    if (!isDraggingImage || !engine || !selectedImageId) return;

    const deltaX = mouseX - dragStartPos.x;
    const deltaY = mouseY - dragStartPos.y;

    const newX = dragImageStartPos.x + deltaX - MARGIN_LEFT;
    const newY = dragImageStartPos.y + deltaY - MARGIN_TOP;

    // Update image position (converts to fixed-position mode)
    engine.set_image_position(selectedImageId, newX, newY, pageIdx);
    recomputeAndRender();
  }

  /**
   * Handle image drag end
   */
  function endImageDrag() {
    if (isDraggingImage) {
      isDraggingImage = false;
    }
  }

  /**
   * Get bounds for the currently selected image
   */
  function getSelectedImageBounds(): { x: number; y: number; width: number; height: number; pageIndex: number } | null {
    if (!engine || !selectedImageId) return null;

    const displayLinesJson = engine.get_display_lines_json();
    const imageJson = engine.get_image(selectedImageId);
    if (!imageJson) return null;

    const image = JSON.parse(imageJson);
    const columnWidth = (PAGE_WIDTH - MARGIN_LEFT - MARGIN_RIGHT - (COLUMNS - 1) * COLUMN_GAP) / COLUMNS;
    const imgWidth = Math.min(image.width, columnWidth);
    const imgHeight = image.height * ((100 - (image.cropTop || 0) - (image.cropBottom || 0)) / 100);

    // Check if image has fixed position (after dragging)
    if (image.positionMode === 'fixed-position' && image.x != null && image.y != null && image.pageIndex != null) {
      // Use the fixed position directly (image.x/y are relative to margins, so add them back)
      return {
        x: MARGIN_LEFT + image.x,
        y: MARGIN_TOP + image.y,
        width: imgWidth,
        height: imgHeight,
        pageIndex: image.pageIndex
      };
    }

    // For move-with-text images, find the display line position
    try {
      const displayLines = JSON.parse(displayLinesJson);
      const imageLine = displayLines.find((dl: { isImage: boolean; imageId: string }) =>
        dl.isImage && dl.imageId === selectedImageId
      );

      if (!imageLine) return null;

      const imgY = MARGIN_TOP + imageLine.yPosition;
      const columnOffset = imageLine.columnIndex * (columnWidth + COLUMN_GAP);

      // Calculate X position based on horizontal alignment (matching render.rs calculate_image_x)
      let imgX: number;
      const horizontalAlign = image.horizontalAlign || 'left';
      switch (horizontalAlign) {
        case 'center':
          imgX = MARGIN_LEFT + columnOffset + (columnWidth - imgWidth) / 2;
          break;
        case 'right':
          imgX = MARGIN_LEFT + columnOffset + columnWidth - imgWidth;
          break;
        case 'left':
        default:
          imgX = MARGIN_LEFT + columnOffset;
          break;
      }

      return {
        x: imgX,
        y: imgY,
        width: imgWidth,
        height: imgHeight,
        pageIndex: imageLine.pageIndex
      };
    } catch (e) {
      console.error('Failed to get image bounds:', e);
      return null;
    }
  }

  /**
   * Check if a point is on a resize handle
   */
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

  /**
   * Start resizing an image
   */
  function startResize(handle: ResizeHandle, clientX: number, clientY: number) {
    if (!engine || !selectedImageId || !handle) return;

    const imageJson = engine.get_image(selectedImageId);
    if (!imageJson) return;

    const image = JSON.parse(imageJson);

    isResizing = true;
    resizeHandle = handle;
    resizeStartX = clientX;
    resizeStartY = clientY;
    resizeStartWidth = image.width;
    resizeStartHeight = image.height;
    showImageOptions = false;
  }

  /**
   * Handle resize move
   */
  function handleResizeMove(clientX: number, clientY: number) {
    if (!engine || !isResizing || !selectedImageId || !resizeHandle) return;

    const imageJson = engine.get_image(selectedImageId);
    if (!imageJson) return;

    const image = JSON.parse(imageJson);
    const aspectRatio = image.naturalWidth / image.naturalHeight;

    const zoomFactor = ZOOM / 100;
    const deltaX = (clientX - resizeStartX) / zoomFactor;
    const deltaY = (clientY - resizeStartY) / zoomFactor;

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
    const maxWidth = PAGE_WIDTH - MARGIN_LEFT - MARGIN_RIGHT;
    if (newWidth > maxWidth) {
      newWidth = maxWidth;
      newHeight = newWidth / aspectRatio;
    }

    engine.update_image_size(selectedImageId, newWidth, newHeight);
    recomputeAndRender();
  }

  /**
   * End resizing
   */
  function endResize() {
    isResizing = false;
    resizeHandle = null;
  }

  /**
   * Start cropping mode
   */
  function startCropMode() {
    if (!engine || !selectedImageId) return;

    const imageJson = engine.get_image(selectedImageId);
    if (!imageJson) return;

    const image = JSON.parse(imageJson);
    cropOriginalValues = {
      top: image.cropTop || 0,
      right: image.cropRight || 0,
      bottom: image.cropBottom || 0,
      left: image.cropLeft || 0,
    };

    isCropping = true;
    showImageOptions = false;
    renderAllPages();
  }

  /**
   * End cropping mode
   */
  function endCropMode() {
    isCropping = false;
    cropHandle = null;
    renderAllPages();
  }

  /**
   * Cancel crop and restore original values
   */
  function cancelCrop() {
    if (!engine || !selectedImageId) return;

    // Restore original crop values via engine
    // For now, just end cropping mode - full crop support would need API update
    endCropMode();
  }

  /**
   * Reset crop values to zero
   */
  function resetCrop() {
    if (!engine || !selectedImageId) return;
    saveUndoState();

    const imageJson = engine.get_image(selectedImageId);
    if (!imageJson) return;

    const image = JSON.parse(imageJson);
    // Reset all crop values to 0 by updating the image
    // We need to update crop through the engine - for now we can use the existing methods
    // This would require an add_image call with original dimensions
    // For now, just reset the display
    cropOriginalValues = { top: 0, right: 0, bottom: 0, left: 0 };
    recomputeAndRender();
  }

  /**
   * Get the selected image data
   */
  function getSelectedImageData(): { wrapStyle: string; horizontalAlign: string; positionMode: string } | null {
    if (!engine || !selectedImageId) return null;

    const imageJson = engine.get_image(selectedImageId);
    if (!imageJson) return null;

    try {
      const image = JSON.parse(imageJson);
      return {
        wrapStyle: image.wrapStyle || 'inline',
        horizontalAlign: image.horizontalAlign || 'left',
        positionMode: image.positionMode || 'move-with-text',
      };
    } catch (e) {
      return null;
    }
  }

  /**
   * Set the image position mode
   */
  function setImagePositionMode(mode: string) {
    if (!engine || !selectedImageId) return;
    saveUndoState();
    if (mode === 'move-with-text') {
      engine.clear_image_position(selectedImageId);
    }
    // Fixed position is handled by dragging
    recomputeAndRender();
  }

  /**
   * Close image options popup
   */
  function closeImageOptionsPopup() {
    showImageOptions = false;
  }

  /**
   * Start crop drag
   */
  function startCropDrag(handle: ResizeHandle, clientX: number, clientY: number) {
    if (!engine || !selectedImageId || !handle) return;

    const imageJson = engine.get_image(selectedImageId);
    if (!imageJson) return;

    const image = JSON.parse(imageJson);

    cropHandle = handle;
    cropStartX = clientX;
    cropStartY = clientY;
    cropStartValues = {
      top: image.cropTop || 0,
      right: image.cropRight || 0,
      bottom: image.cropBottom || 0,
      left: image.cropLeft || 0,
    };
  }

  /**
   * Handle crop move - Note: Full crop implementation would need engine support
   */
  function handleCropMove(_clientX: number, _clientY: number) {
    // Crop functionality would require updating the image crop values in the engine
    // This is a placeholder - the actual implementation would need engine API updates
    if (!isCropping || !selectedImageId || !cropHandle) return;
  }

  /**
   * End crop drag
   */
  function endCropDrag() {
    cropHandle = null;
  }

  /**
   * Draw resize handles on a selected image
   */
  function drawResizeHandles(ctx: CanvasRenderingContext2D, bounds: { x: number; y: number; width: number; height: number }) {
    const handleSize = 8;
    const { x, y, width, height } = bounds;

    ctx.fillStyle = isCropping ? '#ff9800' : '#1a73e8';
    ctx.strokeStyle = '#fff';
    ctx.lineWidth = 2;

    // Define handle positions
    const handles = [
      { x: x, y: y }, // nw
      { x: x + width, y: y }, // ne
      { x: x, y: y + height }, // sw
      { x: x + width, y: y + height }, // se
      { x: x + width / 2, y: y }, // n
      { x: x + width / 2, y: y + height }, // s
      { x: x, y: y + height / 2 }, // w
      { x: x + width, y: y + height / 2 }, // e
    ];

    for (const handle of handles) {
      ctx.fillRect(handle.x - handleSize / 2, handle.y - handleSize / 2, handleSize, handleSize);
      ctx.strokeRect(handle.x - handleSize / 2, handle.y - handleSize / 2, handleSize, handleSize);
    }

    // Draw selection border
    ctx.strokeStyle = isCropping ? '#ff9800' : '#1a73e8';
    ctx.lineWidth = 2;
    ctx.setLineDash([5, 5]);
    ctx.strokeRect(x, y, width, height);
    ctx.setLineDash([]);
  }

  /**
   * Insert a page break at the current cursor position
   */
  function insertPageBreak() {
    if (!engine) return;

    saveUndoState();

    const text = engine.get_paragraph(cursorPara) || '';
    const before = text.slice(0, cursorOffset);
    const after = text.slice(cursorOffset);

    // Set current paragraph to text before cursor
    engine.set_paragraph(cursorPara, before);

    // Insert page break paragraph
    engine.insert_page_break(cursorPara + 1);

    // Insert new paragraph with text after cursor
    engine.insert_paragraph(cursorPara + 2, after);

    // Move cursor to the new paragraph after page break
    cursorPara += 2;
    cursorOffset = 0;

    recomputeAndRender();
  }

  /**
   * Show table insertion dialog
   */
  function showTableInsertDialog() {
    showTableDialog = true;
    tableRows = 3;
    tableCols = 3;
    tableHasHeaderRow = false;
    tableHasHeaderCol = false;
  }

  /**
   * Close table insertion dialog
   */
  function closeTableDialog() {
    showTableDialog = false;
    hiddenTextarea?.focus();
  }

  /**
   * Insert a table at the current cursor position
   */
  function insertTable() {
    if (!engine) return;

    saveUndoState();

    // Create the table
    const tableId = engine.create_table(tableRows, tableCols);

    // Apply header styling if selected
    const headerBgColor = '#f3f3f3';
    if (tableHasHeaderRow) {
      for (let col = 0; col < tableCols; col++) {
        engine.set_cell_background(tableId, 0, col, headerBgColor);
      }
    }
    if (tableHasHeaderCol) {
      for (let row = 0; row < tableRows; row++) {
        engine.set_cell_background(tableId, row, 0, headerBgColor);
      }
    }

    // Split current paragraph at cursor
    const currentText = engine.get_paragraph(cursorPara) || '';
    const beforeCursor = currentText.slice(0, cursorOffset);
    const afterCursor = currentText.slice(cursorOffset);

    // Update current paragraph with text before cursor
    engine.set_paragraph(cursorPara, beforeCursor);

    // Insert table paragraph after current paragraph
    engine.insert_table_paragraph(cursorPara + 1, tableId);

    // Insert new paragraph with text after cursor
    engine.insert_paragraph(cursorPara + 2, afterCursor);

    // Move cursor to paragraph after the table
    cursorPara += 2;
    cursorOffset = 0;

    closeTableDialog();
    recomputeAndRender();
  }

  // =========================================================================
  // Table Cell Editing Functions
  // =========================================================================

  /**
   * Check if cursor is currently in a table paragraph
   */
  function isInTable(): boolean {
    if (!engine) return false;
    const paraText = engine.get_paragraph(cursorPara) || '';
    return paraText.startsWith(TABLE_MARKER);
  }

  /**
   * Get table ID from current paragraph if it's a table
   */
  function getCurrentTableId(): string | null {
    if (!engine) return null;
    const paraText = engine.get_paragraph(cursorPara) || '';
    if (paraText.startsWith(TABLE_MARKER)) {
      return paraText.slice(3); // Skip the 3-byte UTF-8 marker
    }
    return null;
  }

  /**
   * Get table cell at a given canvas position
   * Returns table ID, paragraph index, row and column if click is inside a table cell
   */
  function getTableCellAtPosition(pageIdx: number, x: number, y: number): { tableId: string; tablePara: number; row: number; col: number } | null {
    if (!engine) return null;

    try {
      const displayLinesJson = engine.get_display_lines_json();
      const displayLines = JSON.parse(displayLinesJson);

      // Find table display lines on this page
      const tableLinesOnPage = displayLines.filter(
        (dl: { pageIndex: number; isTable: boolean }) => dl.pageIndex === pageIdx && dl.isTable
      );

      for (const line of tableLinesOnPage) {
        if (!line.tableId || !line.tableLayout) continue;

        const tableX = line.xPosition || MARGIN_LEFT;
        const tableY = MARGIN_TOP + line.yPosition;
        const tableWidth = line.tableLayout.totalWidth || (PAGE_WIDTH - MARGIN_LEFT - MARGIN_RIGHT);
        const tableHeight = line.tableLayout.totalHeight;

        // Check if click is within table bounds
        if (x >= tableX && x <= tableX + tableWidth && y >= tableY && y <= tableY + tableHeight) {
          // Get cell at relative position within table
          const relX = x - tableX;
          const relY = y - tableY;

          const cellJson = engine.get_cell_at_position(line.tableId, relX, relY);
          if (cellJson) {
            try {
              const cell = JSON.parse(cellJson);
              return {
                tableId: line.tableId,
                tablePara: line.paraIndex,
                row: cell.row,
                col: cell.col
              };
            } catch {
              // Failed to parse cell
            }
          }
        }
      }
    } catch (e) {
      console.error('Failed to get table cell at position:', e);
    }

    return null;
  }

  /**
   * Handle context menu on canvas (right-click)
   */
  function handleCanvasContextMenu(event: MouseEvent, pageIdx: number) {
    if (!engine) return;

    const canvas = pageCanvases[pageIdx];
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    const zoomFactor = ZOOM / 100;
    const x = (event.clientX - rect.left) / zoomFactor;
    const y = (event.clientY - rect.top) / zoomFactor;

    // Check if right-clicking on a table cell
    const tableClick = getTableCellAtPosition(pageIdx, x, y);
    if (tableClick) {
      event.preventDefault();

      // If not already in this cell, enter it first
      if (activeTableId !== tableClick.tableId ||
          activeCell?.row !== tableClick.row ||
          activeCell?.col !== tableClick.col) {
        enterTableCell(tableClick.tableId, tableClick.tablePara, tableClick.row, tableClick.col);
      }

      // Show context menu at click position
      showTableMenu(event.clientX, event.clientY);
      return;
    }

    // If right-clicking outside table while editing, close context menu
    if (showTableContextMenu) {
      closeTableMenu();
    }
  }

  /**
   * Enter a table cell for editing
   */
  function enterTableCell(tableId: string, tablePara: number, row: number, col: number) {
    if (!engine) return;

    activeTableId = tableId;
    activeTablePara = tablePara;
    activeCell = { row, col };
    cellText = engine.get_cell_text(tableId, row, col) || '';
    cellCursorOffset = cellText.length;
    cellSelectionStart = null;
    cellSelectionEnd = null;

    // Clear regular cursor/selection
    selectedImageId = null;
    showImageOptions = false;
  }

  /**
   * Exit table cell editing mode
   */
  function exitTableCell() {
    if (activeTableId && activeCell) {
      // Save current cell text
      saveCellText();
    }
    activeTableId = null;
    activeTablePara = null;
    activeCell = null;
    cellText = '';
    cellCursorOffset = 0;
    cellSelectionStart = null;
    cellSelectionEnd = null;
    showTableContextMenu = false;
  }

  /**
   * Save current cell text to engine
   */
  function saveCellText() {
    if (!engine || !activeTableId || !activeCell) return;
    engine.set_cell_text(activeTableId, activeCell.row, activeCell.col, cellText);
  }

  /**
   * Get table dimensions
   */
  function getTableDimensions(tableId: string): { rows: number; cols: number } | null {
    if (!engine) return null;
    const json = engine.get_table_dimensions(tableId);
    if (!json) return null;
    try {
      return JSON.parse(json as string);
    } catch {
      return null;
    }
  }

  /**
   * Navigate to next cell (Tab key behavior)
   * Returns true if navigation succeeded, false if at last cell
   */
  function goToNextCell(): boolean {
    if (!engine || !activeTableId || !activeCell) return false;

    const dims = getTableDimensions(activeTableId);
    if (!dims) return false;

    saveCellText();

    const { row, col } = activeCell;

    // Move to next column
    if (col < dims.cols - 1) {
      enterTableCell(activeTableId, activeTablePara!, row, col + 1);
      return true;
    }

    // Move to first column of next row
    if (row < dims.rows - 1) {
      enterTableCell(activeTableId, activeTablePara!, row + 1, 0);
      return true;
    }

    // At last cell
    return false;
  }

  /**
   * Navigate to previous cell (Shift+Tab key behavior)
   */
  function goToPreviousCell(): boolean {
    if (!engine || !activeTableId || !activeCell) return false;

    const dims = getTableDimensions(activeTableId);
    if (!dims) return false;

    saveCellText();

    const { row, col } = activeCell;

    // Move to previous column
    if (col > 0) {
      enterTableCell(activeTableId, activeTablePara!, row, col - 1);
      return true;
    }

    // Move to last column of previous row
    if (row > 0) {
      enterTableCell(activeTableId, activeTablePara!, row - 1, dims.cols - 1);
      return true;
    }

    // At first cell
    return false;
  }

  /**
   * Navigate to cell in direction (arrow key behavior)
   */
  function navigateCellDirection(direction: 'up' | 'down' | 'left' | 'right'): 'moved' | 'exit' | 'blocked' {
    if (!engine || !activeTableId || !activeCell) return 'blocked';

    const dims = getTableDimensions(activeTableId);
    if (!dims) return 'blocked';

    saveCellText();

    const { row, col } = activeCell;
    let newRow = row;
    let newCol = col;

    switch (direction) {
      case 'up':
        if (row > 0) newRow = row - 1;
        else return 'exit'; // Exit table upward
        break;
      case 'down':
        if (row < dims.rows - 1) newRow = row + 1;
        else return 'exit'; // Exit table downward
        break;
      case 'left':
        if (col > 0) newCol = col - 1;
        else if (row > 0) { newRow = row - 1; newCol = dims.cols - 1; }
        else return 'exit'; // Exit table to the left/up
        break;
      case 'right':
        if (col < dims.cols - 1) newCol = col + 1;
        else if (row < dims.rows - 1) { newRow = row + 1; newCol = 0; }
        else return 'exit'; // Exit table to the right/down
        break;
    }

    enterTableCell(activeTableId, activeTablePara!, newRow, newCol);
    return 'moved';
  }

  /**
   * Add a new row and navigate to it (when Tab at last cell)
   */
  function addRowAndNavigate() {
    if (!engine || !activeTableId || !activeCell) return;

    saveUndoState();
    saveCellText();

    const dims = getTableDimensions(activeTableId);
    if (!dims) return;

    // Add row at end
    engine.add_table_row(activeTableId, dims.rows);

    // Navigate to first cell of new row
    enterTableCell(activeTableId, activeTablePara!, dims.rows, 0);

    recomputeAndRender();
  }

  /**
   * Handle Tab key in table
   */
  function handleTableTab(isShift: boolean): boolean {
    if (!activeTableId || !activeCell) return false;

    if (isShift) {
      return goToPreviousCell();
    } else {
      const moved = goToNextCell();
      if (!moved) {
        // At last cell, add new row
        addRowAndNavigate();
        return true;
      }
      return moved;
    }
  }

  /**
   * Handle Enter key in table cell (insert line break)
   */
  function handleTableEnter(): boolean {
    if (!activeTableId || !activeCell) return false;

    // Insert newline at cursor position
    const before = cellText.slice(0, cellCursorOffset);
    const after = cellText.slice(cellCursorOffset);
    cellText = before + '\n' + after;
    cellCursorOffset++;

    return true;
  }

  /**
   * Handle character input in table cell
   */
  function handleTableCharInput(char: string) {
    if (!activeTableId || !activeCell) return;

    const before = cellText.slice(0, cellCursorOffset);
    const after = cellText.slice(cellCursorOffset);
    cellText = before + char + after;
    cellCursorOffset++;
  }

  /**
   * Handle Backspace in table cell
   */
  function handleTableBackspace(): boolean {
    if (!activeTableId || !activeCell) return false;

    if (cellCursorOffset > 0) {
      const before = cellText.slice(0, cellCursorOffset - 1);
      const after = cellText.slice(cellCursorOffset);
      cellText = before + after;
      cellCursorOffset--;
      return true;
    }
    return false;
  }

  /**
   * Handle Delete in table cell
   */
  function handleTableDelete(): boolean {
    if (!activeTableId || !activeCell) return false;

    if (cellCursorOffset < cellText.length) {
      const before = cellText.slice(0, cellCursorOffset);
      const after = cellText.slice(cellCursorOffset + 1);
      cellText = before + after;
      return true;
    }
    return false;
  }

  /**
   * Handle arrow keys within table cell
   */
  function handleTableArrowKey(key: string): boolean {
    if (!engine || !activeTableId || !activeCell || activeTablePara === null) return false;

    if (key === 'ArrowLeft') {
      if (cellCursorOffset > 0) {
        cellCursorOffset--;
        return true;
      }
      // At start of cell, try to move to previous cell
      const result = navigateCellDirection('left');
      if (result === 'exit') {
        // Exit table to paragraph above
        exitTableAndMoveTo(activeTablePara - 1, 'end');
      }
      return result !== 'blocked';
    } else if (key === 'ArrowRight') {
      if (cellCursorOffset < cellText.length) {
        cellCursorOffset++;
        return true;
      }
      // At end of cell, try to move to next cell
      const result = navigateCellDirection('right');
      if (result === 'exit') {
        // Exit table to paragraph below
        exitTableAndMoveTo(activeTablePara + 1, 'start');
      }
      return result !== 'blocked';
    } else if (key === 'ArrowUp') {
      // Check if we're on the first line of the cell
      const textBeforeCursor = cellText.slice(0, cellCursorOffset);
      const lastNewline = textBeforeCursor.lastIndexOf('\n');
      if (lastNewline === -1) {
        // On first line, move to cell above
        const result = navigateCellDirection('up');
        if (result === 'exit') {
          // Exit table to paragraph above
          exitTableAndMoveTo(activeTablePara - 1, 'end');
        }
        return result !== 'blocked';
      }
      // Move cursor up within cell (simplified: go to previous line)
      const lineStart = lastNewline + 1;
      const posInLine = cellCursorOffset - lineStart;
      const prevLineEnd = lastNewline;
      const prevLineStart = textBeforeCursor.lastIndexOf('\n', prevLineEnd - 1) + 1;
      const prevLineLen = prevLineEnd - prevLineStart;
      cellCursorOffset = prevLineStart + Math.min(posInLine, prevLineLen);
      return true;
    } else if (key === 'ArrowDown') {
      // Check if we're on the last line of the cell
      const textAfterCursor = cellText.slice(cellCursorOffset);
      const nextNewline = textAfterCursor.indexOf('\n');
      if (nextNewline === -1) {
        // On last line, move to cell below
        const result = navigateCellDirection('down');
        if (result === 'exit') {
          // Exit table to paragraph below
          exitTableAndMoveTo(activeTablePara + 1, 'start');
        }
        return result !== 'blocked';
      }
      // Move cursor down within cell
      const textBeforeCursor = cellText.slice(0, cellCursorOffset);
      const currentLineStart = textBeforeCursor.lastIndexOf('\n') + 1;
      const posInLine = cellCursorOffset - currentLineStart;
      const nextLineStart = cellCursorOffset + nextNewline + 1;
      const nextLineEnd = cellText.indexOf('\n', nextLineStart);
      const nextLineLen = (nextLineEnd === -1 ? cellText.length : nextLineEnd) - nextLineStart;
      cellCursorOffset = nextLineStart + Math.min(posInLine, nextLineLen);
      return true;
    }
    return false;
  }

  /**
   * Exit table and move cursor to specified paragraph
   * Creates a new paragraph if needed to avoid cursor landing on the table
   */
  function exitTableAndMoveTo(paraIndex: number, position: 'start' | 'end') {
    if (!engine) return;

    // Save table paragraph index before exiting
    const tablePara = activeTablePara;

    saveCellText();
    exitTableCell();

    const paraCount = engine.paragraph_count();

    // Helper to check if a paragraph is a normal text paragraph (not special)
    const isNormalParagraph = (idx: number): boolean => {
      if (idx < 0 || idx >= engine!.paragraph_count()) return false;
      const text = engine!.get_paragraph(idx) || '';
      return !isSpecialParagraph(text);
    };

    // Helper to create a new paragraph after the table
    const createParagraphAfterTable = () => {
      if (tablePara === null) return;
      engine!.insert_paragraph(tablePara + 1, '');
      cursorPara = tablePara + 1;
      cursorOffset = 0;
    };

    // Helper to create a new paragraph before the table
    const createParagraphBeforeTable = () => {
      if (tablePara === null) return;
      engine!.insert_paragraph(tablePara, '');
      cursorPara = tablePara; // New paragraph is now at tablePara
      cursorOffset = 0;
    };

    // Handle edge cases
    if (paraIndex < 0) {
      // Before first paragraph - check if we can go there or need to create one
      const firstText = engine.get_paragraph(0) || '';
      if (isSpecialParagraph(firstText)) {
        // First paragraph is special (like a table), create paragraph before it
        createParagraphBeforeTable();
      } else {
        cursorPara = 0;
        cursorOffset = 0;
      }
    } else if (paraIndex >= paraCount) {
      // After last paragraph - check if last paragraph is the table itself
      const lastPara = paraCount - 1;
      const lastText = engine.get_paragraph(lastPara) || '';
      if (isSpecialParagraph(lastText)) {
        // Last paragraph is special (the table we just exited), create new one after
        createParagraphAfterTable();
      } else {
        cursorPara = lastPara;
        cursorOffset = lastText.length;
      }
    } else {
      const text = engine.get_paragraph(paraIndex) || '';
      // Skip special paragraphs (images, page breaks, tables)
      if (isSpecialParagraph(text)) {
        // Check if it's a table - if so, enter it
        if (text.startsWith(TABLE_MARKER)) {
          const tableId = text.substring(1);
          if (position === 'start') {
            enterTableCell(tableId, paraIndex, 0, 0);
          } else {
            const dims = engine.get_table_dimensions(tableId);
            if (dims) {
              const { rows } = JSON.parse(dims);
              enterTableCell(tableId, paraIndex, rows - 1, 0);
            } else {
              enterTableCell(tableId, paraIndex, 0, 0);
            }
          }
          return;
        }
        // For other special paragraphs, try next/previous or create new paragraph
        if (position === 'start') {
          // Moving down - look for next normal paragraph or create one
          let found = false;
          for (let i = paraIndex + 1; i < paraCount; i++) {
            if (isNormalParagraph(i)) {
              cursorPara = i;
              cursorOffset = 0;
              found = true;
              break;
            }
          }
          if (!found) {
            // No normal paragraph found after, create one after the table
            createParagraphAfterTable();
          }
        } else {
          // Moving up - look for previous normal paragraph or create one
          let found = false;
          for (let i = paraIndex - 1; i >= 0; i--) {
            if (isNormalParagraph(i)) {
              const prevText = engine.get_paragraph(i) || '';
              cursorPara = i;
              cursorOffset = prevText.length;
              found = true;
              break;
            }
          }
          if (!found) {
            // No normal paragraph found before, create one before the table
            createParagraphBeforeTable();
          }
        }
      } else {
        cursorPara = paraIndex;
        cursorOffset = position === 'start' ? 0 : text.length;
      }
    }
  }

  // =========================================================================
  // Table Context Menu Functions
  // =========================================================================

  /**
   * Show table context menu
   */
  function showTableMenu(x: number, y: number) {
    tableContextMenuPos = { x, y };
    showTableContextMenu = true;
  }

  /**
   * Close table context menu
   */
  function closeTableMenu() {
    showTableContextMenu = false;
  }

  /**
   * Add row above current cell
   */
  function addRowAbove() {
    if (!engine || !activeTableId || !activeCell) return;
    saveUndoState();
    saveCellText();
    engine.add_table_row(activeTableId, activeCell.row);
    activeCell = { row: activeCell.row + 1, col: activeCell.col };
    closeTableMenu();
    recomputeAndRender();
  }

  /**
   * Add row below current cell
   */
  function addRowBelow() {
    if (!engine || !activeTableId || !activeCell) return;
    saveUndoState();
    saveCellText();
    engine.add_table_row(activeTableId, activeCell.row + 1);
    closeTableMenu();
    recomputeAndRender();
  }

  /**
   * Add column to the left of current cell
   */
  function addColumnLeft() {
    if (!engine || !activeTableId || !activeCell) return;
    saveUndoState();
    saveCellText();
    engine.add_table_column(activeTableId, activeCell.col);
    activeCell = { row: activeCell.row, col: activeCell.col + 1 };
    closeTableMenu();
    recomputeAndRender();
  }

  /**
   * Add column to the right of current cell
   */
  function addColumnRight() {
    if (!engine || !activeTableId || !activeCell) return;
    saveUndoState();
    saveCellText();
    engine.add_table_column(activeTableId, activeCell.col + 1);
    closeTableMenu();
    recomputeAndRender();
  }

  /**
   * Delete current row
   */
  function deleteCurrentRow() {
    if (!engine || !activeTableId || !activeCell) return;
    saveUndoState();
    const dims = getTableDimensions(activeTableId);
    if (!dims || dims.rows <= 1) return; // Don't delete last row

    const deleted = engine.delete_table_row(activeTableId, activeCell.row);
    if (deleted) {
      // Adjust active cell if needed
      if (activeCell.row >= dims.rows - 1) {
        activeCell = { row: dims.rows - 2, col: activeCell.col };
      }
      cellText = engine.get_cell_text(activeTableId, activeCell.row, activeCell.col) || '';
      cellCursorOffset = cellText.length;
    }
    closeTableMenu();
    recomputeAndRender();
  }

  /**
   * Delete current column
   */
  function deleteCurrentColumn() {
    if (!engine || !activeTableId || !activeCell) return;
    saveUndoState();
    const dims = getTableDimensions(activeTableId);
    if (!dims || dims.cols <= 1) return; // Don't delete last column

    const deleted = engine.delete_table_column(activeTableId, activeCell.col);
    if (deleted) {
      // Adjust active cell if needed
      if (activeCell.col >= dims.cols - 1) {
        activeCell = { row: activeCell.row, col: dims.cols - 2 };
      }
      cellText = engine.get_cell_text(activeTableId, activeCell.row, activeCell.col) || '';
      cellCursorOffset = cellText.length;
    }
    closeTableMenu();
    recomputeAndRender();
  }

  /**
   * Delete entire table
   */
  function deleteCurrentTable() {
    if (!engine || !activeTableId || activeTablePara === null) return;
    saveUndoState();
    engine.delete_table(activeTableId);
    exitTableCell();
    closeTableMenu();
    recomputeAndRender();
  }

  /**
   * Handle format commands from the toolbar
   */
  function handleFormat(command: string, value?: string) {
    if (!engine) return;

    switch (command) {
      // Undo/Redo
      case 'undo':
        undo();
        break;
      case 'redo':
        redo();
        break;

      // Block formatting
      case 'formatBlock':
        if (value) {
          setBlockType(value);
        }
        break;

      // Alignment
      case 'justifyLeft':
        setAlignment('left');
        break;
      case 'justifyCenter':
        setAlignment('center');
        break;
      case 'justifyRight':
        setAlignment('right');
        break;
      case 'justifyFull':
        setAlignment('justify');
        break;

      // Lists
      case 'insertUnorderedList':
        toggleList('bullet');
        break;
      case 'insertOrderedList':
        toggleList('numbered');
        break;

      // Text styling
      case 'bold':
        applyInlineStyle('bold');
        break;
      case 'italic':
        applyInlineStyle('italic');
        break;
      case 'underline':
        applyInlineStyle('underline');
        break;
      case 'strikeThrough':
        applyInlineStyle('strikethrough');
        break;

      // Colors
      case 'foreColor':
        if (value) {
          applyTextColor(value);
        }
        break;
      case 'hiliteColor':
        if (value) {
          applyHighlightColor(value);
        }
        break;

      // Images
      case 'insertImage':
        showImageInsertDialog();
        break;

      // Tables
      case 'insertTable':
        showTableInsertDialog();
        break;

      default:
        console.log(`Unknown format command: ${command}`);
    }

    // Refocus the hidden textarea after toolbar action
    hiddenTextarea?.focus();
  }

  /**
   * Handle save action from toolbar
   */
  function handleSave() {
    if (!engine) return;

    const docJson = engine.save_document();
    const blob = new Blob([docJson], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'document.json';
    a.click();
    URL.revokeObjectURL(url);
  }

  /**
   * Apply page configuration from store to WASM engine
   */
  function applyPageConfig() {
    if (!engine) return;

    engine.set_page_config(
      PAGE_WIDTH,
      PAGE_HEIGHT,
      MARGIN_TOP,
      MARGIN_RIGHT,
      MARGIN_BOTTOM,
      MARGIN_LEFT,
      COLUMNS,
      COLUMN_GAP
    );

    engine.set_font_config(
      FONT_SIZE,
      LINE_HEIGHT,
      LETTER_SPACING,
      PARAGRAPH_SPACING
    );

    recomputeAndRender();
  }

  /**
   * Start cursor blinking
   */
  function startCursorBlink() {
    stopCursorBlink();
    cursorVisible = true;
    cursorBlinkInterval = setInterval(() => {
      cursorVisible = !cursorVisible;
      renderAllPages();
    }, 530); // Standard cursor blink rate
  }

  /**
   * Stop cursor blinking and show cursor
   */
  function stopCursorBlink() {
    if (cursorBlinkInterval) {
      clearInterval(cursorBlinkInterval);
      cursorBlinkInterval = null;
    }
    cursorVisible = true;
  }

  /**
   * Reset cursor blink and render immediately (call after typing/navigation/click)
   * This ensures the cursor is visible immediately, then starts blinking
   */
  function resetCursorBlink() {
    // Stop any existing blink interval
    if (cursorBlinkInterval) {
      clearInterval(cursorBlinkInterval);
      cursorBlinkInterval = null;
    }
    // Make cursor visible immediately and render right away
    cursorVisible = true;
    renderAllPages(); // Immediate render with cursor visible
    // Start new blink interval if focused
    if (isFocused) {
      cursorBlinkInterval = setInterval(() => {
        cursorVisible = !cursorVisible;
        renderAllPages();
      }, 530);
    }
  }

  // Store subscriptions
  let unsubscribers: (() => void)[] = [];

  onMount(async () => {
    try {
      // Initialize WASM module
      await init();
      engine = new Engine();

      // Subscribe to page config store
      const unsubPageConfig = pageConfig.subscribe(config => {
        const dims = getPageDimensions(config);
        PAGE_WIDTH = dims.width;
        PAGE_HEIGHT = dims.height;
        MARGIN_TOP = mmToPixels(config.margins.top);
        MARGIN_RIGHT = mmToPixels(config.margins.right);
        MARGIN_BOTTOM = mmToPixels(config.margins.bottom);
        MARGIN_LEFT = mmToPixels(config.margins.left);
        COLUMNS = config.columns;
        COLUMN_GAP = mmToPixels(config.columnGap);
        if (isReady) {
          applyPageConfig();
        }
      });
      unsubscribers.push(unsubPageConfig);

      // Subscribe to typography stores
      const unsubFontSize = fontSize.subscribe(value => {
        FONT_SIZE = value;
        if (isReady) applyPageConfig();
      });
      unsubscribers.push(unsubFontSize);

      const unsubLineHeight = lineHeight.subscribe(value => {
        LINE_HEIGHT = value;
        if (isReady) applyPageConfig();
      });
      unsubscribers.push(unsubLineHeight);

      const unsubLetterSpacing = letterSpacing.subscribe(value => {
        LETTER_SPACING = value;
        if (isReady) applyPageConfig();
      });
      unsubscribers.push(unsubLetterSpacing);

      const unsubParagraphSpacing = paragraphSpacing.subscribe(value => {
        PARAGRAPH_SPACING = value;
        if (isReady) applyPageConfig();
      });
      unsubscribers.push(unsubParagraphSpacing);

      const unsubFontFamily = fontFamily.subscribe(value => {
        FONT_FAMILY = value;
        if (isReady) applyPageConfig();
      });
      unsubscribers.push(unsubFontFamily);

      const unsubZoomLevel = zoomLevel.subscribe(value => {
        ZOOM = value;
        if (isReady) renderAllPages();
      });
      unsubscribers.push(unsubZoomLevel);

      // Configure the engine with initial values
      applyPageConfig();

      // Start with an empty document (single empty paragraph)
      engine.set_paragraph(0, '');

      isReady = true;
      recomputeAndRender();
      hiddenTextarea?.focus();
    } catch (e) {
      error = `Failed to initialize WASM engine: ${e}`;
      console.error(e);
    }
  });

  onDestroy(() => {
    stopCursorBlink();
    unsubscribers.forEach(unsub => unsub());
  });

  function createMeasureFunction(): (text: string, fontSize: number) => number {
    const ctx = measureCanvas?.getContext('2d');
    if (!ctx) {
      return (text, fontSize) => text.length * fontSize * 0.5;
    }

    return (text: string, fontSize: number): number => {
      ctx.font = `${fontSize}px ${FONT_FAMILY}`;
      return ctx.measureText(text).width;
    };
  }

  async function recomputeAndRender() {
    if (!engine) return;

    // Recompute layout
    const measureFn = createMeasureFunction();
    engine.recompute_layout(measureFn);

    // Update page count
    const newPageCount = engine.page_count();

    // Update current page based on cursor position
    currentPage = engine.get_page_for_position(cursorPara, cursorOffset);

    if (newPageCount !== pageCount) {
      console.log(`Page count changed: ${pageCount} -> ${newPageCount}`);
      // Page count changed - update and wait for DOM to create new canvases
      pageCount = newPageCount;
      // Use tick() to wait for Svelte to update the DOM with new canvases
      await tick();
      console.log(`After tick(), pageCanvases has ${pageCanvases.length} entries`);
    }

    // Render all pages
    renderAllPages();
    scrollCursorIntoView();
  }

  function renderAllPages() {
    if (!engine) return;

    // Render each page
    for (let pageIdx = 0; pageIdx < pageCount; pageIdx++) {
      renderPage(pageIdx);
    }
  }

  function renderPage(pageIdx: number) {
    if (!engine) return;

    const canvas = pageCanvases[pageIdx];
    if (!canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Apply zoom to displayed size
    const zoomFactor = ZOOM / 100;
    const displayWidth = PAGE_WIDTH * zoomFactor;
    const displayHeight = PAGE_HEIGHT * zoomFactor;

    // Set up canvas with device pixel ratio AND zoom for crisp rendering
    const dpr = window.devicePixelRatio || 1;
    const scaleFactor = dpr * zoomFactor;
    canvas.width = PAGE_WIDTH * scaleFactor;
    canvas.height = PAGE_HEIGHT * scaleFactor;
    canvas.style.width = `${displayWidth}px`;
    canvas.style.height = `${displayHeight}px`;
    ctx.scale(scaleFactor, scaleFactor);

    // Clear canvas
    ctx.fillStyle = 'white';
    ctx.fillRect(0, 0, PAGE_WIDTH, PAGE_HEIGHT);

    // Get render commands from engine for this page
    const commandsJson = engine.get_render_commands(pageIdx);
    const commands = parseRenderCommands(commandsJson);

    // Debug: log image commands
    const imageCommands = commands.filter(c => c.type === 'drawImage');
    if (imageCommands.length > 0) {
      console.log(`Page ${pageIdx} render commands include ${imageCommands.length} image(s):`, JSON.stringify(imageCommands, null, 2));
      console.log(`loadedImages cache keys:`, Array.from(loadedImages.keys()));
      // Check if image exists in cache
      for (const cmd of imageCommands) {
        const imgCmd = cmd as { imageId?: string; image_id?: string };
        const id = imgCmd.imageId || imgCmd.image_id;
        console.log(`Looking for image with id: "${id}", found: ${loadedImages.has(id || '')}`);
      }
    }

    // Draw selection first (behind text) - only for pages that have selection
    drawSelectionOnPage(ctx, pageIdx);

    // Execute render commands
    ctx.textBaseline = 'top';
    executeRenderCommands(ctx, commands, loadedImages, FONT_FAMILY);

    // Draw cursor only on the current page
    if (pageIdx === currentPage) {
      drawCursor(ctx);
    }

    // Draw cell cursor when editing a table cell (on appropriate page)
    if (activeTableId && activeCell) {
      drawCellCursor(ctx, pageIdx);
    }

    // Draw resize handles if an image is selected on this page
    if (selectedImageId) {
      const bounds = getSelectedImageBounds();
      if (bounds && bounds.pageIndex === pageIdx) {
        drawResizeHandles(ctx, bounds);
      }
    }
  }

  function render() {
    // Re-render all pages (simpler but less efficient - can optimize later)
    renderAllPages();
  }

  // Track the last page we scrolled to, to avoid unnecessary scrolling
  let lastScrolledToPage = -1;

  function scrollCursorIntoView(forceScroll = false) {
    if (!pagesContainer) return;

    // Only scroll if page changed or forced
    if (!forceScroll && currentPage === lastScrolledToPage) {
      return;
    }

    // Calculate the Y position of the cursor page (accounting for zoom)
    const zoomFactor = ZOOM / 100;
    const displayHeight = PAGE_HEIGHT * zoomFactor;
    const pageTop = currentPage * (displayHeight + PAGE_GAP);
    const pageBottom = pageTop + displayHeight;

    // Get container scroll position and visible area
    const containerTop = pagesContainer.scrollTop;
    const containerBottom = containerTop + pagesContainer.clientHeight;

    // Check if cursor page is visible
    if (pageTop < containerTop) {
      // Page is above visible area, scroll up
      pagesContainer.scrollTo({
        top: Math.max(0, pageTop - PAGE_GAP),
        behavior: 'smooth'
      });
      lastScrolledToPage = currentPage;
    } else if (pageBottom > containerBottom) {
      // Page is below visible area, scroll down
      pagesContainer.scrollTo({
        top: pageBottom - pagesContainer.clientHeight + PAGE_GAP,
        behavior: 'smooth'
      });
      lastScrolledToPage = currentPage;
    } else {
      // Page is visible, just update tracker
      lastScrolledToPage = currentPage;
    }
  }

  function drawSelectionOnPage(ctx: CanvasRenderingContext2D, pageIdx: number) {
    if (!engine || !selectionStart || !selectionEnd) return;

    // Normalize selection direction
    let startPos = selectionStart;
    let endPos = selectionEnd;
    if (startPos.para > endPos.para ||
        (startPos.para === endPos.para && startPos.offset > endPos.offset)) {
      [startPos, endPos] = [endPos, startPos];
    }

    const lineHeight = FONT_SIZE * LINE_HEIGHT;
    ctx.fillStyle = '#b4d7ff';
    ctx.font = `${FONT_SIZE}px ${FONT_FAMILY}`;

    // Get display positions for start and end
    const startDisplayJson = engine.para_to_display_pos(startPos.para, startPos.offset);
    const endDisplayJson = engine.para_to_display_pos(endPos.para, endPos.offset);
    if (!startDisplayJson || !endDisplayJson) return;

    try {
      const startDisplay = JSON.parse(startDisplayJson);
      const endDisplay = JSON.parse(endDisplayJson);

      // Draw selection for each line on this page
      for (let lineIdx = startDisplay.line; lineIdx <= endDisplay.line; lineIdx++) {
        const lineJson = engine.display_to_para(lineIdx, 0);
        if (!lineJson) continue;

        const lineInfo = JSON.parse(lineJson);

        // Get line Y position and page
        const linePosJson = engine.para_to_display_pos(lineInfo.para, lineInfo.offset);
        if (!linePosJson) continue;
        const linePos = JSON.parse(linePosJson);

        // Skip lines not on this page
        if (linePos.page !== pageIdx) continue;

        const paraText = engine.get_paragraph(lineInfo.para) || '';

        let selStartOffset = 0;
        let selEndOffset = paraText.length;

        // Adjust for first line of selection
        if (lineIdx === startDisplay.line) {
          selStartOffset = startPos.offset;
        }

        // Adjust for last line of selection
        if (lineIdx === endDisplay.line) {
          selEndOffset = endPos.offset;
        }

        // Calculate X positions
        const startX = MARGIN_LEFT + ctx.measureText(paraText.substring(0, selStartOffset)).width;
        const endX = MARGIN_LEFT + ctx.measureText(paraText.substring(0, selEndOffset)).width;
        const y = MARGIN_TOP + linePos.y;

        ctx.fillRect(startX, y, endX - startX, lineHeight);
      }
    } catch (e) {
      console.error('Failed to draw selection:', e);
    }
  }

  function drawCellCursor(ctx: CanvasRenderingContext2D, pageIdx: number) {
    if (!engine || !cursorVisible || !isFocused) return;
    if (!activeTableId || !activeCell) return;

    try {
      const displayLinesJson = engine.get_display_lines_json();
      const displayLines = JSON.parse(displayLinesJson);

      // Find the display line for this table on this page
      const tableLine = displayLines.find(
        (dl: { pageIndex: number; isTable: boolean; tableId?: string }) =>
          dl.pageIndex === pageIdx && dl.isTable && dl.tableId === activeTableId
      );

      if (!tableLine || !tableLine.tableLayout) return;

      const layout = tableLine.tableLayout;
      const { row, col } = activeCell;

      // Calculate cell position
      const tableX = tableLine.xPosition || MARGIN_LEFT;
      const tableY = MARGIN_TOP + tableLine.yPosition;
      const borderWidth = 1; // Default border width
      const cellPadding = 4;

      // Calculate column X position
      let cellX = tableX + borderWidth;
      for (let c = 0; c < col; c++) {
        cellX += (layout.columnWidths[c] || 0) + borderWidth;
      }

      // Calculate row Y position
      let cellY = tableY + borderWidth;
      for (let r = 0; r < row; r++) {
        cellY += (layout.rowHeights[r] || 0) + borderWidth;
      }

      // Get text up to cursor position
      const textBeforeCursor = cellText.substring(0, cellCursorOffset);

      // Handle multi-line text in cell
      const lines = textBeforeCursor.split('\n');
      const currentLineIndex = lines.length - 1;
      const currentLineText = lines[currentLineIndex];

      ctx.font = `${FONT_SIZE}px ${FONT_FAMILY}`;
      const textWidth = ctx.measureText(currentLineText).width;

      const lineHeightPx = FONT_SIZE * LINE_HEIGHT;
      const cursorX = cellX + cellPadding + textWidth;
      const cursorY = cellY + cellPadding + (currentLineIndex * lineHeightPx);

      ctx.fillStyle = '#000';
      ctx.fillRect(cursorX, cursorY + 2, 2, lineHeightPx - 4);
    } catch (e) {
      console.error('Failed to draw cell cursor:', e);
    }
  }

  function drawCursor(ctx: CanvasRenderingContext2D) {
    if (!engine || !cursorVisible || !isFocused) return;

    // If editing a table cell, don't draw the regular cursor
    if (activeTableId && activeCell) return;

    const posJson = engine.para_to_display_pos(cursorPara, cursorOffset);
    if (!posJson) return;

    try {
      const pos = JSON.parse(posJson);
      // Cursor is drawn on the correct page (caller checks currentPage)

      // Get the display lines to find the line for this cursor position
      const displayLinesJson = engine.get_display_lines_json();
      const displayLines = JSON.parse(displayLinesJson);
      const line = displayLines[pos.line];

      if (!line) return;

      // Measure text width from line start to cursor offset
      const paraText = engine.get_paragraph(cursorPara) || '';
      const textBefore = paraText.substring(line.startOffset, cursorOffset);

      ctx.font = `${FONT_SIZE}px ${FONT_FAMILY}`;
      const textWidth = ctx.measureText(textBefore).width;

      const lineHeightPx = FONT_SIZE * LINE_HEIGHT;
      // pos.x already includes margin_left + column offset
      const cursorX = pos.x + textWidth;
      const cursorY = MARGIN_TOP + pos.y;

      ctx.fillStyle = '#000';
      ctx.fillRect(cursorX, cursorY + 2, 2, lineHeightPx - 4);
    } catch (e) {
      console.error('Failed to parse cursor position:', e);
    }
  }

  function hasSelection(): boolean {
    return selectionStart !== null && selectionEnd !== null &&
      (selectionStart.para !== selectionEnd.para || selectionStart.offset !== selectionEnd.offset);
  }

  function clearSelection() {
    selectionStart = null;
    selectionEnd = null;
  }

  function getSelectedText(): string {
    if (!engine || !hasSelection()) return '';

    let startPos = selectionStart!;
    let endPos = selectionEnd!;
    if (startPos.para > endPos.para ||
        (startPos.para === endPos.para && startPos.offset > endPos.offset)) {
      [startPos, endPos] = [endPos, startPos];
    }

    let text = '';
    for (let para = startPos.para; para <= endPos.para; para++) {
      const paraText = engine.get_paragraph(para) || '';
      const start = para === startPos.para ? startPos.offset : 0;
      const end = para === endPos.para ? endPos.offset : paraText.length;
      text += paraText.substring(start, end);
      if (para < endPos.para) text += '\n';
    }
    return text;
  }

  function deleteSelection() {
    if (!engine || !hasSelection()) return;

    let startPos = selectionStart!;
    let endPos = selectionEnd!;
    if (startPos.para > endPos.para ||
        (startPos.para === endPos.para && startPos.offset > endPos.offset)) {
      [startPos, endPos] = [endPos, startPos];
    }

    if (startPos.para === endPos.para) {
      // Same paragraph - just remove the text
      const text = engine.get_paragraph(startPos.para) || '';
      const newText = text.substring(0, startPos.offset) + text.substring(endPos.offset);
      engine.set_paragraph(startPos.para, newText);
    } else {
      // Multiple paragraphs - merge first and last, delete middle
      const firstText = engine.get_paragraph(startPos.para) || '';
      const lastText = engine.get_paragraph(endPos.para) || '';
      const newText = firstText.substring(0, startPos.offset) + lastText.substring(endPos.offset);

      // Delete paragraphs from end to start (to maintain indices)
      for (let i = endPos.para; i > startPos.para; i--) {
        engine.delete_paragraph(i);
      }
      engine.set_paragraph(startPos.para, newText);
    }

    cursorPara = startPos.para;
    cursorOffset = startPos.offset;
    clearSelection();
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (!engine) return;

    const key = event.key;
    const isShift = event.shiftKey;

    // =========================================================================
    // Table cell editing mode
    // =========================================================================
    if (activeTableId && activeCell) {
      // Escape exits table editing
      if (key === 'Escape') {
        event.preventDefault();
        saveCellText();
        exitTableCell();
        recomputeAndRender();
        return;
      }

      // Tab/Shift+Tab navigation
      if (key === 'Tab') {
        event.preventDefault();
        handleTableTab(isShift);
        recomputeAndRender();
        return;
      }

      // Enter inserts newline in cell
      if (key === 'Enter' && !event.ctrlKey && !event.metaKey) {
        event.preventDefault();
        handleTableEnter();
        saveCellText();
        recomputeAndRender();
        return;
      }

      // Arrow keys for navigation within/between cells
      if (['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(key)) {
        event.preventDefault();
        handleTableArrowKey(key);
        recomputeAndRender();
        return;
      }

      // Backspace
      if (key === 'Backspace') {
        event.preventDefault();
        handleTableBackspace();
        saveCellText();
        recomputeAndRender();
        return;
      }

      // Delete
      if (key === 'Delete') {
        event.preventDefault();
        handleTableDelete();
        saveCellText();
        recomputeAndRender();
        return;
      }

      // Home/End within cell
      if (key === 'Home') {
        event.preventDefault();
        const lineStart = cellText.lastIndexOf('\n', cellCursorOffset - 1) + 1;
        cellCursorOffset = lineStart;
        return;
      }
      if (key === 'End') {
        event.preventDefault();
        const lineEnd = cellText.indexOf('\n', cellCursorOffset);
        cellCursorOffset = lineEnd === -1 ? cellText.length : lineEnd;
        return;
      }

      // Character input
      if (key.length === 1 && !event.ctrlKey && !event.metaKey) {
        event.preventDefault();
        handleTableCharInput(key);
        saveCellText();
        recomputeAndRender();
        return;
      }

      // Pass through Ctrl+C, Ctrl+V, etc. for now
      return;
    }

    // =========================================================================
    // Normal editing mode
    // =========================================================================

    // Handle Ctrl/Cmd key combinations
    if (event.ctrlKey || event.metaKey) {
      // Ctrl+Enter for page break
      if (key === 'Enter') {
        event.preventDefault();
        insertPageBreak();
        return;
      }
      // Handle Ctrl+Home and Ctrl+End
      if (key === 'Home') {
        event.preventDefault();
        if (isShift) startOrExtendSelection();
        cursorPara = 0;
        cursorOffset = 0;
        if (isShift) extendSelection();
        else clearSelection();
        resetCursorBlink();
        return;
      } else if (key === 'End') {
        event.preventDefault();
        if (isShift) startOrExtendSelection();
        const lastPara = engine.paragraph_count() - 1;
        cursorPara = lastPara;
        cursorOffset = (engine.get_paragraph(lastPara) || '').length;
        if (isShift) extendSelection();
        else clearSelection();
        resetCursorBlink();
        return;
      } else if (key === 'a' || key === 'A') {
        // Select all
        event.preventDefault();
        selectAll();
        return;
      } else if (key === 'c' || key === 'C') {
        // Copy
        event.preventDefault();
        handleCopy();
        return;
      } else if (key === 'x' || key === 'X') {
        // Cut
        event.preventDefault();
        handleCut();
        return;
      } else if (key === 'v' || key === 'V') {
        // Paste
        event.preventDefault();
        handlePaste();
        return;
      } else if (key === 'z' || key === 'Z') {
        // Undo (Ctrl+Z) or Redo (Ctrl+Shift+Z)
        event.preventDefault();
        if (isShift) {
          redo();
        } else {
          undo();
        }
        return;
      } else if (key === 'y' || key === 'Y') {
        // Redo (Ctrl+Y)
        event.preventDefault();
        redo();
        return;
      }
      return; // Let browser handle other Ctrl combos
    }

    event.preventDefault();

    // Start selection on shift+arrow
    if (isShift && ['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', 'Home', 'End'].includes(key)) {
      startOrExtendSelection();
    }

    if (key === 'Backspace') {
      handleBackspace();
    } else if (key === 'Delete') {
      handleDelete();
    } else if (key === 'Enter') {
      handleEnter();
    } else if (key === 'ArrowLeft') {
      moveCursor(-1, isShift);
    } else if (key === 'ArrowRight') {
      moveCursor(1, isShift);
    } else if (key === 'ArrowUp') {
      moveCursorVertical(-1, isShift);
    } else if (key === 'ArrowDown') {
      moveCursorVertical(1, isShift);
    } else if (key === 'Home') {
      cursorOffset = 0;
      if (isShift) extendSelection();
      else clearSelection();
      resetCursorBlink();
    } else if (key === 'End') {
      cursorOffset = (engine.get_paragraph(cursorPara) || '').length;
      if (isShift) extendSelection();
      else clearSelection();
      resetCursorBlink();
    } else if (key.length === 1) {
      insertChar(key);
    }
  }

  function startOrExtendSelection() {
    if (!selectionStart) {
      selectionStart = { para: cursorPara, offset: cursorOffset };
      selectionEnd = { para: cursorPara, offset: cursorOffset };
    }
  }

  function extendSelection() {
    selectionEnd = { para: cursorPara, offset: cursorOffset };
  }

  function selectAll() {
    if (!engine) return;
    selectionStart = { para: 0, offset: 0 };
    const lastPara = engine.paragraph_count() - 1;
    const lastParaText = engine.get_paragraph(lastPara) || '';
    selectionEnd = { para: lastPara, offset: lastParaText.length };
    cursorPara = lastPara;
    cursorOffset = lastParaText.length;
    resetCursorBlink();
  }

  async function handleCopy() {
    const text = getSelectedText();
    if (text) {
      try {
        await navigator.clipboard.writeText(text);
      } catch (e) {
        console.error('Failed to copy:', e);
      }
    }
  }

  async function handleCut() {
    const text = getSelectedText();
    if (text) {
      try {
        await navigator.clipboard.writeText(text);
        saveUndoState();
        deleteSelection();
        recomputeAndRender();
      } catch (e) {
        console.error('Failed to cut:', e);
      }
    }
  }

  async function handlePaste() {
    try {
      const text = await navigator.clipboard.readText();
      if (text) {
        saveUndoState();

        // Delete selection if any
        if (hasSelection()) {
          deleteSelection();
        }

        // Insert text, handling newlines
        const lines = text.split('\n');
        for (let i = 0; i < lines.length; i++) {
          if (i > 0) {
            // Insert new paragraph
            handleEnterInternal();
          }
          // Insert text into current paragraph
          const paraText = engine!.get_paragraph(cursorPara) || '';
          const newText = paraText.slice(0, cursorOffset) + lines[i] + paraText.slice(cursorOffset);
          engine!.set_paragraph(cursorPara, newText);
          cursorOffset += lines[i].length;
        }
        recomputeAndRender();
      }
    } catch (e) {
      console.error('Failed to paste:', e);
    }
  }

  function handleEnterInternal() {
    if (!engine) return;

    const text = engine.get_paragraph(cursorPara) || '';
    const before = text.slice(0, cursorOffset);
    const after = text.slice(cursorOffset);

    // Get the current list type to preserve it during paste
    const listType = engine.get_list_type(cursorPara);

    engine.set_paragraph(cursorPara, before);

    if (listType !== 'none') {
      engine.insert_paragraph_with_list(cursorPara + 1, after, cursorPara);
    } else {
      engine.insert_paragraph(cursorPara + 1, after);
    }

    cursorPara++;
    cursorOffset = 0;
  }

  function insertChar(char: string) {
    if (!engine) return;

    saveUndoState();

    // Delete selection if any
    if (hasSelection()) {
      deleteSelection();
    }

    const text = engine.get_paragraph(cursorPara) || '';

    // If on a special paragraph (image/page break), move to next paragraph first
    if (isSpecialParagraph(text)) {
      // Move to next paragraph, or create one if at end
      if (cursorPara < engine.paragraph_count() - 1) {
        cursorPara++;
        cursorOffset = 0;
        // Insert at beginning of next paragraph
        const nextText = engine.get_paragraph(cursorPara) || '';
        if (!isSpecialParagraph(nextText)) {
          engine.set_paragraph(cursorPara, char + nextText);
          cursorOffset = 1;
          recomputeAndRender();
          return;
        }
      }
      // If next is also special, create a new paragraph
      engine.insert_paragraph(cursorPara + 1, char);
      cursorPara++;
      cursorOffset = 1;
      recomputeAndRender();
      return;
    }

    const newText = text.slice(0, cursorOffset) + char + text.slice(cursorOffset);
    engine.set_paragraph(cursorPara, newText);
    cursorOffset++;

    recomputeAndRender();
  }

  function handleBackspace() {
    if (!engine) return;

    saveUndoState();

    // Delete selection if any
    if (hasSelection()) {
      deleteSelection();
      recomputeAndRender();
      return;
    }

    const text = engine.get_paragraph(cursorPara) || '';

    // If on a special paragraph, select the image or delete the page break
    if (isSpecialParagraph(text)) {
      if (text.startsWith(IMAGE_MARKER)) {
        // Select the image for deletion
        const imageId = text.substring(1);
        selectedImageId = imageId;
        showImageOptions = true;
        renderAllPages();
      } else {
        // Delete page break
        engine.delete_paragraph(cursorPara);
        if (cursorPara > 0) {
          cursorPara--;
          cursorOffset = (engine.get_paragraph(cursorPara) || '').length;
        }
        recomputeAndRender();
      }
      return;
    }

    if (cursorOffset > 0) {
      const newText = text.slice(0, cursorOffset - 1) + text.slice(cursorOffset);
      engine.set_paragraph(cursorPara, newText);
      cursorOffset--;
    } else if (cursorPara > 0) {
      // Check if previous paragraph is special
      const prevText = engine.get_paragraph(cursorPara - 1) || '';
      if (isSpecialParagraph(prevText)) {
        if (prevText.startsWith(IMAGE_MARKER)) {
          // Select the image for deletion
          const imageId = prevText.substring(1);
          selectedImageId = imageId;
          showImageOptions = true;
          renderAllPages();
          return;
        } else {
          // Delete page break
          engine.delete_paragraph(cursorPara - 1);
          cursorPara--;
          recomputeAndRender();
          return;
        }
      }
      // Merge with previous paragraph
      const currText = text;
      engine.set_paragraph(cursorPara - 1, prevText + currText);
      engine.delete_paragraph(cursorPara);
      cursorPara--;
      cursorOffset = prevText.length;
    }

    recomputeAndRender();
  }

  function handleDelete() {
    if (!engine) return;

    saveUndoState();

    // Delete selection if any
    if (hasSelection()) {
      deleteSelection();
      recomputeAndRender();
      return;
    }

    const text = engine.get_paragraph(cursorPara) || '';

    // If on a special paragraph, select the image or delete the page break
    if (isSpecialParagraph(text)) {
      if (text.startsWith(IMAGE_MARKER)) {
        // Select the image for deletion
        const imageId = text.substring(1);
        selectedImageId = imageId;
        showImageOptions = true;
        renderAllPages();
      } else {
        // Delete page break
        engine.delete_paragraph(cursorPara);
        recomputeAndRender();
      }
      return;
    }

    if (cursorOffset < text.length) {
      const newText = text.slice(0, cursorOffset) + text.slice(cursorOffset + 1);
      engine.set_paragraph(cursorPara, newText);
    } else if (cursorPara < engine.paragraph_count() - 1) {
      // Check if next paragraph is special
      const nextText = engine.get_paragraph(cursorPara + 1) || '';
      if (isSpecialParagraph(nextText)) {
        if (nextText.startsWith(IMAGE_MARKER)) {
          // Select the image for deletion
          const imageId = nextText.substring(1);
          selectedImageId = imageId;
          showImageOptions = true;
          renderAllPages();
          return;
        } else {
          // Delete page break
          engine.delete_paragraph(cursorPara + 1);
          recomputeAndRender();
          return;
        }
      }
      // Merge with next paragraph
      engine.set_paragraph(cursorPara, text + nextText);
      engine.delete_paragraph(cursorPara + 1);
    }

    recomputeAndRender();
  }

  function handleEnter() {
    if (!engine) return;

    saveUndoState();

    // Delete selection if any
    if (hasSelection()) {
      deleteSelection();
    }

    const text = engine.get_paragraph(cursorPara) || '';
    const before = text.slice(0, cursorOffset);
    const after = text.slice(cursorOffset);

    // Get the current list type
    const listType = engine.get_list_type(cursorPara);

    // Check if current paragraph is a list item
    if (listType !== 'none') {
      // If the current paragraph is empty (double Enter), exit the list
      if (text.trim() === '') {
        engine.set_list_type(cursorPara, 'none');
        recomputeAndRender();
        return;
      }

      // Otherwise, create a new list item with the same list type
      engine.set_paragraph(cursorPara, before);
      engine.insert_paragraph_with_list(cursorPara + 1, after, cursorPara);
    } else {
      // Normal paragraph behavior
      engine.set_paragraph(cursorPara, before);
      engine.insert_paragraph(cursorPara + 1, after);
    }

    cursorPara++;
    cursorOffset = 0;

    recomputeAndRender();
  }

  function moveCursor(delta: number, extendSel = false) {
    if (!engine) return;

    const text = engine.get_paragraph(cursorPara) || '';

    if (delta < 0) {
      if (cursorOffset > 0) {
        cursorOffset--;
      } else if (cursorPara > 0) {
        cursorPara--;
        const prevText = engine.get_paragraph(cursorPara) || '';
        cursorOffset = prevText.length;
      }
    } else {
      if (cursorOffset < text.length) {
        cursorOffset++;
      } else if (cursorPara < engine.paragraph_count() - 1) {
        cursorPara++;
        cursorOffset = 0;
      }
    }

    if (extendSel) {
      extendSelection();
    } else {
      clearSelection();
    }

    resetCursorBlink(); // Show cursor immediately and render
  }

  function moveCursorVertical(delta: number, extendSel = false) {
    if (!engine) return;

    // Get current display line position
    const currentPosJson = engine.para_to_display_pos(cursorPara, cursorOffset);
    if (!currentPosJson) return;

    try {
      const currentPos = JSON.parse(currentPosJson);
      const newLine = currentPos.line + delta;
      const displayLineCount = engine.display_line_count();

      if (newLine >= 0 && newLine < displayLineCount) {
        // Convert new display line back to paragraph position
        const newPosJson = engine.display_to_para(newLine, currentPos.col);
        if (newPosJson) {
          const newPos = JSON.parse(newPosJson);
          const targetPara = newPos.para;
          const targetText = engine.get_paragraph(targetPara) || '';

          // Check if target is a table paragraph
          if (targetText.startsWith(TABLE_MARKER)) {
            const tableId = targetText.substring(1); // Skip the marker (1 char, but 3 bytes in UTF-8)
            // Enter the table at the appropriate cell
            if (delta > 0) {
              // Moving down: enter first row
              enterTableCell(tableId, targetPara, 0, 0);
            } else {
              // Moving up: enter last row
              const dims = engine.get_table_dimensions(tableId);
              if (dims) {
                const { rows, cols } = JSON.parse(dims);
                enterTableCell(tableId, targetPara, rows - 1, 0);
              } else {
                enterTableCell(tableId, targetPara, 0, 0);
              }
            }
            resetCursorBlink();
            return;
          }

          cursorPara = targetPara;
          cursorOffset = newPos.offset;
        }
      }
    } catch (e) {
      console.error('Failed to move cursor vertically:', e);
    }

    if (extendSel) {
      extendSelection();
    } else {
      clearSelection();
    }

    resetCursorBlink(); // Show cursor immediately and render
  }

  /**
   * Get cursor position from mouse coordinates
   */
  function getCursorPositionFromMouse(pageIdx: number, x: number, y: number): { para: number; offset: number } | null {
    if (!engine) return null;

    const canvas = pageCanvases[pageIdx];
    if (!canvas) return null;

    const lineHeightPx = FONT_SIZE * LINE_HEIGHT;

    const displayLinesJson = engine.get_display_lines_json();
    try {
      const displayLines = JSON.parse(displayLinesJson);
      // Fields are now camelCase from Rust
      const linesOnPage = displayLines.filter((dl: { pageIndex: number }) => dl.pageIndex === pageIdx);

      // Find the line by comparing y positions directly
      let clickedLine = null;
      for (const line of linesOnPage) {
        const lineY = MARGIN_TOP + line.yPosition;
        if (y >= lineY && y < lineY + lineHeightPx) {
          clickedLine = line;
          break;
        }
      }

      // If no exact match, find closest line
      if (!clickedLine && linesOnPage.length > 0) {
        let minDist = Infinity;
        for (const line of linesOnPage) {
          const lineY = MARGIN_TOP + line.yPosition;
          const dist = Math.abs(y - lineY);
          if (dist < minDist) {
            minDist = dist;
            clickedLine = line;
          }
        }
      }

      if (clickedLine) {
        const para = clickedLine.paraIndex;
        const text = engine.get_paragraph(para) || '';
        const lineStartOffset = clickedLine.startOffset;
        // xPosition from engine includes column offset
        const lineX = clickedLine.xPosition || MARGIN_LEFT;

        const ctx = canvas.getContext('2d');
        if (ctx) {
          ctx.font = `${FONT_SIZE}px ${FONT_FAMILY}`;
          let charPos = lineStartOffset;

          // Check if click is before the text starts
          if (x < lineX) {
            return { para, offset: lineStartOffset };
          }

          for (let i = lineStartOffset; i <= clickedLine.endOffset; i++) {
            const textFromLineStart = text.slice(lineStartOffset, i);
            const width = ctx.measureText(textFromLineStart).width;
            if (lineX + width > x) {
              // Check if we're closer to the previous or current character
              const prevWidth = i > lineStartOffset ? ctx.measureText(text.slice(lineStartOffset, i - 1)).width : 0;
              const midPoint = lineX + (prevWidth + width) / 2;
              charPos = x < midPoint ? Math.max(lineStartOffset, i - 1) : i;
              break;
            }
            charPos = i;
          }
          return { para, offset: Math.min(charPos, clickedLine.endOffset) };
        }
      }
    } catch (e) {
      console.error('Failed to get cursor position:', e);
    }
    return null;
  }

  function handleCanvasMouseDown(event: MouseEvent, pageIdx: number) {
    if (!engine) return;

    const canvas = pageCanvases[pageIdx];
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    // Scale screen coordinates to canvas coordinates (account for zoom)
    const zoomFactor = ZOOM / 100;
    const x = (event.clientX - rect.left) / zoomFactor;
    const y = (event.clientY - rect.top) / zoomFactor;

    // Check if clicking on a resize handle for a selected image
    if (selectedImageId && !isCropping) {
      const bounds = getSelectedImageBounds();
      if (bounds && bounds.pageIndex === pageIdx) {
        const handle = getResizeHandleAtPoint(x, y, bounds);
        if (handle) {
          event.preventDefault();
          startResize(handle, event.clientX, event.clientY);
          return;
        }
        // Check if clicking inside the selected image (for dragging)
        if (x >= bounds.x && x <= bounds.x + bounds.width && y >= bounds.y && y <= bounds.y + bounds.height) {
          event.preventDefault();
          startImageDrag(pageIdx, x, y, bounds.x, bounds.y);
          return;
        }
      }
    }

    // Check if clicking on a crop handle
    if (selectedImageId && isCropping) {
      const bounds = getSelectedImageBounds();
      if (bounds && bounds.pageIndex === pageIdx) {
        const handle = getResizeHandleAtPoint(x, y, bounds);
        if (handle) {
          event.preventDefault();
          startCropDrag(handle, event.clientX, event.clientY);
          return;
        }
      }
    }

    // Check if clicking on an image
    const imageAtPos = getImageAtPosition(pageIdx, x, y);
    if (imageAtPos) {
      // Select the image
      selectImage(imageAtPos.id, pageIdx, x, y);
      return;
    }

    // Clicking elsewhere - deselect image if selected
    if (selectedImageId) {
      deselectImage();
      // Also end crop mode if active
      if (isCropping) {
        endCropMode();
      }
    }

    // Check if clicking on a table cell
    const tableClick = getTableCellAtPosition(pageIdx, x, y);
    if (tableClick) {
      // Enter table cell editing mode
      enterTableCell(tableClick.tableId, tableClick.tablePara, tableClick.row, tableClick.col);
      hiddenTextarea?.focus();
      return;
    }

    // If clicking outside table while editing, exit table editing
    if (activeTableId) {
      exitTableCell();
    }

    const pos = getCursorPositionFromMouse(pageIdx, x, y);
    if (!pos) return;

    isMouseDown = true;
    mouseDownPage = pageIdx;

    if (event.shiftKey) {
      // Extend existing selection
      startOrExtendSelection();
      cursorPara = pos.para;
      cursorOffset = pos.offset;
      extendSelection();
    } else {
      // Start new selection
      cursorPara = pos.para;
      cursorOffset = pos.offset;
      clearSelection();
      selectionStart = { para: pos.para, offset: pos.offset };
      selectionEnd = { para: pos.para, offset: pos.offset };
    }

    currentPage = pageIdx;

    // Ensure cursor is visible immediately on click and reset blink
    isFocused = true;
    resetCursorBlink(); // This now renders immediately with cursor visible

    hiddenTextarea?.focus();
  }

  function handleCanvasMouseMove(event: MouseEvent, pageIdx: number) {
    if (!engine) return;

    const canvas = pageCanvases[pageIdx];
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    // Scale screen coordinates to canvas coordinates (account for zoom)
    const zoomFactor = ZOOM / 100;
    const x = (event.clientX - rect.left) / zoomFactor;
    const y = (event.clientY - rect.top) / zoomFactor;

    // Handle image resizing
    if (isResizing) {
      handleResizeMove(event.clientX, event.clientY);
      return;
    }

    // Handle image cropping
    if (isCropping && cropHandle) {
      handleCropMove(event.clientX, event.clientY);
      return;
    }

    // Handle image dragging
    if (isDraggingImage) {
      handleImageDrag(pageIdx, x, y);
      return;
    }

    // Update cursor based on hover position when image is selected
    if (selectedImageId) {
      const bounds = getSelectedImageBounds();
      if (bounds && bounds.pageIndex === pageIdx) {
        const handle = getResizeHandleAtPoint(x, y, bounds);
        if (handle) {
          canvas.style.cursor = getResizeCursor(handle);
        } else if (x >= bounds.x && x <= bounds.x + bounds.width && y >= bounds.y && y <= bounds.y + bounds.height) {
          canvas.style.cursor = 'move';
        } else {
          canvas.style.cursor = 'text';
        }
      } else {
        canvas.style.cursor = 'text';
      }
    }

    // Handle text selection
    if (!isMouseDown) return;

    const pos = getCursorPositionFromMouse(pageIdx, x, y);
    if (!pos) return;

    cursorPara = pos.para;
    cursorOffset = pos.offset;
    selectionEnd = { para: pos.para, offset: pos.offset };

    currentPage = pageIdx;
    // Keep cursor visible during drag
    cursorVisible = true;
    renderAllPages();
  }

  function handleCanvasMouseUp(event: MouseEvent) {
    isMouseDown = false;
    endImageDrag();
    endResize();
    endCropDrag();
    hiddenTextarea?.focus();
  }
</script>

<div class="editor-wasm-wrapper">
  <Toolbar onFormat={handleFormat} {canUndo} {canRedo} onSave={handleSave} />

  <div class="editor-main">
    <div class="status-bar">
      <span class="status" class:ready={isReady} class:error={!!error}>
        {#if error}
          {error}
        {:else if isReady}
          Page {currentPage + 1} of {pageCount}
        {:else}
          Loading WASM...
        {/if}
      </span>
    </div>

    <!-- Hidden canvas for text measurement -->
    <canvas bind:this={measureCanvas} class="measure-canvas"></canvas>

    <!-- Hidden textarea for keyboard input -->
    <textarea
      bind:this={hiddenTextarea}
      class="hidden-input"
      onkeydown={handleKeyDown}
      onfocus={() => { isFocused = true; startCursorBlink(); }}
      onblur={() => { isFocused = false; stopCursorBlink(); render(); }}
    ></textarea>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div
      class="pages-container"
      bind:this={pagesContainer}
      onmouseup={handleCanvasMouseUp}
    >
      {#each Array(pageCount) as _, pageIdx}
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <canvas
          bind:this={pageCanvases[pageIdx]}
          class="page-canvas"
          onmousedown={(e) => handleCanvasMouseDown(e, pageIdx)}
          onmousemove={(e) => handleCanvasMouseMove(e, pageIdx)}
          oncontextmenu={(e) => handleCanvasContextMenu(e, pageIdx)}
        ></canvas>
      {/each}
    </div>
  </div>

  <!-- Image insertion dialog -->
  {#if showImageDialog}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="popup-overlay" onclick={closeImageDialog}>
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="popup-dialog" onclick={(e) => e.stopPropagation()}>
        <div class="popup-header">
          <h3>Insert Image</h3>
          <button class="popup-close" onclick={closeImageDialog}>&times;</button>
        </div>
        <div class="popup-content">
          <!-- URL input -->
          <div class="input-section">
            <label for="image-url-wasm">Image URL</label>
            <div class="url-input-row">
              <input
                id="image-url-wasm"
                type="text"
                bind:value={imageUrlInput}
                placeholder="https://example.com/image.jpg"
                onkeydown={(e) => e.key === 'Enter' && insertImageFromUrl()}
              />
              <button onclick={insertImageFromUrl}>Insert</button>
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
            ondragleave={() => { dragOver = false; }}
            ondrop={handleImageDrop}
            onclick={() => document.getElementById('file-input-wasm')?.click()}
          >
            <svg width="48" height="48" viewBox="0 0 24 24" fill="#9aa0a6">
              <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
            </svg>
            <p>Drag and drop an image here</p>
            <p class="small">or click to select from your computer</p>
          </div>
          <input
            id="file-input-wasm"
            type="file"
            accept="image/*"
            class="file-input"
            onchange={handleImageFileSelect}
          />

          <p class="tip">Tip: You can also paste an image directly in the editor (Ctrl+V)</p>
        </div>
      </div>
    </div>
  {/if}

  <!-- Table insertion dialog -->
  {#if showTableDialog}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="popup-overlay" onclick={closeTableDialog}>
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div class="popup-dialog table-dialog" onclick={(e) => e.stopPropagation()}>
        <div class="popup-header">
          <h3>Insert Table</h3>
          <button class="popup-close" onclick={closeTableDialog}>&times;</button>
        </div>
        <div class="popup-content">
          <div class="table-size-inputs">
            <div class="input-group">
              <label for="table-rows">Rows</label>
              <input
                id="table-rows"
                type="number"
                min="1"
                max="50"
                bind:value={tableRows}
              />
            </div>
            <div class="input-group">
              <label for="table-cols">Columns</label>
              <input
                id="table-cols"
                type="number"
                min="1"
                max="20"
                bind:value={tableCols}
              />
            </div>
          </div>
          <div class="table-header-options">
            <label class="checkbox-label">
              <input type="checkbox" bind:checked={tableHasHeaderRow} />
              <span>Header row</span>
            </label>
            <label class="checkbox-label">
              <input type="checkbox" bind:checked={tableHasHeaderCol} />
              <span>Header column</span>
            </label>
          </div>
          <div class="table-preview-container">
            <div class="table-grid" style="--rows: {Math.min(tableRows, 8)}; --cols: {Math.min(tableCols, 8)};">
              {#each Array(Math.min(tableRows, 8)) as _, r}
                {#each Array(Math.min(tableCols, 8)) as _, c}
                  <div
                    class="table-cell-preview"
                    class:header-cell={(tableHasHeaderRow && r === 0) || (tableHasHeaderCol && c === 0)}
                  ></div>
                {/each}
              {/each}
            </div>
            {#if tableRows > 8 || tableCols > 8}
              <p class="preview-note">Preview limited to 8×8</p>
            {/if}
          </div>
          <div class="dialog-actions">
            <button class="btn-secondary" onclick={closeTableDialog}>Cancel</button>
            <button class="btn-primary" onclick={insertTable}>Insert Table</button>
          </div>
        </div>
      </div>
    </div>
  {/if}

  <!-- Image options popup -->
  {#if showImageOptions && selectedImageId && !isCropping}
    {@const selectedImage = getSelectedImageData()}
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
              onclick={() => setImageWrapStyle('inline')}
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
              onclick={() => setImageWrapStyle('square')}
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
              onclick={() => setImageWrapStyle('tight')}
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
              onclick={() => setImageWrapStyle('through')}
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
              onclick={() => setImageWrapStyle('top-bottom')}
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
              onclick={() => setImageWrapStyle('behind')}
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
              onclick={() => setImageWrapStyle('in-front')}
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
                onclick={() => setImageAlign('left')}
                title="Align left"
              >
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="2" y="6" width="10" height="12" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
                </svg>
              </button>
              <button
                class="layout-btn small"
                class:active={selectedImage?.horizontalAlign === 'center'}
                onclick={() => setImageAlign('center')}
                title="Center"
              >
                <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
                  <rect x="7" y="6" width="10" height="12" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
                </svg>
              </button>
              <button
                class="layout-btn small"
                class:active={selectedImage?.horizontalAlign === 'right'}
                onclick={() => setImageAlign('right')}
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
                onchange={() => setImagePositionMode('move-with-text')}
              />
              <span>Move with text</span>
            </label>
            <label class="radio-option">
              <input
                type="radio"
                name="positionMode"
                checked={selectedImage?.positionMode === 'fixed-position'}
                onchange={() => setImagePositionMode('fixed-position')}
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

  <!-- Table context menu -->
  {#if showTableContextMenu && activeTableId}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
    <div class="table-context-overlay" onclick={closeTableMenu}>
      <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
      <div
        class="table-context-menu"
        style="left: {tableContextMenuPos.x}px; top: {tableContextMenuPos.y}px;"
        onclick={(e) => e.stopPropagation()}
      >
        <div class="context-menu-section">
          <div class="context-menu-label">Rows</div>
          <button class="context-menu-btn" onclick={addRowAbove}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M21 6H3v2h18V6zm0-4H3v2h18V2z"/>
              <path d="M12 22l4-4h-3v-6h-2v6H8l4 4z"/>
            </svg>
            Insert Row Above
          </button>
          <button class="context-menu-btn" onclick={addRowBelow}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M21 18H3v-2h18v2zm0 4H3v-2h18v2z"/>
              <path d="M12 2l-4 4h3v6h2V6h3l-4-4z"/>
            </svg>
            Insert Row Below
          </button>
          <button class="context-menu-btn danger" onclick={deleteCurrentRow}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
            </svg>
            Delete Row
          </button>
        </div>
        <div class="context-menu-divider"></div>
        <div class="context-menu-section">
          <div class="context-menu-label">Columns</div>
          <button class="context-menu-btn" onclick={addColumnLeft}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 3v18h2V3H6zM2 3v18h2V3H2z"/>
              <path d="M22 12l-4-4v3h-6v2h6v3l4-4z"/>
            </svg>
            Insert Column Left
          </button>
          <button class="context-menu-btn" onclick={addColumnRight}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M16 3v18h2V3h-2zm4 0v18h2V3h-2z"/>
              <path d="M2 12l4 4v-3h6v-2H6V8l-4 4z"/>
            </svg>
            Insert Column Right
          </button>
          <button class="context-menu-btn danger" onclick={deleteCurrentColumn}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
              <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
            </svg>
            Delete Column
          </button>
        </div>
        <div class="context-menu-divider"></div>
        <button class="context-menu-btn danger" onclick={deleteCurrentTable}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
            <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
          </svg>
          Delete Table
        </button>
      </div>
    </div>
  {/if}

  <!-- Active table cell indicator -->
  {#if activeTableId && activeCell}
    <div class="table-cell-indicator">
      Editing cell ({activeCell.row + 1}, {activeCell.col + 1})
      <button class="exit-cell-btn" onclick={exitTableCell}>Exit</button>
    </div>
  {/if}
</div>

<style>
  .editor-wasm-wrapper {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #f0f0f0;
  }

  .editor-main {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  .status-bar {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    padding: 4px 20px;
    background: #f8f9fa;
    border-bottom: 1px solid #e0e0e0;
  }

  .status {
    font-size: 14px;
    color: #5f6368;
  }

  .status.ready {
    color: #1a73e8;
  }

  .status.error {
    color: #d93025;
  }

  .measure-canvas {
    position: absolute;
    left: -9999px;
    visibility: hidden;
  }

  .hidden-input {
    position: absolute;
    left: -9999px;
    opacity: 0;
  }

  .pages-container {
    flex: 1;
    overflow: auto;
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 40px;
    gap: 40px;
  }

  .page-canvas {
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12), 0 1px 2px rgba(0, 0, 0, 0.24);
    cursor: text;
    flex-shrink: 0;
  }

  /* Image insertion dialog styles */
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
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.2);
    max-width: 500px;
    width: 90%;
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
    color: #202124;
  }

  .popup-close {
    background: none;
    border: none;
    font-size: 24px;
    cursor: pointer;
    color: #5f6368;
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

  .url-input-row {
    display: flex;
    gap: 8px;
  }

  .url-input-row input {
    flex: 1;
    padding: 8px 12px;
    border: 1px solid #dadce0;
    border-radius: 4px;
    font-size: 14px;
  }

  .url-input-row input:focus {
    outline: none;
    border-color: #1a73e8;
  }

  .url-input-row button {
    padding: 8px 16px;
    background: #1a73e8;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .url-input-row button:hover {
    background: #1557b0;
  }

  /* Table dialog styles */
  .table-dialog {
    max-width: 400px;
  }

  .table-size-inputs {
    display: flex;
    gap: 16px;
    margin-bottom: 16px;
  }

  .input-group {
    flex: 1;
  }

  .input-group label {
    display: block;
    margin-bottom: 6px;
    font-size: 14px;
    color: #5f6368;
  }

  .input-group input {
    width: 100%;
    padding: 8px 12px;
    border: 1px solid #dadce0;
    border-radius: 4px;
    font-size: 14px;
  }

  .input-group input:focus {
    outline: none;
    border-color: #1a73e8;
  }

  .table-header-options {
    display: flex;
    gap: 20px;
    margin-bottom: 16px;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 14px;
    color: #3c4043;
    cursor: pointer;
  }

  .checkbox-label input[type="checkbox"] {
    width: 16px;
    height: 16px;
    cursor: pointer;
  }

  .table-preview-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 220px;
    margin-bottom: 20px;
  }

  .table-grid {
    display: inline-grid;
    grid-template-rows: repeat(var(--rows), 24px);
    grid-template-columns: repeat(var(--cols), 32px);
    gap: 1px;
    background: #dadce0;
    border: 1px solid #dadce0;
    border-radius: 4px;
    overflow: hidden;
  }

  .table-cell-preview {
    background: #f8f9fa;
  }

  .table-cell-preview.header-cell {
    background: #e8eaed;
  }

  .preview-note {
    margin: 8px 0 0;
    font-size: 12px;
    color: #9aa0a6;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .btn-secondary {
    padding: 8px 16px;
    background: white;
    border: 1px solid #dadce0;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
    color: #5f6368;
  }

  .btn-secondary:hover {
    background: #f1f3f4;
  }

  .btn-primary {
    padding: 8px 16px;
    background: #1a73e8;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 14px;
  }

  .btn-primary:hover {
    background: #1557b0;
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

  .popup-close {
    background: none;
    border: none;
    font-size: 18px;
    cursor: pointer;
    color: #5f6368;
    padding: 0;
    line-height: 1;
  }

  .popup-close:hover {
    color: #202124;
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

  .image-options-buttons {
    display: flex;
    padding: 0;
    gap: 4px;
  }

  .wrap-buttons {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
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

  /* Table context menu styles */
  .table-context-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 1001;
  }

  .table-context-menu {
    position: fixed;
    background: white;
    border-radius: 8px;
    box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
    min-width: 180px;
    padding: 8px 0;
    z-index: 1002;
  }

  .context-menu-section {
    padding: 4px 0;
  }

  .context-menu-label {
    padding: 4px 16px;
    font-size: 11px;
    font-weight: 600;
    color: #5f6368;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .context-menu-btn {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 16px;
    border: none;
    background: transparent;
    cursor: pointer;
    font-size: 14px;
    color: #202124;
    text-align: left;
    transition: background 0.15s;
  }

  .context-menu-btn:hover {
    background: #f1f3f4;
  }

  .context-menu-btn.danger {
    color: #d93025;
  }

  .context-menu-btn.danger:hover {
    background: #fce8e6;
  }

  .context-menu-btn svg {
    flex-shrink: 0;
    opacity: 0.7;
  }

  .context-menu-divider {
    height: 1px;
    background: #e0e0e0;
    margin: 4px 0;
  }

  /* Table cell indicator */
  .table-cell-indicator {
    position: fixed;
    bottom: 20px;
    left: 50%;
    transform: translateX(-50%);
    background: #1a73e8;
    color: white;
    padding: 8px 16px;
    border-radius: 20px;
    font-size: 13px;
    display: flex;
    align-items: center;
    gap: 12px;
    box-shadow: 0 2px 8px rgba(26, 115, 232, 0.4);
    z-index: 100;
  }

  .exit-cell-btn {
    background: rgba(255, 255, 255, 0.2);
    border: none;
    color: white;
    padding: 4px 10px;
    border-radius: 12px;
    cursor: pointer;
    font-size: 12px;
    transition: background 0.15s;
  }

  .exit-cell-btn:hover {
    background: rgba(255, 255, 255, 0.3);
  }
</style>
