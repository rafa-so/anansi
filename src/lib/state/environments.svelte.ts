// Ambientes: lista, ambiente ativo e nomes conhecidos do ativo (Seção 13.2).
import { envApi } from '../api/environments';
import { errorMessage } from '../types';
import { toasts } from './dialogs.svelte';

class EnvironmentsState {
  list = $state<string[]>([]);
  active = $state<string | null>(null);
  /** Nomes de variáveis e segredos do ambiente ativo (nunca valores). */
  known = $state<ReadonlySet<string>>(new Set());

  async load() {
    try {
      this.list = (await envApi.list()) ?? [];
      this.active = (await envApi.getActive()) ?? null;
      await this.loadKnown();
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  async loadKnown() {
    try {
      this.known = this.active ? new Set((await envApi.variableNames(this.active)) ?? []) : new Set();
    } catch (e) {
      this.known = new Set();
      toasts.error(errorMessage(e));
    }
  }

  async setActive(name: string | null) {
    try {
      await envApi.setActive(name);
      this.active = name;
      await this.loadKnown();
    } catch (e) {
      toasts.error(errorMessage(e));
    }
  }

  reset() {
    this.list = [];
    this.active = null;
    this.known = new Set();
  }
}

export const environments = new EnvironmentsState();
