<script lang="ts">
  // Tooltip de variáveis (RF-13 da spec do Bearer Token): mostra o valor de cada
  // {{nome}} do campo. Variável: valor. Segredo: mascarado, com "Mostrar" e
  // "Copiar". Não definido: "Definir", que abre Ambientes.
  // O valor revelado só existe enquanto o tooltip está aberto.
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import { envApi } from '../api/environments';
  import { toasts } from '../state/dialogs.svelte';
  import { environments } from '../state/environments.svelte';
  import { ui } from '../state/ui.svelte';
  import { errorMessage, type Peek } from '../types';

  let {
    names,
    id,
    onclose,
  }: {
    names: string[];
    /** Para o `aria-describedby` do campo. */
    id?: string;
    onclose?: () => void;
  } = $props();

  type Row = { name: string; peek: Peek | null; revealed: string | null; error: string | null };

  // svelte-ignore state_referenced_locally
  let rows = $state<Row[]>(names.map((name) => ({ name, peek: null, revealed: null, error: null })));
  const env = environments.active;

  $effect(() => {
    if (!env) return;
    for (const row of rows) {
      envApi
        .peek(env, row.name)
        .then((p) => (row.peek = p))
        .catch((e) => (row.error = errorMessage(e)));
    }
  });

  async function reveal(row: Row) {
    if (!env) return;
    try {
      row.revealed = await envApi.revealSecret(env, row.name);
    } catch (e) {
      row.error = errorMessage(e);
    }
  }

  async function copy(row: Row) {
    if (!env) return;
    try {
      await writeText(row.revealed ?? (await envApi.revealSecret(env, row.name)));
      toasts.show(`Valor de ${row.name} copiado.`);
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  function define(name: string) {
    ui.environmentsTarget = name;
    ui.environmentsOpen = true;
    onclose?.();
  }
</script>

<div class="var-tooltip" role="tooltip" {id} data-testid="var-tooltip">
  {#if !env}
    <p class="muted">Nenhum ambiente ativo.</p>
  {:else}
    {#each rows as row (row.name)}
      <div class="row" data-testid="var-tooltip-row">
        <div class="head">
          <span class="mono name">{row.name}</span>
          <span class="muted">· {env}</span>
          {#if row.peek?.kind === 'secret'}<span class="tag">segredo</span>{/if}
        </div>
        {#if row.error}
          <p class="error">{row.error}</p>
        {:else if !row.peek}
          <p class="muted">Carregando…</p>
        {:else if row.peek.kind === 'missing' || row.peek.empty}
          <p class="missing" data-testid="var-tooltip-missing">Não definido em "{env}".</p>
          <button onclick={() => define(row.name)} data-testid="var-tooltip-define">Definir</button>
        {:else if row.peek.kind === 'variable'}
          <p class="mono value" data-testid="var-tooltip-value">{row.peek.value}</p>
        {:else}
          <p class="mono value" data-testid="var-tooltip-value">{row.revealed ?? row.peek.masked}</p>
          <div class="actions">
            {#if row.revealed === null}
              <button onclick={() => reveal(row)} data-testid="var-tooltip-reveal">Mostrar</button>
            {:else}
              <button onclick={() => (row.revealed = null)}>Ocultar</button>
            {/if}
            <button onclick={() => copy(row)} data-testid="var-tooltip-copy">Copiar</button>
          </div>
        {/if}
      </div>
    {/each}
  {/if}
</div>

<style>
  .var-tooltip {
    display: grid;
    gap: 8px;
    min-width: 220px;
    max-width: 420px;
    padding: 8px 10px;
    background: var(--bg);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    box-shadow: 0 4px 16px rgb(0 0 0 / 0.18);
    font-size: 12px;
  }
  .row {
    display: grid;
    gap: 4px;
  }
  .head {
    display: flex;
    gap: 4px;
    align-items: baseline;
  }
  .name {
    font-weight: 600;
  }
  .tag {
    margin-left: auto;
    padding: 0 6px;
    border-radius: 8px;
    background: var(--bg-soft);
    color: var(--text-muted);
    font-size: 11px;
  }
  p {
    margin: 0;
  }
  .value {
    overflow-wrap: anywhere;
  }
  .missing,
  .error {
    color: var(--danger);
  }
  .actions {
    display: flex;
    gap: 4px;
  }
  button {
    justify-self: start;
  }
</style>
