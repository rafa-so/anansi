// "Copiar como cURL" (RF-15): usado pelo editor e pelo menu da árvore.
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { curlApi } from './api/http';
import { forSave } from './request';
import { environments } from './state/environments.svelte';
import { toasts } from './state/dialogs.svelte';
import { errorMessage, type CurlFormat, type RequestFile } from './types';
import type { MenuItem } from './components/menu';

export async function copyCurl(request: RequestFile, format: CurlFormat, resolve: boolean) {
  try {
    const { curl, containsSecrets } = await curlApi.export(
      forSave(request),
      environments.active,
      resolve,
      format,
    );
    await writeText(curl);
    if (containsSecrets) toasts.show('cURL copiado. Atenção: ele contém segredos.', 'error');
    else toasts.show('cURL copiado.');
  } catch (e) {
    toasts.error(errorMessage(e));
  }
}

/** As quatro opções de cópia, para menus. */
export function curlMenuItems(getRequest: () => Promise<RequestFile> | RequestFile): MenuItem[] {
  const item = (label: string, format: CurlFormat, resolve: boolean, testid: string): MenuItem => ({
    label,
    testid,
    action: async () => copyCurl(await getRequest(), format, resolve),
  });
  return [
    item('Copiar como cURL (bash)', 'bash', false, 'menu-curl-bash'),
    item('Copiar como cURL (PowerShell 7)', 'powershell', false, 'menu-curl-ps'),
    item('Copiar como cURL com valores (bash)', 'bash', true, 'menu-curl-bash-values'),
    item('Copiar como cURL com valores (PowerShell 7)', 'powershell', true, 'menu-curl-ps-values'),
  ];
}
