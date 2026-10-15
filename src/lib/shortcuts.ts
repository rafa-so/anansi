// Atalhos de teclado (Seção 13.3). Tratados na fase de captura, com
// preventDefault(), para não disparar os atalhos nativos do WebView2
// (ex.: Ctrl+P abriria a impressão).

export type ShortcutAction = 'save' | 'send' | 'new' | 'close' | 'search' | 'cancel' | 'nextTab' | 'prevTab';

export function shortcutFor(
  e: Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>,
): ShortcutAction | null {
  const ctrl = e.ctrlKey || e.metaKey;
  if (e.altKey) return null;
  if (!ctrl) return e.key === 'Escape' ? 'cancel' : null;
  if (e.key === 'Tab') return e.shiftKey ? 'prevTab' : 'nextTab';
  if (e.shiftKey) return null;
  switch (e.key.toLowerCase()) {
    case 's':
      return 'save';
    case 'enter':
      return 'send';
    case 'n':
      return 'new';
    case 'w':
      return 'close';
    case 'p':
      return 'search';
    default:
      return null;
  }
}

/** Instala o listener; devolve a função que o remove. */
export function installShortcuts(
  target: Window,
  run: (action: ShortcutAction) => boolean | void,
): () => void {
  const listener = (e: KeyboardEvent) => {
    const action = shortcutFor(e);
    if (!action) return;
    // Esc só é consumido quando há algo para cancelar.
    const handled = run(action);
    if (action === 'cancel' && handled !== true) return;
    e.preventDefault();
    e.stopPropagation();
  };
  target.addEventListener('keydown', listener, { capture: true });
  return () => target.removeEventListener('keydown', listener, { capture: true });
}
