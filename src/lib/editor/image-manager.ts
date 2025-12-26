/**
 * @fileoverview Image management utilities for the rich text editor.
 *
 * This module provides pure functions for image operations including
 * loading, positioning, resizing, cropping, and manipulation state management.
 * All functions operate on data structures without side effects.
 *
 * @module editor/image-manager
 */

import type {
  DocumentImage,
  ImageWrapStyle,
  ImagePositionMode,
  ResizeHandle,
  ParagraphMeta,
} from './types';
import { IMAGE_MARKER, createDefaultMeta } from './types';

// ============================================================================
// Types
// ============================================================================

/**
 * Crop values as percentages (0-100).
 */
export interface CropValues {
  /** Percentage cropped from top */
  top: number;
  /** Percentage cropped from right */
  right: number;
  /** Percentage cropped from bottom */
  bottom: number;
  /** Percentage cropped from left */
  left: number;
}

/**
 * Image bounds in screen coordinates.
 */
export interface ImageBounds {
  /** X position in pixels */
  x: number;
  /** Y position in pixels */
  y: number;
  /** Width in pixels */
  width: number;
  /** Height in pixels */
  height: number;
  /** Page index (0-based) */
  pageIndex: number;
}

/**
 * Result of an image insertion operation.
 */
export interface ImageInsertResult {
  /** Updated paragraphs array */
  paragraphs: string[];
  /** Updated paragraph metadata array */
  paragraphMeta: ParagraphMeta[];
  /** The created document image */
  image: DocumentImage;
  /** New cursor paragraph index */
  cursorPara: number;
  /** New cursor offset */
  cursorOffset: number;
}

/**
 * Result of an image deletion operation.
 */
export interface ImageDeleteResult {
  /** Updated paragraphs array */
  paragraphs: string[];
  /** Updated paragraph metadata array */
  paragraphMeta: ParagraphMeta[];
  /** Updated images array */
  images: DocumentImage[];
  /** New cursor paragraph index */
  cursorPara: number;
  /** New cursor offset */
  cursorOffset: number;
}

/**
 * Resize operation state.
 */
export interface ResizeState {
  /** Whether resizing is active */
  isResizing: boolean;
  /** Which handle is being dragged */
  handle: ResizeHandle;
  /** Mouse X at resize start */
  startX: number;
  /** Mouse Y at resize start */
  startY: number;
  /** Image width at resize start */
  startWidth: number;
  /** Image height at resize start */
  startHeight: number;
}

/**
 * Crop operation state.
 */
export interface CropState {
  /** Whether cropping is active */
  isCropping: boolean;
  /** Which handle is being dragged */
  handle: ResizeHandle;
  /** Mouse X at crop start */
  startX: number;
  /** Mouse Y at crop start */
  startY: number;
  /** Crop values at drag start */
  startValues: CropValues;
  /** Original values for cancel */
  originalValues: CropValues;
}

/**
 * Drag operation state.
 */
export interface DragState {
  /** Whether dragging is active */
  isDragging: boolean;
  /** Mouse X at drag start */
  startX: number;
  /** Mouse Y at drag start */
  startY: number;
  /** Image X at drag start */
  startImageX: number;
  /** Image Y at drag start */
  startImageY: number;
}

// ============================================================================
// Image Loading
// ============================================================================

/**
 * Loads an image from a URL.
 *
 * @param src - Image source URL or data URL
 * @returns Promise resolving to the loaded HTMLImageElement
 *
 * @example
 * ```typescript
 * const img = await loadImage('https://example.com/photo.jpg');
 * console.log(img.naturalWidth, img.naturalHeight);
 * ```
 */
export function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const img = new Image();
    img.crossOrigin = 'anonymous';
    img.onload = () => resolve(img);
    img.onerror = reject;
    img.src = src;
  });
}

/**
 * Generates a unique image ID.
 *
 * @returns Unique string identifier
 */
