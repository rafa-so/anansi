import { describe, expect, it, vi } from 'vitest';
import { secretsPayload, validateRows, variablesPayload } from '../envEditor';

describe('validateRows', () => {
  it('aceita nomes válidos e únicos', () => {
    expect(
      validateRows(
        [{ key: 'base_url', value: 'x' }],
        [{ key: 'token', originalKey: 'token', value: null, revealed: false }],
      ),
    ).toEqual([]);
  });

  it('acusa nome inválido e nome repetido entre variável e segredo', () => {
    const problems = validateRows(
      [
        { key: 'com espaço', value: '' },
        { key: 'token', value: '' },
      ],
      [{ key: 'token', originalKey: null, value: 'x', revealed: false }],
    );
    expect(problems).toHaveLength(2);
    expect(problems[1]).toContain('variável e segredo');
  });

  it('ignora linhas sem nome', () => {
    expect(validateRows([{ key: '', value: 'x' }], [])).toEqual([]);
  });
});

describe('payloads', () => {
  it('variáveis: descarta linhas sem nome e tira espaços do nome', () => {
    expect(
      variablesPayload([
        { key: ' a ', value: '1' },
        { key: '', value: '2' },
      ]),
    ).toEqual({ a: '1' });
  });

  it('segredos: null mantém, valor novo substitui e renomeado busca o valor antigo', async () => {
    const reveal = vi.fn(async (key: string) => `valor-de-${key}`);
    const payload = await secretsPayload(
      [
        { key: 'mantido', originalKey: 'mantido', value: null, revealed: false },
        { key: 'alterado', originalKey: 'alterado', value: 'novo', revealed: true },
        { key: 'renomeado', originalKey: 'antigo', value: null, revealed: false },
        { key: 'criado', originalKey: null, value: 'x', revealed: false },
        { key: '', originalKey: null, value: 'sem nome', revealed: false },
      ],
      reveal,
    );
    expect(payload).toEqual({ mantido: null, alterado: 'novo', renomeado: 'valor-de-antigo', criado: 'x' });
    expect(reveal).toHaveBeenCalledTimes(1);
  });
});
