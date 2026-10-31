// Pull e push do repositório do workspace.
import { invoke } from '@tauri-apps/api/core';
import type { GitPullResult, GitPushResult, GitStatus } from '../types';

export const gitApi = {
  status: () => invoke<GitStatus>('git_status'),
  pull: () => invoke<GitPullResult>('git_pull'),
  /** `message` é o título do commit; só é obrigatório se houver alterações. */
  push: (message: string | null) => invoke<GitPushResult>('git_push', { message }),
};
