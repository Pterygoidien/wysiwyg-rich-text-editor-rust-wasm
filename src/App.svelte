<script lang="ts">
  import Editor from './lib/Editor.svelte';
</script>

<main>
  <Editor />
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
</style>
