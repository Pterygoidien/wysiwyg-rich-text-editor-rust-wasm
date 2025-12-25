<script lang="ts">
  import { pageConfig, zoomLevel } from './stores';
  import { mmToPixels, getPageDimensions } from './types';

  import type { Snippet } from 'svelte';

  interface Props {
    pageNumber: number;
    isActive?: boolean;
    children: Snippet;
  }

  let { pageNumber, isActive = false, children }: Props = $props();

  let pageDims = $derived(getPageDimensions($pageConfig));
  let scaledWidth = $derived((pageDims.width * $zoomLevel) / 100);
  let scaledHeight = $derived((pageDims.height * $zoomLevel) / 100);

  let marginTop = $derived((mmToPixels($pageConfig.margins.top) * $zoomLevel) / 100);
  let marginRight = $derived((mmToPixels($pageConfig.margins.right) * $zoomLevel) / 100);
  let marginBottom = $derived((mmToPixels($pageConfig.margins.bottom) * $zoomLevel) / 100);
  let marginLeft = $derived((mmToPixels($pageConfig.margins.left) * $zoomLevel) / 100);
</script>

<div
  class="page"
  class:active={isActive}
  style:width="{scaledWidth}px"
  style:height="{scaledHeight}px"
  data-page-number={pageNumber}
>
  <div
    class="page-content"
    style:padding-top="{marginTop}px"
    style:padding-right="{marginRight}px"
    style:padding-bottom="{marginBottom}px"
    style:padding-left="{marginLeft}px"
  >
    {@render children()}
  </div>
  <div class="page-number">{pageNumber}</div>
</div>

<style>
  .page {
    position: relative;
    background: white;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.12), 0 1px 2px rgba(0, 0, 0, 0.24);
    margin: 20px auto;
    flex-shrink: 0;
    overflow: hidden;
  }

  .page.active {
    box-shadow: 0 0 0 2px #1a73e8, 0 1px 3px rgba(0, 0, 0, 0.12), 0 1px 2px rgba(0, 0, 0, 0.24);
  }

  .page-content {
    width: 100%;
    height: 100%;
    box-sizing: border-box;
    overflow: hidden;
  }

  .page-number {
    position: absolute;
    bottom: 8px;
    left: 50%;
    transform: translateX(-50%);
    font-size: 10px;
    color: #999;
    pointer-events: none;
  }
</style>
