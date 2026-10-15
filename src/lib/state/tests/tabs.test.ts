import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { tabs } from '../tabs.svelte';
import { dialogs, toasts } from '../dialogs.svelte';
import type { RequestFile } from '../../types';

const stored: RequestFile = {
  schemaVersion: 1,
  name: 'Listar produtos',
  method: 'GET',
  url: '{{baseUrl}}/products',
  headers: [{ key: 'Authorization', value: 'Bearer {{token}}', enabled: true }],
};

type Call = { cmd: string; args: Record<string, unknown> };
let calls: Call[];
let handlers: Record<string, (args: Record<string, unknown>) => unknown>;

/** Espera o próximo diálogo aparecer. */
async function nextDialog() {
  await vi.waitFor(() => expect(dialogs.current).not.toBeNull());
  return dialogs.current!;
}

beforeEach(() => {
  calls = [];
  handlers = {
    request_get: () => structuredClone(stored),
    request_save: (a) => ({ path: a.path }),
    request_create: (a) => ({ path: `${a.parentPath}/novo.json` }),
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args: args as Record<string, unknown> });
    const handler = handlers[cmd];
    if (!handler) throw new Error(`comando não esperado: ${cmd}`);
    return handler(args as Record<string, unknown>);
  });
  tabs.closeAllNow();
  toasts.items = [];
});

afterEach(() => {
  if (dialogs.current?.kind === 'prompt') dialogs.current.resolve(null);
  else dialogs.current?.resolve('cancel');
  clearMocks();
});

describe('abrir requests', () => {
  it('abre o request numa aba nova e ativa', async () => {
    await tabs.openRequest('c/listar.json');
    expect(tabs.tabs).toHaveLength(1);
    expect(tabs.active?.path).toBe('c/listar.json');
    expect(tabs.isDirty(tabs.active!)).toBe(false);
  });

  it('não abre a mesma aba duas vezes', async () => {
    await tabs.openRequest('c/listar.json');
    tabs.newRequest('c');
    await tabs.openRequest('c/listar.json');
    expect(tabs.tabs).toHaveLength(2);
    expect(tabs.active?.path).toBe('c/listar.json');
    expect(calls.filter((c) => c.cmd === 'request_get')).toHaveLength(1);
  });

  it('mostra o erro do backend sem abrir aba', async () => {
    handlers.request_get = () => {
      throw { code: 'NOT_FOUND', message: 'Item não encontrado: x' };
    };
    await tabs.openRequest('c/x.json');
    expect(tabs.tabs).toHaveLength(0);
    expect(toasts.items[0].text).toBe('Item não encontrado: x');
  });
});

describe('salvar', () => {
  it('marca "não salvo" ao editar e limpa ao salvar', async () => {
    await tabs.openRequest('c/listar.json');
    const tab = tabs.active!;
    tab.draft.headers[0].value = 'Bearer outro';
    expect(tabs.isDirty(tab)).toBe(true);
    expect(await tabs.save()).toBe(true);
    expect(tabs.isDirty(tab)).toBe(false);
    const save = calls.find((c) => c.cmd === 'request_save')!;
    expect((save.args.request as RequestFile).headers![0].value).toBe('Bearer outro');
  });

  it('atualiza o caminho quando o nome muda de slug', async () => {
    handlers.request_save = () => ({ path: 'c/novo-nome.json' });
    await tabs.openRequest('c/listar.json');
    tabs.active!.draft.name = 'Novo nome';
    await tabs.save();
    expect(tabs.active!.path).toBe('c/novo-nome.json');
  });

  it('request novo pede o nome no primeiro salvar', async () => {
    tabs.newRequest('c');
    const saving = tabs.save();
    const dialog = await nextDialog();
    expect(dialog.kind).toBe('prompt');
    if (dialog.kind === 'prompt') dialog.resolve('Criar produto');
    expect(await saving).toBe(true);
    const create = calls.find((c) => c.cmd === 'request_create')!;
    expect(create.args.parentPath).toBe('c');
    expect((create.args.request as RequestFile).name).toBe('Criar produto');
    expect(tabs.active!.path).toBe('c/novo.json');
  });

  it('cancelar o nome não cria o arquivo', async () => {
    tabs.newRequest('c');
    const saving = tabs.save();
    const dialog = await nextDialog();
    if (dialog.kind === 'prompt') dialog.resolve(null);
    expect(await saving).toBe(false);
    expect(calls.some((c) => c.cmd === 'request_create')).toBe(false);
  });

  it('erro ao salvar mantém a aba como não salva', async () => {
    handlers.request_save = () => {
      throw { code: 'IO_ERROR', message: 'o arquivo está em uso por outro programa' };
    };
    await tabs.openRequest('c/listar.json');
    tabs.active!.draft.url = 'x';
    expect(await tabs.save()).toBe(false);
    expect(tabs.isDirty(tabs.active!)).toBe(true);
    expect(toasts.items[0].kind).toBe('error');
  });
});

