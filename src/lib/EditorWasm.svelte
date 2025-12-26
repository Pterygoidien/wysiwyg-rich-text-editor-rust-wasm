<script lang="ts">
  import { onMount } from 'svelte';
  import init, { Engine } from './engine-wasm/editor_engine.js';
  import { parseRenderCommands, executeRenderCommands, type RenderCommand } from './engine-bridge';

  let canvas: HTMLCanvasElement;
  let measureCanvas: HTMLCanvasElement;
  let hiddenTextarea: HTMLTextAreaElement;
  let engine: Engine | null = null;
  let isReady = $state(false);
  let error = $state<string | null>(null);

  // Editor state
  let cursorPara = $state(0);
  let cursorOffset = $state(0);
  let pageCount = $state(1);

  // Configuration
  const PAGE_WIDTH = 816;
  const PAGE_HEIGHT = 1056;
  const MARGIN = 96;
  const FONT_SIZE = 16;
  const LINE_HEIGHT = 1.5;
  const FONT_FAMILY = 'Arial';

  // Loaded images cache
  const loadedImages = new Map<string, HTMLImageElement>();

  onMount(async () => {
    try {
      // Initialize WASM module
      await init();
      engine = new Engine();

      // Configure the engine
      engine.set_page_config(
        PAGE_WIDTH,
        PAGE_HEIGHT,
        MARGIN,
        MARGIN,
        MARGIN,
        MARGIN,
        1,  // columns
        48  // column gap
      );

      engine.set_font_config(
        FONT_SIZE,
        LINE_HEIGHT,
        0,   // letter spacing
        12   // paragraph spacing
      );

      // Add some initial content
      engine.set_paragraph(0, 'Welcome to the WASM-powered editor!');
      engine.insert_paragraph(1, '');
      engine.insert_paragraph(2, 'This editor uses a Rust/WebAssembly engine for layout computation.');
      engine.insert_paragraph(3, '');
      engine.insert_paragraph(4, 'Try typing to see it in action...');

      isReady = true;
      recomputeAndRender();
      hiddenTextarea?.focus();
    } catch (e) {
      error = `Failed to initialize WASM engine: ${e}`;
      console.error(e);
    }
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

  function recomputeAndRender() {
    if (!engine || !canvas) return;

    // Recompute layout
    const measureFn = createMeasureFunction();
    engine.recompute_layout(measureFn);

    // Update page count
    pageCount = engine.page_count();

    // Render
    render();
  }

  function render() {
    if (!engine || !canvas) return;

    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    // Set up canvas with device pixel ratio
    const dpr = window.devicePixelRatio || 1;
    canvas.width = PAGE_WIDTH * dpr;
    canvas.height = PAGE_HEIGHT * dpr;
    canvas.style.width = `${PAGE_WIDTH}px`;
    canvas.style.height = `${PAGE_HEIGHT}px`;
    ctx.scale(dpr, dpr);

    // Clear canvas
    ctx.fillStyle = 'white';
    ctx.fillRect(0, 0, PAGE_WIDTH, PAGE_HEIGHT);

    // Get render commands from engine
    const commandsJson = engine.get_render_commands(0);
    const commands = parseRenderCommands(commandsJson);

    // Execute render commands
    ctx.textBaseline = 'top';
    executeRenderCommands(ctx, commands, loadedImages, FONT_FAMILY);

    // Draw cursor
    drawCursor(ctx);
  }

  function drawCursor(ctx: CanvasRenderingContext2D) {
    if (!engine) return;

    const posJson = engine.para_to_display_pos(cursorPara, cursorOffset);
    if (!posJson) return;

    try {
      const pos = JSON.parse(posJson);
      if (pos && pos.page === 0) {
        // Measure text to get cursor X position
        const paraText = engine.get_paragraph(cursorPara) || '';
        const textBefore = paraText.substring(0, cursorOffset);

        ctx.font = `${FONT_SIZE}px ${FONT_FAMILY}`;
        const textWidth = ctx.measureText(textBefore).width;

        const lineHeight = FONT_SIZE * LINE_HEIGHT;
        const cursorX = MARGIN + textWidth;
        const cursorY = MARGIN + pos.y;

        ctx.fillStyle = '#000';
        ctx.fillRect(cursorX, cursorY + 2, 2, lineHeight - 4);
      }
    } catch (e) {
      console.error('Failed to parse cursor position:', e);
    }
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (!engine) return;

    const key = event.key;

    // Handle special keys
    if (event.ctrlKey || event.metaKey) {
      // Handle Ctrl+Home and Ctrl+End
      if (key === 'Home') {
        event.preventDefault();
        cursorPara = 0;
        cursorOffset = 0;
        render();
        return;
      } else if (key === 'End') {
        event.preventDefault();
        const lastPara = engine.paragraph_count() - 1;
        cursorPara = lastPara;
        cursorOffset = (engine.get_paragraph(lastPara) || '').length;
        render();
        return;
      }
      return; // Let browser handle Ctrl+C, Ctrl+V, etc.
    }

    event.preventDefault();

    if (key === 'Backspace') {
      handleBackspace();
    } else if (key === 'Delete') {
      handleDelete();
    } else if (key === 'Enter') {
      handleEnter();
    } else if (key === 'ArrowLeft') {
      moveCursor(-1);
    } else if (key === 'ArrowRight') {
      moveCursor(1);
    } else if (key === 'ArrowUp') {
      moveCursorVertical(-1);
    } else if (key === 'ArrowDown') {
      moveCursorVertical(1);
    } else if (key === 'Home') {
      cursorOffset = 0;
      render();
    } else if (key === 'End') {
      cursorOffset = (engine.get_paragraph(cursorPara) || '').length;
      render();
    } else if (key.length === 1) {
      insertChar(key);
    }
  }

  function insertChar(char: string) {
    if (!engine) return;

    const text = engine.get_paragraph(cursorPara) || '';
    const newText = text.slice(0, cursorOffset) + char + text.slice(cursorOffset);
    engine.set_paragraph(cursorPara, newText);
    cursorOffset++;

    recomputeAndRender();
  }

  function handleBackspace() {
    if (!engine) return;

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

    const text = engine.get_paragraph(cursorPara) || '';
    const before = text.slice(0, cursorOffset);
    const after = text.slice(cursorOffset);

    engine.set_paragraph(cursorPara, before);
    engine.insert_paragraph(cursorPara + 1, after);

    cursorPara++;
    cursorOffset = 0;

    recomputeAndRender();
  }

  function moveCursor(delta: number) {
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

    render();
  }

  function moveCursorVertical(delta: number) {
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

    render();
  }

  function handleCanvasClick(event: MouseEvent) {
    if (!engine || !canvas) return;

    const rect = canvas.getBoundingClientRect();
    const x = event.clientX - rect.left;
    const y = event.clientY - rect.top;

    // Use engine's display_to_para for accurate line detection
    const lineHeight = FONT_SIZE * LINE_HEIGHT;
    const clickedLine = Math.floor((y - MARGIN) / lineHeight);
    const displayLineCount = engine.display_line_count();

    if (clickedLine >= 0 && clickedLine < displayLineCount) {
      // Use Rust engine to convert display line to paragraph position
      const posJson = engine.display_to_para(clickedLine, 0);
      if (posJson) {
        try {
          const pos = JSON.parse(posJson);
          cursorPara = pos.para;

          // Find character position based on X
          const text = engine.get_paragraph(cursorPara) || '';
          const ctx = canvas.getContext('2d');
          if (ctx) {
            ctx.font = `${FONT_SIZE}px ${FONT_FAMILY}`;
            let charPos = 0;
            for (let i = 0; i <= text.length; i++) {
              const width = ctx.measureText(text.slice(0, i)).width;
              if (MARGIN + width > x) {
                charPos = Math.max(0, i - 1);
                break;
              }
              charPos = i;
            }
            cursorOffset = charPos;
          }

          render();
        } catch (e) {
          console.error('Failed to parse position:', e);
        }
      }
    }

    hiddenTextarea?.focus();
  }
</script>

<div class="editor-wasm-wrapper">
  <div class="header">
    <h2>WASM Editor Demo</h2>
    <span class="status" class:ready={isReady} class:error={!!error}>
      {#if error}
        {error}
      {:else if isReady}
        Engine Ready - Page {1} of {pageCount}
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
  ></textarea>

  <div class="canvas-container">
    <canvas
      bind:this={canvas}
      class="page-canvas"
      onclick={handleCanvasClick}
    ></canvas>
  </div>
</div>

<style>
  .editor-wasm-wrapper {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #f0f0f0;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 20px;
    background: white;
    border-bottom: 1px solid #e0e0e0;
  }

  .header h2 {
    margin: 0;
    font-size: 18px;
    font-weight: 500;
    color: #202124;
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

  .canvas-container {
    flex: 1;
    overflow: auto;
    display: flex;
    justify-content: center;
    padding: 40px;
  }

  .page-canvas {
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12), 0 1px 2px rgba(0, 0, 0, 0.24);
    cursor: text;
  }
</style>
