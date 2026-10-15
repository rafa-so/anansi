<script lang="ts">
  // Workspace atual, alerta do .gitignore, ambiente ativo, recarregar e preferências (Seção 13.1).
  import { environments } from '../state/environments.svelte';
  import { ui } from '../state/ui.svelte';
  import { workspace } from '../state/workspace.svelte';
</script>

<header class="topbar">
  <button
    class="workspace"
    title={workspace.info?.path}
    onclick={() => workspace.pickAndOpen()}
    data-testid="workspace-name"
  >
    {workspace.info?.name ?? 'Abrir workspace'} ▾
  </button>
  {#if workspace.info && !workspace.info.gitignoreOk}
    <span class="warning" role="alert" data-testid="gitignore-warning">
      ⚠ O .gitignore não protege os arquivos de segredos
      <button onclick={() => workspace.fixGitignore()} data-testid="gitignore-fix">Corrigir</button>
    </span>
  {/if}
  <span class="spacer"></span>
  <label class="env">
    <span class="muted">Ambiente</span>
    <select
      value={environments.active ?? ''}
      onchange={(e) => environments.setActive(e.currentTarget.value || null)}
      aria-label="Ambiente ativo"
      data-testid="env-select"
    >
      <option value="">Nenhum</option>
      {#each environments.list as name (name)}<option value={name}>{name}</option>{/each}
    </select>
  </label>
  <button onclick={() => (ui.environmentsOpen = true)} data-testid="open-environments">Ambientes…</button>
  <button
    class="icon"
    title="Recarregar workspace"
    aria-label="Recarregar workspace"
    onclick={() => workspace.reload()}
    data-testid="reload">⟳</button
  >
  <button
    class="icon"
    title="Preferências"
    aria-label="Preferências"
    onclick={() => (ui.prefsOpen = true)}
    data-testid="open-prefs">⚙</button
  >
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-soft);
    min-width: 0;
  }
  .workspace {
    font-weight: 600;
    max-width: 30vw;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .warning {
    display: flex;
    align-items: center;
    gap: 8px;
    background: var(--warning-bg);
    color: var(--warning-text);
    padding: 3px 4px 3px 8px;
    border-radius: var(--radius);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .spacer {
    flex: 1;
  }
  .env {
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .icon {
    font-size: 16px;
  }
</style>