describe('fechar', () => {
  it('fecha direto quando não há alterações', async () => {
    await tabs.openRequest('c/listar.json');
    expect(await tabs.close(tabs.active!.id)).toBe(true);
    expect(tabs.tabs).toHaveLength(0);
  });

  it('com alterações, "Cancelar" mantém a aba aberta', async () => {
    await tabs.openRequest('c/listar.json');
    tabs.active!.draft.url = 'x';
    const closing = tabs.close(tabs.active!.id);
    const dialog = await nextDialog();
    expect(dialog.kind === 'confirm' && dialog.choices.map((c) => c.id)).toEqual([
      'cancel',
      'discard',
      'save',
    ]);
    if (dialog.kind === 'confirm') dialog.resolve('cancel');
    expect(await closing).toBe(false);
    expect(tabs.tabs).toHaveLength(1);
  });

  it('com alterações, "Descartar" fecha sem salvar', async () => {
    await tabs.openRequest('c/listar.json');
    tabs.active!.draft.url = 'x';
    const closing = tabs.close(tabs.active!.id);
    const dialog = await nextDialog();
    if (dialog.kind === 'confirm') dialog.resolve('discard');
    expect(await closing).toBe(true);
    expect(calls.some((c) => c.cmd === 'request_save')).toBe(false);
  });

  it('com alterações, "Salvar" salva e fecha', async () => {
    await tabs.openRequest('c/listar.json');
    tabs.active!.draft.url = 'x';
    const closing = tabs.close(tabs.active!.id);
    const dialog = await nextDialog();
    if (dialog.kind === 'confirm') dialog.resolve('save');
    expect(await closing).toBe(true);
    expect(calls.some((c) => c.cmd === 'request_save')).toBe(true);
    expect(tabs.tabs).toHaveLength(0);
  });

  it('ativa a aba vizinha ao fechar a ativa', async () => {
    handlers.request_get = (a) => ({ ...stored, name: String(a.path) });
    await tabs.openRequest('c/a.json');
    await tabs.openRequest('c/b.json');
    await tabs.openRequest('c/c.json');
    tabs.activeId = tabs.tabs[1].id;
    tabs.closeNow(tabs.tabs[1].id);
    expect(tabs.active?.path).toBe('c/c.json');
  });
});

describe('mudanças na árvore', () => {
  it('renomear pasta corrige o caminho das abas dentro dela', async () => {
    await tabs.openRequest('c/pasta/listar.json');
    tabs.newRequest('c/pasta');
    tabs.onPathMoved('c/pasta', 'c/nova');
    expect(tabs.tabs.map((t) => t.path ?? t.parentPath)).toEqual(['c/nova/listar.json', 'c/nova']);
  });

  it('remover fecha abas limpas e marca as editadas como removidas do disco', async () => {
    handlers.request_get = (a) => ({ ...stored, name: String(a.path) });
    await tabs.openRequest('c/p/a.json');
    await tabs.openRequest('c/p/b.json');
    await tabs.openRequest('c/outro.json');
    tabs.tabs[1].draft.url = 'editado';
    tabs.onPathRemoved('c/p');
    expect(tabs.tabs.map((t) => t.path)).toEqual(['c/p/b.json', 'c/outro.json']);
    expect(tabs.tabs[0].diskState).toBe('removed');
    expect(tabs.isDirty(tabs.tabs[0])).toBe(true);
  });

  it('renomear request pela árvore atualiza a aba aberta', async () => {
    handlers.request_save = () => ({ path: 'c/novo.json' });
    await tabs.openRequest('c/listar.json');
    const newPath = await tabs.renameRequest('c/listar.json', 'Novo');
    expect(newPath).toBe('c/novo.json');
    expect(tabs.active!.path).toBe('c/novo.json');
    expect(tabs.active!.draft.name).toBe('Novo');
    expect(tabs.isDirty(tabs.active!)).toBe(false);
  });
});

import { environments } from '../environments.svelte';
import type { HttpResponse } from '../../types';

const okResponse: HttpResponse = {
  status: 200,
  statusText: 'OK',
  headers: [['content-type', 'application/json']],
  body: '{"ok":true}',
  bodyEncoding: 'utf8',
  bodyTruncated: false,
  sizeBytes: 11,
  elapsedMs: 12,
  tlsVerified: true,
  unresolved: [],
  url: 'http://localhost:8080/products',
};

