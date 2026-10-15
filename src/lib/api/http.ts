import { invoke } from '@tauri-apps/api/core';
import type { CurlFormat, GlobalPrefs, HttpResponse, RequestFile } from '../types';

export const httpApi = {
  send: (execId: string, request: RequestFile, env: string | null) =>
    invoke<HttpResponse>('http_send', { execId, request, env }),
  cancel: (execId: string) => invoke<void>('http_cancel', { execId }),
  saveBody: (execId: string, destPath: string) => invoke<void>('http_save_body', { execId, destPath }),
  release: (execId: string) => invoke<void>('http_release', { execId }),
};

export const curlApi = {
  export: (request: RequestFile, env: string | null, resolve: boolean, format: CurlFormat) =>
    invoke<{ curl: string; containsSecrets: boolean }>('curl_export', { request, env, resolve, format }),
  import: (text: string) => invoke<{ request: RequestFile; warnings: string[] }>('curl_import', { text }),
};

export const prefsApi = {
  get: () => invoke<GlobalPrefs>('prefs_get'),
  set: (prefs: GlobalPrefs) => invoke<void>('prefs_set', { prefs }),
};
