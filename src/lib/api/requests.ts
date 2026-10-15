import { invoke } from '@tauri-apps/api/core';
import type { RequestFile } from '../types';

export const requestApi = {
  get: (path: string) => invoke<RequestFile>('request_get', { path }),
  create: (parentPath: string, request: RequestFile) =>
    invoke<{ path: string }>('request_create', { parentPath, request }),
  save: (path: string, request: RequestFile) => invoke<{ path: string }>('request_save', { path, request }),
  remove: (path: string) => invoke<void>('request_delete', { path }),
  duplicate: (path: string) => invoke<{ path: string }>('request_duplicate', { path }),
};
