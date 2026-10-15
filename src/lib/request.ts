// Funções puras sobre requests: criação, normalização, comparação e URL ↔ query.
import type { Body, BodyType, KeyValue, RequestFile } from './types';

export function newRequest(name = 'Novo request'): RequestFile {
  return {
    schemaVersion: 1,
    name,
    method: 'GET',
    url: '',
    query: [],
    headers: [],
    body: { type: 'none' },
    settings: {},
  };
}

/** Preenche os campos opcionais, para comparar e editar sempre a mesma forma. */
export function normalizeRequest(r: RequestFile): Required<RequestFile> {
  return {
    schemaVersion: r.schemaVersion ?? 1,
    name: r.name,
    method: r.method,
    url: r.url,
    query: (r.query ?? []).map(cloneKv),
    headers: (r.headers ?? []).map(cloneKv),
    body: cloneBody(r.body ?? { type: 'none' }),
    settings: { ...(r.settings ?? {}) },
  };
}

function cloneKv(kv: KeyValue): KeyValue {
  return { key: kv.key, value: kv.value, enabled: kv.enabled ?? true };
}

function cloneBody(body: Body): Body {
  if (body.type === 'form-urlencoded') return { type: body.type, fields: body.fields.map(cloneKv) };
  if (body.type === 'json' || body.type === 'text') return { type: body.type, content: body.content };
  return { type: 'none' };
}

/** Cópia profunda já normalizada. */
export function cloneRequest(r: RequestFile): Required<RequestFile> {
  return normalizeRequest(r);
}

/** Forma canônica para comparação: linhas totalmente vazias das tabelas não contam. */
function canonical(r: RequestFile): string {
  const n = normalizeRequest(r);
  const notEmpty = (kv: KeyValue) => kv.key !== '' || kv.value !== '';
  const body =
    n.body.type === 'form-urlencoded' ? { ...n.body, fields: n.body.fields.filter(notEmpty) } : n.body;
  const settings = Object.fromEntries(
    Object.entries(n.settings)
      .filter(([, v]) => v !== undefined && v !== null)
      .sort(([a], [b]) => a.localeCompare(b)),
  );
  return JSON.stringify({
    ...n,
    query: n.query.filter(notEmpty),
    headers: n.headers.filter(notEmpty),
    body,
    settings,
  });
}

/** RF-06: a aba tem alterações não salvas. */
export function isDirty(saved: RequestFile, draft: RequestFile): boolean {
  return canonical(saved) !== canonical(draft);
}

/** Versão para salvar: sem linhas vazias nas tabelas. */
export function forSave(r: RequestFile): RequestFile {
  return JSON.parse(canonical(r)) as RequestFile;
}

/** Troca o tipo do body, aproveitando o conteúdo quando possível. */
export function changeBodyType(body: Body, type: BodyType): Body {
  if (type === body.type) return body;
  const content = body.type === 'json' || body.type === 'text' ? body.content : '';
  if (type === 'json' || type === 'text') return { type, content };
  if (type === 'form-urlencoded') return { type, fields: body.type === 'form-urlencoded' ? body.fields : [] };
  return { type: 'none' };
}

function decode(s: string): string {
  try {
    return decodeURIComponent(s.replace(/\+/g, ' '));
  } catch {
    return s;
  }
}

/**
 * Campo de URL e query string (Seção 4.5): a tabela de Params é a fonte da verdade.
 * Se a URL tiver `?a=1&b=2`, os pares saem da URL e vão para a lista.
 * Placeholders `{{...}}` são preservados como estão.
 */
export function splitUrlQuery(url: string): { url: string; query: KeyValue[] } {
  const hashIndex = url.indexOf('#');
  const withoutHash = hashIndex >= 0 ? url.slice(0, hashIndex) : url;
  const qIndex = withoutHash.indexOf('?');
  if (qIndex < 0) return { url, query: [] };
  const base = withoutHash.slice(0, qIndex);
  const query = withoutHash
    .slice(qIndex + 1)
    .split('&')
    .filter((part) => part !== '')
    .map((part) => {
      const eq = part.indexOf('=');
      const key = eq >= 0 ? part.slice(0, eq) : part;
      const value = eq >= 0 ? part.slice(eq + 1) : '';
      return { key: decode(key), value: decode(value), enabled: true };
    });
  return { url: base, query };
}

/** Aplica `splitUrlQuery` num request, somando os pares novos aos já existentes. */
export function moveQueryFromUrl(r: RequestFile): RequestFile {
  const { url, query } = splitUrlQuery(r.url);
  if (query.length === 0 && url === r.url) return r;
  return { ...r, url, query: [...(r.query ?? []), ...query] };
}

const PLACEHOLDER = /\{\{\s*[A-Za-z0-9_.-]+\s*\}\}/g;

/** Validação do body JSON: placeholders viram um valor fictício antes (Seção 6.5). */
export function jsonProblem(content: string): string | null {
  if (!content.trim()) return null;
  try {
    JSON.parse(content.replace(PLACEHOLDER, '0'));
    return null;
  } catch (e) {
    return e instanceof Error ? e.message : String(e);
  }
}

/** Reindenta o JSON com 2 espaços, preservando os placeholders. `null` se for inválido. */
export function formatJson(content: string): string | null {
  // Placeholders fora de string viram uma string-marcador; dentro de string, só o marcador.
  const marks: { text: string; quoted: boolean }[] = [];
  let sample = '';
  let inString = false;
  let i = 0;
  while (i < content.length) {
    const ch = content[i];
    if (ch === '{' && content[i + 1] === '{') {
      const match = /^\{\{\s*[A-Za-z0-9_.-]+\s*\}\}/.exec(content.slice(i));
      if (match) {
        const id = `__anansi_${marks.length}__`;
        marks.push({ text: match[0], quoted: !inString });
        sample += inString ? id : `"${id}"`;
        i += match[0].length;
        continue;
      }
    }
    if (inString && ch === '\\') {
      sample += ch + (content[i + 1] ?? '');
      i += 2;
      continue;
    }
    if (ch === '"') inString = !inString;
    sample += ch;
    i++;
  }
  try {
    let pretty = JSON.stringify(JSON.parse(sample), null, 2);
    marks.forEach((m, n) => {
      const id = `__anansi_${n}__`;
      pretty = pretty.replace(m.quoted ? `"${id}"` : id, m.text);
    });
    return pretty;
  } catch {
    return null;
  }
}
