<script lang="ts">
  // Busca + árvore + menu de contexto (Seção 13.4).
  import { workspace } from '../state/workspace.svelte';
  import { ui } from '../state/ui.svelte';
  import { requestApi } from '../api/requests';
  import { curlMenuItems } from '../curl';
  import { tabs } from '../state/tabs.svelte';
  import type { TreeNode } from '../types';
  import TreeNodeView from './TreeNode.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import type { MenuItem } from './menu';

  let { searchInput = $bindable(null) }: { searchInput?: HTMLInputElement | null } = $props();

  let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

  const view = $derived(workspace.view);
  const activePath = $derived(tabs.active?.path ?? null);

  function activate(node: TreeNode) {
    if (node.kind === 'request') tabs.openRequest(node.path);
    else workspace.toggle(node.path);
  }

  function containerItems(node: TreeNode): MenuItem[] {
    return [
      { label: 'Nova pasta', action: () => workspace.createFolder(node.path), testid: 'menu-new-folder' },
      {
        label: 'Novo request',
        action: () => {
          workspace.expand(node.path);
          tabs.newRequest(node.path);
        },
        testid: 'menu-new-request',
      },
      {
        label: 'Importar cURL aqui…',
        action: () => (ui.curlImport = { open: true, parentPath: node.path }),
        testid: 'menu-import-curl',
      },
      'separator',
      { label: 'Renomear', action: () => workspace.rename(node), testid: 'menu-rename' },
      { label: 'Remover', action: () => workspace.remove(node), danger: true, testid: 'menu-remove' },
    ];
  }

  function requestItems(node: TreeNode): MenuItem[] {
    return [
      { label: 'Abrir', action: () => tabs.openRequest(node.path), testid: 'menu-open' },
      { label: 'Duplicar', action: () => workspace.duplicate(node.path), testid: 'menu-duplicate' },
      { label: 'Renomear', action: () => workspace.rename(node), testid: 'menu-rename' },
      'separator',
      ...curlMenuItems(() => tabs.tabs.find((t) => t.path === node.path)?.draft ?? requestApi.get(node.path)),
      'separator',
      { label: 'Remover', action: () => workspace.remove(node), danger: true, testid: 'menu-remove' },
    ];
  }

  function openMenu(node: TreeNode, e: MouseEvent) {
    const items = node.kind === 'request' ? requestItems(node) : containerItems(node);
    menu = { x: e.clientX, y: e.clientY, items };
  }
</script>

<aside class="sidebar">
  <div class="search">
    <input
      type="search"
      placeholder="Buscar por nome ou URL (Ctrl+P)"
      aria-label="Buscar requests"
      bind:value={workspace.search}
      bind:this={searchInput}
      data-testid="tree-search"
      onkeydown={(e) => {
        if (e.key === 'Escape' && workspace.search) {
          e.stopPropagation();
          workspace.search = '';
        }
      }}
    />
  </div>
  <div class="tree">
    {#if view.nodes.length === 0}
      <p class="empty muted">
        {workspace.search.trim() ? 'Nada encontrado.' : 'Nenhuma collection ainda. Crie a primeira abaixo.'}
      </p>
    {:else}
      <ul role="tree" aria-label="Collections">
        {#each view.nodes as node (node.path)}
          <TreeNodeView
            {node}
            open={view.open}
            selected={workspace.selected}
            {activePath}
            onselect={(n) => (workspace.selected = n.path)}
            onactivate={activate}
            oncontext={openMenu}
          />
        {/each}
      </ul>
    {/if}
  </div>
  <div class="footer">
    <button onclick={() => workspace.createCollection()} data-testid="new-collection">+ Collection</button>
    <button
      onclick={() => (ui.curlImport = { open: true, parentPath: workspace.targetFor(workspace.selected) })}
      data-testid="import-curl">Importar cURL</button
    >
  </div>
</aside>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />
{/if}

<style>
  .sidebar {
    display: grid;
    grid-template-rows: auto 1fr auto;
    border-right: 1px solid var(--border);
    background: var(--bg-soft);
    min-width: 0;
    min-height: 0;
  }
  .search {
    padding: 8px;
  }
  .search input {
    width: 100%;
  }
  .tree {
    overflow: auto;
    padding: 0 4px 8px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .empty {
    padding: 8px;
    margin: 0;
  }
  .footer {
    display: flex;
    gap: 8px;
    padding: 8px;
    border-top: 1px solid var(--border);
  }
</style>
