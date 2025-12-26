/**
 * Bridge between the Svelte UI and the Rust/WASM engine.
 *
 * This module handles:
 * - Loading and initializing the WASM module
 * - Providing text measurement functions to Rust
 * - Executing render commands from Rust
 * - Managing the engine instance lifecycle
 */

// Types for render commands from Rust
export interface RenderCommand {
  type: string;
  [key: string]: unknown;
}

export interface SetFontCommand {
  type: 'setFont';
  font: string;
  size: number;
  bold: boolean;
  italic: boolean;
}

export interface SetFillColorCommand {
  type: 'setFillColor';
  color: string;
}

export interface DrawTextCommand {
  type: 'drawText';
  text: string;
  x: number;
  y: number;
}

export interface DrawTextJustifiedCommand {
  type: 'drawTextJustified';
  words: string[];
  x: number;
  y: number;
  wordSpacing: number;
}

export interface FillRectCommand {
  type: 'fillRect';
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface FillCircleCommand {
  type: 'fillCircle';
  x: number;
  y: number;
  radius: number;
}

export interface DrawImageCommand {
  type: 'drawImage';
  imageId: string;
  x: number;
  y: number;
  width: number;
  height: number;
  cropTop: number;
  cropRight: number;
  cropBottom: number;
  cropLeft: number;
}

export interface DrawCursorCommand {
  type: 'drawCursor';
  x: number;
  y: number;
  height: number;
}

export interface DrawSelectionCommand {
  type: 'drawSelection';
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface DrawPageNumberCommand {
  type: 'drawPageNumber';
  number: number;
  x: number;
  y: number;
}

export interface DrawLineCommand {
  type: 'drawLine';
  x1: number;
  y1: number;
  x2: number;
  y2: number;
  width: number;
}

export interface DrawUnderlineCommand {
  type: 'drawUnderline';
  x: number;
  y: number;
  width: number;
}

export interface DrawStrikethroughCommand {
  type: 'drawStrikethrough';
  x: number;
  y: number;
  width: number;
}

// Position mapping result types
export interface DisplayPosition {
  line: number;
  col: number;
  page: number;
  x: number;
  y: number;
}

export interface ParagraphPosition {
  para: number;
  offset: number;
}

// Paragraph metadata types
export type TextAlign = 'left' | 'center' | 'right' | 'justify';
export type BlockType = 'p' | 'h1' | 'h2' | 'h3' | 'h4' | 'blockquote';
export type ListType = 'none' | 'bullet' | 'numbered';

export interface ParagraphMeta {
  align: TextAlign;
  blockType: BlockType;
  listType: ListType;
}

// Engine type (will be filled when WASM loads)
export interface Engine {
  new(): Engine;
  set_page_config(
    pageWidth: number,
    pageHeight: number,
    marginTop: number,
    marginRight: number,
    marginBottom: number,
    marginLeft: number,
    columns: number,
    columnGap: number
  ): void;
  set_font_config(
    fontSize: number,
    lineHeight: number,
    letterSpacing: number,
    paragraphSpacing: number
  ): void;
  paragraph_count(): number;
  get_paragraph(index: number): string | undefined;
  set_paragraph(index: number, text: string): void;
  insert_paragraph(index: number, text: string): void;
  delete_paragraph(index: number): void;
  recompute_layout(measureFn: (text: string, fontSize: number) => number): boolean;
  page_count(): number;
  get_render_commands(pageIndex: number): string;
  get_display_lines_json(): string;
  display_line_count(): number;

  // Position mapping functions
  para_to_display_pos(paraIndex: number, charOffset: number): string | null;
  display_to_para(line: number, col: number): string | null;
  get_page_for_position(paraIndex: number, charOffset: number): number;

  // Document I/O
  load_document(json: string): void;
  save_document(): string;

  // Paragraph metadata functions
  get_paragraph_meta(index: number): string | null;
  set_block_type(index: number, blockType: string): void;
  set_alignment(index: number, align: string): void;
  set_list_type(index: number, listType: string): void;
  toggle_list(index: number, listType: string): void;

