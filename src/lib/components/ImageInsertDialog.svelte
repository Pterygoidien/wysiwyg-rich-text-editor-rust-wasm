<script lang="ts">
  /**
   * @component ImageInsertDialog
   * @description A dialog for inserting images into the document via URL or file upload.
   *
   * This component provides multiple ways to insert images:
   * - Entering an image URL
   * - Dragging and dropping an image file
   * - Clicking to select a file from the filesystem
   *
   * @example
   * ```svelte
   * <ImageInsertDialog
   *   onClose={() => showDialog = false}
   *   onInsertUrl={(url) => insertImage(url)}
   *   onInsertFile={(file) => insertImageFromFile(file)}
   * />
   * ```
   */

  interface Props {
    /** Called when dialog should close */
    onClose: () => void;
    /** Called when an image URL is submitted */
    onInsertUrl: (url: string) => void;
    /** Called when an image file is selected */
    onInsertFile: (file: File) => void;
  }

  let { onClose, onInsertUrl, onInsertFile }: Props = $props();

  let imageUrl = $state('');
  let dragOver = $state(false);

  function handleUrlSubmit() {
    if (imageUrl.trim()) {
      onInsertUrl(imageUrl.trim());
      imageUrl = '';
    }
  }

  function handleFileDrop(event: DragEvent) {
    event.preventDefault();
    dragOver = false;

    const files = event.dataTransfer?.files;
    if (files && files.length > 0) {
      onInsertFile(files[0]);
    }
  }

  function handleFileSelect(event: Event) {
    const input = event.target as HTMLInputElement;
    if (input.files && input.files.length > 0) {
      onInsertFile(input.files[0]);
    }
  }

  function openFileDialog() {
    document.getElementById('image-file-input')?.click();
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
<div class="popup-overlay" onclick={onClose}>
  <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
  <div class="popup-dialog" onclick={(e) => e.stopPropagation()}>
    <div class="popup-header">
      <h3>Insert Image</h3>
      <button class="popup-close" onclick={onClose}>&times;</button>
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
            onkeydown={(e) => e.key === 'Enter' && handleUrlSubmit()}
          />
          <button onclick={handleUrlSubmit}>Insert</button>
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
        onclick={openFileDialog}
      >
        <svg width="48" height="48" viewBox="0 0 24 24" fill="#9aa0a6">
          <path d="M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6v2z"/>
        </svg>
        <p>Drag and drop an image here</p>
        <p class="small">or click to select from your computer</p>
      </div>
      <input
        id="image-file-input"
        type="file"
        accept="image/*"
        class="file-input"
        onchange={handleFileSelect}
      />

      <p class="tip">Tip: You can also paste an image directly in the editor (Ctrl+V)</p>
    </div>
  </div>
</div>

<style>
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
    box-shadow: 0 4px 20px rgba(0, 0, 0, 0.15);
    width: 400px;
    max-width: 90vw;
    overflow: hidden;
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
  }

  .popup-close {
    background: none;
    border: none;
    font-size: 24px;
    cursor: pointer;
    color: #666;
    padding: 0;
    line-height: 1;
  }

  .popup-close:hover {
    color: #333;
  }

  .popup-content {
    padding: 20px;
  }

  .input-section {
    margin-bottom: 16px;
  }

  .input-section label {
    display: block;
    font-size: 14px;
    font-weight: 500;
    margin-bottom: 8px;
    color: #333;
  }

  .url-input-row {
    display: flex;
    gap: 8px;
  }

  .url-input-row input {
    flex: 1;
    padding: 10px 12px;
    border: 1px solid #ddd;
    border-radius: 6px;
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
    border-radius: 6px;
    font-size: 14px;
    cursor: pointer;
    transition: background 0.15s ease;
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
    color: #999;
    font-size: 13px;
  }

  .drop-zone {
    border: 2px dashed #ddd;
    border-radius: 8px;
    padding: 40px 20px;
    text-align: center;
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .drop-zone:hover {
    border-color: #1a73e8;
    background: #f8f9fa;
  }

  .drop-zone.drag-over {
    border-color: #1a73e8;
    background: #e8f0fe;
  }

  .drop-zone p {
    margin: 8px 0 0 0;
    color: #666;
    font-size: 14px;
  }

  .drop-zone p.small {
    font-size: 12px;
    color: #999;
  }

  .file-input {
    display: none;
  }

  .tip {
    margin-top: 16px;
    font-size: 12px;
    color: #999;
    text-align: center;
  }
</style>
