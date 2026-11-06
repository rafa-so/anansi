// "Mover para…" (docs/mover-request): diálogo e fluxo depois de mover.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import userEvent from '@testing-library/user-event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import MoveRequestDialog from '../MoveRequestDialog.svelte';
import MoveDialogHost from './MoveDialogHost.svelte';
import { workspace } from '../../state/workspace.svelte';
import { tabs } from '../../state/tabs.svelte';
import { toasts } from '../../state/dialogs.svelte';
import { ui } from '../../state/ui.svelte';
import type { RequestFile, TreeNode } from '../../types';

vi.mock('@tauri-apps/plugin-dialog', () => ({ open: vi.fn(), save: vi.fn() }));

const REQUEST = 'store-api/products/reviews/listar.json';

const tree: TreeNode[] = [
  { kind: 'collection', path: 'billing-api', name: 'Billing API', children: [] },
  {
    kind: 'collection',
    path: 'store-api',
    name: 'Store API',
    children: [
      {
        kind: 'folder',
        path: 'store-api/products',
        name: 'products',
        children: [
          {
            kind: 'folder',
            path: 'store-api/products/reviews',
            name: 'reviews',
            children: [{ kind: 'request', path: REQUEST, name: 'Listar', method: 'GET', url: '/x' }],
          },
          { kind: 'folder', path: 'store-api/products/ações', name: 'Ações', children: [] },
        ],
      },
    ],
  },
];

const stored: RequestFile = { schemaVersion: 1, name: 'Listar', method: 'GET', url: '/x' };

let moveResult: { path: string; renamed: boolean };
let moveCalls: Record<string, unknown>[];

beforeEach(() => {
  moveCalls = [];
  moveResult = { path: 'billing-api/listar.json', renamed: false };
  mockIPC((cmd, args) => {
    switch (cmd) {
      case 'request_move':
        moveCalls.push(args as Record<string, unknown>);
        return moveResult;
      case 'request_get':
        return structuredClone(stored);
      case 'workspace_tree':
        return tree;
      default:
        throw new Error(`comando não esperado: ${cmd}`);
    }
  });
  workspace.info = { path: '/w', name: 'w', environments: [], gitignoreOk: true };
  workspace.tree = tree;
  workspace.expanded.clear();
  workspace.selected = null;
  tabs.closeAllNow();
  toasts.items = [];
  ui.moveRequest = { path: REQUEST, name: 'Listar' };
});

afterEach(() => {
  clearMocks();
  workspace.info = null;
  ui.moveRequest = null;
});

function renderDialog() {
  render(MoveRequestDialog, { path: REQUEST, name: 'Listar' });
  return userEvent.setup();
}

const labels = () =>
  screen.queryAllByTestId('move-option').map((el) => el.querySelector('.label')!.textContent);
const option = (path: string) => screen.getAllByTestId('move-option').find((el) => el.dataset.path === path)!;

