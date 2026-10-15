// Placeholders {{nome}} no frontend: só nomes, nunca valores (Seção 6.4).
import type { KeyValue, RequestFile } from './types';

export type Placeholder = { start: number; end: number; name: string };

const NAME = /^[A-Za-z0-9_.-]+$/;

/** Mesma regra do backend: `{{ nome }}`, com espaços opcionais e nome `[A-Za-z0-9_.-]+`. */
export function findPlaceholders(text: string): Placeholder[] {
  const found: Placeholder[] = [];
  let from = 0;
  for (;;) {
    const open = text.indexOf('{{', from);
    if (open < 0) break;
    const close = text.indexOf('}}', open + 2);
    if (close < 0) break;
    const inner = text.slice(open + 2, close);
    const name = inner.trim();
    if (NAME.test(name) && !inner.includes('{')) {
      found.push({ start: open, end: close + 2, name });
      from = close + 2;
    } else {
      from = open + 1;
    }
  }
  return found;
}

/** Nomes usados no texto que não estão entre os conhecidos. */
export function unresolvedIn(text: string, known: ReadonlySet<string>): string[] {
  return findPlaceholders(text)
    .map((p) => p.name)
    .filter((n) => !known.has(n));
}

function fromList(items: KeyValue[] | undefined, known: ReadonlySet<string>): string[] {
  return (items ?? [])
    .filter((i) => i.enabled)
    .flatMap((i) => [...unresolvedIn(i.key, known), ...unresolvedIn(i.value, known)]);
}

/** Placeholders não resolvidos no que será enviado (itens desabilitados não contam). */
export function collectUnresolved(request: RequestFile, known: ReadonlySet<string>): string[] {
  const body = request.body ?? { type: 'none' };
  const names = [
    ...unresolvedIn(request.url, known),
    ...fromList(request.query, known),
    ...fromList(request.headers, known),
    ...(body.type === 'json' || body.type === 'text' ? unresolvedIn(body.content, known) : []),
    ...(body.type === 'form-urlencoded' ? fromList(body.fields, known) : []),
  ];
  return [...new Set(names)].sort();
}
