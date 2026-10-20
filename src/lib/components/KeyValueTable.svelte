<script lang="ts">
  // Tabela chave/valor para query, headers e form-urlencoded.
  // Sempre há uma linha vazia no fim; digitar nela cria uma nova linha.
  // Com `fileFields`, cada linha escolhe o tipo: texto ou arquivo (o valor vira o caminho).
  import { tick } from 'svelte';
  import { open as openDialog } from '@tauri-apps/plugin-dialog';
  import type { FormField, FormFieldType } from '../types';
  import { fileName } from '../request';
  import { unresolvedIn } from '../variables';

  let {
    items = $bindable(),
    keyPlaceholder = 'Chave',
    valuePlaceholder = 'Valor',
    label,
    known = null,
    fileFields = false,
  }: {
    items: FormField[];
    keyPlaceholder?: string;
    valuePlaceholder?: string;
    label: string;
    /** Nomes do ambiente ativo; `{{x}}` fora dele deixa o campo em vermelho. `null` desliga. */
    known?: ReadonlySet<string> | null;
    /** Mostra a coluna de tipo (texto/arquivo). Só para o body de formulário. */
    fileFields?: boolean;
  } = $props();

  /** Trocar o tipo limpa o valor: um texto não é um caminho de arquivo, e vice-versa. */
  function setType(item: FormField, type: FormFieldType) {
    if ((item.type ?? 'text') === type) return;
    item.value = '';
    if (type === 'file') item.type = 'file';
    else delete item.type;
  }

  /** Escolher o arquivo na linha vazia já cria a linha do tipo arquivo. */
  async function pickDraftFile() {
    const path = await openDialog({ multiple: false, directory: false, title: 'Escolher arquivo' });
    if (typeof path !== 'string') return;
    draftValue = path;
    await addFromDraft('value');
  }

  async function pickFile(item: FormField) {
    const path = await openDialog({
      multiple: false,
      directory: false,
      title: `Arquivo do campo ${item.key}`,
    });
    if (typeof path === 'string') item.value = path;
  }

  function missing(text: string): string[] {
    return known ? unresolvedIn(text, known) : [];
  }

  function hint(text: string): string | undefined {
    const names = missing(text);
    return names.length ? `Variáveis não definidas: ${names.join(', ')}` : undefined;
  }

  let draftKey = $state('');
  let draftValue = $state('');
  let draftType = $state<FormFieldType>('text');
  let body = $state<HTMLTableSectionElement | null>(null);

  /** Digitar na linha vazia cria a linha de verdade e leva o foco para ela. */
  async function addFromDraft(field: 'key' | 'value') {
    if (!draftKey && !draftValue) return;
    const row: FormField = { key: draftKey, value: draftValue, enabled: true };
    if (draftType === 'file') row.type = 'file';
    items.push(row);
    draftKey = '';
    draftValue = '';
    draftType = 'text';
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
      {#if fileFields}<th class="type">Tipo</th>{/if}
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
        {#if fileFields}
          <td class="type"
            ><select
              value={item.type ?? 'text'}
              onchange={(e) => setType(item, e.currentTarget.value as FormFieldType)}
              aria-label="Tipo {i + 1}"
              data-testid="kv-type"
            >
              <option value="text">Texto</option>
              <option value="file">Arquivo</option>
            </select></td
          >
        {/if}
        <td>
          {#if item.type === 'file'}
            <div class="file">
              <input
                class="mono"
                data-field="value"
                class:missing={item.enabled && missing(item.value).length > 0}
                title={item.enabled ? (hint(item.value) ?? item.value) : undefined}
                bind:value={item.value}
                placeholder="Caminho do arquivo"
                aria-label="Arquivo {i + 1}"
              />
              <button
                onclick={() => pickFile(item)}
                title={item.value ? `Trocar ${fileName(item.value)}` : 'Escolher arquivo'}
                data-testid="kv-pick-file">Escolher…</button
              >
            </div>
          {:else}
            <input
              class="mono"
              data-field="value"
              class:missing={item.enabled && missing(item.value).length > 0}
              title={item.enabled ? hint(item.value) : undefined}
              bind:value={item.value}
              placeholder={valuePlaceholder}
              aria-label="{valuePlaceholder} {i + 1}"
            />
          {/if}
        </td>
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
      {#if fileFields}
        <td class="type"
          ><select bind:value={draftType} aria-label="Tipo da nova linha" data-testid="kv-new-type">
            <option value="text">Texto</option>
            <option value="file">Arquivo</option>
          </select></td
        >
      {/if}
      <td>
        {#if draftType === 'file'}
          <div class="file">
            <input
              class="mono"
              bind:value={draftValue}
              oninput={() => addFromDraft('value')}
              placeholder="Caminho do arquivo"
              aria-label="Arquivo da nova linha"
            />
            <button onclick={pickDraftFile} data-testid="kv-new-pick-file">Escolher…</button>
          </div>
        {:else}
          <input
            class="mono"
            bind:value={draftValue}
            oninput={() => addFromDraft('value')}
            placeholder={valuePlaceholder}
            aria-label="Valor da nova linha"
          />
        {/if}
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
  .type {
    width: 92px;
  }
  .type select {
    width: 100%;
  }
  .file {
    display: flex;
    gap: 4px;
  }
  .file input {
    flex: 1;
    min-width: 0;
  }
  input.missing {
    border-color: var(--danger);
    color: var(--danger);
  }
  tr.disabled input:not([type='checkbox']),
  tr.disabled select {
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
