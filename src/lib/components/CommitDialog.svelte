<script lang="ts">
  // Título do commit antes do push. Salva as abas pendentes, se o usuário quiser.
  import { git } from '../state/git.svelte';
  import { tabs } from '../state/tabs.svelte';
  import Modal from './Modal.svelte';

  let message = $state('');
  // Lido uma vez: a lista não muda com o diálogo aberto.
  const savable = tabs.savableDirty;
  const skipped = tabs.tabs.filter((t) => tabs.isDirty(t) && !savable.includes(t));
  let saveTabs = $state(savable.length > 0);

  const status = $derived(git.status);
  const names = (list: typeof savable) => list.map((t) => `"${t.draft.name}"`).join(', ');

  function submit(e: Event) {
    e.preventDefault();
    if (message.trim()) git.commitAndPush(message.trim(), saveTabs);
  }
</script>

<Modal title="Commit e push" onclose={() => (git.commitOpen = false)} testid="commit-dialog">
  <form id="commit-form" class="form" onsubmit={submit}>
    <p>
      {#if status?.changes}
        {status.changes} arquivo(s) alterado(s) no workspace vão num commit
      {:else}
        As alterações salvas vão num commit
      {/if}
      na branch <strong>{status?.branch}</strong> e depois para
      <strong>{status?.upstream ?? 'o remote'}</strong>.
    </p>
    {#if savable.length}
      <label class="check">
        <input type="checkbox" bind:checked={saveTabs} data-testid="commit-save-tabs" />
        Salvar antes as abas não salvas: {names(savable)}
      </label>
    {/if}
    {#if skipped.length}
      <p class="muted" data-testid="commit-skipped">
        Ficam de fora (requests novos ou alterados por fora; salve-os antes para incluir): {names(skipped)}
      </p>
    {/if}
    <label>
      <span>Título do commit</span>
      <input
        bind:value={message}
        placeholder="ex.: adiciona requests de pedidos"
        data-testid="commit-message"
      />
    </label>
  </form>
  {#snippet footer()}
    <button onclick={() => (git.commitOpen = false)}>Cancelar</button>
    <button
      type="submit"
      form="commit-form"
      class="primary"
      disabled={!message.trim()}
      data-testid="commit-ok">Commit e push</button
    >
  {/snippet}
</Modal>

<style>
  .form {
    display: grid;
    gap: 10px;
  }
  p {
    margin: 0;
    line-height: 1.45;
    overflow-wrap: anywhere;
  }
  label {
    display: grid;
    gap: 4px;
  }
  label span {
    color: var(--text-muted);
  }
  .check {
    display: flex;
    gap: 8px;
    align-items: baseline;
  }
</style>
