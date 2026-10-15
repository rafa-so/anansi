import { invoke } from '@tauri-apps/api/core';
import type { EnvView } from '../types';

export const envApi = {
  list: () => invoke<string[]>('env_list'),
  get: (name: string) => invoke<EnvView>('env_get', { name }),
  create: (name: string) => invoke<void>('env_create', { name }),
  remove: (name: string) => invoke<void>('env_delete', { name }),
  saveVariables: (name: string, variables: Record<string, string>) =>
    invoke<void>('env_save_variables', { name, variables }),
  /** `null` mantém o valor atual; chaves ausentes são removidas. */
  saveSecrets: (name: string, secrets: Record<string, string | null>) =>
    invoke<void>('env_save_secrets', { name, secrets }),
  revealSecret: (name: string, key: string) => invoke<string>('env_reveal_secret', { name, key }),
  getActive: () => invoke<string | null>('env_get_active'),
  setActive: (name: string | null) => invoke<void>('env_set_active', { name }),
  variableNames: (name: string) => invoke<string[]>('env_variable_names', { name }),
  gitignoreStatus: () => invoke<{ ok: boolean }>('gitignore_status'),
  gitignoreFix: () => invoke<void>('gitignore_fix'),
};
