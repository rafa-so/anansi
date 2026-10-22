// Pull e push do repositório de collections (botões da barra superior).
import { gitApi } from '../api/git';
import { errorMessage, type GitStatus } from '../types';
import { toasts } from './dialogs.svelte';
import { tabs } from './tabs.svelte';

class GitState {
  status = $state<GitStatus | null>(null);
  busy = $state<'pull' | 'push' | null>(null);
  /** Diálogo do título do commit. */
  commitOpen = $state(false);
  /** Chamado depois de um pull ou push que pode ter trazido arquivos (recarrega o workspace). */
  onSynced: (() => Promise<void> | void) | null = null;

  get ready(): boolean {
    return this.status?.availability === 'ready' && this.status.branch !== null;
  }

  /** Por que os botões estão desabilitados (texto do tooltip). */
  get unavailableReason(): string | null {
    const status = this.status;
    if (!status) return 'Verificando o repositório…';
    if (status.availability === 'git-missing')
      return 'O git não foi encontrado. Instale o Git for Windows e reabra o anansi.';
    if (status.availability === 'not-a-repo') return 'A pasta do workspace não está num repositório git.';
    if (status.branch === null) return 'O repositório não está em nenhuma branch (HEAD destacado).';
    return null;
  }

  async refresh() {
    try {
      this.status = await gitApi.status();
    } catch {
      // Sem workspace aberto ou git indisponível: os botões ficam desabilitados.
      this.status = null;
    }
  }

  reset() {
    this.status = null;
    this.commitOpen = false;
  }

  async pull() {
    if (!this.ready || this.busy) return;
    this.busy = 'pull';
    try {
      const result = await gitApi.pull();
      this.status = result.status;
      if (result.updated) {
        await this.onSynced?.();
        toasts.show(`Atualizado com ${result.status.upstream ?? 'o remote'}.`);
      } else {
        toasts.show('Já está atualizado.');
      }
    } catch (e) {
      toasts.error(errorMessage(e));
      await this.refresh();
    } finally {
      this.busy = null;
    }
  }

  /** Com alterações (no disco ou em abas não salvas), pede o título do commit; senão, só envia. */
  async startPush() {
    if (!this.ready || this.busy) return;
    await this.refresh();
    if (!this.ready) return;
    if ((this.status?.changes ?? 0) > 0 || tabs.hasDirty) this.commitOpen = true;
    else await this.push(null);
  }

  /** Confirmação do diálogo: salva as abas (se pedido), commita, puxa e envia. */
  async commitAndPush(message: string, saveTabs: boolean) {
    this.commitOpen = false;
    if (saveTabs && !(await tabs.saveAllSavable())) return;
    await this.push(message);
  }

  private async push(message: string | null) {
    this.busy = 'push';
    try {
      const result = await gitApi.push(message);
      this.status = result.status;
      await this.onSynced?.();
      const target = result.status.upstream ?? 'o remote';
      if (!result.pushed) toasts.show('Nada para enviar: o remote já está atualizado.');
      else if (result.committed) toasts.show(`Commit enviado para ${target}.`);
      else toasts.show(`Commits enviados para ${target}.`);
    } catch (e) {
      toasts.error(errorMessage(e));
      await this.refresh();
    } finally {
      this.busy = null;
    }
  }
}

export const git = new GitState();
