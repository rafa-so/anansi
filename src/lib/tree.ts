// Funções puras sobre a árvore: busca (RF-19), localização e caminhos.
import type { TreeNode } from './types';

/** Minúsculas e sem acentos, para comparar textos. */
export function fold(text: string): string {
  return text.normalize('NFD').replace(/[̀-ͯ]/g, '').toLowerCase();
}

/**
 * Filtra por nome ou URL dos requests, sem diferenciar maiúsculas e acentos.
 * Mantém os ancestrais dos itens encontrados e devolve os caminhos a expandir.
 */
export function filterTree(nodes: TreeNode[], term: string): { nodes: TreeNode[]; expand: Set<string> } {
  const needle = fold(term.trim());
  const expand = new Set<string>();
  if (!needle) return { nodes, expand };

  const visit = (node: TreeNode): TreeNode | null => {
    if (node.kind === 'request') {
      return fold(node.name).includes(needle) || fold(node.url).includes(needle) ? node : null;
    }
    const children = node.children.map(visit).filter((n): n is TreeNode => n !== null);
    if (children.length === 0) return null;
    expand.add(node.path);
    return { ...node, children };
  };

  return { nodes: nodes.map(visit).filter((n): n is TreeNode => n !== null), expand };
}

export function findNode(nodes: TreeNode[], path: string): TreeNode | null {
  for (const node of nodes) {
    if (node.path === path) return node;
    if (node.kind !== 'request') {
      const found = findNode(node.children, path);
      if (found) return found;
    }
  }
  return null;
}

export function parentPath(path: string): string {
  const i = path.lastIndexOf('/');
  return i < 0 ? '' : path.slice(0, i);
}

/** `path` é o próprio `base` ou está dentro dele. */
export function isSameOrInside(path: string, base: string): boolean {
  return path === base || path.startsWith(base + '/');
}

/** Troca o prefixo `from` por `to` (após renomear pasta ou collection). */
export function movePath(path: string, from: string, to: string): string {
  return path === from ? to : to + path.slice(from.length);
}

export function countRequests(node: TreeNode): number {
  return node.kind === 'request' ? 1 : node.children.reduce((sum, c) => sum + countRequests(c), 0);
}
