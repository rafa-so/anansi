import { describe, expect, it } from 'vitest';
import {
  changeBodyType,
  forSave,
  isDirty,
  moveQueryFromUrl,
  newRequest,
  normalizeRequest,
  splitUrlQuery,
} from '../request';
import type { RequestFile } from '../types';

const base = (): RequestFile => ({
  schemaVersion: 1,
  name: 'Listar',
  method: 'GET',
  url: '{{baseUrl}}/products',
});

describe('normalizeRequest', () => {
  it('preenche os campos opcionais com valores vazios', () => {
    expect(normalizeRequest(base())).toEqual({
      ...base(),
      query: [],
      headers: [],
      body: { type: 'none' },
      settings: {},
    });
  });

  it('gera uma cópia independente do original', () => {
    const original: RequestFile = { ...base(), headers: [{ key: 'a', value: '1', enabled: true }] };
    const copy = normalizeRequest(original);
    copy.headers[0].value = '2';
    expect(original.headers![0].value).toBe('1');
  });
});

describe('isDirty', () => {
  it('não marca alteração quando só falta preencher campos opcionais', () => {
    expect(isDirty(base(), normalizeRequest(base()))).toBe(false);
  });

  it('ignora linhas totalmente vazias nas tabelas', () => {
    const draft = normalizeRequest(base());
    draft.headers.push({ key: '', value: '', enabled: true });
    expect(isDirty(base(), draft)).toBe(false);
  });

  it('detecta mudança em header, body, método e settings', () => {
    const cases: ((r: Required<RequestFile>) => void)[] = [
      (r) => r.headers.push({ key: 'X', value: '1', enabled: true }),
      (r) => (r.body = { type: 'json', content: '{}' }),
      (r) => (r.method = 'POST'),
      (r) => (r.settings.verifyTls = false),
      (r) => (r.name = 'Outro'),
    ];
    for (const change of cases) {
      const draft = normalizeRequest(base());
      change(draft);
      expect(isDirty(base(), draft)).toBe(true);
    }
  });

  it('settings com valor indefinido equivale a settings vazio', () => {
    expect(isDirty({ ...base(), settings: {} }, { ...base(), settings: { verifyTls: undefined } })).toBe(
      false,
    );
  });
});

describe('forSave', () => {
  it('remove linhas vazias antes de salvar', () => {
    const draft = normalizeRequest(base());
    draft.query.push({ key: '', value: '', enabled: true }, { key: 'page', value: '1', enabled: true });
    expect(forSave(draft).query).toEqual([{ key: 'page', value: '1', enabled: true }]);
  });
});

describe('splitUrlQuery', () => {
  it('move os pares da query string para a lista, decodificando', () => {
    expect(splitUrlQuery('{{baseUrl}}/products?page=1&q=a%20b&flag')).toEqual({
      url: '{{baseUrl}}/products',
      query: [
        { key: 'page', value: '1', enabled: true },
        { key: 'q', value: 'a b', enabled: true },
        { key: 'flag', value: '', enabled: true },
      ],
    });
  });

  it('preserva placeholders e valores com "="', () => {
    expect(splitUrlQuery('x?token={{token}}&exp=a=b').query).toEqual([
      { key: 'token', value: '{{token}}', enabled: true },
      { key: 'exp', value: 'a=b', enabled: true },
    ]);
  });

  it('não altera URL sem query e descarta o fragmento', () => {
    expect(splitUrlQuery('http://x/a')).toEqual({ url: 'http://x/a', query: [] });
    expect(splitUrlQuery('http://x/a?b=1#frag').url).toBe('http://x/a');
  });

  it('mantém sequências de escape inválidas como texto', () => {
    expect(splitUrlQuery('x?a=%E0%A4%A').query[0].value).toBe('%E0%A4%A');
  });
});

describe('moveQueryFromUrl', () => {
  it('soma os pares novos aos que já existem na tabela', () => {
    const r = { ...base(), url: 'x?b=2', query: [{ key: 'a', value: '1', enabled: true }] };
    expect(moveQueryFromUrl(r).query!.map((q) => q.key)).toEqual(['a', 'b']);
  });

  it('devolve o mesmo objeto quando não há o que mover', () => {
    const r = base();
    expect(moveQueryFromUrl(r)).toBe(r);
  });
});

describe('changeBodyType', () => {
  it('aproveita o conteúdo entre json e texto', () => {
    expect(changeBodyType({ type: 'json', content: '{}' }, 'text')).toEqual({ type: 'text', content: '{}' });
  });

  it('começa vazio ao trocar para form e para nenhum', () => {
    expect(changeBodyType({ type: 'json', content: '{}' }, 'form-urlencoded')).toEqual({
      type: 'form-urlencoded',
      fields: [],
    });
    expect(changeBodyType({ type: 'text', content: 'x' }, 'none')).toEqual({ type: 'none' });
  });
});

describe('newRequest', () => {
  it('cria um GET vazio no schema atual', () => {
    expect(newRequest()).toMatchObject({ schemaVersion: 1, method: 'GET', url: '', body: { type: 'none' } });
  });
});

import { formatJson, jsonProblem } from '../request';

describe('jsonProblem', () => {
  it('aceita placeholders em posição de valor', () => {
    expect(jsonProblem('{ "id": {{produtoId}} }')).toBeNull();
    expect(jsonProblem('')).toBeNull();
  });

  it('aponta JSON inválido', () => {
    expect(jsonProblem('{ "id": }')).not.toBeNull();
  });
});

describe('formatJson', () => {
  it('reindenta preservando os placeholders', () => {
    expect(formatJson('{"id":{{produtoId}},"n":"{{nome}}"}')).toBe(
      '{\n  "id": {{produtoId}},\n  "n": "{{nome}}"\n}',
    );
  });

  it('devolve null para JSON inválido', () => {
    expect(formatJson('{')).toBeNull();
  });
});
