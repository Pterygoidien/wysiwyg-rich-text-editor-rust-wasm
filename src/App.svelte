<script lang="ts">
  import Editor from './lib/Editor.svelte';
  import EditorWasm from './lib/EditorWasm.svelte';
  import Sidebar from './lib/Sidebar.svelte';

  let editorRef: ReturnType<typeof Editor>;
  let useWasm = $state(false);

  function handleNavigate(paraIndex: number) {
    editorRef?.navigateToParagraph(paraIndex);
  }
</script>

<main>
  <div class="mode-toggle">
    <button
      class:active={!useWasm}
      onclick={() => useWasm = false}
    >
      JS Engine
    </button>
    <button
      class:active={useWasm}
      onclick={() => useWasm = true}
    >
      WASM Engine (Rust)
    </button>
  </div>

  {#if useWasm}
    <EditorWasm />
  {:else}
    <div class="app-layout">
      <Sidebar onNavigate={handleNavigate} />
      <div class="editor-wrapper">
        <Editor bind:this={editorRef} />
      </div>
    </div>
  {/if}
</main>

<style>
  :global(*) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
    background: #f8f9fa;
  }

  main {
    width: 100%;
    height: 100vh;
    display: flex;
    flex-direction: column;
  }

  .mode-toggle {
    display: flex;
    gap: 8px;
    padding: 8px 16px;
    background: #1a73e8;
  }

  .mode-toggle button {
    padding: 8px 16px;
    border: none;
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.2);
    color: white;
    cursor: pointer;
    font-size: 14px;
    transition: background 0.2s;
  }

  .mode-toggle button:hover {
    background: rgba(255, 255, 255, 0.3);
  }

  .mode-toggle button.active {
    background: white;
    color: #1a73e8;
  }

  .app-layout {
    display: flex;
    width: 100%;
    flex: 1;
    overflow: hidden;
  }

  .editor-wrapper {
    flex: 1;
    overflow: hidden;
  }
</style>
