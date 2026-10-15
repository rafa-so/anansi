// Abas abertas: rascunho, versão salva e "não salvo" (RF-05, RF-06, Seção 13.2).
import { save as saveDialog } from '@tauri-apps/plugin-dialog';
import { httpApi } from '../api/http';
import { requestApi } from '../api/requests';
import { cloneRequest, forSave, isDirty, newRequest } from '../request';
import { isSameOrInside, movePath } from '../tree';
import { errorMessage, isAppError, type HttpResponse, type RequestFile } from '../types';
import { collectUnresolved } from '../variables';
import { dialogs, toasts } from './dialogs.svelte';
import { environments } from './environments.svelte';

/** `changed`: o arquivo mudou no disco enquanto a aba tinha alterações pendentes. */
export type DiskState = 'ok' | 'removed' | 'changed';

export type Tab = {
  id: string;
  /** Caminho relativo; `null` para request novo ainda não salvo. */
  path: string | null;
  /** Pasta de destino de um request novo. */
  parentPath: string | null;
  saved: RequestFile;
  draft: Required<RequestFile>;
  diskState: DiskState;
  /** Última resposta, erro de execução e execução em andamento (E4). */
  response: HttpResponse | null;
  error: string | null;
  executing: boolean;
  execId: string | null;
};

