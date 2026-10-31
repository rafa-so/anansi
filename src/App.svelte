<script lang="ts">
  // Layout: barra superior, sidebar e área de abas (Seção 13.1).
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { listen } from '@tauri-apps/api/event';
  import TopBar from './lib/components/TopBar.svelte';
  import Sidebar from './lib/components/Sidebar.svelte';
  import TabBar from './lib/components/TabBar.svelte';
  import RequestPane from './lib/components/RequestPane.svelte';
  import ConfirmDialog from './lib/components/ConfirmDialog.svelte';
  import Toasts from './lib/components/Toasts.svelte';
  import EnvironmentsDialog from './lib/components/EnvironmentsDialog.svelte';
  import CurlImportDialog from './lib/components/CurlImportDialog.svelte';
  import PreferencesDialog from './lib/components/PreferencesDialog.svelte';
  import CommitDialog from './lib/components/CommitDialog.svelte';
  import { workspace } from './lib/state/workspace.svelte';
  import { tabs } from './lib/state/tabs.svelte';
  import { dialogs } from './lib/state/dialogs.svelte';
  import { ui } from './lib/state/ui.svelte';
  import { git } from './lib/state/git.svelte';
  import { installShortcuts, type ShortcutAction } from './lib/shortcuts';

  let searchInput = $state<HTMLInputElement | null>(null);

  function runShortcut(action: ShortcutAction): boolean | void {
    if (dialogs.current || ui.anyOpen || git.commitOpen) return action === 'cancel' ? false : true;
    switch (action) {
      case 'save':
        if (tabs.activeId) tabs.save();
        return true;
      case 'new':
        if (workspace.info) tabs.newRequest(workspace.targetFor(workspace.selected));
        return true;
      case 'close':
        if (tabs.activeId) tabs.close(tabs.activeId);
        return true;
      case 'search':
        searchInput?.focus();
        searchInput?.select();
        return true;
      case 'nextTab':
        tabs.activateNext(1);
        return true;
      case 'prevTab':
        tabs.activateNext(-1);
        return true;
      case 'send':
        if (tabs.activeId) tabs.send();
        return true;
      case 'cancel':
        return tabs.cancel();
    }
  }

  onMount(() => {
    workspace.load();
    const removeShortcuts = installShortcuts(window, runShortcut);
    // Commits e pulls feitos pelo terminal: o status do git é relido ao voltar para o app.
    const onFocus = () => {
      if (workspace.info) git.refresh();
    };
    window.addEventListener('focus', onFocus);
    let unlistenClose: (() => void) | undefined;
    // Fechar o app com abas pendentes pede confirmação, no diálogo da interface.
    getCurrentWindow()
      .onCloseRequested(async (event) => {
        if (!tabs.hasDirty) return;
        event.preventDefault();
        if (await tabs.confirmCloseAll()) await getCurrentWindow().destroy();
      })
      .then((fn) => (unlistenClose = fn))
      .catch(() => {});
    // Watcher (E7): mudanças feitas por fora do app.
    const unlisteners: Promise<() => void>[] = [
      listen<{ paths: string[] }>('workspace://changed', (e) => workspace.onChanged(e.payload.paths)),
      listen<{ ok: boolean }>('workspace://gitignore', (e) => {
        if (workspace.info) workspace.info.gitignoreOk = e.payload.ok;
      }),
    ];
    return () => {
      removeShortcuts();
      window.removeEventListener('focus', onFocus);
      unlistenClose?.();
      for (const u of unlisteners) u.then((fn) => fn()).catch(() => {});
    };
  });
</script>

{#if workspace.loading}
  <main class="center muted">Carregando…</main>
{:else if !workspace.info}
  <main class="center welcome">
    <h1>anansi</h1>
    <p class="muted">
      Abra a pasta do repositório de collections (ex.: <span class="mono"
        >C:\\projetos\\minhas-collections</span
      >).
    </p>
    <button class="primary" onclick={() => workspace.pickAndOpen()} data-testid="open-workspace"
      >Abrir pasta…</button
    >
  </main>
{:else}
  <div class="app">
    <TopBar />
    <div class="body">
      <Sidebar bind:searchInput />
      <div class="main">
        <TabBar />
        {#if tabs.active}
          {#key tabs.active.id}
            <RequestPane tab={tabs.active} />
          {/key}
        {:else}
          <div class="center muted empty">
            <p>Abra um request na árvore ou crie um novo com <span class="mono">Ctrl+N</span>.</p>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}

{#if ui.environmentsOpen}<EnvironmentsDialog />{/if}
{#if ui.curlImport.open}<CurlImportDialog />{/if}
{#if ui.prefsOpen}<PreferencesDialog />{/if}
{#if git.commitOpen}<CommitDialog />{/if}
<ConfirmDialog />
<Toasts />

<style>
  .app {
    display: grid;
    grid-template-rows: auto 1fr;
    height: 100%;
  }
  .body {
    display: grid;
    grid-template-columns: minmax(220px, 300px) 1fr;
    min-height: 0;
  }
  .main {
    display: grid;
    grid-template-rows: auto 1fr;
    min-width: 0;
    min-height: 0;
  }
  .center {
    display: grid;
    place-content: center;
    justify-items: center;
    text-align: center;
    height: 100%;
    gap: 8px;
    padding: 16px;
  }
  .welcome h1 {
    margin: 0;
    font-size: 22px;
  }
</style>
