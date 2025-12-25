<script lang="ts">
  import { onMount, tick } from 'svelte';
  import Page from './Page.svelte';
  import Toolbar from './Toolbar.svelte';
  import { pageConfig, zoomLevel, currentPage, totalPages } from './stores';
  import { getContentDimensions, DEFAULT_TEXT_CONFIG, estimateElementHeight } from './types';
  import type { TextConfig } from './types';

  let editorContainer: HTMLDivElement;
  let pagesContainer: HTMLDivElement;
  let pages: { id: number }[] = $state([{ id: 0 }]);
  let activePageIndex = $state(0);
  let pageIdCounter = $state(1);
  let isProcessingOverflow = $state(false);

  const textConfig: TextConfig = DEFAULT_TEXT_CONFIG;

  // Calculate max content height based on page config and zoom (not DOM)
  let contentDims = $derived(getContentDimensions($pageConfig));
  let scaledContentHeight = $derived((contentDims.height * $zoomLevel) / 100);
  let scaledContentWidth = $derived((contentDims.width * $zoomLevel) / 100);

  onMount(() => {
    setupPageObserver();
    setTimeout(() => {
      const firstContent = pagesContainer?.querySelector('.editable-content') as HTMLElement;
      firstContent?.focus();
    }, 100);
  });

  function setupPageObserver() {
    const observer = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting && entry.intersectionRatio > 0.5) {
            const pageNum = parseInt(entry.target.getAttribute('data-page-number') || '1');
            currentPage.set(pageNum);
          }
        });
      },
      {
        root: editorContainer,
        threshold: 0.5,
      }
    );

    const observePages = () => {
      const pageElements = pagesContainer?.querySelectorAll('.page');
      pageElements?.forEach((page) => observer.observe(page));
    };

    setTimeout(observePages, 100);

    $effect(() => {
      if (pages.length) {
        setTimeout(observePages, 50);
      }
    });

    return () => observer.disconnect();
  }

  function handleFormat(command: string, value?: string) {
    document.execCommand(command, false, value);
    setTimeout(() => checkOverflow(), 10);
  }

  function getEditableContents(): HTMLElement[] {
    return Array.from(pagesContainer?.querySelectorAll('.editable-content') || []);
  }

  async function checkOverflow() {
    if (isProcessingOverflow) return;
    isProcessingOverflow = true;

    await tick();

    try {
      const editableContents = getEditableContents();
      if (!editableContents.length) return;

      for (let i = 0; i < editableContents.length; i++) {
        const contentEl = editableContents[i];

        // Use calculation-based height instead of DOM measurements
        const maxHeight = scaledContentHeight;
        const currentHeight = calculateContentHeight(contentEl);

        if (currentHeight > maxHeight) {
          await moveOverflowToNextPage(i, contentEl, maxHeight);
          break;
        }
      }

      await cleanupEmptyPages();
      totalPages.set(pages.length);
    } finally {
      isProcessingOverflow = false;
    }
  }

  // Calculate content height based on element types and text metrics
  function calculateContentHeight(contentEl: HTMLElement): number {
    wrapTextNodes(contentEl);
    const children = Array.from(contentEl.children) as HTMLElement[];

    let totalHeight = 0;
    for (const child of children) {
      totalHeight += estimateElementHeight(child, textConfig, scaledContentWidth, $zoomLevel);
    }

    return totalHeight;
  }

  // Find the index where content overflows based on calculated heights
  function findOverflowIndex(contentEl: HTMLElement, maxHeight: number): number {
    const children = Array.from(contentEl.children) as HTMLElement[];

    let accumulatedHeight = 0;
    for (let i = 0; i < children.length; i++) {
      const elementHeight = estimateElementHeight(children[i], textConfig, scaledContentWidth, $zoomLevel);
      accumulatedHeight += elementHeight;

      if (accumulatedHeight > maxHeight) {
        return i;
      }
    }

    return -1;
  }

  async function moveOverflowToNextPage(pageIndex: number, contentEl: HTMLElement, maxHeight: number) {
    wrapTextNodes(contentEl);

    const children = Array.from(contentEl.children) as HTMLElement[];
    if (children.length === 0) return;

    // Use calculation-based overflow detection instead of DOM measurements
    let overflowStartIndex = findOverflowIndex(contentEl, maxHeight);

    if (overflowStartIndex === -1) return;
    if (overflowStartIndex === 0) overflowStartIndex = 1;

    const overflowElements = children.slice(overflowStartIndex);
    if (overflowElements.length === 0) return;

    // Check if cursor is within any of the overflow elements
    const selection = window.getSelection();
    let cursorInOverflow = false;
    if (selection && selection.rangeCount > 0) {
      const cursorNode = selection.getRangeAt(0).startContainer;
      for (const el of overflowElements) {
        if (el.contains(cursorNode) || el === cursorNode) {
          cursorInOverflow = true;
          break;
        }
      }
    }

    const overflowHTML = overflowElements.map((el) => el.outerHTML).join('');
    overflowElements.forEach((el) => el.remove());

    if (pageIndex + 1 >= pages.length) {
      pages = [...pages, { id: pageIdCounter++ }];
      await tick();
    }

    const editableContents = getEditableContents();
    const nextPageContent = editableContents[pageIndex + 1];
    if (nextPageContent) {
      const existingContent = nextPageContent.innerHTML;
      nextPageContent.innerHTML = overflowHTML + existingContent;
      await tick();

      // If cursor was in overflow content, move focus to next page
      if (cursorInOverflow) {
        nextPageContent.focus();
        activePageIndex = pageIndex + 1;

        // Place cursor at the end of the first element
        const firstChild = nextPageContent.firstChild;
        if (firstChild) {
          const newRange = document.createRange();
          let targetNode: Node = firstChild;
          while (targetNode.lastChild) {
            targetNode = targetNode.lastChild;
          }
          const textLength = targetNode.textContent?.length || 0;
          newRange.setStart(targetNode, textLength);
          newRange.collapse(true);
          selection?.removeAllRanges();
          selection?.addRange(newRange);
        }

        // Scroll the next page into view
        const nextPage = pagesContainer?.querySelectorAll('.page')[pageIndex + 1] as HTMLElement;
        nextPage?.scrollIntoView({ behavior: 'smooth', block: 'center' });
      }

      await checkOverflowRecursive();
    }
  }

  async function checkOverflowRecursive() {
    await tick();
    const editableContents = getEditableContents();

    for (let i = 0; i < editableContents.length; i++) {
      const contentEl = editableContents[i];

      // Use calculation-based height instead of DOM measurements
      const maxHeight = scaledContentHeight;
      const currentHeight = calculateContentHeight(contentEl);

      if (currentHeight > maxHeight) {
        await moveOverflowToNextPage(i, contentEl, maxHeight);
        return;
      }
    }
  }

  function wrapTextNodes(element: HTMLElement) {
    const childNodes = Array.from(element.childNodes);
    childNodes.forEach((node) => {
      if (node.nodeType === Node.TEXT_NODE && node.textContent?.trim()) {
        const div = document.createElement('div');
        div.textContent = node.textContent;
        element.replaceChild(div, node);
      }
    });
  }

  async function cleanupEmptyPages() {
    const editableContents = getEditableContents();
    const emptyPageIndices: number[] = [];

    for (let i = editableContents.length - 1; i > 0; i--) {
      const content = editableContents[i];
      if (!content.textContent?.trim() && !content.querySelector('img')) {
        emptyPageIndices.push(i);
      }
    }

    if (emptyPageIndices.length > 0) {
      pages = pages.filter((_, index) => !emptyPageIndices.includes(index));
      await tick();
    }
  }

  function handleInput() {
    setTimeout(() => checkOverflow(), 10);
  }

  function handleKeyDown(event: KeyboardEvent) {
    if (event.key === 'Enter') {
      setTimeout(() => checkOverflow(), 10);
    }

    if (event.key === 'Backspace') {
      handleBackspaceAtPageStart(event);
    }

    if (event.key === 'Delete') {
      handleDeleteAtPageEnd(event);
    }

    if (event.key === 'ArrowUp' || event.key === 'ArrowDown') {
      handleArrowNavigation(event);
    }
  }

  function handleBackspaceAtPageStart(event: KeyboardEvent) {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return;

    const range = selection.getRangeAt(0);
    if (!range.collapsed) return;

    const contentEl = (event.target as HTMLElement).closest('.editable-content') as HTMLElement;
    if (!contentEl) return;

    const isAtStart = isCaretAtStart(contentEl);
    if (!isAtStart) return;

    const editableContents = getEditableContents();
    const pageIndex = editableContents.indexOf(contentEl);

    if (pageIndex > 0) {
      event.preventDefault();
      mergeWithPreviousPage(pageIndex);
    }
  }

  function handleDeleteAtPageEnd(event: KeyboardEvent) {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return;

    const range = selection.getRangeAt(0);
    if (!range.collapsed) return;

    const contentEl = (event.target as HTMLElement).closest('.editable-content') as HTMLElement;
    if (!contentEl) return;

    const isAtEnd = isCaretAtEnd(contentEl);
    if (!isAtEnd) return;

    const editableContents = getEditableContents();
    const pageIndex = editableContents.indexOf(contentEl);

    if (pageIndex < editableContents.length - 1) {
      event.preventDefault();
      mergeWithNextPage(pageIndex);
    }
  }

  function isCaretAtStart(element: HTMLElement): boolean {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return false;

    const range = selection.getRangeAt(0);
    const preCaretRange = range.cloneRange();
    preCaretRange.selectNodeContents(element);
    preCaretRange.setEnd(range.startContainer, range.startOffset);

    return preCaretRange.toString().length === 0;
  }

  function isCaretAtEnd(element: HTMLElement): boolean {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return false;

    const range = selection.getRangeAt(0);
    const postCaretRange = range.cloneRange();
    postCaretRange.selectNodeContents(element);
    postCaretRange.setStart(range.endContainer, range.endOffset);

    return postCaretRange.toString().length === 0;
  }

  async function mergeWithPreviousPage(pageIndex: number) {
    const editableContents = getEditableContents();
    if (pageIndex <= 0 || pageIndex >= editableContents.length) return;

    const currentContent = editableContents[pageIndex];
    const prevContent = editableContents[pageIndex - 1];

    const marker = document.createElement('span');
    marker.id = 'cursor-marker';
    prevContent.appendChild(marker);

    prevContent.innerHTML += currentContent.innerHTML;
    pages = pages.filter((_, i) => i !== pageIndex);

    await tick();

    const newMarker = document.getElementById('cursor-marker');
    if (newMarker) {
      const range = document.createRange();
      range.setStartAfter(newMarker);
      range.collapse(true);

      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
      newMarker.remove();
    }

    const newEditableContents = getEditableContents();
    newEditableContents[pageIndex - 1]?.focus();
    checkOverflow();
  }

  async function mergeWithNextPage(pageIndex: number) {
    const editableContents = getEditableContents();
    if (pageIndex >= editableContents.length - 1) return;

    const currentContent = editableContents[pageIndex];
    const nextContent = editableContents[pageIndex + 1];

    currentContent.innerHTML += nextContent.innerHTML;
    pages = pages.filter((_, i) => i !== pageIndex + 1);

    await tick();

    const newEditableContents = getEditableContents();
    newEditableContents[pageIndex]?.focus();
    checkOverflow();
  }

  function handleArrowNavigation(event: KeyboardEvent) {
    const selection = window.getSelection();
    if (!selection || selection.rangeCount === 0) return;

    const contentEl = (event.target as HTMLElement).closest('.editable-content') as HTMLElement;
    if (!contentEl) return;

    const editableContents = getEditableContents();
    const pageIndex = editableContents.indexOf(contentEl);

    if (event.key === 'ArrowUp' && isCaretAtStart(contentEl) && pageIndex > 0) {
      event.preventDefault();
      const prevContent = editableContents[pageIndex - 1];
      prevContent.focus();
      const range = document.createRange();
      range.selectNodeContents(prevContent);
      range.collapse(false);
      selection.removeAllRanges();
      selection.addRange(range);
    }

    if (event.key === 'ArrowDown' && isCaretAtEnd(contentEl) && pageIndex < editableContents.length - 1) {
      event.preventDefault();
      const nextContent = editableContents[pageIndex + 1];
      nextContent.focus();
      const range = document.createRange();
      range.selectNodeContents(nextContent);
      range.collapse(true);
      selection.removeAllRanges();
      selection.addRange(range);
    }
  }

  function handlePaste() {
    setTimeout(() => checkOverflow(), 50);
  }

  function handleFocus(index: number) {
    activePageIndex = index;
  }
