// Diálogos grandes abertos (ambientes, importar cURL, preferências).

class UiState {
  environmentsOpen = $state(false);
  /** Nome para preencher ao abrir Ambientes pelo "Definir" do tooltip de variáveis. */
  environmentsTarget = $state<string | null>(null);
  curlImport = $state<{ open: boolean; parentPath: string | null }>({ open: false, parentPath: null });
  prefsOpen = $state(false);
  /** Request sendo movido pelo "Mover para…" (`null` com o diálogo fechado). */
  moveRequest = $state<{ path: string; name: string } | null>(null);

  get anyOpen() {
    return this.environmentsOpen || this.curlImport.open || this.prefsOpen || this.moveRequest !== null;
  }
}

export const ui = new UiState();
