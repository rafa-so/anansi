<script lang="ts">
  // Editor em cima e resposta embaixo, com divisor arrastável (Seção 13.1).
  import type { Tab } from '../state/tabs.svelte';
  import RequestEditor from './RequestEditor.svelte';
  import ResponseViewer from './ResponseViewer.svelte';

  let { tab }: { tab: Tab } = $props();

  const KEY = 'anansi.split';
  function initialRatio(): number {
    try {
      const v = Number(localStorage.getItem(KEY));
      return v >= 0.2 && v <= 0.8 ? v : 0.5;
    } catch {
      return 0.5;
    }
  }

  let ratio = $state(initialRatio());
  let container = $state<HTMLDivElement | null>(null);

  function startDrag(e: PointerEvent) {
    const target = e.currentTarget as HTMLElement;
    target.setPointerCapture(e.pointerId);
    const move = (ev: PointerEvent) => {
      if (!container) return;
      const rect = container.getBoundingClientRect();
      ratio = Math.min(0.8, Math.max(0.2, (ev.clientY - rect.top) / rect.height));
    };
    const up = () => {
      target.removeEventListener('pointermove', move);
      target.removeEventListener('pointerup', up);
      try {
        localStorage.setItem(KEY, String(ratio));
      } catch {
        // sem armazenamento local: só não lembra a posição
      }
    };
    target.addEventListener('pointermove', move);
    target.addEventListener('pointerup', up);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowUp') ratio = Math.max(0.2, ratio - 0.05);
    else if (e.key === 'ArrowDown') ratio = Math.min(0.8, ratio + 0.05);
  }
</script>

<div class="pane" bind:this={container} style:grid-template-rows="{ratio}fr 6px {1 - ratio}fr">
  <RequestEditor {tab} />
  <!-- Separador focável é um widget interativo no ARIA (setas redimensionam). -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
  <div
    class="splitter"
    role="separator"
    aria-orientation="horizontal"
    aria-label="Redimensionar editor e resposta"
    aria-valuenow={Math.round(ratio * 100)}
    tabindex="0"
    onpointerdown={startDrag}
    onkeydown={onKeydown}
  ></div>
  <ResponseViewer {tab} />
</div>

<style>
  .pane {
    display: grid;
    min-height: 0;
    height: 100%;
  }
  .splitter {
    cursor: row-resize;
    background: var(--bg-soft);
    border-top: 1px solid var(--border);
  }
  .splitter:hover,
  .splitter:focus-visible {
    background: var(--bg-hover);
  }
</style>
