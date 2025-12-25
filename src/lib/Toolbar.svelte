<script lang="ts">
  import { pageConfig, zoomLevel, currentPage, totalPages } from './stores';
  import { PAGE_FORMATS, DEFAULT_MARGINS, NARROW_MARGINS, WIDE_MARGINS } from './types';

  interface Props {
    onFormat: (command: string, value?: string) => void;
  }

  let { onFormat }: Props = $props();

  function handleZoomChange(delta: number) {
    zoomLevel.update((z) => Math.min(200, Math.max(25, z + delta)));
  }

  function setZoom(value: number) {
    zoomLevel.set(value);
  }

  function setFormat(formatKey: string) {
    pageConfig.update((config) => ({
      ...config,
      format: PAGE_FORMATS[formatKey],
    }));
  }

  function setMargins(marginType: 'default' | 'narrow' | 'wide') {
    const margins = {
      default: DEFAULT_MARGINS,
      narrow: NARROW_MARGINS,
      wide: WIDE_MARGINS,
    };
    pageConfig.update((config) => ({
      ...config,
      margins: margins[marginType],
    }));
  }

  function toggleOrientation() {
    pageConfig.update((config) => ({
      ...config,
      orientation: config.orientation === 'portrait' ? 'landscape' : 'portrait',
    }));
  }

  function execFormat(command: string, value?: string) {
    onFormat(command, value);
  }
</script>

