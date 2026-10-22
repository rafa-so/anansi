// Estado do workspace: dados, árvore, busca e nós expandidos (Seção 13.2).
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { SvelteSet } from 'svelte/reactivity';
import { collectionApi, folderApi, workspaceApi } from '../api/workspace';
import { requestApi } from '../api/requests';
import { envApi } from '../api/environments';
import { filterTree, parentPath } from '../tree';
import { errorMessage, isAppError, type TreeNode, type WorkspaceInfo } from '../types';
import { dialogs, toasts } from './dialogs.svelte';
import { environments } from './environments.svelte';
import { git } from './git.svelte';
import { tabs } from './tabs.svelte';

class WorkspaceState {
  info = $state<WorkspaceInfo | null>(null);
  tree = $state<TreeNode[]>([]);
  loading = $state(true);
  search = $state('');
  readonly expanded = new SvelteSet<string>();
  selected = $state<string | null>(null);

  /** Árvore exibida (com a busca aplicada) e nós abertos. */
  get view() {
    const { nodes, expand } = filterTree(this.tree, this.search);
    const open = this.search.trim() ? new Set([...this.expanded, ...expand]) : this.expanded;
    return { nodes, open };
  }

  /** Startup: reabre o último workspace, se houver. */
  async load() {
    this.loading = true;
    try {
      this.info = await workspaceApi.current();
      if (this.info) await Promise.all([this.refresh(), environments.load()]);
    } catch (e) {
      toasts.error(errorMessage(e));
    } finally {
      this.loading = false;
    }
  }

  /** Relê a árvore e o status do git (que muda a cada arquivo salvo). */
  async refresh() {
    if (!this.info) return;
    void git.refresh();
    try {
      this.tree = await workspaceApi.tree();
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  /** Escolhe uma pasta e abre; oferece inicializar se ainda não for workspace. */
  async pickAndOpen() {
    const path = await openDialog({ directory: true, multiple: false, title: 'Abrir workspace' });
    if (typeof path !== 'string') return;
    await this.openPath(path);
  }

  async openPath(path: string) {
    if (!(await tabs.confirmCloseAll())) return;
    try {
      this.info = await workspaceApi.open(path);
    } catch (e) {
      if (!isAppError(e) || e.code !== 'NOT_A_WORKSPACE') return toasts.error(errorMessage(e));
      const ok = await dialogs.confirm(
        'Inicializar workspace',
        `A pasta ${path} ainda não é um workspace do anansi. Criar workspace.json, environments/, .gitignore e .gitattributes?`,
        'Inicializar',
      );
      if (!ok) return;
      try {
        this.info = await workspaceApi.init(path);
      } catch (err) {
        return toasts.error(errorMessage(err));
      }
    }
    tabs.closeAllNow();
    this.expanded.clear();
    this.search = '';
    environments.reset();
    git.reset();
    await Promise.all([this.refresh(), environments.load()]);
  }

  // ----- mudanças no disco (E7) -----

  /** Evento `workspace://changed` do watcher. */
  async onChanged(paths: string[]) {
    await this.refresh();
    await tabs.onFilesChanged(paths);
    if (paths.some((p) => p === 'environments' || p.startsWith('environments/'))) await environments.load();
  }

  /** "Recarregar workspace": para quando o watcher não recebe eventos (ex.: unidade de rede). */
  async reload() {
    if (!this.info) return;
    await this.refresh();
    await environments.load();
    await this.refreshGitignore();
    for (const tab of tabs.tabs) await tabs.syncWithDisk(tab.id);
  }

  async refreshGitignore() {
    if (!this.info) return;
    try {
      this.info.gitignoreOk = (await envApi.gitignoreStatus()).ok;
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  /** Acrescenta `environments/*.secrets.json` ao .gitignore (RF-14). */
  async fixGitignore() {
    try {
      await envApi.gitignoreFix();
      await this.refreshGitignore();
      toasts.show('.gitignore corrigido: os arquivos de segredos não vão para o git.');
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  toggle(path: string) {
    if (this.expanded.has(path)) this.expanded.delete(path);
    else this.expanded.add(path);
  }

  expand(path: string) {
    this.expanded.add(path);
  }

  // ----- ações da árvore -----

  async createCollection() {
    const name = await dialogs.prompt('Nova collection', 'Nome', '', 'Criar');
    if (!name) return;
    await this.run(async () => {
      const node = await collectionApi.create(name);
      await this.refresh();
      this.selected = node.path;
    });
  }

  async createFolder(parent: string) {
    const name = await dialogs.prompt('Nova pasta', 'Nome', '', 'Criar');
    if (!name) return;
    await this.run(async () => {
      const node = await folderApi.create(parent, name);
      this.expand(parent);
      await this.refresh();
      this.selected = node.path;
    });
  }

  async rename(node: TreeNode) {
    const name = await dialogs.prompt('Renomear', 'Nome', node.name, 'Renomear');
    if (!name || name === node.name) return;
    await this.run(async () => {
      const newPath =
        node.kind === 'collection'
          ? (await collectionApi.update(node.path, name, node.description)).path
          : node.kind === 'folder'
            ? (await folderApi.rename(node.path, name)).path
            : await tabs.renameRequest(node.path, name);
      if (newPath !== node.path) {
        tabs.onPathMoved(node.path, newPath);
        if (this.expanded.has(node.path)) this.expand(newPath);
      }
      await this.refresh();
      this.selected = newPath;
    });
  }

  async remove(node: TreeNode) {
    await this.run(async () => {
      const count = await workspaceApi.requestCount(node.path);
      const what =
        node.kind === 'request'
          ? `o request "${node.name}"`
          : `${node.kind === 'collection' ? 'a collection' : 'a pasta'} "${node.name}" e ${count} request(s) dentro dela`;
      const ok = await dialogs.confirm(
        'Remover',
        `Remover ${what}? O arquivo é apagado do disco (o histórico fica no git).`,
        'Remover',
        true,
      );
      if (!ok) return;
      if (node.kind === 'collection') await collectionApi.remove(node.path);
      else if (node.kind === 'folder') await folderApi.remove(node.path);
      else await requestApi.remove(node.path);
      tabs.onPathRemoved(node.path);
      await this.refresh();
    });
  }

  async duplicate(path: string) {
    await this.run(async () => {
      const { path: copy } = await requestApi.duplicate(path);
      await this.refresh();
      this.selected = copy;
      await tabs.openRequest(copy);
    });
  }

  /** Pasta onde um novo request deve ser criado, a partir da seleção. */
  targetFor(path: string | null): string | null {
    if (!path) return this.tree[0]?.path ?? null;
    return path.endsWith('.json') ? parentPath(path) : path;
  }

  private async run(action: () => Promise<void>) {
    try {
      await action();
    } catch (e) {
      toasts.error(errorMessage(e));
      await this.refresh();
    }
  }
}

export const workspace = new WorkspaceState();
tabs.onTreeChanged = () => void workspace.refresh();
git.onSynced = () => workspace.reload();
