<script lang="ts">
  import { headings, type HeadingItem } from './stores';

  interface Props {
    onNavigate: (paraIndex: number) => void;
  }

  let { onNavigate }: Props = $props();

  // Build hierarchical tree from flat headings list
  interface TreeNode extends HeadingItem {
    children: TreeNode[];
  }

  function buildTree(items: HeadingItem[]): TreeNode[] {
    const root: TreeNode[] = [];
    const stack: TreeNode[] = [];

    for (const item of items) {
      const node: TreeNode = { ...item, children: [] };

      // Pop nodes from stack that are >= current level
      while (stack.length > 0 && stack[stack.length - 1].level >= item.level) {
        stack.pop();
      }

      if (stack.length === 0) {
        // Top level item
        root.push(node);
      } else {
        // Child of the last item in stack
        stack[stack.length - 1].children.push(node);
      }

      stack.push(node);
    }

    return root;
  }

  let tree = $derived(buildTree($headings));
</script>

<aside class="sidebar">
  <div class="sidebar-header">
    <h3>Document Outline</h3>
  </div>
  <nav class="sidebar-nav">
    {#if tree.length === 0}
      <p class="empty-message">No headings yet</p>
    {:else}
      <ul class="heading-list">
        {#each tree as node}
          {@render headingNode(node)}
        {/each}
      </ul>
    {/if}
  </nav>
</aside>

{#snippet headingNode(node: TreeNode)}
  <li class="heading-item level-{node.level}">
    <button
      class="heading-link"
      onclick={() => onNavigate(node.paraIndex)}
      title={node.text}
    >
      {node.text || '(Empty heading)'}
    </button>
    {#if node.children.length > 0}
      <ul class="heading-list nested">
        {#each node.children as child}
          {@render headingNode(child)}
        {/each}
      </ul>
    {/if}
  </li>
{/snippet}

<style>
  .sidebar {
    width: 240px;
    min-width: 240px;
    background: #f8f9fa;
    border-right: 1px solid #e0e0e0;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    padding: 12px 16px;
    border-bottom: 1px solid #e0e0e0;
  }

  .sidebar-header h3 {
    margin: 0;
    font-size: 13px;
    font-weight: 600;
    color: #5f6368;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  .sidebar-nav {
    flex: 1;
    overflow-y: auto;
    padding: 8px 0;
  }

  .empty-message {
    padding: 16px;
    margin: 0;
    color: #9aa0a6;
    font-size: 13px;
    font-style: italic;
  }

  .heading-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .heading-list.nested {
    margin-left: 12px;
  }

  .heading-item {
    margin: 0;
  }

  .heading-link {
    display: block;
    width: 100%;
    padding: 6px 16px;
    border: none;
    background: transparent;
    text-align: left;
    font-size: 13px;
    color: #3c4043;
    cursor: pointer;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: background-color 0.15s;
  }

  .heading-link:hover {
    background: #e8eaed;
  }

  .level-1 .heading-link {
    font-weight: 600;
    font-size: 14px;
  }

  .level-2 .heading-link {
    font-weight: 500;
    padding-left: 24px;
  }

  .level-3 .heading-link {
    font-weight: 400;
    padding-left: 32px;
    font-size: 12px;
  }

  .level-4 .heading-link {
    font-weight: 400;
    padding-left: 40px;
    font-size: 12px;
    color: #5f6368;
  }
</style>
