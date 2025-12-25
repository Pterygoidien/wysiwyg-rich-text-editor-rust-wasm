export interface PageFormat {
  name: string;
  width: number;  // in mm
  height: number; // in mm
}

export interface PageMargins {
  top: number;    // in mm
  right: number;  // in mm
  bottom: number; // in mm
  left: number;   // in mm
}

export interface PageConfig {
  format: PageFormat;
  margins: PageMargins;
  orientation: 'portrait' | 'landscape';
}

export const PAGE_FORMATS: Record<string, PageFormat> = {
  A4: { name: 'A4', width: 210, height: 297 },
  A5: { name: 'A5', width: 148, height: 210 },
  LETTER: { name: 'Letter', width: 215.9, height: 279.4 },
  LEGAL: { name: 'Legal', width: 215.9, height: 355.6 },
};

export const DEFAULT_MARGINS: PageMargins = {
  top: 25.4,    // 1 inch
  right: 25.4,
  bottom: 25.4,
  left: 25.4,
};

export const NARROW_MARGINS: PageMargins = {
  top: 12.7,    // 0.5 inch
  right: 12.7,
  bottom: 12.7,
  left: 12.7,
};

export const WIDE_MARGINS: PageMargins = {
  top: 25.4,
  right: 50.8,  // 2 inches
  bottom: 25.4,
  left: 50.8,
};

// Convert mm to pixels at 96 DPI (standard screen resolution)
export function mmToPixels(mm: number): number {
  return (mm / 25.4) * 96;
}

// Get effective page dimensions based on config
export function getPageDimensions(config: PageConfig): { width: number; height: number } {
  const { format, orientation } = config;

  if (orientation === 'landscape') {
    return {
      width: mmToPixels(format.height),
      height: mmToPixels(format.width),
    };
  }

  return {
    width: mmToPixels(format.width),
    height: mmToPixels(format.height),
  };
}

// Get content area dimensions (page minus margins)
export function getContentDimensions(config: PageConfig): { width: number; height: number } {
  const pageDims = getPageDimensions(config);
  const { margins } = config;

  return {
    width: pageDims.width - mmToPixels(margins.left) - mmToPixels(margins.right),
    height: pageDims.height - mmToPixels(margins.top) - mmToPixels(margins.bottom),
  };
}

// Text metrics configuration
export interface TextConfig {
  baseFontSize: number;      // in pixels
  lineHeight: number;        // multiplier (e.g., 1.5)
  paragraphSpacing: number;  // in em units
}

export const DEFAULT_TEXT_CONFIG: TextConfig = {
  baseFontSize: 16,
  lineHeight: 1.5,
  paragraphSpacing: 1,  // 1em margin-bottom
};

// Element height calculations based on text config
export interface ElementHeights {
  line: number;           // single line height
  paragraph: number;      // paragraph with spacing
  h1: number;
  h2: number;
  h3: number;
  h4: number;
  blockquote: number;
  listItem: number;
}

export function calculateElementHeights(textConfig: TextConfig, zoomLevel: number = 100): ElementHeights {
  const scaledFontSize = (textConfig.baseFontSize * zoomLevel) / 100;
  const lineHeight = scaledFontSize * textConfig.lineHeight;
  const paragraphSpacing = scaledFontSize * textConfig.paragraphSpacing;

  return {
    line: lineHeight,
    paragraph: lineHeight + paragraphSpacing,
    h1: (scaledFontSize * 2) * textConfig.lineHeight + (scaledFontSize * 0.5),    // 2em font + 0.5em margin
    h2: (scaledFontSize * 1.5) * textConfig.lineHeight + (scaledFontSize * 0.5),  // 1.5em font + 0.5em margin
    h3: (scaledFontSize * 1.17) * textConfig.lineHeight + (scaledFontSize * 0.5), // 1.17em font + 0.5em margin
    h4: scaledFontSize * textConfig.lineHeight + (scaledFontSize * 0.5),          // 1em font + 0.5em margin
    blockquote: lineHeight + (scaledFontSize * 2),  // line + 1em top/bottom margin
    listItem: lineHeight + (scaledFontSize * 0.25), // line + 0.25em margin
  };
}

// Calculate how many lines fit in content area
export function getMaxLinesPerPage(config: PageConfig, textConfig: TextConfig, zoomLevel: number = 100): number {
  const contentDims = getContentDimensions(config);
  const scaledContentHeight = (contentDims.height * zoomLevel) / 100;
  const heights = calculateElementHeights(textConfig, zoomLevel);

  return Math.floor(scaledContentHeight / heights.line);
}

// Estimate height of an element based on its type
export function estimateElementHeight(
  element: HTMLElement,
  textConfig: TextConfig,
  contentWidth: number,
  zoomLevel: number = 100
): number {
  const heights = calculateElementHeights(textConfig, zoomLevel);
  const tagName = element.tagName.toLowerCase();
  const scaledFontSize = (textConfig.baseFontSize * zoomLevel) / 100;

  // Get font size multiplier based on element type
  let fontSizeMultiplier = 1;
  let marginBottom = textConfig.paragraphSpacing * scaledFontSize;

  switch (tagName) {
    case 'h1':
      fontSizeMultiplier = 2;
      marginBottom = 0.5 * scaledFontSize;
      break;
    case 'h2':
      fontSizeMultiplier = 1.5;
      marginBottom = 0.5 * scaledFontSize;
      break;
    case 'h3':
      fontSizeMultiplier = 1.17;
      marginBottom = 0.5 * scaledFontSize;
      break;
    case 'h4':
      fontSizeMultiplier = 1;
      marginBottom = 0.5 * scaledFontSize;
      break;
    case 'blockquote':
      marginBottom = scaledFontSize; // 1em margin
      break;
    case 'ul':
    case 'ol':
      // Count list items
      const items = element.querySelectorAll('li');
      return items.length * heights.listItem + marginBottom;
    case 'li':
      return heights.listItem;
  }

  // Calculate number of lines based on text content and width
  const text = element.textContent || '';
  const effectiveFontSize = scaledFontSize * fontSizeMultiplier;
  const avgCharWidth = effectiveFontSize * 0.5; // Approximate average character width
  const charsPerLine = Math.floor(contentWidth / avgCharWidth);
  const numLines = Math.max(1, Math.ceil(text.length / charsPerLine));

  const lineHeight = effectiveFontSize * textConfig.lineHeight;
  return (numLines * lineHeight) + marginBottom;
}
