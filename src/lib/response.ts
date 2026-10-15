// Funções puras do painel de resposta.
import type { HttpResponse } from './types';

/** Tamanho legível: `512 B`, `4,2 KB`, `1,5 MB`. */
export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ['KB', 'MB', 'GB'];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit++;
  }
  return `${value.toFixed(1).replace('.0', '').replace('.', ',')} ${units[unit]}`;
}

export function headerValue(response: HttpResponse, name: string): string | undefined {
  return response.headers.find(([k]) => k.toLowerCase() === name.toLowerCase())?.[1];
}

/** JSON (pelo Content-Type) é formatado com 2 espaços; se não fizer parse, vai cru (Seção 8.2). */
export function prettyBody(response: HttpResponse): { text: string; language: 'json' | 'text' } {
  const contentType = headerValue(response, 'content-type') ?? '';
  if (!contentType.includes('json') || response.bodyTruncated)
    return { text: response.body, language: contentType.includes('json') ? 'json' : 'text' };
  try {
    return { text: JSON.stringify(JSON.parse(response.body), null, 2), language: 'json' };
  } catch {
    return { text: response.body, language: 'text' };
  }
}