function newExecId(): string {
  return globalThis.crypto?.randomUUID?.() ?? `exec-${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

/** Caminhos de body que acompanham um request (`x.json` → `x.body.json`, `x.body.txt`). */
function relatedPaths(path: string): string[] {
  const stem = path.slice(0, -'.json'.length);
  return [path, `${stem}.body.json`, `${stem}.body.txt`];
}

let seq = 0;
const nextId = () => `tab-${++seq}`;

class TabsState {
  tabs = $state<Tab[]>([]);
  activeId = $state<string | null>(null);
  /** Chamado depois de gravar um request (a árvore mostra nome, método e URL). */
  onTreeChanged: (() => void) | null = null;

  get active(): Tab | null {
    return this.tabs.find((t) => t.id === this.activeId) ?? null;
  }

  get hasDirty(): boolean {
    return this.tabs.some((t) => this.isDirty(t));
  }

  isDirty(tab: Tab): boolean {
    return tab.path === null || tab.diskState === 'removed' || isDirty(tab.saved, tab.draft);
  }

  /** Abre o request; se já estiver aberto, só ativa a aba. */
  async openRequest(path: string) {
    const existing = this.tabs.find((t) => t.path === path);
    if (existing) {
      this.activeId = existing.id;
      return;
    }
    try {
      const request = await requestApi.get(path);
      const tab: Tab = {
        id: nextId(),
        path,
        parentPath: null,
        saved: cloneRequest(request),
        draft: cloneRequest(request),
        diskState: 'ok',
        response: null,
        error: null,
        executing: false,
        execId: null,
      };
      this.tabs.push(tab);
      this.activeId = tab.id;
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  /** Request novo, ainda sem arquivo (o primeiro "Salvar" pergunta o nome). */
  newRequest(parentPath: string | null) {
    const request = newRequest();
    const tab: Tab = {
      id: nextId(),
      path: null,
      parentPath,
      saved: cloneRequest(request),
      draft: cloneRequest(request),
      diskState: 'ok',
      response: null,
      error: null,
      executing: false,
      execId: null,
    };
    this.tabs.push(tab);
    this.activeId = tab.id;
  }

  /** Salva a aba. Devolve `false` se o usuário cancelou ou houve erro. */
  async save(id = this.activeId): Promise<boolean> {
    const tab = this.tabs.find((t) => t.id === id);
    if (!tab) return false;
    try {
      if (tab.path === null) {
        if (!tab.parentPath) {
          toasts.error('Crie uma collection antes de salvar o request.');
          return false;
        }
        const name = await dialogs.prompt('Salvar request', 'Nome', tab.draft.name, 'Salvar');
        if (!name) return false;
        tab.draft.name = name;
        const { path } = await requestApi.create(tab.parentPath, forSave(tab.draft));
        tab.path = path;
      } else {
        const { path } = await requestApi.save(tab.path, forSave(tab.draft));
        tab.path = path;
      }
      tab.saved = cloneRequest(forSave(tab.draft));
      tab.diskState = 'ok';
      this.onTreeChanged?.();
      return true;
    } catch (e) {
      if (isAppError(e) && e.code === 'NOT_FOUND') {
        toasts.error('A pasta deste request não existe mais. Salve em outro lugar.');
      } else {
        toasts.error(errorMessage(e));
      }
      return false;
    }
  }

  /** Fecha a aba; com alterações pendentes, pergunta "Salvar", "Descartar" ou "Cancelar". */
  async close(id: string): Promise<boolean> {
    const tab = this.tabs.find((t) => t.id === id);
    if (!tab) return true;
    if (this.isDirty(tab)) {
      const choice = await dialogs.choose(
        'Alterações não salvas',
        `Salvar as alterações em "${tab.draft.name}"?`,
        [
          { id: 'cancel', label: 'Cancelar' },
          { id: 'discard', label: 'Descartar', variant: 'danger' },
          { id: 'save', label: 'Salvar', variant: 'primary' },
        ],
      );
      if (choice === 'cancel') return false;
      if (choice === 'save' && !(await this.save(id))) return false;
    }
    this.closeNow(id);
    return true;
  }

  closeNow(id: string) {
    const index = this.tabs.findIndex((t) => t.id === id);
    if (index < 0) return;
    const tab = this.tabs[index];
    if (tab.executing && tab.execId) void httpApi.cancel(tab.execId).catch(() => {});
    if (tab.execId) void httpApi.release(tab.execId).catch(() => {});
    this.tabs.splice(index, 1);
    if (this.activeId === id) this.activeId = this.tabs[Math.min(index, this.tabs.length - 1)]?.id ?? null;
  }

  closeAllNow() {
    this.tabs = [];
    this.activeId = null;
  }

  /** Antes de trocar de workspace ou fechar o app: `true` se pode seguir. */
  async confirmCloseAll(): Promise<boolean> {
    const dirty = this.tabs.filter((t) => this.isDirty(t));
    if (dirty.length === 0) return true;
    const names = dirty.map((t) => `"${t.draft.name}"`).join(', ');
    return dialogs.confirm(
      'Alterações não salvas',
      `Há alterações não salvas em ${names}. Descartar e continuar?`,
      'Descartar',
      true,
    );
  }

  /** Renomear request pela árvore: aplica no rascunho aberto, se houver, e salva. */
  async renameRequest(path: string, name: string): Promise<string> {
    const tab = this.tabs.find((t) => t.path === path);
    const base = tab ? tab.saved : await requestApi.get(path);
    const { path: newPath } = await requestApi.save(path, { ...base, name });
    if (tab) {
      tab.saved = cloneRequest({ ...tab.saved, name });
      tab.draft.name = name;
      tab.path = newPath;
    }
    return newPath;
  }

  /** Após renomear pasta, collection ou request: corrige os caminhos das abas. */
  onPathMoved(from: string, to: string) {
    for (const tab of this.tabs) {
      if (tab.path && isSameOrInside(tab.path, from)) tab.path = movePath(tab.path, from, to);
      if (tab.parentPath && isSameOrInside(tab.parentPath, from))
        tab.parentPath = movePath(tab.parentPath, from, to);
    }
  }

  /** Após remover um item: fecha as abas limpas e marca as outras como "removido do disco". */
  onPathRemoved(path: string) {
    for (const tab of [...this.tabs]) {
      if (!tab.path || !isSameOrInside(tab.path, path)) continue;
      if (isDirty(tab.saved, tab.draft)) tab.diskState = 'removed';
      else this.closeNow(tab.id);
    }
  }

  // ----- execução (E4) -----

  /** Envia o rascunho (RF-08). Avisa antes se houver variáveis não definidas (Seção 6.4). */
  async send(id = this.activeId): Promise<void> {
    const tab = this.tabs.find((t) => t.id === id);
    if (!tab || tab.executing) return;
    const unresolved = collectUnresolved(tab.draft, environments.known);
    if (unresolved.length > 0) {
      const where = environments.active
        ? `no ambiente "${environments.active}"`
        : 'porque nenhum ambiente está ativo';
      const ok = await dialogs.confirm(
        'Variáveis não definidas',
        `Variáveis não definidas ${where}: ${unresolved.join(', ')}. Enviar mesmo assim (o texto {{...}} vai literal)?`,
        'Enviar mesmo assim',
      );
      if (!ok) return;
    }
    const previous = tab.execId;
    const execId = newExecId();
    tab.execId = execId;
    tab.executing = true;
    tab.error = null;
    if (previous) void httpApi.release(previous).catch(() => {});
    try {
      const response = await httpApi.send(execId, forSave(tab.draft), environments.active);
      if (tab.execId === execId) tab.response = response;
    } catch (e) {
      if (tab.execId !== execId) return;
      tab.response = null;
      tab.error = isAppError(e) && e.code === 'CANCELLED' ? 'Requisição cancelada.' : errorMessage(e);
    } finally {
      if (tab.execId === execId) tab.executing = false;
    }
  }

  /** Cancela a requisição em andamento (RF-20). `true` se havia algo para cancelar. */
  cancel(id = this.activeId): boolean {
    const tab = this.tabs.find((t) => t.id === id);
    if (!tab?.executing || !tab.execId) return false;
    void httpApi.cancel(tab.execId).catch(() => {});
    return true;
  }

  /** Salva o body completo da última resposta num arquivo (RNF-10). */
  async saveResponseBody(id = this.activeId) {
    const tab = this.tabs.find((t) => t.id === id);
    if (!tab?.execId || !tab.response) return;
    const dest = await saveDialog({ title: 'Salvar resposta', defaultPath: 'resposta.txt' });
    if (!dest) return;
    try {
      await httpApi.saveBody(tab.execId, dest);
      toasts.show(`Resposta salva em ${dest}`);
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  // ----- mudanças no disco (E7) -----

  /** Recebe os caminhos alterados por fora do app (watcher ou "Recarregar workspace"). */
  async onFilesChanged(paths: string[]) {
    const affected = this.tabs.filter(
      (tab) =>
        tab.path !== null &&
        paths.some((p) => relatedPaths(tab.path!).includes(p) || isSameOrInside(tab.path!, p)),
    );
    for (const tab of affected) await this.syncWithDisk(tab.id);
  }

  /** Compara a aba com o disco: recarrega se estiver limpa; senão, avisa. */
  async syncWithDisk(id: string) {
    const tab = this.tabs.find((t) => t.id === id);
    if (!tab?.path) return;
    let onDisk: RequestFile;
    try {
      onDisk = await requestApi.get(tab.path);
    } catch (e) {
      if (isAppError(e) && e.code === 'NOT_FOUND') tab.diskState = 'removed';
      return;
    }
    if (!isDirty(tab.saved, onDisk)) {
      if (tab.diskState === 'removed') tab.diskState = 'ok';
      return;
    }
    if (isDirty(tab.saved, tab.draft)) {
      tab.diskState = 'changed';
    } else {
      tab.saved = cloneRequest(onDisk);
      tab.draft = cloneRequest(onDisk);
      tab.diskState = 'ok';
    }
  }

  /** "Recarregar": descarta o rascunho e usa o que está no disco. */
  async reloadFromDisk(id: string) {
    const tab = this.tabs.find((t) => t.id === id);
    if (!tab?.path) return;
    try {
      const onDisk = await requestApi.get(tab.path);
      tab.saved = cloneRequest(onDisk);
      tab.draft = cloneRequest(onDisk);
      tab.diskState = 'ok';
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  /** "Manter minhas alterações": o próximo "Salvar" sobrescreve o disco. */
  async keepMine(id: string) {
    const tab = this.tabs.find((t) => t.id === id);
    if (!tab?.path) return;
    try {
      tab.saved = cloneRequest(await requestApi.get(tab.path));
    } catch {
      // removido: o "Salvar" recria
    }
    tab.diskState = 'ok';
  }

  activateNext(step: 1 | -1) {
    if (this.tabs.length === 0) return;
    const index = this.tabs.findIndex((t) => t.id === this.activeId);
    this.activeId = this.tabs[(index + step + this.tabs.length) % this.tabs.length].id;
  }
}

export const tabs = new TabsState();
