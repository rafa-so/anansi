import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { environments } from '../environments.svelte';

let active: string | null;

beforeEach(() => {
  active = 'hml';
  mockIPC((cmd, args) => {
    const a = args as Record<string, unknown>;
    if (cmd === 'env_list') return ['hml', 'local'];
    if (cmd === 'env_get_active') return active;
    if (cmd === 'env_set_active') {
      active = a.name as string | null;
      return null;
    }
    if (cmd === 'env_variable_names') return a.name === 'hml' ? ['baseUrl', 'token'] : ['baseUrl'];
    throw new Error(cmd);
  });
});

afterEach(() => {
  environments.reset();
  clearMocks();
});

describe('ambientes', () => {
  it('carrega a lista, o ativo e só os nomes conhecidos', async () => {
    await environments.load();
    expect(environments.list).toEqual(['hml', 'local']);
    expect(environments.active).toBe('hml');
    expect([...environments.known]).toEqual(['baseUrl', 'token']);
  });

  it('trocar o ativo grava a preferência e recarrega os nomes', async () => {
    await environments.load();
    await environments.setActive('local');
    expect(active).toBe('local');
    expect([...environments.known]).toEqual(['baseUrl']);
    await environments.setActive(null);
    expect(environments.known.size).toBe(0);
  });
});
