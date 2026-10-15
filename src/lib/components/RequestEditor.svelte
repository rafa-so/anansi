<script lang="ts">
  // Método, URL, Params / Headers / Body / Configurações e ações do request ativo.
  import { curlMenuItems } from '../curl';
  import { moveQueryFromUrl } from '../request';
  import { environments } from '../state/environments.svelte';
  import { tabs, type Tab } from '../state/tabs.svelte';
  import { HTTP_METHODS } from '../types';
  import { collectUnresolved, unresolvedIn } from '../variables';
  import BodyEditor from './BodyEditor.svelte';
  import ContextMenu from './ContextMenu.svelte';
  import KeyValueTable from './KeyValueTable.svelte';
  import type { MenuItem } from './menu';

  let { tab }: { tab: Tab } = $props();

  let section = $state<'params' | 'headers' | 'body' | 'settings'>('params');
  let menu = $state<{ x: number; y: number; items: MenuItem[] } | null>(null);

  const known = $derived(environments.known);
  const counts = $derived({
    params: tab.draft.query.filter((q) => q.enabled && q.key).length,
    headers: tab.draft.headers.filter((h) => h.enabled && h.key).length,
  });
  const urlMissing = $derived(unresolvedIn(tab.draft.url, known));
  const allMissing = $derived(collectUnresolved(tab.draft, known));

  /** URL com `?a=1`: os pares vão para a tabela de Params (Seção 4.5). */
  function normalizeUrl() {
    const moved = moveQueryFromUrl(tab.draft);
    if (moved === tab.draft) return;
    tab.draft.url = moved.url;
    tab.draft.query = moved.query ?? [];
    section = 'params';
  }

  function openCurlMenu(e: MouseEvent) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    menu = { x: rect.left, y: rect.bottom + 4, items: curlMenuItems(() => tab.draft) };
  }

  // Configurações: "padrão" (undefined) usa as preferências globais.
  function triState(value: boolean | undefined): string {
    return value === undefined ? 'default' : value ? 'on' : 'off';
  }
  function fromTriState(v: string): boolean | undefined {
    return v === 'default' ? undefined : v === 'on';
  }
  function setVerifyTls(v: string) {
    tab.draft.settings = { ...tab.draft.settings, verifyTls: fromTriState(v) };
  }
  function setRedirects(v: string) {
    tab.draft.settings = { ...tab.draft.settings, followRedirects: fromTriState(v) };
  }
  function setTimeoutSeconds(v: string) {
    const seconds = Number(v.replace(',', '.'));
    tab.draft.settings = {
      ...tab.draft.settings,
      timeoutMs: v.trim() && seconds > 0 ? Math.round(seconds * 1000) : undefined,
    };
  }
</script>

