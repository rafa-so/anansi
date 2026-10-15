// Diálogos grandes abertos (ambientes, importar cURL, preferências).

class UiState {
  environmentsOpen = $state(false);
  curlImport = $state<{ open: boolean; parentPath: string | null }>({ open: false, parentPath: null });
  prefsOpen = $state(false);

  get anyOpen() {
    return this.environmentsOpen || this.curlImport.open || this.prefsOpen;
  }
}

export const ui = new UiState();