</script>

<div class="editor-wrapper">
  <Toolbar onFormat={handleFormat} />

  <div class="editor-container" bind:this={editorContainer}>
    <div class="pages-container" bind:this={pagesContainer}>
      {#each pages as page, index (page.id)}
        <Page pageNumber={index + 1} isActive={activePageIndex === index}>
          <div
            class="editable-content"
            contenteditable="true"
            role="textbox"
            tabindex="0"
            aria-multiline="true"
            oninput={handleInput}
            onkeydown={handleKeyDown}
            onpaste={handlePaste}
            onfocus={() => handleFocus(index)}
            style:font-size="{(16 * $zoomLevel) / 100}px"
            style:line-height="1.5"
          ></div>
        </Page>
      {/each}
    </div>
  </div>
</div>

<style>
  .editor-wrapper {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #f8f9fa;
  }

  .editor-container {
    flex: 1;
    overflow-y: auto;
    padding: 20px;
  }

  .pages-container {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .editable-content {
    outline: none;
    width: 100%;
    height: 100%;
    font-family: 'Arial', sans-serif;
    color: #202124;
    word-wrap: break-word;
    overflow-wrap: break-word;
    overflow: hidden;
  }

  .editable-content :global(p) {
    margin: 0 0 1em 0;
  }

  .editable-content :global(h1) {
    font-size: 2em;
    font-weight: 400;
    margin: 0 0 0.5em 0;
  }

  .editable-content :global(h2) {
    font-size: 1.5em;
    font-weight: 400;
    margin: 0 0 0.5em 0;
  }

  .editable-content :global(h3) {
    font-size: 1.17em;
    font-weight: 500;
    margin: 0 0 0.5em 0;
  }

  .editable-content :global(h4) {
    font-size: 1em;
    font-weight: 500;
    margin: 0 0 0.5em 0;
  }

  .editable-content :global(blockquote) {
    border-left: 4px solid #dadce0;
    margin: 1em 0;
    padding: 0.5em 1em;
    color: #5f6368;
  }

  .editable-content :global(ul),
  .editable-content :global(ol) {
    margin: 0 0 1em 0;
    padding-left: 2em;
  }

  .editable-content :global(li) {
    margin: 0.25em 0;
  }

  .editable-content :global(div) {
    word-wrap: break-word;
    overflow-wrap: break-word;
  }
</style>
