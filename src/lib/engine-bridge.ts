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
  para_to_display_pos(paraIndex: number, charOffset: number): string | null;
  load_document(json: string): void;
  save_document(): string;
}

// WASM module state
let wasmModule: { Engine: new() => Engine } | null = null;
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
    // @ts-expect-error - WASM module types
    const wasm = await import('./engine-wasm/editor_engine.js');
    await wasm.default();

    wasmModule = wasm;
    engineInstance = new wasm.Engine();

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
 */
export function executeRenderCommands(
  ctx: CanvasRenderingContext2D,
  commands: RenderCommand[],
  loadedImages: Map<string, HTMLImageElement>,
  fontFamily: string
): void {
  for (const cmd of commands) {
    switch (cmd.type) {
      case 'setFont': {
        const c = cmd as SetFontCommand;
        const style = c.italic ? 'italic ' : '';
        const weight = c.bold ? 'bold ' : '';
        ctx.font = `${style}${weight}${c.size}px ${c.font || fontFamily}`;
        break;
      }

      case 'setFillColor': {
        const c = cmd as SetFillColorCommand;
        ctx.fillStyle = c.color;
        break;
      }

      case 'setStrokeColor': {
        const c = cmd as { type: string; color: string };
        ctx.strokeStyle = c.color;
        break;
      }

      case 'drawText': {
        const c = cmd as DrawTextCommand;
        ctx.fillText(c.text, c.x, c.y);
        break;
      }

      case 'drawTextJustified': {
        const c = cmd as DrawTextJustifiedCommand;
        let x = c.x;
        for (let i = 0; i < c.words.length; i++) {
          ctx.fillText(c.words[i], x, c.y);
          x += ctx.measureText(c.words[i]).width;
          if (i < c.words.length - 1) {
            x += ctx.measureText(' ').width + c.wordSpacing;
          }
        }
        break;
      }

      case 'fillRect': {
        const c = cmd as FillRectCommand;
        ctx.fillRect(c.x, c.y, c.width, c.height);
        break;
      }

      case 'strokeRect': {
        const c = cmd as FillRectCommand;
        ctx.strokeRect(c.x, c.y, c.width, c.height);
        break;
      }

      case 'drawLine': {
        const c = cmd as DrawLineCommand;
        ctx.lineWidth = c.width;
        ctx.beginPath();
        ctx.moveTo(c.x1, c.y1);
        ctx.lineTo(c.x2, c.y2);
        ctx.stroke();
        break;
      }

      case 'fillCircle': {
        const c = cmd as FillCircleCommand;
        ctx.beginPath();
        ctx.arc(c.x, c.y, c.radius, 0, Math.PI * 2);
        ctx.fill();
        break;
      }

      case 'drawImage': {
        const c = cmd as DrawImageCommand;
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
        const c = cmd as DrawCursorCommand;
        ctx.fillStyle = '#000';
        ctx.fillRect(c.x, c.y, 2, c.height);
        break;
      }

      case 'drawSelection': {
        const c = cmd as DrawSelectionCommand;
        ctx.fillStyle = '#b4d7ff';
        ctx.fillRect(c.x, c.y, c.width, c.height);
        break;
      }

      case 'drawPageNumber': {
        const c = cmd as DrawPageNumberCommand;
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
