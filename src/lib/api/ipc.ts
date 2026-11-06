// Ponto único de chamada ao backend. Com o log ligado, cada comando aparece no
// console do DevTools com argumentos, resultado (ou erro) e duração.
import { invoke as tauriInvoke, type InvokeArgs } from '@tauri-apps/api/core';

const FLAG = 'anansi:debug-ipc';

/** Ligado por padrão no `tauri dev`; no build, com `localStorage['anansi:debug-ipc'] = '1'`. */
export function ipcLogEnabled(): boolean {
  try {
    const flag = localStorage.getItem(FLAG);
    if (flag !== null) return flag === '1';
  } catch {
    // Sem storage (ex.: testes): vale o padrão.
  }
  return import.meta.env.DEV && import.meta.env.MODE !== 'test';
}

export async function invoke<T>(cmd: string, args?: InvokeArgs): Promise<T> {
  if (!ipcLogEnabled()) return tauriInvoke<T>(cmd, args);
  const started = performance.now();
  const ms = () => `${Math.round(performance.now() - started)}ms`;
  try {
    const result = await tauriInvoke<T>(cmd, args);
    console.debug(`[ipc] ${cmd} ✓ ${ms()}`, { args, result });
    return result;
  } catch (error) {
    console.error(`[ipc] ${cmd} ✗ ${ms()}`, { args, error });
    throw error;
  }
}
