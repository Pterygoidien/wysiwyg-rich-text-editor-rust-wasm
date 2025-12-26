<script lang="ts">
  /**
   * @component ImageOptionsPopup
   * @description A popup dialog for configuring image layout, wrapping, and positioning options.
   *
   * This component provides a comprehensive set of controls for:
   * - Wrap style (inline, square, tight, through, top-bottom, behind, in-front)
   * - Horizontal alignment (left, center, right)
   * - Position mode (move with text, fixed position)
   * - Image editing (crop, reset)
   *
   * @example
   * ```svelte
   * <ImageOptionsPopup
   *   image={selectedImage}
   *   position={{ x: 100, y: 200 }}
   *   onClose={() => showPopup = false}
   *   onWrapStyleChange={(style) => updateWrapStyle(style)}
   *   onHorizontalAlignChange={(align) => updateAlign(align)}
   *   onPositionModeChange={(mode) => updatePositionMode(mode)}
   *   onStartCrop={() => startCropMode()}
   *   onResetCrop={() => resetCrop()}
   *   onDelete={() => deleteImage()}
   * />
   * ```
   */
  import type { DocumentImage, ImageWrapStyle, ImagePositionMode } from '../editor/types';

  interface Props {
    /** The image being configured */
    image: DocumentImage;
    /** Position of the popup */
    position: { x: number; y: number };
    /** Called when popup should close */
    onClose: () => void;
    /** Called when wrap style changes */
    onWrapStyleChange: (style: ImageWrapStyle) => void;
    /** Called when horizontal alignment changes */
    onHorizontalAlignChange: (align: 'left' | 'center' | 'right') => void;
    /** Called when position mode changes */
    onPositionModeChange: (mode: ImagePositionMode) => void;
    /** Called when crop mode should start */
    onStartCrop: () => void;
    /** Called when crop should be reset */
    onResetCrop: () => void;
    /** Called when image should be deleted */
    onDelete: () => void;
  }

  let {
    image,
    position,
    onClose,
    onWrapStyleChange,
    onHorizontalAlignChange,
    onPositionModeChange,
    onStartCrop,
    onResetCrop,
    onDelete,
  }: Props = $props();
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="image-options-overlay" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div
    class="image-options-popup"
    style="left: {position.x}px; top: {position.y}px;"
    onclick={(e) => e.stopPropagation()}
  >
    <div class="image-options-header">
      <span>Layout Options</span>
      <button class="popup-close" onclick={onClose}>&times;</button>
    </div>

    <!-- In Line with Text -->
    <div class="image-options-section">
      <div class="section-label">In Line with Text</div>
      <div class="image-options-buttons">
        <button
          class="layout-btn"
          class:active={image.wrapStyle === 'inline'}
          onclick={() => onWrapStyleChange('inline')}
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
          class:active={image.wrapStyle === 'square'}
          onclick={() => onWrapStyleChange('square')}
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
          class:active={image.wrapStyle === 'tight'}
          onclick={() => onWrapStyleChange('tight')}
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
          class:active={image.wrapStyle === 'through'}
          onclick={() => onWrapStyleChange('through')}
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
          class:active={image.wrapStyle === 'top-bottom'}
          onclick={() => onWrapStyleChange('top-bottom')}
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
          class:active={image.wrapStyle === 'behind'}
          onclick={() => onWrapStyleChange('behind')}
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
          class:active={image.wrapStyle === 'in-front'}
          onclick={() => onWrapStyleChange('in-front')}
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
    {#if image.wrapStyle !== 'inline'}
      <div class="image-options-section">
        <div class="section-label">Horizontal Position</div>
        <div class="image-options-buttons">
          <button
            class="layout-btn small"
            class:active={image.horizontalAlign === 'left'}
            onclick={() => onHorizontalAlignChange('left')}
            title="Align left"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
              <rect x="2" y="6" width="10" height="12" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
            </svg>
          </button>
          <button
            class="layout-btn small"
            class:active={image.horizontalAlign === 'center'}
            onclick={() => onHorizontalAlignChange('center')}
            title="Center"
          >
            <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
              <rect x="7" y="6" width="10" height="12" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
            </svg>
          </button>
          <button
            class="layout-btn small"
            class:active={image.horizontalAlign === 'right'}
            onclick={() => onHorizontalAlignChange('right')}
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
            checked={image.positionMode === 'move-with-text'}
            onchange={() => onPositionModeChange('move-with-text')}
          />
          <span>Move with text</span>
        </label>
        <label class="radio-option">
          <input
            type="radio"
            name="positionMode"
            checked={image.positionMode === 'fixed-position'}
            onchange={() => onPositionModeChange('fixed-position')}
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
          onclick={onStartCrop}
          title="Crop image"
        >
          <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
            <path d="M17 15h2V7c0-1.1-.9-2-2-2H9v2h8v8zM7 17V1H5v4H1v2h4v10c0 1.1.9 2 2 2h10v4h2v-4h4v-2H7z"/>
          </svg>
          <span>Crop</span>
        </button>
        <button
          class="layout-btn"
          onclick={onResetCrop}
          title="Reset crop"
        >
          <svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor">
            <path d="M12 5V1L7 6l5 5V7c3.31 0 6 2.69 6 6s-2.69 6-6 6-6-2.69-6-6H4c0 4.42 3.58 8 8 8s8-3.58 8-8-3.58-8-8-8z"/>
          </svg>
          <span>Reset</span>
        </button>
      </div>
    </div>

    <button class="delete-image-btn" onclick={onDelete}>
      <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
        <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
      </svg>
      Delete Image
    </button>
  </div>
</div>

<style>
  .image-options-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 1000;
  }

  .image-options-popup {
    position: absolute;
    background: white;
    border-radius: 8px;
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    min-width: 280px;
    max-width: 320px;
    z-index: 1001;
    padding: 0;
    overflow: hidden;
  }

  .image-options-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    background: #f8f9fa;
    border-bottom: 1px solid #e0e0e0;
    font-weight: 500;
  }

  .popup-close {
    background: none;
    border: none;
    font-size: 20px;
    cursor: pointer;
    color: #666;
    padding: 0;
    line-height: 1;
  }

  .popup-close:hover {
    color: #333;
  }

  .image-options-section {
    padding: 12px 16px;
    border-bottom: 1px solid #f0f0f0;
  }

  .section-label {
    font-size: 11px;
    color: #666;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    margin-bottom: 8px;
  }

  .image-options-buttons {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }

  .wrap-buttons {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
  }

  .layout-btn {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px;
    border: 1px solid #e0e0e0;
    border-radius: 6px;
    background: white;
    cursor: pointer;
    font-size: 10px;
    color: #666;
    transition: all 0.15s ease;
  }

  .layout-btn:hover {
    border-color: #1a73e8;
    background: #f8f9fa;
  }

  .layout-btn.active {
    border-color: #1a73e8;
    background: #e8f0fe;
    color: #1a73e8;
  }

  .layout-btn.small {
    padding: 6px 12px;
    flex-direction: row;
  }

  .layout-btn svg {
    flex-shrink: 0;
  }

  .position-radio-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .radio-option {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 13px;
    cursor: pointer;
  }

  .radio-option input {
    cursor: pointer;
  }

  .delete-image-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    width: calc(100% - 32px);
    margin: 12px 16px;
    padding: 10px;
    background: #fff;
    border: 1px solid #dc3545;
    border-radius: 6px;
    color: #dc3545;
    font-size: 13px;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .delete-image-btn:hover {
    background: #dc3545;
    color: white;
  }
</style>
