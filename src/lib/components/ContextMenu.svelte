<script lang="ts">
  import type { MenuItem } from './menu';

  let { x, y, items, onclose }: { x: number; y: number; items: MenuItem[]; onclose: () => void } = $props();

  let menuEl = $state<HTMLDivElement | null>(null);

  $effect(() => {
    menuEl?.querySelector<HTMLButtonElement>('button')?.focus();
    const close = (e: Event) => {
      if (menuEl && e.target instanceof Node && menuEl.contains(e.target)) return;
      onclose();
    };
    window.addEventListener('mousedown', close, true);
    window.addEventListener('blur', onclose);
    return () => {
      window.removeEventListener('mousedown', close, true);
      window.removeEventListener('blur', onclose);
    };
  });

  // Mantém o menu dentro da janela.
  const left = $derived(Math.min(x, window.innerWidth - 220));
  const top = $derived(Math.min(y, window.innerHeight - items.length * 30 - 16));

  function onKeydown(e: KeyboardEvent) {
    const buttons = [...(menuEl?.querySelectorAll<HTMLButtonElement>('button') ?? [])];
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onclose();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      const next = (index + (e.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length;
      buttons[next]?.focus();
    }
  }
</script>

<div
  class="menu"
  role="menu"
  tabindex="-1"
  style:left="{left}px"
  style:top="{top}px"
  bind:this={menuEl}
  onkeydown={onKeydown}
>
  {#each items as item, i (i)}
    {#if item === 'separator'}
      <hr />
    {:else}
      <button
        role="menuitem"
        class:danger={item.danger}
        data-testid={item.testid}
        onclick={() => {
          onclose();
          item.action();
        }}>{item.label}</button
      >
    {/if}
  {/each}
</div>

<style>
  .menu {
    position: fixed;
    z-index: 90;
    min-width: 200px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 4px;
    box-shadow: 0 8px 24px rgb(0 0 0 / 0.2);
    display: grid;
  }
  button {
    border: none;
    background: transparent;
    text-align: left;
    padding: 6px 10px;
    border-radius: 4px;
  }
  button:focus-visible,
  button:hover {
    background: var(--bg-hover);
    outline: none;
  }
  hr {
    border: none;
    border-top: 1px solid var(--border);
    margin: 4px 0;
  }
</style>
