<script lang="ts">
  import { onMount } from 'svelte';
  import { pageConfig, zoomLevel, currentPage, totalPages, fontSize, fontFamily, FONT_SIZES, FONT_FAMILIES, lineHeight, letterSpacing, paragraphSpacing, LINE_HEIGHT_OPTIONS, LETTER_SPACING_OPTIONS, PARAGRAPH_SPACING_OPTIONS } from './stores';
  import { PAGE_FORMATS, DEFAULT_MARGINS, NARROW_MARGINS, WIDE_MARGINS } from './types';

  interface Props {
    onFormat: (command: string, value?: string) => void;
    canUndo?: boolean;
    canRedo?: boolean;
  }

  let { onFormat, canUndo = false, canRedo = false }: Props = $props();

  let showPageSettings = $state(false);
  let showColumnsPopup = $state(false);
  let showHeadingDropdown = $state(false);
  let showSpacingPopup = $state(false);
  let currentBlockType = $state('p');

  onMount(() => {
    const handleClickOutside = (e: MouseEvent) => {
      const target = e.target as HTMLElement;
      if (!target.closest('.dropdown-wrapper')) {
        showPageSettings = false;
        showColumnsPopup = false;
        showHeadingDropdown = false;
        showSpacingPopup = false;
      }
    };
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  });

  const blockTypeLabels: Record<string, string> = {
    p: 'Normal text',
    h1: 'Heading 1',
    h2: 'Heading 2',
    h3: 'Heading 3',
    h4: 'Heading 4',
    blockquote: 'Quote',
  };

  function selectBlockType(type: string) {
    currentBlockType = type;
    execFormat('formatBlock', type);
    showHeadingDropdown = false;
  }

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

  function setColumns(columns: 1 | 2) {
    pageConfig.update((config) => ({
      ...config,
      columns,
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

  function setFontSize(size: number) {
    // Apply to selected paragraphs via format command
    execFormat('fontSize', size.toString());
  }

  function increaseFontSize() {
    const currentIndex = FONT_SIZES.indexOf($fontSize);
    if (currentIndex < FONT_SIZES.length - 1) {
      setFontSize(FONT_SIZES[currentIndex + 1]);
    }
  }

  function decreaseFontSize() {
    const currentIndex = FONT_SIZES.indexOf($fontSize);
    if (currentIndex > 0) {
      setFontSize(FONT_SIZES[currentIndex - 1]);
    }
  }

  function setFontFamily(family: string) {
    fontFamily.set(family);
  }
</script>

<div class="toolbar">
  <div class="toolbar-group">
    <button
      class="toolbar-btn"
      onclick={() => execFormat('undo')}
      title="Undo (Ctrl+Z)"
      disabled={!canUndo}
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M12.5 8c-2.65 0-5.05.99-6.9 2.6L2 7v9h9l-3.62-3.62c1.39-1.16 3.16-1.88 5.12-1.88 3.54 0 6.55 2.31 7.6 5.5l2.37-.78C21.08 11.03 17.15 8 12.5 8z"/>
      </svg>
    </button>
    <button
      class="toolbar-btn"
      onclick={() => execFormat('redo')}
      title="Redo (Ctrl+Y)"
      disabled={!canRedo}
    >
      <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M18.4 10.6C16.55 8.99 14.15 8 11.5 8c-4.65 0-8.58 3.03-9.96 7.22L3.9 16c1.05-3.19 4.05-5.5 7.6-5.5 1.95 0 3.73.72 5.12 1.88L13 16h9V7l-3.6 3.6z"/>
      </svg>
    </button>
  </div>

  <div class="toolbar-divider"></div>

  <div class="toolbar-group">
    <div class="dropdown-wrapper">
      <button
        class="toolbar-btn heading-dropdown-btn"
        class:active={showHeadingDropdown}
        onclick={() => { showHeadingDropdown = !showHeadingDropdown; showPageSettings = false; showColumnsPopup = false; }}
        title="Text style"
      >
        <span class="heading-btn-label">{blockTypeLabels[currentBlockType]}</span>
        <svg class="dropdown-arrow" width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
          <path d="M7 10l5 5 5-5z"/>
        </svg>
      </button>
      {#if showHeadingDropdown}
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div class="dropdown-menu heading-dropdown-menu" onclick={(e) => e.stopPropagation()}>
          <button class="heading-option style-p" class:selected={currentBlockType === 'p'} onclick={() => selectBlockType('p')}>
            Normal text
          </button>
          <button class="heading-option style-h1" class:selected={currentBlockType === 'h1'} onclick={() => selectBlockType('h1')}>
            Heading 1
          </button>
          <button class="heading-option style-h2" class:selected={currentBlockType === 'h2'} onclick={() => selectBlockType('h2')}>
            Heading 2
          </button>
          <button class="heading-option style-h3" class:selected={currentBlockType === 'h3'} onclick={() => selectBlockType('h3')}>
            Heading 3
          </button>
          <button class="heading-option style-h4" class:selected={currentBlockType === 'h4'} onclick={() => selectBlockType('h4')}>
            Heading 4
          </button>
          <div class="heading-divider"></div>
          <button class="heading-option style-blockquote" class:selected={currentBlockType === 'blockquote'} onclick={() => selectBlockType('blockquote')}>
            Quote
          </button>
        </div>
      {/if}
    </div>
  </div>

  <div class="toolbar-divider"></div>

  <div class="toolbar-group">
    <select
      class="toolbar-select font-family-select"
      value={$fontFamily}
      onchange={(e) => setFontFamily(e.currentTarget.value)}
      title="Font family"
    >
      {#each FONT_FAMILIES as family}
        <option value={family} style="font-family: {family}">{family}</option>
      {/each}
    </select>
    <button
      class="toolbar-btn font-size-btn"
      onclick={decreaseFontSize}
      title="Decrease font size"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
        <path d="M5.12 14L7.5 7.67 9.87 14H5.12zM6.5 5L1 19h2.25l1.12-3h6.25l1.12 3H14L8.5 5h-2z"/>
        <path d="M18 7h-4v2h4v-2z"/>
      </svg>
    </button>
    <select
      class="toolbar-select font-size-select"
      value={$fontSize}
      onchange={(e) => setFontSize(parseInt(e.currentTarget.value))}
      title="Font size"
    >
      {#each FONT_SIZES as size}
        <option value={size}>{size}</option>
      {/each}
    </select>
    <button
      class="toolbar-btn font-size-btn"
      onclick={increaseFontSize}
      title="Increase font size"
    >
      <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
        <path d="M5.12 14L7.5 7.67 9.87 14H5.12zM6.5 5L1 19h2.25l1.12-3h6.25l1.12 3H14L8.5 5h-2z"/>
        <path d="M20 9h-2v2h-2v2h2v2h2v-2h2v-2h-2V9z"/>
      </svg>
    </button>
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
    <div class="color-picker-wrapper">
      <input
        type="color"
        class="color-input"
        value="#000000"
        onchange={(e) => execFormat('foreColor', e.currentTarget.value)}
        title="Text color"
      />
      <svg class="color-icon" width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M11 2L5.5 16h2.25l1.12-3h6.25l1.12 3h2.25L13 2h-2zm-1.38 9L12 4.67 14.38 11H9.62z"/>
        <rect x="5" y="18" width="14" height="3" fill="currentColor"/>
      </svg>
    </div>
    <div class="color-picker-wrapper">
      <input
        type="color"
        class="color-input"
        value="#FFFF00"
        onchange={(e) => execFormat('hiliteColor', e.currentTarget.value)}
        title="Highlight color"
      />
      <svg class="color-icon" width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
        <path d="M16.56 8.94L7.62 0 6.21 1.41l2.38 2.38-5.15 5.15c-.59.59-.59 1.54 0 2.12l5.5 5.5c.29.29.68.44 1.06.44s.77-.15 1.06-.44l5.5-5.5c.59-.58.59-1.53 0-2.12zM5.21 10L10 5.21 14.79 10H5.21zM19 11.5s-2 2.17-2 3.5c0 1.1.9 2 2 2s2-.9 2-2c0-1.33-2-3.5-2-3.5z"/>
        <rect x="4" y="18" width="14" height="3" fill="#FFFF00"/>
      </svg>
    </div>
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
    <!-- Spacing button -->
    <div class="dropdown-wrapper">
      <button
        class="toolbar-btn"
        class:active={showSpacingPopup}
        onclick={() => { showSpacingPopup = !showSpacingPopup; showPageSettings = false; showColumnsPopup = false; showHeadingDropdown = false; }}
        title="Line and paragraph spacing"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <path d="M6 7h2.5L5 3.5 1.5 7H4v10H1.5L5 20.5 8.5 17H6V7zm4-2v2h12V5H10zm0 14h12v-2H10v2zm0-6h12v-2H10v2z"/>
        </svg>
      </button>
      {#if showSpacingPopup}
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div class="dropdown-menu spacing-menu" onclick={(e) => e.stopPropagation()}>
          <div class="dropdown-section">
            <label class="dropdown-label">Line Height</label>
            <div class="dropdown-options spacing-options">
              {#each LINE_HEIGHT_OPTIONS as option}
                <button
                  class="dropdown-option spacing-option"
                  class:selected={$lineHeight === option.value}
                  onclick={() => lineHeight.set(option.value)}
                >
                  {option.label}
                </button>
              {/each}
            </div>
          </div>
          <div class="dropdown-section">
            <label class="dropdown-label">Letter Spacing</label>
            <div class="dropdown-options spacing-options">
              {#each LETTER_SPACING_OPTIONS as option}
                <button
                  class="dropdown-option spacing-option"
                  class:selected={$letterSpacing === option.value}
                  onclick={() => letterSpacing.set(option.value)}
                >
                  {option.label}
                </button>
              {/each}
            </div>
          </div>
          <div class="dropdown-section">
            <label class="dropdown-label">Paragraph Spacing</label>
            <div class="dropdown-options spacing-options">
              {#each PARAGRAPH_SPACING_OPTIONS as option}
                <button
                  class="dropdown-option spacing-option"
                  class:selected={$paragraphSpacing === option.value}
                  onclick={() => paragraphSpacing.set(option.value)}
                >
                  {option.label}
                </button>
              {/each}
            </div>
          </div>
        </div>
      {/if}
    </div>
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
    <!-- Page Settings Button -->
    <div class="dropdown-wrapper">
      <button
        class="toolbar-btn"
        class:active={showPageSettings}
        onclick={() => { showPageSettings = !showPageSettings; showColumnsPopup = false; }}
        title="Page settings"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <path d="M14 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V8l-6-6zM6 20V4h7v5h5v11H6z"/>
        </svg>
      </button>
      {#if showPageSettings}
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div class="dropdown-menu page-settings-menu" onclick={(e) => e.stopPropagation()}>
          <div class="dropdown-section">
            <label class="dropdown-label">Page Size</label>
            <div class="dropdown-options">
              <button class="dropdown-option" class:selected={$pageConfig.format.name === 'A4'} onclick={() => setFormat('A4')}>A4</button>
              <button class="dropdown-option" class:selected={$pageConfig.format.name === 'A5'} onclick={() => setFormat('A5')}>A5</button>
              <button class="dropdown-option" class:selected={$pageConfig.format.name === 'Letter'} onclick={() => setFormat('LETTER')}>Letter</button>
              <button class="dropdown-option" class:selected={$pageConfig.format.name === 'Legal'} onclick={() => setFormat('LEGAL')}>Legal</button>
              <button class="dropdown-option" class:selected={$pageConfig.format.name === 'US Textbook'} onclick={() => setFormat('TEXTBOOK')}>US Textbook</button>
            </div>
          </div>
          <div class="dropdown-section">
            <label class="dropdown-label">Orientation</label>
            <div class="dropdown-options">
              <button class="dropdown-option orientation-option" class:selected={$pageConfig.orientation === 'portrait'} onclick={() => pageConfig.update(c => ({...c, orientation: 'portrait'}))}>
                <svg width="16" height="20" viewBox="0 0 16 20" fill="none" stroke="currentColor" stroke-width="1.5">
                  <rect x="1" y="1" width="14" height="18" rx="1"/>
                </svg>
                Portrait
              </button>
              <button class="dropdown-option orientation-option" class:selected={$pageConfig.orientation === 'landscape'} onclick={() => pageConfig.update(c => ({...c, orientation: 'landscape'}))}>
                <svg width="20" height="16" viewBox="0 0 20 16" fill="none" stroke="currentColor" stroke-width="1.5">
                  <rect x="1" y="1" width="18" height="14" rx="1"/>
                </svg>
                Landscape
              </button>
            </div>
          </div>
          <div class="dropdown-section">
            <label class="dropdown-label">Margins</label>
            <div class="dropdown-options margins-options">
              <button class="dropdown-option margin-option" class:selected={$pageConfig.margins === DEFAULT_MARGINS} onclick={() => setMargins('default')}>
                <svg width="24" height="30" viewBox="0 0 24 30" fill="none" stroke="currentColor" stroke-width="1">
                  <rect x="1" y="1" width="22" height="28" rx="1"/>
                  <rect x="4" y="4" width="16" height="22" fill="#e0e0e0" stroke="none"/>
                </svg>
                Normal
              </button>
              <button class="dropdown-option margin-option" class:selected={$pageConfig.margins === NARROW_MARGINS} onclick={() => setMargins('narrow')}>
                <svg width="24" height="30" viewBox="0 0 24 30" fill="none" stroke="currentColor" stroke-width="1">
                  <rect x="1" y="1" width="22" height="28" rx="1"/>
                  <rect x="2" y="2" width="20" height="26" fill="#e0e0e0" stroke="none"/>
                </svg>
                Narrow
              </button>
              <button class="dropdown-option margin-option" class:selected={$pageConfig.margins === WIDE_MARGINS} onclick={() => setMargins('wide')}>
                <svg width="24" height="30" viewBox="0 0 24 30" fill="none" stroke="currentColor" stroke-width="1">
                  <rect x="1" y="1" width="22" height="28" rx="1"/>
                  <rect x="6" y="6" width="12" height="18" fill="#e0e0e0" stroke="none"/>
                </svg>
                Wide
              </button>
            </div>
          </div>
        </div>
      {/if}
    </div>

    <!-- Columns Button -->
    <div class="dropdown-wrapper">
      <button
        class="toolbar-btn"
        class:active={showColumnsPopup}
        onclick={() => { showColumnsPopup = !showColumnsPopup; showPageSettings = false; }}
        title="Columns"
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
          <rect x="4" y="4" width="7" height="16" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
          <rect x="13" y="4" width="7" height="16" rx="1" stroke="currentColor" stroke-width="1.5" fill="none"/>
        </svg>
      </button>
      {#if showColumnsPopup}
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div class="dropdown-menu columns-menu" onclick={(e) => e.stopPropagation()}>
          <div class="dropdown-section">
            <label class="dropdown-label">Columns</label>
            <div class="dropdown-options columns-options">
              <button class="dropdown-option column-option" class:selected={$pageConfig.columns === 1} onclick={() => { setColumns(1); showColumnsPopup = false; }}>
                <svg width="32" height="40" viewBox="0 0 32 40" fill="none" stroke="currentColor" stroke-width="1">
                  <rect x="1" y="1" width="30" height="38" rx="1"/>
                  <rect x="4" y="4" width="24" height="32" fill="#e0e0e0" stroke="none"/>
                </svg>
                One
              </button>
              <button class="dropdown-option column-option" class:selected={$pageConfig.columns === 2} onclick={() => { setColumns(2); showColumnsPopup = false; }}>
                <svg width="32" height="40" viewBox="0 0 32 40" fill="none" stroke="currentColor" stroke-width="1">
                  <rect x="1" y="1" width="30" height="38" rx="1"/>
                  <rect x="4" y="4" width="11" height="32" fill="#e0e0e0" stroke="none"/>
                  <rect x="17" y="4" width="11" height="32" fill="#e0e0e0" stroke="none"/>
                </svg>
                Two
              </button>
            </div>
          </div>
        </div>
      {/if}
    </div>
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

  .toolbar-btn.active {
    background: #e8f0fe;
    color: #1a73e8;
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

  .font-family-select {
    width: 130px;
  }

  .font-size-select {
    width: 60px;
  }

  .font-size-btn {
    width: 28px;
    height: 28px;
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

  .color-picker-wrapper {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    border-radius: 4px;
    cursor: pointer;
  }

  .color-picker-wrapper:hover {
    background: #f1f3f4;
  }

  .color-input {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
  }

  .color-icon {
    pointer-events: none;
    color: #5f6368;
  }

  .dropdown-wrapper {
    position: relative;
  }

  .dropdown-menu {
    position: absolute;
    top: 100%;
    left: 0;
    z-index: 1000;
    min-width: 200px;
    padding: 8px;
    background: white;
    border: 1px solid #e0e0e0;
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    margin-top: 4px;
  }

  .dropdown-section {
    margin-bottom: 12px;
  }

  .dropdown-section:last-child {
    margin-bottom: 0;
  }

  .dropdown-label {
    display: block;
    font-size: 11px;
    font-weight: 600;
    color: #5f6368;
    text-transform: uppercase;
    margin-bottom: 6px;
    padding: 0 4px;
  }

  .dropdown-options {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .dropdown-option {
    padding: 6px 12px;
    border: 1px solid #e0e0e0;
    border-radius: 4px;
    background: white;
    font-size: 13px;
    color: #3c4043;
    cursor: pointer;
    transition: all 0.15s;
  }

  .dropdown-option:hover {
    background: #f1f3f4;
    border-color: #d0d0d0;
  }

  .dropdown-option.selected {
    background: #e8f0fe;
    border-color: #1a73e8;
    color: #1a73e8;
  }

  .orientation-option {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 12px;
  }

  .margins-options {
    gap: 8px;
  }

  .margin-option {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    padding: 8px;
    min-width: 60px;
  }

  .columns-options {
    gap: 12px;
  }

  .column-option {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    padding: 8px 16px;
  }

  .page-settings-menu {
    min-width: 280px;
  }

  .columns-menu {
    min-width: 180px;
  }

  /* Heading dropdown styles */
  .heading-dropdown-btn {
    width: auto;
    padding: 0 8px;
    gap: 4px;
  }

  .heading-btn-label {
    font-size: 13px;
    white-space: nowrap;
  }

  .dropdown-arrow {
    flex-shrink: 0;
  }

  .heading-dropdown-menu {
    min-width: 200px;
    padding: 4px;
  }

  .heading-option {
    display: block;
    width: 100%;
    padding: 8px 12px;
    border: none;
    border-radius: 4px;
    background: transparent;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.15s;
  }

  .heading-option:hover {
    background: #f1f3f4;
  }

  .heading-option.selected {
    background: #e8f0fe;
  }

  .heading-divider {
    height: 1px;
    background: #e0e0e0;
    margin: 4px 0;
  }

  .style-p {
    font-size: 14px;
    color: #3c4043;
  }

  .style-h1 {
    font-size: 24px;
    font-weight: 700;
    color: #202124;
  }

  .style-h2 {
    font-size: 20px;
    font-weight: 600;
    color: #202124;
  }

  .style-h3 {
    font-size: 16px;
    font-weight: 600;
    color: #3c4043;
  }

  .style-h4 {
    font-size: 14px;
    font-weight: 600;
    color: #5f6368;
  }

  .style-blockquote {
    font-size: 14px;
    font-style: italic;
    color: #5f6368;
    padding-left: 16px;
    border-left: 3px solid #dadce0;
  }

  /* Spacing menu styles */
  .spacing-menu {
    min-width: 200px;
  }

  .spacing-options {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .spacing-option {
    flex: 1;
    min-width: fit-content;
    padding: 6px 10px;
    font-size: 12px;
    text-align: center;
  }
</style>