describe('MoveRequestDialog', () => {
  it('lista todas as collections e subpastas com o caminho exibido, na ordem da árvore', () => {
    renderDialog();
    expect(labels()).toEqual([
      'Billing API',
      'Store API',
      'Store API / products',
      'Store API / products / reviews',
      'Store API / products / Ações',
    ]);
  });

  it('mostra a pasta atual desabilitada e não deixa escolhê-la', async () => {
    const user = renderDialog();
    const current = option('store-api/products/reviews');
    expect(current).toHaveAttribute('aria-disabled', 'true');
    expect(current).toHaveTextContent('(pasta atual)');
    await user.click(current);
    expect(screen.getByTestId('move-confirm')).toBeDisabled();
  });

  it('foca a busca e filtra pelo caminho sem diferenciar maiúsculas e acentos', async () => {
    const user = renderDialog();
    await vi.waitFor(() => expect(screen.getByTestId('move-search')).toHaveFocus());
    await user.type(screen.getByTestId('move-search'), 'ACOES');
    expect(labels()).toEqual(['Store API / products / Ações']);
    await user.clear(screen.getByTestId('move-search'));
    await user.type(screen.getByTestId('move-search'), 'billing');
    expect(labels()).toEqual(['Billing API']);
  });

  it('avisa quando a busca não encontra nenhuma pasta', async () => {
    const user = renderDialog();
    await user.type(screen.getByTestId('move-search'), 'nada-disso');
    expect(screen.getByText('Nenhuma pasta encontrada.')).toBeInTheDocument();
    expect(screen.getByTestId('move-confirm')).toBeDisabled();
  });

  it('botão Mover fica desabilitado até escolher um destino', async () => {
    const user = renderDialog();
    expect(screen.getByTestId('move-confirm')).toBeDisabled();
    await user.click(option('billing-api'));
    expect(screen.getByTestId('move-confirm')).toBeEnabled();
  });

  it('setas pulam a pasta atual e Enter move para o item destacado', async () => {
    const user = renderDialog();
    const search = screen.getByTestId('move-search');
    await user.click(search);
    // billing-api → store-api → products → (reviews é a atual) → Ações
    await user.keyboard('{ArrowDown}{ArrowDown}{ArrowDown}{ArrowDown}');
    expect(option('store-api/products/ações')).toHaveAttribute('aria-selected', 'true');
    await user.keyboard('{ArrowUp}');
    expect(option('store-api/products')).toHaveAttribute('aria-selected', 'true');
    await user.keyboard('{Enter}');
    await vi.waitFor(() => expect(moveCalls).toHaveLength(1));
    expect(moveCalls[0]).toEqual({ path: REQUEST, targetParent: 'store-api/products' });
    expect(ui.moveRequest).toBeNull();
  });

  it('montado como no App (props vindas de ui.moveRequest), o clique em Mover chama o backend', async () => {
    render(MoveDialogHost);
    const user = userEvent.setup();
    await user.click(option('billing-api'));
    await user.click(screen.getByTestId('move-confirm'));
    await vi.waitFor(() => expect(moveCalls).toHaveLength(1));
    expect(moveCalls[0]).toEqual({ path: REQUEST, targetParent: 'billing-api' });
    expect(ui.moveRequest).toBeNull();
    await vi.waitFor(() => expect(workspace.selected).toBe('billing-api/listar.json'));
  });

  it('Esc fecha sem mover', async () => {
    const user = renderDialog();
    await user.click(screen.getByTestId('move-search'));
    await user.keyboard('{ArrowDown}{Escape}');
    expect(ui.moveRequest).toBeNull();
    expect(moveCalls).toHaveLength(0);
  });
});

describe('depois de mover', () => {
  it('abre o destino e as pastas acima, seleciona o request e avisa no toast', async () => {
    moveResult = { path: 'store-api/products/listar.json', renamed: false };
    const user = renderDialog();
    await user.dblClick(option('store-api/products'));
    await vi.waitFor(() => expect(workspace.selected).toBe('store-api/products/listar.json'));
    expect(workspace.expanded.has('store-api')).toBe(true);
    expect(workspace.expanded.has('store-api/products')).toBe(true);
    expect(toasts.items.map((t) => t.text)).toContain('Request "Listar" movido para Store API / products.');
  });

  it('com conflito de nome, o toast informa o novo nome do arquivo', async () => {
    moveResult = { path: 'billing-api/listar-2.json', renamed: true };
    await workspace.move(REQUEST, { path: 'billing-api', label: 'Billing API' });
    expect(toasts.items.map((t) => t.text)).toContain(
      'Request "Listar" movido para Billing API como listar-2.json, porque já havia um arquivo com esse nome.',
    );
  });

  it('a aba aberta passa para o novo caminho e mantém as alterações não salvas', async () => {
    await tabs.openRequest(REQUEST);
    tabs.active!.draft.url = '/alterado';
    await workspace.move(REQUEST, { path: 'billing-api', label: 'Billing API' });
    expect(tabs.active!.path).toBe('billing-api/listar.json');
    expect(tabs.active!.draft.url).toBe('/alterado');
    expect(tabs.isDirty(tabs.active!)).toBe(true);
  });

  it('erro do backend vira toast e não muda a seleção', async () => {
    mockIPC((cmd) => {
      if (cmd === 'request_move') throw { code: 'IO_ERROR', message: 'Arquivo em uso.' };
      if (cmd === 'workspace_tree') return tree;
      throw new Error(`comando não esperado: ${cmd}`);
    });
    await workspace.move(REQUEST, { path: 'billing-api', label: 'Billing API' });
    expect(workspace.selected).toBeNull();
    expect(toasts.items.map((t) => t.text)).toContain('Arquivo em uso.');
  });
});
