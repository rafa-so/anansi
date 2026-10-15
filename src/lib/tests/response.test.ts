import { describe, expect, it } from 'vitest';
import { formatSize, prettyBody } from '../response';
import type { HttpResponse } from '../types';

const base: HttpResponse = {
  status: 200,
  statusText: 'OK',
  headers: [['Content-Type', 'application/json; charset=utf-8']],
  body: '{"a":1,"b":[1,2]}',
  bodyEncoding: 'utf8',
  bodyTruncated: false,
  sizeBytes: 17,
  elapsedMs: 10,
  tlsVerified: true,
  unresolved: [],
  url: 'http://x',
};

describe('formatSize', () => {
  it('formata em B, KB e MB com vírgula', () => {
    expect(formatSize(512)).toBe('512 B');
    expect(formatSize(4300)).toBe('4,2 KB');
    expect(formatSize(1024)).toBe('1 KB');
    expect(formatSize(3 * 1024 * 1024)).toBe('3 MB');
  });
});

describe('prettyBody', () => {
  it('formata JSON pelo Content-Type', () => {
    expect(prettyBody(base)).toEqual({
      text: '{\n  "a": 1,\n  "b": [\n    1,\n    2\n  ]\n}',
      language: 'json',
    });
  });

  it('mostra o texto cru quando o JSON não faz parse', () => {
    expect(prettyBody({ ...base, body: '{quebrado' })).toEqual({ text: '{quebrado', language: 'text' });
  });

  it('não formata outros tipos nem body truncado', () => {
    expect(prettyBody({ ...base, headers: [['content-type', 'text/html']], body: '<p>' }).language).toBe(
      'text',
    );
    expect(prettyBody({ ...base, bodyTruncated: true }).text).toBe(base.body);
  });
});
