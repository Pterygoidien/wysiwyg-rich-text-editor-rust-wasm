<script lang="ts">
  import { onMount, tick, onDestroy } from 'svelte';
  import init, { Engine } from './engine-wasm/editor_engine.js';
  import { parseRenderCommands, executeRenderCommands, type RenderCommand } from './engine-bridge';
  import Toolbar from './Toolbar.svelte';
  import { pageConfig, fontSize, lineHeight, letterSpacing, paragraphSpacing, fontFamily, zoomLevel } from './stores';
  import { getPageDimensions, getContentDimensions, mmToPixels } from './types';

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

  // Mouse drag selection state
  let isMouseDown = $state(false);
  let mouseDownPage = $state(0);

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

    // Calculate display size (max width within margins, maintain aspect ratio)
    const maxWidth = PAGE_WIDTH - MARGIN_LEFT - MARGIN_RIGHT;
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

    // Insert image paragraph after current paragraph
    engine.insert_image_paragraph(cursorPara + 1, id);

    console.log(`Image paragraph inserted at index ${cursorPara + 1}, loadedImages has ${loadedImages.size} entries`);

    // Move cursor to after the image
    cursorPara += 1;
    cursorOffset = 0;

    recomputeAndRender();
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

  function drawCursor(ctx: CanvasRenderingContext2D) {
    if (!engine || !cursorVisible || !isFocused) return;

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
      const textBefore = paraText.substring(line.start_offset, cursorOffset);

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

    if (cursorOffset > 0) {
      const text = engine.get_paragraph(cursorPara) || '';
      const newText = text.slice(0, cursorOffset - 1) + text.slice(cursorOffset);
      engine.set_paragraph(cursorPara, newText);
      cursorOffset--;
    } else if (cursorPara > 0) {
      // Merge with previous paragraph
      const prevText = engine.get_paragraph(cursorPara - 1) || '';
      const currText = engine.get_paragraph(cursorPara) || '';
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

    if (cursorOffset < text.length) {
      const newText = text.slice(0, cursorOffset) + text.slice(cursorOffset + 1);
      engine.set_paragraph(cursorPara, newText);
    } else if (cursorPara < engine.paragraph_count() - 1) {
      // Merge with next paragraph
      const nextText = engine.get_paragraph(cursorPara + 1) || '';
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
          cursorPara = newPos.para;
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
      const linesOnPage = displayLines.filter((dl: { page_index: number }) => dl.page_index === pageIdx);

      // Find the line by comparing y positions directly
      let clickedLine = null;
      for (const line of linesOnPage) {
        const lineY = MARGIN_TOP + line.y_position;
        if (y >= lineY && y < lineY + lineHeightPx) {
          clickedLine = line;
          break;
        }
      }

      // If no exact match, find closest line
      if (!clickedLine && linesOnPage.length > 0) {
        let minDist = Infinity;
        for (const line of linesOnPage) {
          const lineY = MARGIN_TOP + line.y_position;
          const dist = Math.abs(y - lineY);
          if (dist < minDist) {
            minDist = dist;
            clickedLine = line;
          }
        }
      }

      if (clickedLine) {
        const para = clickedLine.para_index;
        const text = engine.get_paragraph(para) || '';
        const lineStartOffset = clickedLine.start_offset;
        // x_position from engine includes column offset
        const lineX = clickedLine.x_position || MARGIN_LEFT;

        const ctx = canvas.getContext('2d');
        if (ctx) {
          ctx.font = `${FONT_SIZE}px ${FONT_FAMILY}`;
          let charPos = lineStartOffset;

          // Check if click is before the text starts
          if (x < lineX) {
            return { para, offset: lineStartOffset };
          }

          for (let i = lineStartOffset; i <= clickedLine.end_offset; i++) {
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
          return { para, offset: Math.min(charPos, clickedLine.end_offset) };
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
    if (!isMouseDown || !engine) return;

    const canvas = pageCanvases[pageIdx];
    if (!canvas) return;

    const rect = canvas.getBoundingClientRect();
    // Scale screen coordinates to canvas coordinates (account for zoom)
    const zoomFactor = ZOOM / 100;
    const x = (event.clientX - rect.left) / zoomFactor;
    const y = (event.clientY - rect.top) / zoomFactor;

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
          <button class="popup-close" onclick={closeImageDialog}>×</button>
        </div>
        <div class="popup-content">
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div
            class="drop-zone"
            class:drag-over={dragOver}
            ondragover={(e) => { e.preventDefault(); dragOver = true; }}
            ondragleave={() => { dragOver = false; }}
            ondrop={handleImageDrop}
          >
            <p>Drag and drop an image here</p>
            <p>or</p>
            <label class="file-input-label">
              Choose File
              <input type="file" accept="image/*" onchange={handleImageFileSelect} />
            </label>
          </div>
          <div class="url-input-section">
            <label>Or enter image URL:</label>
            <div class="url-input-row">
              <input
                type="text"
                bind:value={imageUrlInput}
                placeholder="https://example.com/image.jpg"
              />
              <button onclick={insertImageFromUrl}>Insert</button>
            </div>
          </div>
        </div>
      </div>
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

  .drop-zone {
    border: 2px dashed #dadce0;
    border-radius: 8px;
    padding: 40px 20px;
    text-align: center;
    transition: all 0.2s;
  }

  .drop-zone.drag-over {
    border-color: #1a73e8;
    background: #e8f0fe;
  }

  .drop-zone p {
    margin: 8px 0;
    color: #5f6368;
  }

  .file-input-label {
    display: inline-block;
    padding: 8px 16px;
    background: #1a73e8;
    color: white;
    border-radius: 4px;
    cursor: pointer;
    margin-top: 8px;
  }

  .file-input-label:hover {
    background: #1557b0;
  }

  .file-input-label input {
    display: none;
  }

  .url-input-section {
    margin-top: 20px;
    padding-top: 20px;
    border-top: 1px solid #e0e0e0;
  }

  .url-input-section label {
    display: block;
    margin-bottom: 8px;
    color: #5f6368;
    font-size: 14px;
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
</style>
