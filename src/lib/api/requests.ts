import { invoke } from './ipc';
import type { RequestFile, SavedRequest, SecretAction } from '../types';

export const requestApi = {
  get: (path: string) => invoke<RequestFile>('request_get', { path }),
  /** `env`: onde guardar um token Bearer literal (segredo `bearer_token`). */
  create: (parentPath: string, request: RequestFile, env: string | null) =>
    invoke<SavedRequest>('request_create', { parentPath, request, env }),
  save: (path: string, request: RequestFile, env: string | null) =>
    invoke<SavedRequest>('request_save', { path, request, env }),
  remove: (path: string) => invoke<void>('request_delete', { path }),
  duplicate: (path: string, env: string | null) =>
    invoke<{ path: string }>('request_duplicate', { path, env }),
  /** Prévia do import: `null` se não há token literal. */
  bearerPreview: (request: RequestFile, env: string | null) =>
    invoke<SecretAction | null>('bearer_preview', { request, env }),
};
