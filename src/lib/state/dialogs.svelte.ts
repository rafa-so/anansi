// Diálogos da própria interface (sem diálogo nativo, para o fluxo ser testável no E2E).

export type DialogChoice = { id: string; label: string; variant?: 'primary' | 'danger' | 'default' };

export type ConfirmDialog = {
  kind: 'confirm';
  title: string;
  message: string;
  choices: DialogChoice[];
  resolve: (choice: string) => void;
};

export type PromptDialog = {
  kind: 'prompt';
  title: string;
  label: string;
  value: string;
  confirmLabel: string;
  resolve: (value: string | null) => void;
};

export type Dialog = ConfirmDialog | PromptDialog;

class DialogsState {
  current = $state<Dialog | null>(null);
  private queue: Dialog[] = [];

  private show(dialog: Dialog) {
    if (this.current) this.queue.push(dialog);
    else this.current = dialog;
  }

  private next() {
    this.current = this.queue.shift() ?? null;
  }

  /** Pergunta com escolhas; devolve o `id` escolhido, ou `cancel` ao fechar. */
  choose(title: string, message: string, choices: DialogChoice[]): Promise<string> {
    return new Promise((resolve) => {
      this.show({
        kind: 'confirm',
        title,
        message,
        choices,
        resolve: (choice) => {
          this.next();
          resolve(choice);
        },
      });
    });
  }

  /** Confirmação simples: `true` se confirmou. */
  async confirm(
    title: string,
    message: string,
    confirmLabel = 'Confirmar',
    danger = false,
  ): Promise<boolean> {
    const choice = await this.choose(title, message, [
      { id: 'cancel', label: 'Cancelar' },
      { id: 'ok', label: confirmLabel, variant: danger ? 'danger' : 'primary' },
    ]);
    return choice === 'ok';
  }

  /** Pede um texto; `null` se cancelou. */
  prompt(title: string, label: string, value = '', confirmLabel = 'OK'): Promise<string | null> {
    return new Promise((resolve) => {
      this.show({
        kind: 'prompt',
        title,
        label,
        value,
        confirmLabel,
        resolve: (v) => {
          this.next();
          resolve(v);
        },
      });
    });
  }
}

export const dialogs = new DialogsState();

/** Mensagens rápidas no canto da tela. */
export type Toast = { id: number; text: string; kind: 'error' | 'info' };

class ToastsState {
  items = $state<Toast[]>([]);
  private seq = 0;

  show(text: string, kind: Toast['kind'] = 'info', ms = 5000) {
    const id = ++this.seq;
    this.items.push({ id, text, kind });
    setTimeout(() => this.dismiss(id), ms);
  }

  error(text: string) {
    this.show(text, 'error', 8000);
  }

  dismiss(id: number) {
    this.items = this.items.filter((t) => t.id !== id);
  }
}

export const toasts = new ToastsState();
