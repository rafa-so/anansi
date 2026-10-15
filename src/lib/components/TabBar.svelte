<script lang="ts">
  import { tabs } from '../state/tabs.svelte';
</script>

<div class="tabbar" role="tablist" aria-label="Requests abertos">
  {#each tabs.tabs as tab (tab.id)}
    {@const dirty = tabs.isDirty(tab)}
    <div class="tab" class:active={tab.id === tabs.activeId} data-testid="tab">
      <button
        role="tab"
        class="title"
        aria-selected={tab.id === tabs.activeId}
        title={tab.path ?? 'Request novo, ainda não salvo'}
        onclick={() => (tabs.activeId = tab.id)}
        onauxclick={(e) => e.button === 1 && tabs.close(tab.id)}
      >
        <span class="method method-{tab.draft.method}">{tab.draft.method}</span>
        <span class="name" class:removed={tab.diskState === 'removed'}>{tab.draft.name}</span>
        {#if dirty}<span class="dirty" aria-label="não salvo" data-testid="tab-dirty">●</span>{/if}
      </button>
      <button class="close icon" aria-label="Fechar {tab.draft.name}" onclick={() => tabs.close(tab.id)}
        >×</button
      >
    </div>
  {/each}
</div>

<style>
  .tabbar {
    display: flex;
    overflow-x: auto;
    border-bottom: 1px solid var(--border);
    background: var(--bg-soft);
    min-height: 34px;
  }
  .tab {
    display: flex;
    align-items: center;
    border-right: 1px solid var(--border);
    max-width: 240px;
    flex-shrink: 0;
  }
  .tab.active {
    background: var(--bg);
    box-shadow: inset 0 -2px 0 var(--accent);
  }
  .title {
    display: flex;
    align-items: center;
    gap: 6px;
    border: none;
    background: transparent;
    padding: 6px 4px 6px 10px;
    min-width: 0;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .name.removed {
    text-decoration: line-through;
  }
  .dirty {
    color: var(--accent);
    font-size: 10px;
  }
  .close {
    padding: 2px 8px;
  }
</style>
