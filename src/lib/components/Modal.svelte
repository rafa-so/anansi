<script lang="ts">
  // Janela modal da interface (ambientes, importar cURL, preferências).
  import type { Snippet } from 'svelte';

  let {
    title,
    onclose,
    children,
    footer,
    wide = false,
    testid,
  }: {
    title: string;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
    wide?: boolean;
    testid?: string;
  } = $props();

  let box = $state<HTMLDivElement | null>(null);

  $effect(() => {
    queueMicrotask(() => box?.querySelector<HTMLElement>('input, select, textarea, button')?.focus());
  });

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onclose();
    }
  }
</script>

<div class="overlay" role="presentation" onkeydown={onKeydown}>
  <div
    class="modal"
    class:wide
    role="dialog"
    aria-modal="true"
    aria-label={title}
    bind:this={box}
    data-testid={testid}
  >
    <header>
      <h2>{title}</h2>
      <button class="icon" aria-label="Fechar" onclick={onclose}>×</button>
    </header>
    <div class="body">{@render children()}</div>
    {#if footer}<footer>{@render footer()}</footer>{/if}
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.35);
    display: grid;
    place-items: center;
    z-index: 80;
  }
  .modal {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    width: min(560px, calc(100vw - 32px));
    max-height: calc(100vh - 48px);
    display: grid;
    grid-template-rows: auto 1fr auto;
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.25);
  }
  .modal.wide {
    width: min(920px, calc(100vw - 32px));
    height: min(640px, calc(100vh - 48px));
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px 4px;
  }
  h2 {
    margin: 0;
    font-size: 15px;
  }
  .body {
    padding: 8px 16px;
    overflow: auto;
    min-height: 0;
  }
  footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 10px 16px 14px;
  }
</style>
