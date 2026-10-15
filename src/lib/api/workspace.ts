// Wrappers tipados dos comandos do backend: o único ponto de contato com o invoke.
import { invoke } from '@tauri-apps/api/core';
import type { TreeNode, WorkspaceInfo } from '../types';

export const workspaceApi = {
  open: (path: string) => invoke<WorkspaceInfo>('workspace_open', { path }),
  init: (path: string) => invoke<WorkspaceInfo>('workspace_init', { path }),
  current: () => invoke<WorkspaceInfo | null>('workspace_current'),
  tree: () => invoke<TreeNode[]>('workspace_tree'),
  requestCount: (path: string) => invoke<number>('item_request_count', { path }),
};

export const collectionApi = {
  create: (name: string, description?: string) =>
    invoke<TreeNode>('collection_create', { name, description: description ?? null }),
  update: (path: string, name: string, description?: string) =>
    invoke<TreeNode>('collection_update', { path, name, description: description ?? null }),
  remove: (path: string) => invoke<void>('collection_delete', { path }),
};

export const folderApi = {
  create: (parentPath: string, name: string) => invoke<TreeNode>('folder_create', { parentPath, name }),
  rename: (path: string, name: string) => invoke<TreeNode>('folder_rename', { path, name }),
  remove: (path: string) => invoke<void>('folder_delete', { path }),
};