<section class="editor">
  {#if tab.diskState === 'removed'}
    <div class="banner" role="alert">O arquivo deste request foi removido do disco. "Salvar" o recria.</div>
  {:else if tab.diskState === 'changed'}
    <div class="banner" role="alert" data-testid="disk-changed">
      O arquivo mudou no disco.
      <button onclick={() => tabs.reloadFromDisk(tab.id)} data-testid="disk-reload">Recarregar</button>
      <button onclick={() => tabs.keepMine(tab.id)} data-testid="disk-keep">Manter minhas alterações</button>
    </div>
  {/if}
  <div class="name-row">
    <input class="name" bind:value={tab.draft.name} aria-label="Nome do request" data-testid="request-name" />
    <span class="path muted" title={tab.path ?? ''}>{tab.path ?? 'não salvo'}</span>
    <button onclick={openCurlMenu} title="Copiar como cURL" data-testid="curl-menu">cURL ▾</button>
    <button onclick={() => tabs.save(tab.id)} disabled={!tabs.isDirty(tab)} data-testid="save">Salvar</button>
  </div>
  <div class="url-row">
    <select
      bind:value={tab.draft.method}
      class="method method-{tab.draft.method}"
      aria-label="Método"
      data-testid="method"
    >
      {#each HTTP_METHODS as m (m)}<option value={m}>{m}</option>{/each}
    </select>
    <input
      class="url mono"
      class:missing={urlMissing.length > 0}
      bind:value={tab.draft.url}
      onblur={normalizeUrl}
      onpaste={() => setTimeout(normalizeUrl)}
      onkeydown={(e) => e.key === 'Enter' && !e.ctrlKey && normalizeUrl()}
      placeholder={'{{baseUrlStore}}/products'}
      aria-label="URL"
      aria-invalid={urlMissing.length > 0}
      spellcheck="false"
      data-testid="url"
    />
    {#if tab.executing}
      <button onclick={() => tabs.cancel(tab.id)} data-testid="cancel-send">Cancelar</button>
    {:else}
      <button
        class="primary"
        onclick={() => tabs.send(tab.id)}
        disabled={!tab.draft.url.trim()}
        title="Enviar (Ctrl+Enter)"
        data-testid="send">Enviar</button
      >
    {/if}
  </div>
  {#if allMissing.length > 0}
    <p class="missing-note" data-testid="unresolved">
      {environments.active ? `Não definidas em "${environments.active}"` : 'Nenhum ambiente ativo'}:
      {#each allMissing as name, i (name)}<span class="mono var">{`{{${name}}}`}</span>{i <
        allMissing.length - 1
          ? ', '
          : ''}{/each}
    </p>
  {/if}

  <div class="sections" role="tablist">
    <button
      role="tab"
      aria-selected={section === 'params'}
      class:active={section === 'params'}
      onclick={() => (section = 'params')}
    >
      Params {#if counts.params}<span class="count">{counts.params}</span>{/if}
    </button>
    <button
      role="tab"
      aria-selected={section === 'headers'}
      class:active={section === 'headers'}
      onclick={() => (section = 'headers')}
    >
      Headers {#if counts.headers}<span class="count">{counts.headers}</span>{/if}
    </button>
    <button
      role="tab"
      aria-selected={section === 'body'}
      class:active={section === 'body'}
      onclick={() => (section = 'body')}
    >
      Body {#if tab.draft.body.type !== 'none'}<span class="count">●</span>{/if}
    </button>
    <button
      role="tab"
      aria-selected={section === 'settings'}
      class:active={section === 'settings'}
      onclick={() => (section = 'settings')}
      data-testid="section-settings"
    >
      Configurações {#if Object.values(tab.draft.settings).some((v) => v !== undefined)}<span class="count"
          >●</span
        >{/if}
    </button>
  </div>

  <div class="content">
    {#if section === 'params'}
      <KeyValueTable bind:items={tab.draft.query} label="Query params" keyPlaceholder="Parâmetro" {known} />
    {:else if section === 'headers'}
      <KeyValueTable bind:items={tab.draft.headers} label="Headers" keyPlaceholder="Header" {known} />
      {#if tab.draft.body.type !== 'none' && !tab.draft.headers.some((h) => h.enabled && h.key.toLowerCase() === 'content-type')}
        <p class="muted auto">
          Content-Type automático: <span class="mono"
            >{tab.draft.body.type === 'json'
              ? 'application/json'
              : tab.draft.body.type === 'text'
                ? 'text/plain; charset=utf-8'
                : 'application/x-www-form-urlencoded'}</span
          >
        </p>
      {/if}
    {:else if section === 'body'}
      <BodyEditor bind:body={tab.draft.body} {known} />
    {:else}
      <div class="settings">
        <label>
          <span>Verificar certificado TLS</span>
          <select
            value={triState(tab.draft.settings.verifyTls)}
            onchange={(e) => setVerifyTls(e.currentTarget.value)}
            data-testid="setting-tls"
          >
            <option value="default">Padrão das preferências</option>
            <option value="on">Sim</option>
            <option value="off">Não (aceita certificado autoassinado)</option>
          </select>
        </label>
        <label>
          <span>Seguir redirecionamentos</span>
          <select
            value={triState(tab.draft.settings.followRedirects)}
            onchange={(e) => setRedirects(e.currentTarget.value)}
          >
            <option value="default">Padrão (sim, até 10)</option>
            <option value="on">Sim</option>
            <option value="off">Não</option>
          </select>
        </label>
        <label>
          <span>Timeout (segundos)</span>
          <input
            type="number"
            min="1"
            placeholder="Padrão das preferências"
            value={tab.draft.settings.timeoutMs ? tab.draft.settings.timeoutMs / 1000 : ''}
            onchange={(e) => setTimeoutSeconds(e.currentTarget.value)}
          />
        </label>
      </div>
    {/if}
  </div>
</section>

{#if menu}
  <ContextMenu x={menu.x} y={menu.y} items={menu.items} onclose={() => (menu = null)} />
{/if}

<style>
  .editor {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 10px 12px;
    min-height: 0;
    height: 100%;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--warning-bg);
    color: var(--warning-text);
    padding: 6px 10px;
    border-radius: var(--radius);
  }
  .name-row,
  .url-row {
    display: flex;
    gap: 8px;
    align-items: center;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    border-color: transparent;
    background: transparent;
    min-width: 120px;
    flex: 0 1 320px;
  }
  .name:hover,
  .name:focus {
    border-color: var(--border);
  }
  .path {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: var(--mono);
    font-size: 11px;
  }
  .method {
    width: 100px;
  }
  .url {
    flex: 1;
    min-width: 0;
  }
  .url.missing {
    border-color: var(--danger);
  }
  .missing-note {
    margin: 0;
    color: var(--danger);
    font-size: 12px;
  }
  .missing-note:empty {
    display: none;
  }
  .var {
    font-weight: 600;
  }
  .sections {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid var(--border);
  }
  .sections button {
    border: none;
    background: transparent;
    border-radius: 0;
    padding: 6px 12px;
  }
  .sections button.active {
    box-shadow: inset 0 -2px 0 var(--accent);
    font-weight: 600;
  }
  .count {
    font-size: 11px;
    color: var(--accent);
    margin-left: 2px;
  }
  .content {
    overflow: auto;
    min-height: 0;
    flex: 1;
  }
  .auto {
    margin: 8px 4px 0;
  }
  .settings {
    display: grid;
    gap: 10px;
    max-width: 420px;
    padding: 4px;
  }
  .settings label {
    display: grid;
    gap: 4px;
  }
  .settings span {
    color: var(--text-muted);
  }
</style>
