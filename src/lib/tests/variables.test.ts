import { describe, expect, it } from 'vitest';
import { collectUnresolved, findPlaceholders, unresolvedIn } from '../variables';
import type { RequestFile } from '../types';

describe('findPlaceholders', () => {
  it('segue a mesma regra do backend', () => {
    expect(findPlaceholders('x {{ a }} {{b.c-d_e}}')).toEqual([
      { start: 2, end: 9, name: 'a' },
      { start: 10, end: 21, name: 'b.c-d_e' },
    ]);
  });

  it('ignora nomes inválidos e chaves soltas', () => {
    expect(findPlaceholders('{{com espaço}} {x} {{')).toEqual([]);
    expect(findPlaceholders('{{{a}}}').map((p) => p.name)).toEqual(['a']);
  });
});

describe('unresolvedIn', () => {
  it('lista só os nomes fora do ambiente', () => {
    expect(unresolvedIn('{{base}}/x/{{id}}', new Set(['base']))).toEqual(['id']);
  });
});

describe('collectUnresolved', () => {
  const request: RequestFile = {
    schemaVersion: 1,
    name: 'x',
    method: 'POST',
    url: '{{base_url_orders_api}}/orders',
    query: [
      { key: 'id', value: '{{id}}', enabled: true },
      { key: 'off', value: '{{desligado}}', enabled: false },
    ],
    headers: [{ key: 'x-api-key', value: '{{x-api-key}}', enabled: true }],
    body: { type: 'json', content: '{"a": {{produtoId}}, "b": {{id}}}' },
  };

  it('junta URL, query, headers e body, sem repetir e ignorando desabilitados', () => {
    expect(collectUnresolved(request, new Set(['base_url_orders_api']))).toEqual([
      'id',
      'produtoId',
      'x-api-key',
    ]);
  });

  it('não acusa nada quando tudo está definido', () => {
    expect(
      collectUnresolved(request, new Set(['base_url_orders_api', 'id', 'x-api-key', 'produtoId'])),
    ).toEqual([]);
  });

  it('olha os campos de form-urlencoded', () => {
    const form: RequestFile = {
      ...request,
      body: { type: 'form-urlencoded', fields: [{ key: 'x', value: '{{y}}', enabled: true }] },
    };
    expect(collectUnresolved(form, new Set(['base_url_orders_api', 'id', 'x-api-key']))).toEqual(['y']);
  });
});
