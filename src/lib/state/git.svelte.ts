// Pull e push do repositório de collections (botões da barra superior).
import { gitApi } from '../api/git';
import { announceBearer, withBearerEnvironment } from '../bearer';
import { BEARER_SECRET_NAME, errorMessage, isAppError, type GitStatus } from '../types';
import { dialogs, toasts } from './dialogs.svelte';
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

  /**
   * Push recusado por token Bearer em texto puro: oferece "Converter agora"
   * (token para o segredo, arquivos com {{bearer_token}}). `true` se converteu.
   */
  private async offerBearerConversion(): Promise<boolean> {
    const files = await gitApi.bearerLiterals();
    if (files.length === 0) return true;
    const choice = await dialogs.choose(
      'Token Bearer em texto puro',
      `Estes arquivos têm token Bearer em texto puro e não podem ir para o git: ${files.join(', ')}. ` +
        `Converter agora guarda o token no segredo ${BEARER_SECRET_NAME} do ambiente e grava os arquivos com Bearer {{${BEARER_SECRET_NAME}}}.`,
      [
        { id: 'cancel', label: 'Cancelar' },
        { id: 'convert', label: 'Converter agora', variant: 'primary' },
      ],
    );
    if (choice !== 'convert') return false;
    const outcome = await withBearerEnvironment((env) => gitApi.convertBearer(files, env));
    if (outcome === null) return false;
    await announceBearer(outcome);
    await this.onSynced?.();
    return true;
  }

  private async push(message: string | null, retried = false): Promise<void> {
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
      if (isAppError(e) && e.code === 'BEARER_LITERAL_FOUND' && !retried) {
        this.busy = null;
        try {
          if (await this.offerBearerConversion()) return await this.push(message, true);
        } catch (err) {
          toasts.error(errorMessage(err));
        }
        await this.refresh();
        return;
      }
      toasts.error(errorMessage(e));
      await this.refresh();
    } finally {
      this.busy = null;
    }
  }
}

export const git = new GitState();
