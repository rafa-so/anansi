import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { cleanup, render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import { clearMocks, mockIPC, mockWindows } from '@tauri-apps/api/mocks';
import { emit } from '@tauri-apps/api/event';
import App from '../App.svelte';
import { workspace } from '../lib/state/workspace.svelte';
import { tabs } from '../lib/state/tabs.svelte';
import type { TreeNode, WorkspaceInfo } from '../lib/types';

const info: WorkspaceInfo = {
  path: 'C:\\\\projetos\\\\minhas-collections',
  name: 'Demo',
  environments: ['hml'],
  gitignoreOk: true,
};
const tree: TreeNode[] = [
  {
    kind: 'collection',
    path: 'store-api',
    name: 'Store API',
    children: [
      {
        kind: 'request',
        path: 'store-api/health.json',
        name: 'Health check',
        method: 'GET',
        url: '{{base}}/health',
      },
    ],
  },
];

let current: WorkspaceInfo | null;
let saved: unknown[];
let treeCalls: number;
let sent: unknown[];
let fixed: boolean;

beforeEach(() => {
  saved = [];
  treeCalls = 0;
  sent = [];
  fixed = false;
  mockWindows('main');
  mockIPC(
    (cmd, args) => {
      switch (cmd) {
        case 'workspace_current':
          return current;
        case 'workspace_tree':
          treeCalls++;
          return tree;
        case 'request_get':
          return { schemaVersion: 1, name: 'Health check', method: 'GET', url: '{{base}}/health' };
        case 'request_save':
          saved.push(args);
          return { path: (args as { path: string }).path };
        case 'env_list':
          return ['local'];
        case 'env_get_active':
          return 'local';
        case 'env_variable_names':
          return ['base'];
        case 'http_send':
          sent.push(args);
          return {
            status: 200,
            statusText: 'OK',
            headers: [['content-type', 'application/json']],
            body: '{"status":"ok"}',
            bodyEncoding: 'utf8',
            bodyTruncated: false,
            sizeBytes: 15,
            elapsedMs: 7,
            tlsVerified: true,
            unresolved: [],
            url: 'http://localhost:8081/health',
          };
        case 'gitignore_fix':
          fixed = true;
          return null;
        case 'gitignore_status':
          return { ok: fixed };
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
  tabs.closeAllNow();
  workspace.info = null;
  workspace.expanded.clear();
  workspace.search = '';
});

afterEach(() => {
  cleanup(); // desmonta antes de remover os mocks do Tauri
  clearMocks();
});

describe('App', () => {
  it('sem workspace anterior, mostra a tela para abrir uma pasta', async () => {
    current = null;
    render(App);
    expect(await screen.findByTestId('open-workspace')).toBeInTheDocument();
  });

  it('reabre o último workspace e mostra a árvore', async () => {
    current = info;
    render(App);
    expect(await screen.findByTestId('workspace-name')).toHaveTextContent('Demo');
    expect(screen.getByText('Store API')).toBeInTheDocument();
    expect(screen.queryByTestId('gitignore-warning')).toBeNull();
  });

  it('avisa quando o .gitignore não protege os segredos', async () => {
    current = { ...info, gitignoreOk: false };
    render(App);
    expect(await screen.findByTestId('gitignore-warning')).toBeInTheDocument();
  });

  it('abre um request, edita, mostra "não salvo" e salva com Ctrl+S', async () => {
    current = info;
    render(App);
    await fireEvent.click(await screen.findByText('Store API'));
    await fireEvent.click(await screen.findByText('Health check'));
    const url = (await screen.findByTestId('url')) as HTMLInputElement;
    await fireEvent.input(url, { target: { value: '{{base}}/health?full=1' } });
    await fireEvent.blur(url);
    expect(screen.getByTestId('tab-dirty')).toBeInTheDocument();
    expect(url.value).toBe('{{base}}/health');
    await fireEvent.keyDown(window, { key: 's', ctrlKey: true });
    await waitFor(() => expect(saved).toHaveLength(1));
    expect(saved[0]).toMatchObject({
      request: { url: '{{base}}/health', query: [{ key: 'full', value: '1', enabled: true }] },
    });
    await waitFor(() => expect(screen.queryByTestId('tab-dirty')).toBeNull());
    // A árvore é recarregada depois de salvar (nome, método e URL aparecem nela).
    await waitFor(() => expect(treeCalls).toBe(2));
  });

  it('Ctrl+P foca a busca da árvore', async () => {
    current = info;
    render(App);
    const search = await screen.findByTestId('tree-search');
    await fireEvent.keyDown(window, { key: 'p', ctrlKey: true });
    expect(document.activeElement).toBe(search);
  });

  it('Ctrl+Enter envia com o ambiente ativo e mostra a resposta', async () => {
    current = info;
    render(App);
    await fireEvent.click(await screen.findByText('Store API'));
    await fireEvent.click(await screen.findByText('Health check'));
    await screen.findByTestId('url');
    await fireEvent.keyDown(window, { key: 'Enter', ctrlKey: true });
    expect(await screen.findByTestId('response-status')).toHaveTextContent('200 OK');
    expect(sent[0]).toMatchObject({ env: 'local', request: { url: '{{base}}/health' } });
    expect((screen.getByTestId('env-select') as HTMLSelectElement).value).toBe('local');
  });

  it('"Corrigir" no alerta do .gitignore resolve o aviso', async () => {
    current = { ...info, gitignoreOk: false };
    render(App);
    await fireEvent.click(await screen.findByTestId('gitignore-fix'));
    await waitFor(() => expect(screen.queryByTestId('gitignore-warning')).toBeNull());
    expect(fixed).toBe(true);
  });

  it('evento do watcher recarrega a árvore', async () => {
    current = info;
    render(App);
    await screen.findByText('Store API');
    const before = treeCalls;
    await emit('workspace://changed', { paths: ['store-api/novo.json'] });
    await waitFor(() => expect(treeCalls).toBe(before + 1));
  });
});
