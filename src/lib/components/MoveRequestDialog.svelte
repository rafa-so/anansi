<script lang="ts">
  // "Mover para…" (docs/mover-request): escolhe a pasta de destino de um request.
  import { workspace } from '../state/workspace.svelte';
  import { ui } from '../state/ui.svelte';
  import { fold, listContainers, parentPath, type ContainerOption } from '../tree';
  import Modal from './Modal.svelte';

  let { path, name }: { path: string; name: string } = $props();

  let search = $state('');
  let highlighted = $state<string | null>(null);
  let searchInput = $state<HTMLInputElement | null>(null);

  // O Modal foca o primeiro botão (o "×" do cabeçalho); aqui o foco começa na busca.
  $effect(() => {
    queueMicrotask(() => searchInput?.focus());
  });

  const current = $derived(parentPath(path));
  const all = $derived(listContainers(workspace.tree));
  const options = $derived.by(() => {
    const needle = fold(search.trim());
    return needle ? all.filter((o) => fold(o.label).includes(needle)) : all;
  });
  const enabled = $derived(options.filter((o) => o.path !== current));
  const target = $derived(enabled.find((o) => o.path === highlighted) ?? null);

  const optionId = (p: string) => `move-option-${p.replace(/[^a-zA-Z0-9_-]/g, '_')}`;

  function close() {
    ui.moveRequest = null;
  }

  async function confirm(option: ContainerOption | null = target) {
    if (!option || option.path === current) return;
    // Lê a prop antes de fechar: no App ela é um getter de `ui.moveRequest.path`,
    // que vira `null` no close() (antes do {#if} desmontar o diálogo).
    const from = path;
    close();
    await workspace.move(from, option);
  }

  function step(delta: 1 | -1) {
    if (enabled.length === 0) return;
    const i = enabled.findIndex((o) => o.path === highlighted);
    const next =
      i < 0 ? (delta === 1 ? 0 : enabled.length - 1) : (i + delta + enabled.length) % enabled.length;
    highlighted = enabled[next].path;
    queueMicrotask(() =>
      document.getElementById(optionId(highlighted!))?.scrollIntoView?.({ block: 'nearest' }),
    );
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      step(e.key === 'ArrowDown' ? 1 : -1);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      void confirm();
    }
  }
</script>

<Modal title={`Mover "${name}"`} onclose={close} testid="move-dialog">
  <div class="content">
    <input
      type="search"
      placeholder="Buscar pasta"
      aria-label="Buscar pasta de destino"
      role="combobox"
      aria-expanded="true"
      aria-controls="move-options"
      aria-activedescendant={target ? optionId(target.path) : undefined}
      bind:value={search}
      bind:this={searchInput}
      onkeydown={onKeydown}
      data-testid="move-search"
    />
    {#if options.length === 0}
      <p class="muted empty">Nenhuma pasta encontrada.</p>
    {:else}
      <ul id="move-options" role="listbox" aria-label="Pastas de destino">
        {#each options as option (option.path)}
          {@const isCurrent = option.path === current}
          <!-- O teclado fica no campo de busca (combobox: ↑/↓/Enter), como no padrão ARIA. -->
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <li
            id={optionId(option.path)}
            role="option"
            aria-selected={highlighted === option.path}
            aria-disabled={isCurrent}
            class:current={isCurrent}
            class:highlighted={highlighted === option.path}
            data-testid="move-option"
            data-path={option.path}
            onclick={() => {
              if (!isCurrent) highlighted = option.path;
            }}
            ondblclick={() => confirm(option)}
          >
            <span class="label">{option.label}</span>
            {#if isCurrent}<span class="muted">(pasta atual)</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
  {#snippet footer()}
    <button onclick={close}>Cancelar</button>
    <button class="primary" onclick={() => confirm()} disabled={!target} data-testid="move-confirm"
      >Mover</button
    >
  {/snippet}
</Modal>

<style>
  .content {
    display: grid;
    gap: 8px;
  }
  input {
    width: 100%;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 4px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    max-height: 320px;
    overflow: auto;
  }
  li {
    display: flex;
    gap: 6px;
    align-items: center;
    padding: 4px 8px;
    border-radius: 4px;
    cursor: pointer;
    white-space: nowrap;
  }
  li:hover:not(.current) {
    background: var(--bg-hover);
  }
  li.highlighted {
    background: var(--bg-selected);
  }
  li.current {
    cursor: default;
    color: var(--text-muted);
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .empty {
    margin: 0;
    padding: 8px 0;
  }
</style>
