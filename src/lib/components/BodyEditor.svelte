<script lang="ts">
  // Editor do body (RF-09): JSON e texto no CodeMirror, form-urlencoded em tabela.
  // Campos do formulário podem ser texto ou arquivo; com arquivo, o envio é multipart/form-data.
  import { changeBodyType, formatJson, hasFiles, jsonProblem } from '../request';
  import type { Body, BodyType } from '../types';
  import CodeEditor from './CodeEditor.svelte';
  import KeyValueTable from './KeyValueTable.svelte';

  let { body = $bindable(), known = null }: { body: Body; known?: ReadonlySet<string> | null } = $props();

  const types: { id: BodyType; label: string }[] = [
    { id: 'none', label: 'Nenhum' },
    { id: 'json', label: 'JSON' },
    { id: 'text', label: 'Texto' },
    { id: 'form-urlencoded', label: 'Form URL-encoded' },
  ];

  const jsonError = $derived(body.type === 'json' ? jsonProblem(body.content) : null);

  function format() {
    if (body.type !== 'json') return;
    const pretty = formatJson(body.content);
    if (pretty !== null) body.content = pretty;
  }
</script>

<div class="body-editor">
  <div class="types" role="radiogroup" aria-label="Tipo do body">
    {#each types as t (t.id)}
      <label>
        <input
          type="radio"
          name="body-type"
          checked={body.type === t.id}
          onchange={() => (body = changeBodyType(body, t.id))}
        />
        {t.label}
      </label>
    {/each}
    {#if body.type === 'json'}
      <button class="format" onclick={format} disabled={!!jsonError}>Formatar</button>
    {/if}
  </div>

  {#if body.type === 'json' || body.type === 'text'}
    <div class="code">
      <CodeEditor
        bind:value={body.content}
        language={body.type === 'json' ? 'json' : 'text'}
        {known}
        ariaLabel="Conteúdo do body"
        testid="body-content"
        placeholder={body.type === 'json' ? '{\n  "id": {{produtoId}}\n}' : ''}
      />
    </div>
    {#if jsonError}<p class="warning" role="status">JSON possivelmente inválido: {jsonError}</p>{/if}
  {:else if body.type === 'form-urlencoded'}
    <div class="form">
      <KeyValueTable
        bind:items={body.fields}
        label="Campos do formulário"
        keyPlaceholder="Campo"
        {known}
        fileFields
      />
      {#if hasFiles(body)}
        <p class="muted" data-testid="multipart-hint">
          Com arquivo, o body é enviado como <span class="mono">multipart/form-data</span>.
        </p>
      {/if}
    </div>
  {:else}
    <p class="muted">Este request não envia body.</p>
  {/if}
</div>

<style>
  .body-editor {
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 8px;
    height: 100%;
    min-height: 0;
  }
  .types {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    align-items: center;
  }
  .types label {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .format {
    margin-left: auto;
  }
  .form p {
    margin: 6px 0 0;
  }
  .code {
    min-height: 160px;
    height: 100%;
  }
  .warning {
    margin: 0;
    color: var(--warning-text);
    background: var(--warning-bg);
    padding: 4px 8px;
    border-radius: var(--radius);
  }
</style>
