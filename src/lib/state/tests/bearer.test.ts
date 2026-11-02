// Bearer Token como segredo de ambiente: save, escolha do ambiente e "Converter agora" no push.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import { tabs } from '../tabs.svelte';
import { dialogs, toasts } from '../dialogs.svelte';
import { environments } from '../environments.svelte';
import { git } from '../git.svelte';
import { maskBearer } from '../../bearer';
import type { GitStatus, RequestFile } from '../../types';

const literal: RequestFile = {
  schemaVersion: 1,
  name: 'Listar',
  method: 'GET',
  url: 'https://x',
  headers: [{ key: 'Authorization', value: 'Bearer eyJsegredo', enabled: true }],
};
const converted: RequestFile = {
  ...literal,
  headers: [{ key: 'Authorization', value: 'Bearer {{bearer_token}}', enabled: true }],
};
const status: GitStatus = {
  availability: 'ready',
  branch: 'main',
  upstream: 'origin/main',
  changes: 1,
  ahead: 0,
  behind: 0,
  hook: 'installed',
};

type Call = { cmd: string; args: Record<string, unknown> };
let calls: Call[];
let handlers: Record<string, (args: Record<string, unknown>) => unknown>;

async function nextDialog() {
  await vi.waitFor(() => expect(dialogs.current).not.toBeNull());
  return dialogs.current!;
}

beforeEach(() => {
  calls = [];
  handlers = {
    request_get: () => structuredClone(literal),
    request_save: (a) => {
      if (!a.env) throw { code: 'BEARER_NEEDS_ENVIRONMENT', message: 'Escolha um ambiente' };
      return {
        path: a.path,
        request: structuredClone(converted),
        bearer: { env: a.env, action: 'created', gitignoreFixed: false },
      };
    },
    env_list: () => ['hml', 'prd'],
    env_get_active: () => null,
    env_set_active: () => null,
    env_variable_names: () => ['bearer_token'],
  };
  mockIPC((cmd, args) => {
    calls.push({ cmd, args: args as Record<string, unknown> });
    const handler = handlers[cmd];
    if (!handler) throw new Error(`comando não esperado: ${cmd}`);
    return handler(args as Record<string, unknown>);
  });
  tabs.closeAllNow();
  toasts.items = [];
  environments.list = ['hml', 'prd'];
  environments.active = 'hml';
  git.reset();
});

afterEach(() => {
  if (dialogs.current?.kind === 'prompt') dialogs.current.resolve(null);
  else dialogs.current?.resolve('cancel');
  clearMocks();
});

describe('salvar request com token Bearer literal', () => {
  it('envia o ambiente ativo e passa a mostrar {{bearer_token}} sem ficar "não salvo"', async () => {
    await tabs.openRequest('c/listar.json');
    const tab = tabs.active!;
    expect(await tabs.save()).toBe(true);
    const save = calls.find((c) => c.cmd === 'request_save')!;
    expect(save.args.env).toBe('hml');
    expect(tab.draft.headers[0].value).toBe('Bearer {{bearer_token}}');
    expect(tabs.isDirty(tab)).toBe(false);
    expect(toasts.items.some((t) => t.text.includes('segredo bearer_token em "hml"'))).toBe(true);
  });

  it('sem ambiente ativo, pergunta onde guardar, ativa o escolhido e tenta de novo', async () => {
    environments.active = null;
    await tabs.openRequest('c/listar.json');
    const saving = tabs.save();
    const dialog = await nextDialog();
    expect(dialog.kind).toBe('confirm');
    if (dialog.kind === 'confirm') {
      expect(dialog.choices.map((c) => c.label)).toEqual(['Cancelar', 'hml', 'prd']);
      dialog.resolve('env:prd');
    }
    expect(await saving).toBe(true);
    const saves = calls.filter((c) => c.cmd === 'request_save');
    expect(saves.map((c) => c.args.env)).toEqual([null, 'prd']);
    expect(calls.some((c) => c.cmd === 'env_set_active' && c.args.name === 'prd')).toBe(true);
  });

  it('cancelar a escolha do ambiente não salva', async () => {
    environments.active = null;
    await tabs.openRequest('c/listar.json');
    const saving = tabs.save();
    (await nextDialog()).resolve('cancel' as never);
    expect(await saving).toBe(false);
    expect(tabs.isDirty(tabs.active!)).toBe(false);
    expect(tabs.active!.draft.headers[0].value).toBe('Bearer eyJsegredo');
  });
});

describe('push com token Bearer literal', () => {
  it('oferece "Converter agora", converte e envia', async () => {
    let pushes = 0;
    handlers.git_status = () => status;
    handlers.git_push = () => {
      pushes++;
      if (pushes === 1) throw { code: 'BEARER_LITERAL_FOUND', message: 'token em c/a.json' };
      return { committed: true, pushed: true, status: { ...status, changes: 0 } };
    };
    handlers.git_bearer_literals = () => ['c/a.json'];
    handlers.git_convert_bearer = (a) => ({ env: a.env, action: 'unchanged', gitignoreFixed: false });
    await git.refresh();
    const pushing = git.commitAndPush('com token', false);
    const dialog = await nextDialog();
    if (dialog.kind === 'confirm') {
      expect(dialog.message).toContain('c/a.json');
      dialog.resolve('convert');
    }
    await pushing;
    await vi.waitFor(() => expect(pushes).toBe(2));
    const convert = calls.find((c) => c.cmd === 'git_convert_bearer')!;
    expect(convert.args).toMatchObject({ paths: ['c/a.json'], env: 'hml' });
  });

  it('cancelar a conversão não envia', async () => {
    let pushes = 0;
    handlers.git_status = () => status;
    handlers.git_push = () => {
      pushes++;
      throw { code: 'BEARER_LITERAL_FOUND', message: 'token' };
    };
    handlers.git_bearer_literals = () => ['c/a.json'];
    await git.refresh();
    const pushing = git.commitAndPush('x', false);
    (await nextDialog()).resolve('cancel' as never);
    await pushing;
    expect(pushes).toBe(1);
    expect(calls.some((c) => c.cmd === 'git_convert_bearer')).toBe(false);
  });
});

describe('máscara da prévia do import', () => {
  it('mostra só o começo do token e não mexe em referências', () => {
    expect(maskBearer('Bearer eyJhbGciOi')).toBe('Bearer eyJh…');
    expect(maskBearer('Bearer {{bearer_token}}')).toBe('Bearer {{bearer_token}}');
    expect(maskBearer('Basic abc')).toBe('Basic abc');
  });
});
