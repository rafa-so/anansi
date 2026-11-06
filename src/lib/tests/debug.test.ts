// Ferramentas de debug: log das chamadas IPC e captura de erros não tratados.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { invoke, ipcLogEnabled } from '../api/ipc';
import { installGlobalErrorHandlers } from '../globalErrors';

describe('log de IPC', () => {
  beforeEach(() => {
    mockIPC((cmd) => {
      if (cmd === 'ok') return 42;
      throw { code: 'NOT_FOUND', message: 'Não existe.' };
    });
  });

  afterEach(() => {
    clearMocks();
    localStorage.removeItem('anansi:debug-ipc');
    vi.restoreAllMocks();
  });

  it('fica desligado nos testes por padrão e não escreve no console', async () => {
    const debug = vi.spyOn(console, 'debug').mockImplementation(() => {});
    expect(ipcLogEnabled()).toBe(false);
    expect(await invoke<number>('ok')).toBe(42);
    expect(debug).not.toHaveBeenCalled();
  });

  it('ligado pelo localStorage, loga o comando com argumentos e resultado', async () => {
    localStorage.setItem('anansi:debug-ipc', '1');
    const debug = vi.spyOn(console, 'debug').mockImplementation(() => {});
    expect(await invoke<number>('ok', { a: 1 })).toBe(42);
    expect(debug).toHaveBeenCalledWith(expect.stringMatching(/^\[ipc\] ok ✓ \d+ms$/), {
      args: { a: 1 },
      result: 42,
    });
  });

  it('ligado, loga o erro do backend e repassa o mesmo erro', async () => {
    localStorage.setItem('anansi:debug-ipc', '1');
    const error = vi.spyOn(console, 'error').mockImplementation(() => {});
    await expect(invoke('falha')).rejects.toEqual({ code: 'NOT_FOUND', message: 'Não existe.' });
    expect(error).toHaveBeenCalledWith(expect.stringMatching(/^\[ipc\] falha ✗/), expect.anything());
  });
});

describe('erros não tratados', () => {
  afterEach(() => vi.restoreAllMocks());

  it('promise rejeitada sem catch vira notificação com a mensagem', () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    const notify = vi.fn();
    const uninstall = installGlobalErrorHandlers(window, notify);
    const event = new Event('unhandledrejection') as PromiseRejectionEvent;
    Object.defineProperty(event, 'reason', { value: new TypeError("Cannot read properties of null (reading 'path')") });
    window.dispatchEvent(event);
    expect(notify).toHaveBeenCalledWith("Erro inesperado: Cannot read properties of null (reading 'path')");
    uninstall();
  });

  it('depois de desinstalado, não notifica mais', () => {
    const notify = vi.fn();
    installGlobalErrorHandlers(window, notify)();
    window.dispatchEvent(new Event('unhandledrejection'));
    expect(notify).not.toHaveBeenCalled();
  });
});
