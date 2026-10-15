<script lang="ts">
  // Tabela chave/valor para query, headers e form-urlencoded.
  // Sempre há uma linha vazia no fim; digitar nela cria uma nova linha.
  import { tick } from 'svelte';
  import type { KeyValue } from '../types';
  import { unresolvedIn } from '../variables';

  let {
    items = $bindable(),
    keyPlaceholder = 'Chave',
    valuePlaceholder = 'Valor',
    label,
    known = null,
  }: {
    items: KeyValue[];
    keyPlaceholder?: string;
    valuePlaceholder?: string;
    label: string;
    /** Nomes do ambiente ativo; `{{x}}` fora dele deixa o campo em vermelho. `null` desliga. */
    known?: ReadonlySet<string> | null;
  } = $props();

  function missing(text: string): string[] {
    return known ? unresolvedIn(text, known) : [];
  }

  function hint(text: string): string | undefined {
    const names = missing(text);
    return names.length ? `Variáveis não definidas: ${names.join(', ')}` : undefined;
  }

  let draftKey = $state('');
  let draftValue = $state('');
  let body = $state<HTMLTableSectionElement | null>(null);

  /** Digitar na linha vazia cria a linha de verdade e leva o foco para ela. */
  async function addFromDraft(field: 'key' | 'value') {
    if (!draftKey && !draftValue) return;
    items.push({ key: draftKey, value: draftValue, enabled: true });
    draftKey = '';
    draftValue = '';
    await tick();
    const rows = body?.querySelectorAll('tr[data-testid="kv-row"]');
    const input = rows?.[rows.length - 1]?.querySelector<HTMLInputElement>(`input[data-field="${field}"]`);
    input?.focus();
    input?.setSelectionRange(input.value.length, input.value.length);
  }

  function remove(index: number) {
    items.splice(index, 1);
  }
</script>

<table class="kv" aria-label={label}>
  <thead>
    <tr>
      <th class="check"><span class="sr-only">Ativo</span></th>
      <th>{keyPlaceholder}</th>
      <th>{valuePlaceholder}</th>
      <th class="actions"><span class="sr-only">Ações</span></th>
    </tr>
  </thead>
  <tbody bind:this={body}>
    {#each items as item, i (i)}
      <tr class:disabled={!item.enabled} data-testid="kv-row">
        <td class="check"
          ><input type="checkbox" bind:checked={item.enabled} aria-label="Ativar {item.key || 'linha'}" /></td
        >
        <td
          ><input
            class="mono"
            data-field="key"
            class:missing={item.enabled && missing(item.key).length > 0}
            title={item.enabled ? hint(item.key) : undefined}
            bind:value={item.key}
            placeholder={keyPlaceholder}
            aria-label="{keyPlaceholder} {i + 1}"
          /></td
        >
        <td
          ><input
            class="mono"
            data-field="value"
            class:missing={item.enabled && missing(item.value).length > 0}
            title={item.enabled ? hint(item.value) : undefined}
            bind:value={item.value}
            placeholder={valuePlaceholder}
            aria-label="{valuePlaceholder} {i + 1}"
          /></td
        >
        <td class="actions"
          ><button class="icon" aria-label="Remover {item.key || 'linha'}" onclick={() => remove(i)}>×</button
          ></td
        >
      </tr>
    {/each}
    <tr class="new">
      <td class="check"></td>
      <td>
        <input
          class="mono"
          bind:value={draftKey}
          oninput={() => addFromDraft('key')}
          placeholder="Adicionar {keyPlaceholder.toLowerCase()}"
          aria-label="Adicionar {keyPlaceholder.toLowerCase()}"
        />
      </td>
      <td>
        <input
          class="mono"
          bind:value={draftValue}
          oninput={() => addFromDraft('value')}
          placeholder={valuePlaceholder}
          aria-label="Valor da nova linha"
        />
      </td>
      <td></td>
    </tr>
  </tbody>
</table>

<style>
  .kv {
    width: 100%;
    border-collapse: collapse;
    table-layout: fixed;
  }
  th {
    text-align: left;
    font-weight: 500;
    color: var(--text-muted);
    padding: 4px;
  }
  td {
    padding: 2px 4px;
  }
  td input:not([type='checkbox']) {
    width: 100%;
  }
  .check {
    width: 30px;
    text-align: center;
  }
  .actions {
    width: 34px;
  }
  input.missing {
    border-color: var(--danger);
    color: var(--danger);
  }
  tr.disabled input:not([type='checkbox']) {
    opacity: 0.5;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip: rect(0 0 0 0);
  }
</style>