describe('enviar (E4)', () => {
  beforeEach(() => {
    environments.active = 'local';
    environments.known = new Set(['baseUrl', 'token']);
    handlers.http_send = () => okResponse;
    handlers.http_release = () => null;
    handlers.http_cancel = () => null;
  });

  it('envia o rascunho com o ambiente ativo e guarda a resposta', async () => {
    await tabs.openRequest('c/listar.json');
    tabs.active!.draft.url = '{{baseUrl}}/products';
    await tabs.send();
    const call = calls.find((c) => c.cmd === 'http_send')!;
    expect(call.args.env).toBe('local');
    expect((call.args.request as RequestFile).url).toBe('{{baseUrl}}/products');
    expect(tabs.active!.response?.status).toBe(200);
    expect(tabs.active!.executing).toBe(false);
  });

  it('pede confirmação quando há variáveis não definidas e não envia se cancelar', async () => {
    await tabs.openRequest('c/listar.json');
    tabs.active!.draft.url = '{{naoExiste}}/x';
    const sending = tabs.send();
    const dialog = await nextDialog();
    expect(dialog.kind === 'confirm' && dialog.message).toContain('naoExiste');
    if (dialog.kind === 'confirm') dialog.resolve('cancel');
    await sending;
    expect(calls.some((c) => c.cmd === 'http_send')).toBe(false);
  });

  it('confirmado, envia mesmo com variáveis não definidas', async () => {
    await tabs.openRequest('c/listar.json');
    tabs.active!.draft.url = '{{naoExiste}}/x';
    const sending = tabs.send();
    const dialog = await nextDialog();
    if (dialog.kind === 'confirm') dialog.resolve('ok');
    await sending;
    expect(calls.some((c) => c.cmd === 'http_send')).toBe(true);
  });

  it('mostra o erro e "Requisição cancelada" no cancelamento', async () => {
    handlers.http_send = () => {
      throw { code: 'HTTP_ERROR', message: 'Não foi possível conectar a {{baseUrl}}/products' };
    };
    await tabs.openRequest('c/listar.json');
    await tabs.send();
    expect(tabs.active!.error).toContain('Não foi possível conectar');
    expect(tabs.active!.response).toBeNull();

    handlers.http_send = () => {
      throw { code: 'CANCELLED', message: 'Requisição cancelada.' };
    };
    await tabs.send();
    expect(tabs.active!.error).toBe('Requisição cancelada.');
  });

  it('cancel só age quando há execução em andamento', async () => {
    let release!: (r: HttpResponse) => void;
    handlers.http_send = () => new Promise<HttpResponse>((r) => (release = r));
    await tabs.openRequest('c/listar.json');
    expect(tabs.cancel()).toBe(false);
    const sending = tabs.send();
    await vi.waitFor(() => expect(tabs.active!.executing).toBe(true));
    expect(tabs.cancel()).toBe(true);
    expect(calls.some((c) => c.cmd === 'http_cancel')).toBe(true);
    release(okResponse);
    await sending;
  });

  it('nova execução libera o body da anterior', async () => {
    await tabs.openRequest('c/listar.json');
    await tabs.send();
    const first = tabs.active!.execId;
    await tabs.send();
    expect(calls.filter((c) => c.cmd === 'http_release').map((c) => c.args.execId)).toContain(first);
  });
});

describe('mudanças no disco (E7)', () => {
  it('aba sem edições é recarregada com o conteúdo novo', async () => {
    await tabs.openRequest('c/listar.json');
    handlers.request_get = () => ({ ...structuredClone(stored), url: 'novo' });
    await tabs.onFilesChanged(['c/listar.json']);
    expect(tabs.active!.draft.url).toBe('novo');
    expect(tabs.active!.diskState).toBe('ok');
  });

  it('aba com edições mostra o aviso e mantém o rascunho', async () => {
    await tabs.openRequest('c/listar.json');
    tabs.active!.draft.name = 'Meu rascunho';
    handlers.request_get = () => ({ ...structuredClone(stored), url: 'novo' });
    await tabs.onFilesChanged(['c/listar.body.json']);
    expect(tabs.active!.diskState).toBe('changed');
    expect(tabs.active!.draft.name).toBe('Meu rascunho');

    await tabs.reloadFromDisk(tabs.active!.id);
    expect(tabs.active!.draft.url).toBe('novo');
    expect(tabs.active!.draft.name).toBe('Listar produtos');
    expect(tabs.active!.diskState).toBe('ok');
  });

  it('"Manter minhas alterações" deixa o rascunho como está', async () => {
    await tabs.openRequest('c/listar.json');
    tabs.active!.draft.name = 'Meu rascunho';
    handlers.request_get = () => ({ ...structuredClone(stored), url: 'novo' });
    await tabs.onFilesChanged(['c/listar.json']);
    await tabs.keepMine(tabs.active!.id);
    expect(tabs.active!.diskState).toBe('ok');
    expect(tabs.active!.draft.name).toBe('Meu rascunho');
    expect(tabs.isDirty(tabs.active!)).toBe(true);
  });

  it('arquivo apagado por fora marca a aba como removida', async () => {
    await tabs.openRequest('c/p/listar.json');
    handlers.request_get = () => {
      throw { code: 'NOT_FOUND', message: 'x' };
    };
    await tabs.onFilesChanged(['c/p']);
    expect(tabs.active!.diskState).toBe('removed');
  });

  it('mudança em outro arquivo não mexe na aba', async () => {
    await tabs.openRequest('c/listar.json');
    const before = calls.length;
    await tabs.onFilesChanged(['c/outro.json', 'c/listar-2.json']);
    expect(calls.length).toBe(before);
  });
});
