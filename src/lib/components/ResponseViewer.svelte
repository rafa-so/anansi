<script lang="ts">
  // Painel de resposta: status, tempo, tamanho, Body e Headers (RF-08, RNF-10, RNF-12).
  import { tabs, type Tab } from '../state/tabs.svelte';
  import { formatSize, prettyBody } from '../response';
  import CodeEditor from './CodeEditor.svelte';

  let { tab }: { tab: Tab } = $props();

  let section = $state<'body' | 'headers'>('body');

  const response = $derived(tab.response);
  const pretty = $derived(response ? prettyBody(response) : { text: '', language: 'text' as const });
  const statusClass = $derived(
    !response
      ? ''
      : response.status >= 500
        ? 's5'
        : response.status >= 400
          ? 's4'
          : response.status >= 300
            ? 's3'
            : 's2',
  );
</script>

<section class="response" aria-label="Resposta" data-testid="response">
  {#if tab.executing}
    <div class="state">
      <span class="muted">Enviando…</span>
      <button onclick={() => tabs.cancel(tab.id)} data-testid="cancel">Cancelar (Esc)</button>
    </div>
  {:else if tab.error}
    <div class="error" role="alert" data-testid="response-error">{tab.error}</div>
  {:else if !response}
    <div class="state muted">
      Envie o request com <span class="mono">Ctrl+Enter</span> para ver a resposta.
    </div>
  {:else}
    <div class="summary">
      <span class="status {statusClass}" data-testid="response-status"
        >{response.status} {response.statusText}</span
      >
      <span class="muted">{response.elapsedMs} ms</span>
      <span class="muted">{formatSize(response.sizeBytes)}</span>
      {#if !response.tlsVerified && response.url.startsWith('https')}
        <span class="tls" title="O certificado do servidor não foi verificado">⚠ TLS não verificado</span>
      {/if}
      <span class="spacer"></span>
      <div class="sections" role="tablist">
        <button
          role="tab"
          aria-selected={section === 'body'}
          class:active={section === 'body'}
          onclick={() => (section = 'body')}>Body</button
        >
        <button
          role="tab"
          aria-selected={section === 'headers'}
          class:active={section === 'headers'}
          onclick={() => (section = 'headers')}
        >
          Headers <span class="count">{response.headers.length}</span>
        </button>
      </div>
      <button onclick={() => tabs.saveResponseBody(tab.id)} title="Salvar o body completo em um arquivo"
        >Salvar…</button
      >
    </div>
    {#if response.unresolved.length}
      <p class="note">Enviado com variáveis não definidas: {response.unresolved.join(', ')}</p>
    {/if}
    <div class="content">
      {#if section === 'headers'}
        <table class="headers">
          <tbody>
            {#each response.headers as [k, v], i (i)}
              <tr><th class="mono">{k}</th><td class="mono">{v}</td></tr>
            {/each}
          </tbody>
        </table>
      {:else if response.bodyEncoding === 'binary'}
        <p class="muted">
          Conteúdo binário ({formatSize(response.sizeBytes)}). Use "Salvar…" para gravar em arquivo.
        </p>
      {:else}
        {#if response.bodyTruncated}
          <p class="note" role="status">
            Resposta truncada: mostrando 2 MB de {formatSize(response.sizeBytes)}. Use "Salvar…" para ver
            tudo.
          </p>
        {/if}
        <div class="code" data-testid="response-body">
          <CodeEditor value={pretty.text} language={pretty.language} readonly ariaLabel="Body da resposta" />
        </div>
      {/if}
    </div>
  {/if}
</section>

<style>
  .response {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 8px 12px;
    min-height: 0;
    height: 100%;
    border-top: 1px solid var(--border);
  }
  .state {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .error {
    color: var(--danger);
    background: color-mix(in srgb, var(--danger) 10%, transparent);
    padding: 8px 10px;
    border-radius: var(--radius);
    overflow-wrap: anywhere;
  }
  .summary {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .status {
    font-weight: 700;
    font-family: var(--mono);
  }
  .s2 {
    color: var(--m-get);
  }
  .s3 {
    color: var(--m-put);
  }
  .s4 {
    color: var(--m-post);
  }
  .s5 {
    color: var(--m-delete);
  }
  .tls {
    background: var(--warning-bg);
    color: var(--warning-text);
    padding: 2px 8px;
    border-radius: var(--radius);
  }
  .spacer {
    flex: 1;
  }
  .sections {
    display: flex;
  }
  .sections button {
    border: none;
    background: transparent;
    border-radius: 0;
  }
  .sections button.active {
    box-shadow: inset 0 -2px 0 var(--accent);
    font-weight: 600;
  }
  .count {
    font-size: 11px;
    color: var(--accent);
  }
  .note {
    margin: 0;
    background: var(--warning-bg);
    color: var(--warning-text);
    padding: 4px 8px;
    border-radius: var(--radius);
  }
  .content {
    flex: 1;
    min-height: 0;
    overflow: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .code {
    min-height: 0;
    flex: 1;
  }
  .headers {
    border-collapse: collapse;
    width: 100%;
  }
  .headers th,
  .headers td {
    text-align: left;
    padding: 3px 8px;
    border-bottom: 1px solid var(--border);
    vertical-align: top;
    overflow-wrap: anywhere;
  }
  .headers th {
    font-weight: 600;
    white-space: nowrap;
    width: 1%;
  }
</style>
