import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import TreeNode from '../TreeNode.svelte';
import KeyValueTable from '../KeyValueTable.svelte';
import ConfirmDialog from '../ConfirmDialog.svelte';
import BodyEditor from '../BodyEditor.svelte';
import { dialogs } from '../../state/dialogs.svelte';
import type { Body, KeyValue, TreeNode as Node } from '../../types';

const node: Node = {
  kind: 'collection',
  path: 'c',
  name: 'Store API',
  children: [
    {
      kind: 'folder',
      path: 'c/f',
      name: 'products',
      children: [
        {
          kind: 'folder',
          path: 'c/f/g',
          name: 'reviews',
          children: [{ kind: 'request', path: 'c/f/g/r.json', name: 'Listar', method: 'GET', url: '/x' }],
        },
      ],
    },
    { kind: 'request', path: 'c/q.json', name: 'Quebrado', method: '', url: '', error: 'JSON inválido' },
  ],
};

function renderTree(open: string[]) {
  const props = {
    node,
    open: new Set(open),
    selected: null,
    activePath: null,
    onselect: vi.fn(),
    onactivate: vi.fn(),
    oncontext: vi.fn(),
  };
  render(TreeNode, props);
  return props;
}

describe('TreeNode', () => {
  it('mostra os filhos só dos nós abertos, em qualquer profundidade', () => {
    renderTree(['c', 'c/f', 'c/f/g']);
    const paths = screen.getAllByTestId('tree-node').map((el) => el.dataset.path);
    expect(paths).toEqual(['c', 'c/f', 'c/f/g', 'c/f/g/r.json', 'c/q.json']);
    renderTree(['c']);
    expect(screen.getAllByTestId('tree-node').filter((el) => el.dataset.path === 'c/f/g')).toHaveLength(1);
  });

  it('clique seleciona e ativa; botão direito abre o menu', async () => {
    const props = renderTree(['c', 'c/f', 'c/f/g']);
    const request = screen.getAllByTestId('tree-node').find((el) => el.dataset.path === 'c/f/g/r.json')!;
    await fireEvent.click(request);
    expect(props.onselect).toHaveBeenCalledWith(expect.objectContaining({ path: 'c/f/g/r.json' }));
    expect(props.onactivate).toHaveBeenCalledWith(expect.objectContaining({ path: 'c/f/g/r.json' }));
    await fireEvent.contextMenu(request);
    expect(props.oncontext).toHaveBeenCalled();
  });

  it('marca itens com erro e mostra a mensagem', () => {
    renderTree(['c']);
    expect(screen.getByLabelText('Erro: JSON inválido')).toBeInTheDocument();
  });
});

describe('KeyValueTable', () => {
  it('digitar na linha vazia cria uma linha e mantém o foco nela', async () => {
    const items: KeyValue[] = $state([]);
    render(KeyValueTable, { items, label: 'Headers', keyPlaceholder: 'Header' });
    const user = userEvent.setup();
    await user.click(screen.getByLabelText('Adicionar header'));
    await user.keyboard('Accept');
    expect(items).toEqual([{ key: 'Accept', value: '', enabled: true }]);
    expect(screen.getAllByTestId('kv-row')).toHaveLength(1);
  });

  it('desabilita e remove linhas', async () => {
    const items: KeyValue[] = $state([{ key: 'a', value: '1', enabled: true }]);
    render(KeyValueTable, { items, label: 'Query' });
    await fireEvent.click(screen.getByLabelText('Ativar a'));
    expect(items[0].enabled).toBe(false);
    await fireEvent.click(screen.getByLabelText('Remover a'));
    expect(items).toHaveLength(0);
  });
});

describe('ConfirmDialog', () => {
  afterEach(() => dialogs.current?.kind === 'confirm' && dialogs.current.resolve('cancel'));

  it('resolve a confirmação com o botão escolhido', async () => {
    render(ConfirmDialog);
    const answer = dialogs.confirm('Remover', 'Remover o request?', 'Remover', true);
    await fireEvent.click(await screen.findByTestId('dialog-ok'));
    expect(await answer).toBe(true);
  });

  it('Esc cancela o pedido de texto', async () => {
    render(ConfirmDialog);
    const answer = dialogs.prompt('Nova pasta', 'Nome');
    const input = await screen.findByTestId('dialog-input');
    await fireEvent.keyDown(input, { key: 'Escape' });
    expect(await answer).toBeNull();
  });

  it('não aceita nome vazio e devolve o texto sem espaços nas pontas', async () => {
    render(ConfirmDialog);
    const answer = dialogs.prompt('Nova pasta', 'Nome');
    const user = userEvent.setup();
    expect(await screen.findByTestId('dialog-ok')).toBeDisabled();
    await user.type(screen.getByTestId('dialog-input'), '  products  {Enter}');
    expect(await answer).toBe('products');
  });

  it('mostra um diálogo por vez, na ordem', async () => {
    render(ConfirmDialog);
    const first = dialogs.confirm('Primeiro', 'a');
    const second = dialogs.confirm('Segundo', 'b');
    expect(await screen.findByText('Primeiro')).toBeInTheDocument();
    await fireEvent.click(screen.getByTestId('dialog-ok'));
    expect(await first).toBe(true);
    expect(await screen.findByText('Segundo')).toBeInTheDocument();
    await fireEvent.click(screen.getByTestId('dialog-cancel'));
    expect(await second).toBe(false);
  });
});