<div class="toolbar">
  <div class="toolbar-group">
    <button
      class="toolbar-btn"
      onclick={() => execFormat('undo')}
      title="Undo (Ctrl+Z)"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12.5 8c-2.65 0-5.05.99-6.9 2.6L2 7v9h9l-3.62-3.62c1.39-1.16 3.16-1.88 5.12-1.88 3.54 0 6.55 2.31 7.6 5.5l2.37-.78C21.08 11.03 17.15 8 12.5 8z"/>
      </svg>
    </button>
    <button
      class="toolbar-btn"
      onclick={() => execFormat('redo')}
      title="Redo (Ctrl+Y)"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M18.4 10.6C16.55 8.99 14.15 8 11.5 8c-4.65 0-8.58 3.03-9.96 7.22L3.9 16c1.05-3.19 4.05-5.5 7.6-5.5 1.95 0 3.73.72 5.12 1.88L13 16h9V7l-3.6 3.6z"/>
      </svg>
    </button>
  </div>

  <div class="toolbar-divider"></div>

  <div class="toolbar-group">
    <select class="toolbar-select" onchange={(e) => execFormat('formatBlock', e.currentTarget.value)}>
      <option value="p">Normal text</option>
      <option value="h1">Heading 1</option>
      <option value="h2">Heading 2</option>
      <option value="h3">Heading 3</option>
      <option value="h4">Heading 4</option>
      <option value="blockquote">Quote</option>
    </select>
  </div>

  <div class="toolbar-divider"></div>

  <div class="toolbar-group">
    <button
      class="toolbar-btn"
      onclick={() => execFormat('bold')}
      title="Bold (Ctrl+B)"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M15.6 10.79c.97-.67 1.65-1.77 1.65-2.79 0-2.26-1.75-4-4-4H7v14h7.04c2.09 0 3.71-1.7 3.71-3.79 0-1.52-.86-2.82-2.15-3.42zM10 6.5h3c.83 0 1.5.67 1.5 1.5s-.67 1.5-1.5 1.5h-3v-3zm3.5 9H10v-3h3.5c.83 0 1.5.67 1.5 1.5s-.67 1.5-1.5 1.5z"/>
      </svg>
    </button>
    <button
      class="toolbar-btn"
      onclick={() => execFormat('italic')}
      title="Italic (Ctrl+I)"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M10 4v3h2.21l-3.42 8H6v3h8v-3h-2.21l3.42-8H18V4z"/>
      </svg>
    </button>
    <button
      class="toolbar-btn"
      onclick={() => execFormat('underline')}
      title="Underline (Ctrl+U)"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12 17c3.31 0 6-2.69 6-6V3h-2.5v8c0 1.93-1.57 3.5-3.5 3.5S8.5 12.93 8.5 11V3H6v8c0 3.31 2.69 6 6 6zm-7 2v2h14v-2H5z"/>
      </svg>
    </button>
    <button
      class="toolbar-btn"
      onclick={() => execFormat('strikeThrough')}
      title="Strikethrough"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M10 19h4v-3h-4v3zM5 4v3h5v3h4V7h5V4H5zM3 14h18v-2H3v2z"/>
      </svg>
    </button>
  </div>

  <div class="toolbar-divider"></div>

  <div class="toolbar-group">
    <button
      class="toolbar-btn"
      onclick={() => execFormat('justifyLeft')}
      title="Align left"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M15 15H3v2h12v-2zm0-8H3v2h12V7zM3 13h18v-2H3v2zm0 8h18v-2H3v2zM3 3v2h18V3H3z"/>
      </svg>
    </button>
    <button
      class="toolbar-btn"
      onclick={() => execFormat('justifyCenter')}
      title="Align center"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M7 15v2h10v-2H7zm-4 6h18v-2H3v2zm0-8h18v-2H3v2zm4-6v2h10V7H7zM3 3v2h18V3H3z"/>
      </svg>
    </button>
    <button
      class="toolbar-btn"
      onclick={() => execFormat('justifyRight')}
      title="Align right"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M3 21h18v-2H3v2zm6-4h12v-2H9v2zm-6-4h18v-2H3v2zm6-4h12V7H9v2zM3 3v2h18V3H3z"/>
      </svg>
    </button>
    <button
      class="toolbar-btn"
      onclick={() => execFormat('justifyFull')}
      title="Justify"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M3 21h18v-2H3v2zm0-4h18v-2H3v2zm0-4h18v-2H3v2zm0-4h18V7H3v2zM3 3v2h18V3H3z"/>
      </svg>
    </button>
  </div>

  <div class="toolbar-divider"></div>

  <div class="toolbar-group">
    <button
      class="toolbar-btn"
      onclick={() => execFormat('insertUnorderedList')}
      title="Bulleted list"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M4 10.5c-.83 0-1.5.67-1.5 1.5s.67 1.5 1.5 1.5 1.5-.67 1.5-1.5-.67-1.5-1.5-1.5zm0-6c-.83 0-1.5.67-1.5 1.5S3.17 7.5 4 7.5 5.5 6.83 5.5 6 4.83 4.5 4 4.5zm0 12c-.83 0-1.5.68-1.5 1.5s.68 1.5 1.5 1.5 1.5-.68 1.5-1.5-.67-1.5-1.5-1.5zM7 19h14v-2H7v2zm0-6h14v-2H7v2zm0-8v2h14V5H7z"/>
      </svg>
    </button>
    <button
      class="toolbar-btn"
      onclick={() => execFormat('insertOrderedList')}
      title="Numbered list"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M2 17h2v.5H3v1h1v.5H2v1h3v-4H2v1zm1-9h1V4H2v1h1v3zm-1 3h1.8L2 13.1v.9h3v-1H3.2L5 10.9V10H2v1zm5-6v2h14V5H7zm0 14h14v-2H7v2zm0-6h14v-2H7v2z"/>
      </svg>
    </button>
  </div>

  <div class="toolbar-divider"></div>

  <div class="toolbar-group">
    <button
      class="toolbar-btn"
      onclick={() => execFormat('insertImage')}
      title="Insert image"
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M21 19V5c0-1.1-.9-2-2-2H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2zM8.5 13.5l2.5 3.01L14.5 12l4.5 6H5l3.5-4.5z"/>
      </svg>
    </button>
  </div>

  <div class="toolbar-divider"></div>

  <div class="toolbar-group page-controls">
    <select class="toolbar-select" onchange={(e) => setFormat(e.currentTarget.value)}>
      <option value="A4" selected={$pageConfig.format.name === 'A4'}>A4</option>
      <option value="A5" selected={$pageConfig.format.name === 'A5'}>A5</option>
      <option value="LETTER" selected={$pageConfig.format.name === 'Letter'}>Letter</option>
      <option value="LEGAL" selected={$pageConfig.format.name === 'Legal'}>Legal</option>
    </select>

    <button
      class="toolbar-btn"
      onclick={toggleOrientation}
      title="Toggle orientation"
    >
      {#if $pageConfig.orientation === 'portrait'}
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14z"/>
        </svg>
      {:else}
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor" style="transform: rotate(90deg)">
          <path d="M19 3H5c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V5h14v14z"/>
        </svg>
      {/if}
    </button>

    <select class="toolbar-select" onchange={(e) => setMargins(e.currentTarget.value as 'default' | 'narrow' | 'wide')}>
      <option value="default">Normal margins</option>
      <option value="narrow">Narrow margins</option>
      <option value="wide">Wide margins</option>
    </select>
  </div>

  <div class="toolbar-divider"></div>

  <div class="toolbar-group zoom-controls">
    <button class="toolbar-btn" onclick={() => handleZoomChange(-10)} title="Zoom out">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14zM7 9h5v1H7z"/>
      </svg>
    </button>
    <select class="toolbar-select zoom-select" value={$zoomLevel} onchange={(e) => setZoom(parseInt(e.currentTarget.value))}>
      <option value="50">50%</option>
      <option value="75">75%</option>
      <option value="100">100%</option>
      <option value="125">125%</option>
      <option value="150">150%</option>
      <option value="200">200%</option>
    </select>
    <button class="toolbar-btn" onclick={() => handleZoomChange(10)} title="Zoom in">
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M15.5 14h-.79l-.28-.27C15.41 12.59 16 11.11 16 9.5 16 5.91 13.09 3 9.5 3S3 5.91 3 9.5 5.91 16 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19l-4.99-5zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14zm.5-7H9v2H7v1h2v2h1v-2h2V9h-2z"/>
      </svg>
    </button>
  </div>

  <div class="toolbar-spacer"></div>

  <div class="toolbar-group page-info">
    <span class="page-indicator">Page {$currentPage} of {$totalPages}</span>
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 16px;
    background: white;
    border-bottom: 1px solid #e0e0e0;
    position: sticky;
    top: 0;
    z-index: 100;
    flex-wrap: wrap;
  }

  .toolbar-group {
    display: flex;
    align-items: center;
    gap: 2px;
  }

  .toolbar-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border: none;
    background: transparent;
    border-radius: 4px;
    cursor: pointer;
    color: #5f6368;
    transition: background-color 0.15s;
  }

  .toolbar-btn:hover {
    background: #f1f3f4;
  }

  .toolbar-btn:active {
    background: #e8eaed;
  }

  .toolbar-select {
    height: 32px;
    padding: 0 8px;
    border: 1px solid transparent;
    border-radius: 4px;
    background: transparent;
    font-size: 13px;
    color: #3c4043;
    cursor: pointer;
  }

  .toolbar-select:hover {
    background: #f1f3f4;
  }

  .toolbar-select:focus {
    outline: none;
    border-color: #1a73e8;
  }

  .zoom-select {
    width: 70px;
  }

  .toolbar-divider {
    width: 1px;
    height: 24px;
    background: #e0e0e0;
    margin: 0 4px;
  }

  .toolbar-spacer {
    flex: 1;
  }

  .page-indicator {
    font-size: 13px;
    color: #5f6368;
    white-space: nowrap;
  }
</style>
