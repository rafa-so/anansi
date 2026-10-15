<script lang="ts">
  // Tela de ambientes: variáveis (versionadas) e segredos (locais) (RF-10 a RF-14, Seção 13.5).
  import { untrack } from 'svelte';
  import { envApi } from '../api/environments';
  import { secretsPayload, validateRows, variablesPayload, type SecretRow, type VarRow } from '../envEditor';
  import { dialogs, toasts } from '../state/dialogs.svelte';
  import { environments } from '../state/environments.svelte';
  import { ui } from '../state/ui.svelte';
  import { workspace } from '../state/workspace.svelte';
  import { errorMessage } from '../types';
  import Modal from './Modal.svelte';

  let selected = $state<string | null>(environments.active ?? environments.list[0] ?? null);
  let vars = $state<VarRow[]>([]);
  let secrets = $state<SecretRow[]>([]);
  let warnings = $state<string[]>([]);
  let loadedSnapshot = $state('');
  let saving = $state(false);

  const gitignoreOk = $derived(workspace.info?.gitignoreOk ?? false);
  const problems = $derived(validateRows(vars, secrets));
  const dirty = $derived(JSON.stringify({ vars, secrets }) !== loadedSnapshot);

  async function load(name: string | null) {
    vars = [];
    secrets = [];
    warnings = [];
    if (!name) {
      loadedSnapshot = JSON.stringify({ vars, secrets });
      return;
    }
    try {
      const view = await envApi.get(name);
      vars = Object.entries(view.variables).map(([key, value]) => ({ key, value }));
      secrets = view.secretNames.map((key) => ({ key, originalKey: key, value: null, revealed: false }));
      warnings = view.warnings;
    } catch (e) {
      toasts.error(errorMessage(e));
    }
    loadedSnapshot = JSON.stringify({ vars, secrets });
  }

  $effect(() => {
    const name = selected;
    untrack(() => load(name));
  });

  async function confirmDiscard(): Promise<boolean> {
    if (!dirty) return true;
    return dialogs.confirm(
      'Alterações não salvas',
      'Descartar as alterações neste ambiente?',
      'Descartar',
      true,
    );
  }

  async function select(name: string) {
    if (name === selected || !(await confirmDiscard())) return;
    selected = name;
  }

  async function create() {
    const name = await dialogs.prompt(
      'Novo ambiente',
      'Nome (letras minúsculas, números e hífen)',
      '',
      'Criar',
    );
    if (!name) return;
    try {
      await envApi.create(name);
      await environments.load();
      selected = name;
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  async function remove() {
    if (!selected) return;
    const ok = await dialogs.confirm(
      'Remover ambiente',
      `Remover o ambiente "${selected}"? O arquivo de variáveis e o de segredos locais são apagados.`,
      'Remover',
      true,
    );
    if (!ok) return;
    try {
      await envApi.remove(selected);
      await environments.load();
      selected = environments.list[0] ?? null;
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  async function reveal(row: SecretRow) {
    if (!selected || !row.originalKey) return;
    try {
      row.value = await envApi.revealSecret(selected, row.originalKey);
      row.revealed = true;
      // Revelar não é editar: o valor mostrado vira a nova referência.
      loadedSnapshot = JSON.stringify({ vars, secrets });
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  async function save() {
    if (!selected || problems.length) return;
    saving = true;
    try {
      await envApi.saveVariables(selected, variablesPayload(vars));
      if (gitignoreOk) {
        const name = selected;
        await envApi.saveSecrets(
          name,
          await secretsPayload(secrets, (key) => envApi.revealSecret(name, key)),
        );
      }
      toasts.show(`Ambiente "${selected}" salvo.`);
      await load(selected);
      if (selected === environments.active) await environments.loadKnown();
    } catch (e) {
      toasts.error(errorMessage(e));
    } finally {
      saving = false;
    }
  }

  async function close() {
    if (await confirmDiscard()) ui.environmentsOpen = false;
  }
</script>

<Modal title="Ambientes" onclose={close} wide testid="environments-dialog">
  <div class="layout">
    <aside>
      <ul>
        {#each environments.list as name (name)}
          <li>
            <button class:selected={name === selected} onclick={() => select(name)} data-testid="env-item">
              {name}{#if name === environments.active}<span class="active">&nbsp;· ativo</span>{/if}
            </button>
          </li>
        {/each}
      </ul>
      <button onclick={create} data-testid="env-new">+ Ambiente</button>
    </aside>

    <div class="detail">
      {#if !selected}
        <p class="muted">Crie um ambiente (ex.: local, hml, prd) para guardar as URLs base e os tokens.</p>
      {:else}
        {#each warnings as w (w)}<p class="warning">{w}</p>{/each}

        <h3>Variáveis <span class="muted">— versionadas no git (environments/{selected}.json)</span></h3>
        <table>
          <tbody>
            {#each vars as row, i (i)}
              <tr>
                <td
                  ><input
                    class="mono"
                    bind:value={row.key}
                    placeholder="nome"
                    aria-label="Nome da variável {i + 1}"
                    data-testid="var-key"
                  /></td
                >
                <td
                  ><input
                    class="mono"
                    bind:value={row.value}
                    placeholder="valor"
                    aria-label="Valor da variável {i + 1}"
                    data-testid="var-value"
                  /></td
                >
                <td
                  ><button class="icon" aria-label="Remover {row.key}" onclick={() => vars.splice(i, 1)}
                    >×</button
                  ></td
                >
              </tr>
            {/each}
          </tbody>
        </table>
        <button onclick={() => vars.push({ key: '', value: '' })} data-testid="var-add">+ Variável</button>

        <h3>
          Segredos <span class="muted"
            >— só nesta máquina (environments/{selected}.secrets.json), não vão para o git</span
          >
        </h3>
        {#if !gitignoreOk}
          <p class="warning">
            O .gitignore não ignora os arquivos de segredos, então eles estão bloqueados.
            <button onclick={() => workspace.fixGitignore()}>Corrigir .gitignore</button>
          </p>
        {/if}
        <fieldset disabled={!gitignoreOk}>
          <table>
            <tbody>
              {#each secrets as row, i (i)}
                <tr>
                  <td
                    ><input
                      class="mono"
                      bind:value={row.key}
                      placeholder="nome"
                      aria-label="Nome do segredo {i + 1}"
                      data-testid="secret-key"
                    /></td
                  >
                  <td>
                    {#if row.value === null}
                      <input class="mono" value="••••••••" readonly aria-label="Valor oculto de {row.key}" />
                    {:else}
                      <input
                        class="mono"
                        type={row.revealed ? 'text' : 'password'}
                        bind:value={row.value}
                        placeholder="valor"
                        aria-label="Valor do segredo {i + 1}"
                        data-testid="secret-value"
                      />
                    {/if}
                  </td>
                  <td class="actions">
                    {#if row.value === null}
                      <button onclick={() => reveal(row)} data-testid="secret-reveal">Mostrar</button>
                      <button onclick={() => (row.value = '')}>Alterar</button>
                    {:else}
                      <button onclick={() => (row.revealed = !row.revealed)}
                        >{row.revealed ? 'Ocultar' : 'Mostrar'}</button
                      >
                    {/if}
                    <button class="icon" aria-label="Remover {row.key}" onclick={() => secrets.splice(i, 1)}
                      >×</button
                    >
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          <button
            onclick={() => secrets.push({ key: '', originalKey: null, value: '', revealed: false })}
            data-testid="secret-add">+ Segredo</button
          >
        </fieldset>

        {#each problems as p (p)}<p class="error">{p}</p>{/each}
      {/if}
    </div>
  </div>

  {#snippet footer()}
    {#if selected}
      <button class="danger" onclick={remove} data-testid="env-remove">Remover ambiente</button>
      <span class="spacer"></span>
    {/if}
    <button onclick={close}>Fechar</button>
    <button
      class="primary"
      onclick={save}
      disabled={!selected || !dirty || problems.length > 0 || saving}
      data-testid="env-save">Salvar</button
    >
  {/snippet}
</Modal>

<style>
  .layout {
    display: grid;
    grid-template-columns: 180px 1fr;
    gap: 16px;
    height: 100%;
    min-height: 0;
  }
  aside {
    display: flex;
    flex-direction: column;
    gap: 8px;
    border-right: 1px solid var(--border);
    padding-right: 12px;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  aside li button {
    width: 100%;
    text-align: left;
    border: none;
    background: transparent;
  }
  aside li button.selected {
    background: var(--bg-selected);
    font-weight: 600;
  }
  .active {
    color: var(--text-muted);
    font-weight: 400;
  }
  .detail {
    overflow: auto;
    min-height: 0;
  }
  h3 {
    font-size: 13px;
    margin: 14px 0 6px;
  }
  h3:first-child {
    margin-top: 0;
  }
  h3 .muted {
    font-weight: 400;
    font-size: 12px;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    margin-bottom: 6px;
  }
  td {
    padding: 2px 4px 2px 0;
  }
  td input {
    width: 100%;
  }
  .actions {
    white-space: nowrap;
    width: 1%;
  }
  fieldset {
    border: none;
    padding: 0;
    margin: 0;
  }
  .warning {
    background: var(--warning-bg);
    color: var(--warning-text);
    padding: 6px 10px;
    border-radius: var(--radius);
    margin: 0 0 8px;
  }
  .error {
    color: var(--danger);
    margin: 6px 0 0;
  }
  .spacer {
    flex: 1;
  }
</style>