describe('BodyEditor', () => {
  it('avisa JSON inválido sem considerar os placeholders', async () => {
    const props = $state({ body: { type: 'json', content: '{ "id": {{produtoId}} }' } as Body });
    render(BodyEditor, props);
    expect(screen.queryByRole('status')).toBeNull();
    const user = userEvent.setup();
    await user.type(screen.getByTestId('body-content'), ',');
    expect(await screen.findByRole('status')).toHaveTextContent('JSON possivelmente inválido');
  });
});

import ResponseViewer from '../ResponseViewer.svelte';
import type { Tab } from '../../state/tabs.svelte';
import type { HttpResponse } from '../../types';

function tabWith(partial: Partial<Tab>): Tab {
  const draft = {
    schemaVersion: 1,
    name: 'x',
    method: 'GET' as const,
    url: '',
    query: [],
    headers: [],
    body: { type: 'none' as const },
    settings: {},
  };
  return {
    id: 't',
    path: 'c/x.json',
    parentPath: null,
    saved: draft,
    draft,
    diskState: 'ok',
    response: null,
    error: null,
    executing: false,
    execId: 'e',
    ...partial,
  };
}

const response: HttpResponse = {
  status: 404,
  statusText: 'Not Found',
  headers: [
    ['content-type', 'application/json'],
    ['set-cookie', 'a=1'],
  ],
  body: '{"erro":"não achei"}',
  bodyEncoding: 'utf8',
  bodyTruncated: false,
  sizeBytes: 21,
  elapsedMs: 132,
  tlsVerified: true,
  unresolved: [],
  url: 'http://x',
};

describe('ResponseViewer', () => {
  it('mostra status, tempo, tamanho e quantidade de headers', () => {
    render(ResponseViewer, { tab: tabWith({ response }) });
    expect(screen.getByTestId('response-status')).toHaveTextContent('404 Not Found');
    expect(screen.getByText('132 ms')).toBeInTheDocument();
    expect(screen.getByText('21 B')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: /Headers/ })).toHaveTextContent('2');
  });

  it('avisa resposta truncada, binária e TLS não verificado', () => {
    render(ResponseViewer, {
      tab: tabWith({
        response: {
          ...response,
          status: 200,
          bodyTruncated: true,
          sizeBytes: 5 * 1024 * 1024,
          tlsVerified: false,
          url: 'https://x',
        },
      }),
    });
    expect(screen.getByText(/Resposta truncada/)).toHaveTextContent('5 MB');
    expect(screen.getByText(/TLS não verificado/)).toBeInTheDocument();
  });

  it('conteúdo binário oferece salvar em arquivo', () => {
    render(ResponseViewer, { tab: tabWith({ response: { ...response, bodyEncoding: 'binary', body: '' } }) });
    expect(screen.getByText(/Conteúdo binário/)).toBeInTheDocument();
  });

  it('mostra erro de execução e o estado "Enviando"', () => {
    const { unmount } = render(ResponseViewer, { tab: tabWith({ error: 'Requisição cancelada.' }) });
    expect(screen.getByTestId('response-error')).toHaveTextContent('Requisição cancelada.');
    unmount();
    render(ResponseViewer, { tab: tabWith({ executing: true }) });
    expect(screen.getByTestId('cancel')).toBeInTheDocument();
  });
});

describe('KeyValueTable com ambiente', () => {
  it('marca em vermelho o campo com variável não definida', () => {
    const items: KeyValue[] = $state([
      { key: 'Authorization', value: 'Bearer {{token}}', enabled: true },
      { key: 'X', value: '{{base}}', enabled: true },
    ]);
    render(KeyValueTable, { items, label: 'Headers', known: new Set(['base']) });
    const inputs = screen.getAllByRole('textbox').filter((el) => el.getAttribute('data-field') === 'value');
    expect(inputs[0]).toHaveClass('missing');
    expect(inputs[0]).toHaveAttribute('title', 'Variáveis não definidas: token');
    expect(inputs[1]).not.toHaveClass('missing');
  });
});