  // Text styling functions
  toggle_bold(paraIndex: number, start: number, end: number): void;
  toggle_italic(paraIndex: number, start: number, end: number): void;
  toggle_underline(paraIndex: number, start: number, end: number): void;
  toggle_strikethrough(paraIndex: number, start: number, end: number): void;
  set_text_color(paraIndex: number, start: number, end: number, color: string): void;
  set_highlight_color(paraIndex: number, start: number, end: number, color: string): void;
  get_paragraph_styles(index: number): string;

  // Image functions
  add_image(id: string, src: string, width: number, height: number, naturalWidth: number, naturalHeight: number): void;
  insert_image_paragraph(index: number, imageId: string): void;
  get_image(id: string): string | null;
  update_image_size(id: string, width: number, height: number): void;
  delete_image(id: string): void;

  // Page break functions
  insert_page_break(index: number): void;

  // List functions
  get_list_type(index: number): string;
  insert_paragraph_with_list(index: number, text: string, sourcePara: number): void;
}

/**
 * Parse position result from engine
 */
export function parseDisplayPosition(json: string | null): DisplayPosition | null {
  if (!json) return null;
  try {
    return JSON.parse(json) as DisplayPosition;
  } catch {
    return null;
  }
}

/**
 * Parse paragraph position result from engine
 */
export function parseParagraphPosition(json: string | null): ParagraphPosition | null {
  if (!json) return null;
  try {
    return JSON.parse(json) as ParagraphPosition;
  } catch {
    return null;
  }
}

/**
 * Parse paragraph metadata result from engine
 */
export function parseParagraphMeta(json: string | null): ParagraphMeta | null {
  if (!json) return null;
  try {
    return JSON.parse(json) as ParagraphMeta;
  } catch {
    return null;
  }
}

// WASM module state
let engineInstance: Engine | null = null;

/**
 * Initialize the WASM engine
 */
export async function initEngine(): Promise<Engine> {
  if (engineInstance) {
    return engineInstance;
  }

  try {
    // Dynamic import of the WASM module
    const wasm = await import('./engine-wasm/editor_engine.js');
    await wasm.default();

    engineInstance = new wasm.Engine() as unknown as Engine;

    console.log('WASM engine initialized');
    return engineInstance;
  } catch (error) {
    console.warn('WASM engine not available, falling back to JS implementation', error);
    throw error;
  }
}

/**
 * Get the current engine instance
 */
export function getEngine(): Engine | null {
  return engineInstance;
}

/**
 * Check if the WASM engine is loaded
 */
export function isEngineLoaded(): boolean {
  return engineInstance !== null;
}

/**
 * Create a text measurement function for the engine
 */
export function createMeasureFunction(
  canvas: HTMLCanvasElement,
  fontFamily: string,
  isBold: boolean,
  isItalic: boolean
): (text: string, fontSize: number) => number {
  const ctx = canvas.getContext('2d');
  if (!ctx) {
    return (text, fontSize) => text.length * fontSize * 0.5;
  }

  return (text: string, fontSize: number): number => {
    const fontStyle = isItalic ? 'italic ' : '';
    const fontWeight = isBold ? 'bold ' : '';
    ctx.font = `${fontStyle}${fontWeight}${fontSize}px ${fontFamily}`;
    return ctx.measureText(text).width;
  };
}

/**
 * Execute render commands on a canvas context
 * Tracks text position for proper rendering of styled segments
 */
export function executeRenderCommands(
  ctx: CanvasRenderingContext2D,
  commands: RenderCommand[],
  loadedImages: Map<string, HTMLImageElement>,
  fontFamily: string
): void {
  // Track current X position for styled text segments
  let currentTextX = 0;
  let lastTextY = 0;
  let lastTextWidth = 0;

  for (let i = 0; i < commands.length; i++) {
    const cmd = commands[i];
    switch (cmd.type) {
      case 'setFont': {
        const c = cmd as unknown as SetFontCommand;
        const style = c.italic ? 'italic ' : '';
        const weight = c.bold ? 'bold ' : '';
        ctx.font = `${style}${weight}${c.size}px ${c.font || fontFamily}`;
        break;
      }

      case 'setFillColor': {
        const c = cmd as unknown as SetFillColorCommand;
        ctx.fillStyle = c.color;
        break;
      }

      case 'setStrokeColor': {
        const c = cmd as unknown as { type: string; color: string };
        ctx.strokeStyle = c.color;
        break;
      }

      case 'drawText': {
        const c = cmd as unknown as DrawTextCommand;
        ctx.fillText(c.text, c.x, c.y);
        // Track position for underline/strikethrough
        currentTextX = c.x;
        lastTextY = c.y;
        lastTextWidth = ctx.measureText(c.text).width;
        break;
      }

      case 'drawTextJustified': {
        const c = cmd as unknown as DrawTextJustifiedCommand;
        let x = c.x;
        for (let j = 0; j < c.words.length; j++) {
          ctx.fillText(c.words[j], x, c.y);
          x += ctx.measureText(c.words[j]).width;
          if (j < c.words.length - 1) {
            x += ctx.measureText(' ').width + c.wordSpacing;
          }
        }
        // Track position
        currentTextX = c.x;
        lastTextY = c.y;
        lastTextWidth = x - c.x;
        break;
      }

      case 'fillRect': {
        const c = cmd as unknown as FillRectCommand;
        // If width is 0, use the measured text width (for highlights)
        const width = c.width > 0 ? c.width : lastTextWidth;
        if (width > 0) {
          ctx.fillRect(c.x, c.y, width, c.height);
        }
        break;
      }

      case 'strokeRect': {
        const c = cmd as unknown as FillRectCommand;
        ctx.strokeRect(c.x, c.y, c.width, c.height);
        break;
      }

      case 'drawLine': {
        const c = cmd as unknown as DrawLineCommand;
        ctx.lineWidth = c.width;
        ctx.beginPath();
        ctx.moveTo(c.x1, c.y1);
        ctx.lineTo(c.x2, c.y2);
        ctx.stroke();
        break;
      }

      case 'drawUnderline': {
        const c = cmd as unknown as DrawUnderlineCommand;
        // Use measured text width if width is 0
        const width = c.width > 0 ? c.width : lastTextWidth;
        if (width > 0) {
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(currentTextX, c.y);
          ctx.lineTo(currentTextX + width, c.y);
          ctx.stroke();
        }
        break;
      }

      case 'drawStrikethrough': {
        const c = cmd as unknown as DrawStrikethroughCommand;
        // Use measured text width if width is 0
        const width = c.width > 0 ? c.width : lastTextWidth;
        if (width > 0) {
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.moveTo(currentTextX, c.y);
          ctx.lineTo(currentTextX + width, c.y);
          ctx.stroke();
        }
        break;
      }

      case 'fillCircle': {
        const c = cmd as unknown as FillCircleCommand;
        ctx.beginPath();
        ctx.arc(c.x, c.y, c.radius, 0, Math.PI * 2);
        ctx.fill();
        break;
      }

      case 'drawImage': {
        const c = cmd as unknown as DrawImageCommand;
        const img = loadedImages.get(c.imageId);
        if (img) {
          const srcX = (c.cropLeft / 100) * img.naturalWidth;
          const srcY = (c.cropTop / 100) * img.naturalHeight;
          const srcW = ((100 - c.cropLeft - c.cropRight) / 100) * img.naturalWidth;
          const srcH = ((100 - c.cropTop - c.cropBottom) / 100) * img.naturalHeight;
          ctx.drawImage(img, srcX, srcY, srcW, srcH, c.x, c.y, c.width, c.height);
        }
        break;
      }

      case 'drawCursor': {
        const c = cmd as unknown as DrawCursorCommand;
        ctx.fillStyle = '#000';
        ctx.fillRect(c.x, c.y, 2, c.height);
        break;
      }

      case 'drawSelection': {
        const c = cmd as unknown as DrawSelectionCommand;
        ctx.fillStyle = '#b4d7ff';
        ctx.fillRect(c.x, c.y, c.width, c.height);
        break;
      }

      case 'drawPageNumber': {
        const c = cmd as unknown as DrawPageNumberCommand;
        ctx.textAlign = 'center';
        ctx.fillText(`${c.number}`, c.x, c.y);
        ctx.textAlign = 'left';
        break;
      }
    }
  }
}

/**
 * Parse render commands JSON from the engine
 */
export function parseRenderCommands(json: string): RenderCommand[] {
  try {
    return JSON.parse(json) as RenderCommand[];
  } catch {
    console.error('Failed to parse render commands:', json);
    return [];
  }
}
