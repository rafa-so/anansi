<script lang="ts">
  // Botões de pull e push do repositório do workspace, ao lado do seletor de workspace.
  import { git } from '../state/git.svelte';

  const status = $derived(git.status);
  const disabled = $derived(!git.ready || git.busy !== null);
  const branch = $derived(status?.upstream ?? status?.branch ?? '');
  const pending = $derived((status?.changes ?? 0) + (status?.ahead ?? 0));

  const pullTitle = $derived(
    git.unavailableReason ??
      (status?.upstream
        ? `Trazer as alterações de ${status.upstream}` +
          (status.behind ? ` (${status.behind} commit(s) novos no último fetch)` : '')
        : `A branch ${status?.branch} ainda não existe no remote`),
  );
  const pushTitle = $derived(
    git.unavailableReason ??
      [
        `Commitar e enviar para ${branch}`,
        status?.changes ? `${status.changes} arquivo(s) alterado(s)` : '',
        status?.ahead ? `${status.ahead} commit(s) ainda não enviados` : '',
      ]
        .filter(Boolean)
        .join(' · '),
  );
</script>

<div class="git" role="group" aria-label="Sincronizar com o git">
  <button
    title={pullTitle}
    aria-label="Pull"
    disabled={disabled || !status?.upstream}
    onclick={() => git.pull()}
    data-testid="git-pull"
  >
    {git.busy === 'pull' ? 'Puxando…' : '↓ Pull'}
    {#if status?.behind}<span class="badge" data-testid="git-behind">{status.behind}</span>{/if}
  </button>
  <button
    title={pushTitle}
    aria-label="Push"
    {disabled}
    onclick={() => git.startPush()}
    data-testid="git-push"
  >
    {git.busy === 'push' ? 'Enviando…' : '↑ Push'}
    {#if pending}<span class="badge" data-testid="git-pending">{pending}</span>{/if}
  </button>
</div>

<style>
  .git {
    display: flex;
    gap: 4px;
  }
  button {
    display: flex;
    align-items: center;
    gap: 6px;
    white-space: nowrap;
  }
  .badge {
    min-width: 18px;
    padding: 0 5px;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-text);
    font-size: 11px;
    line-height: 16px;
    text-align: center;
  }
</style>
