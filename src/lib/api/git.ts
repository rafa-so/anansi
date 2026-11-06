// Pull e push do repositório do workspace.
import { invoke } from './ipc';
import type { BearerOutcome, GitPullResult, GitPushResult, GitStatus } from '../types';

export const gitApi = {
  status: () => invoke<GitStatus>('git_status'),
  pull: () => invoke<GitPullResult>('git_pull'),
  /** `message` é o título do commit; só é obrigatório se houver alterações. */
  push: (message: string | null) => invoke<GitPushResult>('git_push', { message }),
  /** Arquivos alterados com token Bearer em texto puro. */
  bearerLiterals: () => invoke<string[]>('git_bearer_literals'),
  /** "Converter agora": token para o segredo `bearer_token` de `env`. */
  convertBearer: (paths: string[], env: string | null) =>
    invoke<BearerOutcome | null>('git_convert_bearer', { paths, env }),
};
