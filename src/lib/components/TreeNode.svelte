<script lang="ts">
  // Nó recursivo da árvore: collection, pasta ou request (RN-10: sem limite de profundidade).
  import type { TreeNode } from '../types';
  import Self from './TreeNode.svelte';

  type Props = {
    node: TreeNode;
    depth?: number;
    open: Set<string>;
    selected: string | null;
    activePath: string | null;
    onselect: (node: TreeNode) => void;
    onactivate: (node: TreeNode) => void;
    oncontext: (node: TreeNode, e: MouseEvent) => void;
  };

  let { node, depth = 0, open, selected, activePath, onselect, onactivate, oncontext }: Props = $props();

  const isOpen = $derived(node.kind !== 'request' && open.has(node.path));
  const error = $derived(node.kind === 'folder' ? undefined : node.error);

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      onactivate(node);
    } else if (e.key === 'ContextMenu' || (e.shiftKey && e.key === 'F10')) {
      e.preventDefault();
      const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
      oncontext(node, new MouseEvent('contextmenu', { clientX: rect.left + 24, clientY: rect.bottom }));
    }
  }
</script>

<li
  role="treeitem"
  aria-expanded={node.kind === 'request' ? undefined : isOpen}
  aria-selected={selected === node.path}
>
  <div
    class="row"
    class:selected={selected === node.path}
    class:active={activePath === node.path}
    class:has-error={!!error}
    style:padding-left="{8 + depth * 14}px"
    tabindex="0"
    role="button"
    title={error ?? (node.kind === 'request' ? node.url : node.name)}
    data-testid="tree-node"
    data-path={node.path}
    onclick={() => {
      onselect(node);
      onactivate(node);
    }}
    oncontextmenu={(e) => {
      e.preventDefault();
      onselect(node);
      oncontext(node, e);
    }}
    onkeydown={onKeydown}
  >
    {#if node.kind === 'request'}
      <span class="method method-{node.method || 'OTHER'}">{node.method || '???'}</span>
    {:else}
      <span class="chevron" aria-hidden="true">{isOpen ? '▾' : '▸'}</span>
    {/if}
    <span class="name" class:collection={node.kind === 'collection'}>{node.name}</span>
    {#if error}<span class="error-icon" aria-label="Erro: {error}">⚠</span>{/if}
  </div>
  {#if isOpen && node.kind !== 'request'}
    <ul role="group">
      {#each node.children as child (child.path)}
        <Self
          node={child}
          depth={depth + 1}
          {open}
          {selected}
          {activePath}
          {onselect}
          {onactivate}
          {oncontext}
        />
      {/each}
    </ul>
  {/if}
</li>

<style>
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    list-style: none;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    height: 26px;
    padding-right: 8px;
    cursor: pointer;
    white-space: nowrap;
    border-radius: 4px;
    user-select: none;
  }
  .row:hover {
    background: var(--bg-hover);
  }
  .row.selected {
    background: var(--bg-selected);
  }
  .row.active .name {
    font-weight: 600;
  }
  .chevron {
    width: 12px;
    color: var(--text-muted);
    font-size: 11px;
  }
  .method {
    width: 44px;
    text-align: right;
    flex-shrink: 0;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .name.collection {
    font-weight: 600;
  }
  .has-error .name {
    color: var(--danger);
  }
  .error-icon {
    color: var(--danger);
  }
</style>
