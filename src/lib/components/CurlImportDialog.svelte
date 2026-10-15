<script lang="ts">
  // Colar cURL (RF-16, Seção 11.2): prévia, avisos e destino. Nada é gravado antes de confirmar.
  import { readText } from '@tauri-apps/plugin-clipboard-manager';
  import { curlApi } from '../api/http';
  import { requestApi } from '../api/requests';
  import { toasts } from '../state/dialogs.svelte';
  import { tabs } from '../state/tabs.svelte';
  import { ui } from '../state/ui.svelte';
  import { workspace } from '../state/workspace.svelte';
  import { errorMessage, type RequestFile, type TreeNode } from '../types';
  import Modal from './Modal.svelte';

  let text = $state('');
  let parsed = $state<{ request: RequestFile; warnings: string[] } | null>(null);
  let parseError = $state<string | null>(null);
  let name = $state('');
  let destination = $state(ui.curlImport.parentPath ?? workspace.tree[0]?.path ?? '');
  let timer: ReturnType<typeof setTimeout> | undefined;

  /** Collections e pastas, para o destino. */
  const containers = $derived.by(() => {
    const list: { path: string; label: string }[] = [];
    const walk = (nodes: TreeNode[], prefix: string) => {
      for (const n of nodes) {
        if (n.kind === 'request') continue;
        const label = prefix ? `${prefix} / ${n.name}` : n.name;
        list.push({ path: n.path, label });
        walk(n.children, label);
      }
    };
    walk(workspace.tree, '');
    return list;
  });

  async function parse() {
    if (!text.trim()) {
      parsed = null;
      parseError = null;
      return;
    }
    try {
      parsed = await curlApi.import(text);
      parseError = null;
      name = parsed.request.name;
    } catch (e) {
      parsed = null;
      parseError = errorMessage(e);
    }
  }

  function onInput() {
    clearTimeout(timer);
    timer = setTimeout(parse, 250);
  }

  async function paste() {
    try {
      text = (await readText()) ?? '';
      await parse();
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  async function confirm() {
    if (!parsed || !destination || !name.trim()) return;
    try {
      const { path } = await requestApi.create(destination, { ...parsed.request, name: name.trim() });
      ui.curlImport = { open: false, parentPath: null };
      workspace.expand(destination);
      await workspace.refresh();
      workspace.selected = path;
      await tabs.openRequest(path);
      toasts.show(`Request "${name.trim()}" importado.`);
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  const close = () => (ui.curlImport = { open: false, parentPath: null });
</script>

<Modal title="Importar cURL" onclose={close} wide testid="curl-import-dialog">
  <div class="layout">
    <div class="input">
      <div class="row">
        <span class="muted"
          >Cole um comando cURL no formato bash ("Copy as cURL (bash)" do Chrome/Edge, ou do Postman).</span
        >
        <button onclick={paste}>Colar da área de transferência</button>
      </div>
      <textarea
        class="mono"
        bind:value={text}
        oninput={onInput}
        spellcheck="false"
        aria-label="Comando cURL"
        data-testid="curl-text"></textarea>
      {#if parseError}<p class="error" role="alert">{parseError}</p>{/if}
    </div>

    {#if parsed}
      <div class="preview" data-testid="curl-preview">
        <p>
          <span class="method method-{parsed.request.method}">{parsed.request.method}</span>
          <span class="mono">{parsed.request.url}</span>
        </p>
        <p class="muted">
          {parsed.request.query?.length ?? 0} parâmetro(s) · {parsed.request.headers?.length ?? 0} header(s) · body:
          {parsed.request.body?.type ?? 'none'}
        </p>
        {#each parsed.warnings as w (w)}<p class="warning">{w}</p>{/each}
        <label>
          <span>Nome</span>
          <input bind:value={name} data-testid="curl-name" />
        </label>
        <label>
          <span>Salvar em</span>
          <select bind:value={destination} data-testid="curl-destination">
            {#each containers as c (c.path)}<option value={c.path}>{c.label}</option>{/each}
          </select>
        </label>
        {#if containers.length === 0}<p class="warning">Crie uma collection antes de importar.</p>{/if}
      </div>
    {/if}
  </div>

  {#snippet footer()}
    <button onclick={close}>Cancelar</button>
    <button
      class="primary"
      onclick={confirm}
      disabled={!parsed || !destination || !name.trim()}
      data-testid="curl-confirm">Importar</button
    >
  {/snippet}
</Modal>

<style>
  .layout {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 16px;
    height: 100%;
    min-height: 0;
  }
  .input {
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 8px;
    min-height: 0;
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
    justify-content: space-between;
  }
  textarea {
    width: 100%;
    min-height: 240px;
    resize: none;
  }
  .preview {
    display: grid;
    align-content: start;
    gap: 8px;
    overflow-wrap: anywhere;
  }
  .preview p {
    margin: 0;
  }
  label {
    display: grid;
    gap: 4px;
  }
  label span {
    color: var(--text-muted);
  }
  .warning {
    background: var(--warning-bg);
    color: var(--warning-text);
    padding: 4px 8px;
    border-radius: var(--radius);
  }
  .error {
    color: var(--danger);
    margin: 0;
  }
</style>
