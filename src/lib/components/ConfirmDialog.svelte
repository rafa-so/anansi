<script lang="ts">
  // Diálogo modal da interface: confirmação com escolhas ou pedido de texto.
  import { dialogs } from '../state/dialogs.svelte';

  let input = $state('');
  let inputEl = $state<HTMLInputElement | null>(null);
  let dialogEl = $state<HTMLDivElement | null>(null);

  const current = $derived(dialogs.current);

  $effect(() => {
    if (current?.kind === 'prompt') {
      input = current.value;
      queueMicrotask(() => {
        inputEl?.focus();
        inputEl?.select();
      });
    } else if (current) {
      queueMicrotask(() =>
        dialogEl?.querySelector<HTMLButtonElement>('button.primary, button.danger')?.focus(),
      );
    }
  });

  function cancel() {
    if (!current) return;
    if (current.kind === 'prompt') current.resolve(null);
    else current.resolve('cancel');
  }

  function submitPrompt(e: Event) {
    e.preventDefault();
    if (current?.kind !== 'prompt') return;
    const value = input.trim();
    if (value) current.resolve(value);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      cancel();
    }
  }
</script>

{#if current}
  <div class="overlay" role="presentation" onkeydown={onKeydown}>
    <div
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="dialog-title"
      bind:this={dialogEl}
      data-testid="dialog"
    >
      <h2 id="dialog-title">{current.title}</h2>
      {#if current.kind === 'confirm'}
        <p>{current.message}</p>
        <div class="actions">
          {#each current.choices as choice (choice.id)}
            <button
              class={choice.variant ?? 'default'}
              data-testid={`dialog-${choice.id}`}
              onclick={() => current.kind === 'confirm' && current.resolve(choice.id)}>{choice.label}</button
            >
          {/each}
        </div>
      {:else}
        <form onsubmit={submitPrompt}>
          <label>
            <span>{current.label}</span>
            <input bind:this={inputEl} bind:value={input} data-testid="dialog-input" />
          </label>
          <div class="actions">
            <button type="button" onclick={cancel} data-testid="dialog-cancel">Cancelar</button>
            <button type="submit" class="primary" disabled={!input.trim()} data-testid="dialog-ok"
              >{current.confirmLabel}</button
            >
          </div>
        </form>
      {/if}
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgb(0 0 0 / 0.35);
    display: grid;
    place-items: center;
    z-index: 100;
  }
  .dialog {
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 18px 20px;
    width: min(460px, calc(100vw - 32px));
    box-shadow: 0 12px 40px rgb(0 0 0 / 0.25);
  }
  h2 {
    margin: 0 0 10px;
    font-size: 15px;
  }
  p {
    margin: 0 0 16px;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  label {
    display: grid;
    gap: 6px;
    margin-bottom: 16px;
  }
  label span {
    color: var(--text-muted);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }
</style>