export function generateImageId(): string {
  return `img-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
}

/**
 * Validates that a file is an image.
 *
 * @param file - File to validate
 * @returns True if file is an image type
 */
export function isImageFile(file: File): boolean {
  return file.type.startsWith('image/');
}

/**
 * Converts a file to a data URL.
 *
 * @param file - Image file to convert
 * @returns Promise resolving to data URL string
 */
export function fileToDataUrl(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const result = e.target?.result;
      if (typeof result === 'string') {
        resolve(result);
      } else {
        reject(new Error('Failed to read file as data URL'));
      }
    };
    reader.onerror = reject;
    reader.readAsDataURL(file);
  });
}

// ============================================================================
// Image Creation
// ============================================================================

/**
 * Creates a DocumentImage from a loaded HTMLImageElement.
 *
 * @param img - Loaded image element
 * @param src - Original image source
 * @param maxWidth - Maximum width constraint
 * @param wrapStyle - Initial wrap style
 * @returns New DocumentImage object
 */
export function createDocumentImage(
  img: HTMLImageElement,
  src: string,
  maxWidth: number,
  wrapStyle: ImageWrapStyle = 'inline'
): DocumentImage {
  const id = generateImageId();

  let width = img.naturalWidth;
  let height = img.naturalHeight;

  // Scale down if exceeds max width
  if (width > maxWidth) {
    const ratio = maxWidth / width;
    width = maxWidth;
    height = height * ratio;
  }

  return {
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
}

/**
 * Inserts an image into the document at the cursor position.
 *
 * @param paragraphs - Current paragraphs
 * @param paragraphMeta - Current paragraph metadata
 * @param cursorPara - Cursor paragraph index
 * @param cursorOffset - Cursor offset within paragraph
 * @param image - Document image to insert
 * @returns Result with updated document state
 */
export function insertImageAtCursor(
  paragraphs: string[],
  paragraphMeta: ParagraphMeta[],
  cursorPara: number,
  cursorOffset: number,
  image: DocumentImage
): ImageInsertResult {
  const newParagraphs = [...paragraphs];
  const newMeta = [...paragraphMeta];

  const before = newParagraphs[cursorPara].substring(0, cursorOffset);
  const after = newParagraphs[cursorPara].substring(cursorOffset);

  // Current paragraph becomes text before cursor
  newParagraphs[cursorPara] = before;

  // Insert image paragraph and text after cursor
  const imagePara = IMAGE_MARKER + image.id;
  newParagraphs.splice(cursorPara + 1, 0, imagePara, after);

  // Add metadata for new paragraphs
  const currentMeta = newMeta[cursorPara] || createDefaultMeta();
  newMeta.splice(
    cursorPara + 1,
    0,
    { ...createDefaultMeta(), align: 'center' }, // Image meta
    { ...currentMeta } // After text meta
  );

  return {
    paragraphs: newParagraphs,
    paragraphMeta: newMeta,
    image,
    cursorPara: cursorPara + 2,
    cursorOffset: 0,
  };
}

// ============================================================================
// Image Deletion
// ============================================================================

/**
 * Deletes an image from the document.
 *
 * @param paragraphs - Current paragraphs
 * @param paragraphMeta - Current paragraph metadata
 * @param images - Current images array
 * @param imageId - ID of image to delete
 * @param cursorPara - Current cursor paragraph
 * @param cursorOffset - Current cursor offset
 * @returns Result with updated document state
 */
export function deleteImage(
  paragraphs: string[],
  paragraphMeta: ParagraphMeta[],
  images: DocumentImage[],
  imageId: string,
  cursorPara: number,
  cursorOffset: number
): ImageDeleteResult {
  // Find the paragraph containing this image
  const imageParaIndex = paragraphs.findIndex(
    (p) => p === IMAGE_MARKER + imageId
  );

  if (imageParaIndex === -1) {
    return {
      paragraphs,
      paragraphMeta,
      images,
      cursorPara,
      cursorOffset,
    };
  }

  // Remove the image from arrays
  const newImages = images.filter((img) => img.id !== imageId);
  const newParagraphs = [
    ...paragraphs.slice(0, imageParaIndex),
    ...paragraphs.slice(imageParaIndex + 1),
  ];
  const newMeta = [
    ...paragraphMeta.slice(0, imageParaIndex),
    ...paragraphMeta.slice(imageParaIndex + 1),
  ];

  // Ensure we have at least one paragraph
  if (newParagraphs.length === 0) {
    return {
      paragraphs: [''],
      paragraphMeta: [createDefaultMeta()],
      images: newImages,
      cursorPara: 0,
      cursorOffset: 0,
    };
  }

  // Adjust cursor position if needed
  let newCursorPara = cursorPara;
  let newCursorOffset = cursorOffset;

  if (newCursorPara >= newParagraphs.length) {
    newCursorPara = newParagraphs.length - 1;
    newCursorOffset = newParagraphs[newCursorPara].length;
  } else if (newCursorPara > imageParaIndex) {
    newCursorPara--;
  }

  return {
    paragraphs: newParagraphs,
    paragraphMeta: newMeta,
    images: newImages,
    cursorPara: newCursorPara,
    cursorOffset: newCursorOffset,
  };
}

// ============================================================================
// Image Properties
// ============================================================================

/**
 * Updates an image's wrap style.
 *
 * @param images - Current images array
 * @param imageId - ID of image to update
 * @param wrapStyle - New wrap style
 * @returns Updated images array
 */
export function updateImageWrapStyle(
  images: DocumentImage[],
  imageId: string,
  wrapStyle: ImageWrapStyle
): DocumentImage[] {
  const index = images.findIndex((img) => img.id === imageId);
  if (index === -1) return images;

  const updatedImage = { ...images[index], wrapStyle };

  // Clear position for inline images or move-with-text mode
  if (
    wrapStyle === 'inline' ||
    updatedImage.positionMode === 'move-with-text'
  ) {
    updatedImage.x = undefined;
    updatedImage.y = undefined;
    updatedImage.pageIndex = undefined;
  }

  const newImages = [...images];
  newImages[index] = updatedImage;
  return newImages;
}

/**
 * Updates an image's position mode.
 *
 * @param images - Current images array
 * @param imageId - ID of image to update
 * @param positionMode - New position mode
 * @returns Updated images array
 */
export function updateImagePositionMode(
  images: DocumentImage[],
  imageId: string,
  positionMode: ImagePositionMode
): DocumentImage[] {
  const index = images.findIndex((img) => img.id === imageId);
  if (index === -1) return images;

  const updatedImage = { ...images[index], positionMode };

  // Clear position when switching to move-with-text
  if (positionMode === 'move-with-text') {
    updatedImage.x = undefined;
    updatedImage.y = undefined;
    updatedImage.pageIndex = undefined;
  }

  const newImages = [...images];
  newImages[index] = updatedImage;
  return newImages;
}

/**
 * Updates an image's horizontal alignment.
 *
 * @param images - Current images array
 * @param imageId - ID of image to update
 * @param align - New alignment
 * @returns Updated images array
 */
export function updateImageHorizontalAlign(
  images: DocumentImage[],
  imageId: string,
  align: 'left' | 'center' | 'right'
): DocumentImage[] {
  const index = images.findIndex((img) => img.id === imageId);
  if (index === -1) return images;

  const newImages = [...images];
  newImages[index] = { ...images[index], horizontalAlign: align };
  return newImages;
}

// ============================================================================
// Resizing
// ============================================================================

/**
 * Calculates new dimensions during a resize operation.
 *
 * @param handle - Which resize handle is being dragged
 * @param deltaX - Mouse X delta (in unscaled units)
 * @param deltaY - Mouse Y delta (in unscaled units)
 * @param startWidth - Width at resize start
 * @param startHeight - Height at resize start
 * @param aspectRatio - Image aspect ratio (width/height)
 * @param maxWidth - Maximum allowed width
 * @returns New width and height
 */
export function calculateResizeDimensions(
  handle: ResizeHandle,
  deltaX: number,
  deltaY: number,
  startWidth: number,
  startHeight: number,
  aspectRatio: number,
  maxWidth: number
): { width: number; height: number } {
  const MIN_SIZE = 50;
  let newWidth = startWidth;
  let newHeight = startHeight;

  switch (handle) {
    case 'se':
    case 'ne':
    case 'e':
      newWidth = Math.max(MIN_SIZE, startWidth + deltaX);
      newHeight = newWidth / aspectRatio;
      break;
    case 'sw':
    case 'nw':
    case 'w':
      newWidth = Math.max(MIN_SIZE, startWidth - deltaX);
      newHeight = newWidth / aspectRatio;
      break;
    case 'n':
    case 's':
      newHeight = Math.max(
        MIN_SIZE,
        startHeight + (handle === 's' ? deltaY : -deltaY)
      );
      newWidth = newHeight * aspectRatio;
      break;
  }

  // Constrain to max width
  if (newWidth > maxWidth) {
    newWidth = maxWidth;
    newHeight = newWidth / aspectRatio;
  }

  return { width: newWidth, height: newHeight };
}

/**
 * Updates an image's dimensions.
 *
 * @param images - Current images array
 * @param imageId - ID of image to update
 * @param width - New width
 * @param height - New height
 * @returns Updated images array
 */
export function updateImageDimensions(
  images: DocumentImage[],
  imageId: string,
  width: number,
  height: number
): DocumentImage[] {
  const index = images.findIndex((img) => img.id === imageId);
  if (index === -1) return images;

  const newImages = [...images];
  newImages[index] = { ...images[index], width, height };
  return newImages;
}

// ============================================================================
// Cropping
// ============================================================================

/**
 * Calculates new crop values during a crop operation.
 *
 * @param handle - Which crop handle is being dragged
 * @param deltaXPercent - Mouse X delta as percentage of image width
 * @param deltaYPercent - Mouse Y delta as percentage of image height
 * @param startValues - Crop values at drag start
 * @returns New crop values
 */
export function calculateCropValues(
  handle: ResizeHandle,
  deltaXPercent: number,
  deltaYPercent: number,
  startValues: CropValues
): CropValues {
  const MIN_VISIBLE = 10; // Minimum 10% must remain visible
  const newCrop = { ...startValues };

  switch (handle) {
    case 'n':
      newCrop.top = Math.max(
        0,
        Math.min(100 - newCrop.bottom - MIN_VISIBLE, startValues.top + deltaYPercent)
      );
      break;
    case 's':
      newCrop.bottom = Math.max(
        0,
        Math.min(100 - newCrop.top - MIN_VISIBLE, startValues.bottom - deltaYPercent)
      );
      break;
    case 'w':
      newCrop.left = Math.max(
        0,
        Math.min(100 - newCrop.right - MIN_VISIBLE, startValues.left + deltaXPercent)
      );
      break;
    case 'e':
      newCrop.right = Math.max(
        0,
        Math.min(100 - newCrop.left - MIN_VISIBLE, startValues.right - deltaXPercent)
      );
      break;
    case 'nw':
      newCrop.top = Math.max(
        0,
        Math.min(100 - newCrop.bottom - MIN_VISIBLE, startValues.top + deltaYPercent)
      );
      newCrop.left = Math.max(
        0,
        Math.min(100 - newCrop.right - MIN_VISIBLE, startValues.left + deltaXPercent)
      );
      break;
    case 'ne':
      newCrop.top = Math.max(
        0,
        Math.min(100 - newCrop.bottom - MIN_VISIBLE, startValues.top + deltaYPercent)
      );
      newCrop.right = Math.max(
        0,
        Math.min(100 - newCrop.left - MIN_VISIBLE, startValues.right - deltaXPercent)
      );
      break;
    case 'sw':
      newCrop.bottom = Math.max(
        0,
        Math.min(100 - newCrop.top - MIN_VISIBLE, startValues.bottom - deltaYPercent)
      );
      newCrop.left = Math.max(
        0,
        Math.min(100 - newCrop.right - MIN_VISIBLE, startValues.left + deltaXPercent)
      );
      break;
    case 'se':
      newCrop.bottom = Math.max(
        0,
        Math.min(100 - newCrop.top - MIN_VISIBLE, startValues.bottom - deltaYPercent)
      );
      newCrop.right = Math.max(
        0,
        Math.min(100 - newCrop.left - MIN_VISIBLE, startValues.right - deltaXPercent)
      );
      break;
  }

  return newCrop;
}

/**
 * Updates an image's crop values.
 *
 * @param images - Current images array
 * @param imageId - ID of image to update
 * @param crop - New crop values
 * @returns Updated images array
 */
export function updateImageCrop(
  images: DocumentImage[],
  imageId: string,
  crop: CropValues
): DocumentImage[] {
  const index = images.findIndex((img) => img.id === imageId);
  if (index === -1) return images;

  const newImages = [...images];
  newImages[index] = {
    ...images[index],
    cropTop: crop.top,
    cropRight: crop.right,
    cropBottom: crop.bottom,
    cropLeft: crop.left,
  };
  return newImages;
}

/**
 * Resets an image's crop to show the full image.
 *
 * @param images - Current images array
 * @param imageId - ID of image to reset
 * @returns Updated images array
 */
export function resetImageCrop(
  images: DocumentImage[],
  imageId: string
): DocumentImage[] {
  return updateImageCrop(images, imageId, {
    top: 0,
    right: 0,
    bottom: 0,
    left: 0,
  });
}

/**
 * Gets current crop values for an image.
 *
 * @param image - Document image
 * @returns Current crop values
 */
export function getImageCropValues(image: DocumentImage): CropValues {
  return {
    top: image.cropTop || 0,
    right: image.cropRight || 0,
    bottom: image.cropBottom || 0,
    left: image.cropLeft || 0,
  };
}

// ============================================================================
// Dragging
// ============================================================================

/**
 * Checks if an image can be dragged (non-inline images only).
 *
 * @param image - Document image to check
 * @returns True if image can be dragged
 */
export function canDragImage(image: DocumentImage): boolean {
  return image.wrapStyle !== 'inline';
}

/**
 * Calculates new position during a drag operation.
 *
 * @param deltaX - Mouse X delta (in unscaled units)
 * @param deltaY - Mouse Y delta (in unscaled units)
 * @param startX - Image X at drag start
 * @param startY - Image Y at drag start
 * @param imageWidth - Image width
 * @param contentWidth - Content area width
 * @param contentHeight - Content area height (per page)
 * @returns New position and page index
 */
export function calculateDragPosition(
  deltaX: number,
  deltaY: number,
  startX: number,
  startY: number,
  imageWidth: number,
  contentWidth: number,
  contentHeight: number
): { x: number; y: number; pageIndex: number } {
  let newX = startX + deltaX;
  let newY = startY + deltaY;

  // Constrain to content area
  const maxX = contentWidth - imageWidth;
  newX = Math.max(0, Math.min(maxX, newX));
  newY = Math.max(0, newY);

  // Calculate page index
  const pageIndex = Math.floor(newY / contentHeight);

  return { x: newX, y: newY, pageIndex };
}

/**
 * Updates an image's position.
 *
 * @param images - Current images array
 * @param imageId - ID of image to update
 * @param x - New X position
 * @param y - New Y position
 * @param pageIndex - New page index
 * @returns Updated images array
 */
export function updateImagePosition(
  images: DocumentImage[],
  imageId: string,
  x: number,
  y: number,
  pageIndex: number
): DocumentImage[] {
  const index = images.findIndex((img) => img.id === imageId);
  if (index === -1) return images;

  const newImages = [...images];
  newImages[index] = { ...images[index], x, y, pageIndex };
  return newImages;
}

// ============================================================================
// Image Bounds Calculation
// ============================================================================

/**
 * Calculates the display aspect ratio accounting for crop.
 *
 * @param image - Document image
 * @param naturalWidth - Original image width
 * @param naturalHeight - Original image height
 * @returns Cropped aspect ratio
 */
export function getCroppedAspectRatio(
  image: DocumentImage,
  naturalWidth: number,
  naturalHeight: number
): number {
  const cropTop = image.cropTop || 0;
  const cropRight = image.cropRight || 0;
  const cropBottom = image.cropBottom || 0;
  const cropLeft = image.cropLeft || 0;

  const srcW = ((100 - cropLeft - cropRight) / 100) * naturalWidth;
  const srcH = ((100 - cropTop - cropBottom) / 100) * naturalHeight;

  return srcW / srcH;
}

/**
 * Calculates displayed dimensions accounting for crop.
 *
 * @param image - Document image
 * @param zoomLevel - Current zoom level (percentage)
 * @returns Display width and height in pixels
 */
export function getDisplayDimensions(
  image: DocumentImage,
  zoomLevel: number
): { width: number; height: number } {
  const scaledWidth = (image.width * zoomLevel) / 100;
  const aspectRatio = getCroppedAspectRatio(
    image,
    image.naturalWidth,
    image.naturalHeight
  );
  const height = scaledWidth / aspectRatio;

  return { width: scaledWidth, height };
}

/**
 * Finds an image by ID.
 *
 * @param images - Images array
 * @param imageId - ID to find
 * @returns The image or undefined
 */
export function findImageById(
  images: DocumentImage[],
  imageId: string
): DocumentImage | undefined {
  return images.find((img) => img.id === imageId);
}

/**
 * Checks if a paragraph is an image marker.
 *
 * @param paragraph - Paragraph text
 * @returns True if paragraph is an image marker
 */
export function isImageParagraph(paragraph: string): boolean {
  return paragraph.startsWith(IMAGE_MARKER);
}

/**
 * Extracts the image ID from an image paragraph.
 *
 * @param paragraph - Image paragraph text
 * @returns The image ID, or null if not an image paragraph
 */
export function getImageIdFromParagraph(paragraph: string): string | null {
  if (!isImageParagraph(paragraph)) return null;
  return paragraph.substring(IMAGE_MARKER.length);
}
