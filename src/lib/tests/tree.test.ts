import { describe, expect, it } from 'vitest';
import {
  countRequests,
  filterTree,
  findNode,
  fold,
  isSameOrInside,
  listContainers,
  movePath,
  parentPath,
  selfAndAncestors,
} from '../tree';
import type { TreeNode } from '../types';

const tree: TreeNode[] = [
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
            kind: 'request',
            path: 'store-api/products/listar.json',
            name: 'Listar Ações',
            method: 'GET',
            url: '{{base}}/products',
          },
          {
            kind: 'request',
            path: 'store-api/products/criar.json',
            name: 'Criar produto',
            method: 'POST',
            url: '{{base}}/products/new',
          },
        ],
      },
      {
        kind: 'request',
        path: 'store-api/health.json',
        name: 'Health',
        method: 'GET',
        url: '{{base}}/health',
      },
    ],
  },
  { kind: 'collection', path: 'billing-api', name: 'Billing API', children: [] },
];

describe('filterTree', () => {
  it('sem termo devolve a árvore inteira e nada a expandir', () => {
    const result = filterTree(tree, '  ');
    expect(result.nodes).toBe(tree);
    expect(result.expand.size).toBe(0);
  });

  it('busca por nome sem diferenciar maiúsculas e acentos e expande os ancestrais', () => {
    const { nodes, expand } = filterTree(tree, 'acoes');
    expect(nodes).toHaveLength(1);
    expect(findNode(nodes, 'store-api/products/listar.json')).not.toBeNull();
    expect(findNode(nodes, 'store-api/health.json')).toBeNull();
    expect([...expand].sort()).toEqual(['store-api', 'store-api/products']);
  });

  it('busca por parte da URL', () => {
    const { nodes } = filterTree(tree, '/NEW');
    expect(findNode(nodes, 'store-api/products/criar.json')).not.toBeNull();
    expect(countRequests(nodes[0])).toBe(1);
  });

  it('esconde collections sem resultado', () => {
    expect(filterTree(tree, 'health').nodes.map((n) => n.path)).toEqual(['store-api']);
  });
});

describe('caminhos', () => {
  it('calcula o pai e compara prefixos por componente', () => {
    expect(parentPath('a/b/c.json')).toBe('a/b');
    expect(parentPath('a')).toBe('');
    expect(isSameOrInside('a/b/c', 'a/b')).toBe(true);
    expect(isSameOrInside('a/bc', 'a/b')).toBe(false);
  });

  it('troca o prefixo após renomear', () => {
    expect(movePath('a/b/c.json', 'a/b', 'a/x')).toBe('a/x/c.json');
    expect(movePath('a/b', 'a/b', 'a/x')).toBe('a/x');
  });

  it('remove acentos ao normalizar', () => {
    expect(fold('AÇÃO')).toBe('acao');
  });
});

describe('listContainers', () => {
  it('lista collections e pastas em ordem de árvore, com o caminho exibido', () => {
    expect(listContainers(tree)).toEqual([
      { path: 'store-api', label: 'Store API' },
      { path: 'store-api/products', label: 'Store API / products' },
      { path: 'billing-api', label: 'Billing API' },
    ]);
  });

  it('inclui pastas em qualquer profundidade e nunca requests', () => {
    const deep: TreeNode[] = [
      {
        kind: 'collection',
        path: 'c',
        name: 'C',
        children: [
          {
            kind: 'folder',
            path: 'c/a',
            name: 'a',
            children: [
              { kind: 'folder', path: 'c/a/b', name: 'b', children: [] },
              { kind: 'request', path: 'c/a/r.json', name: 'R', method: 'GET', url: '' },
            ],
          },
        ],
      },
    ];
    expect(listContainers(deep).map((o) => o.label)).toEqual(['C', 'C / a', 'C / a / b']);
  });

  it('árvore vazia não tem destinos', () => {
    expect(listContainers([])).toEqual([]);
  });
});

describe('selfAndAncestors', () => {
  it('devolve o caminho e todos os acima dele, do topo para baixo', () => {
    expect(selfAndAncestors('a/b/c')).toEqual(['a', 'a/b', 'a/b/c']);
    expect(selfAndAncestors('a')).toEqual(['a']);
  });
});
