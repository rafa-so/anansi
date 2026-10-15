import { describe, expect, it, vi } from 'vitest';
import { installShortcuts, shortcutFor } from '../shortcuts';

const key = (k: string, mods: Partial<KeyboardEventInit> = {}) => ({
  key: k,
  ctrlKey: false,
  metaKey: false,
  shiftKey: false,
  altKey: false,
  ...mods,
});

describe('shortcutFor', () => {
  it('mapeia os atalhos da spec', () => {
    expect(shortcutFor(key('s', { ctrlKey: true }))).toBe('save');
    expect(shortcutFor(key('S', { ctrlKey: true }))).toBe('save');
    expect(shortcutFor(key('Enter', { ctrlKey: true }))).toBe('send');
    expect(shortcutFor(key('n', { ctrlKey: true }))).toBe('new');
    expect(shortcutFor(key('w', { ctrlKey: true }))).toBe('close');
    expect(shortcutFor(key('p', { ctrlKey: true }))).toBe('search');
    expect(shortcutFor(key('Escape'))).toBe('cancel');
    expect(shortcutFor(key('Tab', { ctrlKey: true }))).toBe('nextTab');
    expect(shortcutFor(key('Tab', { ctrlKey: true, shiftKey: true }))).toBe('prevTab');
  });

  it('ignora teclas sem Ctrl e combinações com Alt', () => {
    expect(shortcutFor(key('s'))).toBeNull();
    expect(shortcutFor(key('s', { ctrlKey: true, altKey: true }))).toBeNull();
    expect(shortcutFor(key('x', { ctrlKey: true }))).toBeNull();
  });
});

describe('installShortcuts', () => {
  it('chama preventDefault para Ctrl+P não abrir a impressão do WebView2', () => {
    const run = vi.fn();
    const remove = installShortcuts(window, run);
    const event = new KeyboardEvent('keydown', { key: 'p', ctrlKey: true, cancelable: true });
    window.dispatchEvent(event);
    expect(run).toHaveBeenCalledWith('search');
    expect(event.defaultPrevented).toBe(true);
    remove();
  });

  it('só consome o Esc quando havia algo para cancelar', () => {
    const remove = installShortcuts(window, () => false);
    const event = new KeyboardEvent('keydown', { key: 'Escape', cancelable: true });
    window.dispatchEvent(event);
    expect(event.defaultPrevented).toBe(false);
    remove();
  });

  it('deixa passar teclas comuns e para de ouvir depois de removido', () => {
    const run = vi.fn();
    const remove = installShortcuts(window, run);
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'a' }));
    remove();
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 's', ctrlKey: true }));
    expect(run).not.toHaveBeenCalled();
  });
});
