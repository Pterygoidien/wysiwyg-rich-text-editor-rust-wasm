<script lang="ts">
  import Editor from './lib/Editor.svelte';
  import Sidebar from './lib/Sidebar.svelte';

  let editorRef: ReturnType<typeof Editor>;

  function handleNavigate(paraIndex: number) {
    editorRef?.navigateToParagraph(paraIndex);
  }
</script>

<main>
  <div class="app-layout">
    <Sidebar onNavigate={handleNavigate} />
    <div class="editor-wrapper">
      <Editor bind:this={editorRef} />
    </div>
  </div>
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
  }

  .app-layout {
    display: flex;
    width: 100%;
    height: 100%;
  }

  .editor-wrapper {
    flex: 1;
    overflow: hidden;
  }
</style>
