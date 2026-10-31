// Bearer Token como segredo de ambiente (docs/bearer-token-local/SPEC.md).
// O backend troca o token literal por {{bearer_token}} ao salvar; aqui ficam a
// escolha do ambiente (quando nenhum está ativo) e os avisos do resultado.
import { envApi } from './api/environments';
import { dialogs, toasts } from './state/dialogs.svelte';
import { environments } from './state/environments.svelte';
import { BEARER_SECRET_NAME, isAppError, type BearerOutcome, type SecretAction } from './types';

/** Ambiente criado quando o workspace ainda não tem nenhum. */
export const DEFAULT_ENVIRONMENT = 'local';

/** Avisos do workspace: o `.gitignore` corrigido no backend não gera evento do watcher. */
export const bearerEvents: { onGitignoreFixed: (() => void) | null } = { onGitignoreFixed: null };

/** Frase da prévia/aviso: o que acontece com o segredo. */
export function describeAction(action: SecretAction, env: string): string {
  if (action === 'created') return `O segredo ${BEARER_SECRET_NAME} será criado em "${env}".`;
  if (action === 'updated') return `O segredo ${BEARER_SECRET_NAME} será atualizado em "${env}".`;
  return `O segredo ${BEARER_SECRET_NAME} de "${env}" já tem este token.`;
}

/** Máscara do token na prévia do import: só o começo. */
export function maskBearer(value: string): string {
  const match = /^\s*bearer\s+(.+?)\s*$/i.exec(value);
  if (!match || match[1].includes('{{')) return value;
  const token = match[1];
  return `Bearer ${token.slice(0, 4)}…`;
}

/** Garante que o ambiente existe (cria o `local` se preciso) e o deixa ativo. */
export async function useEnvironment(name: string): Promise<void> {
  if (!environments.list.includes(name)) {
    await envApi.create(name);
    bearerEvents.onGitignoreFixed?.();
  }
  await environments.load();
  if (environments.active !== name) await environments.setActive(name);
}

/** Pergunta onde guardar o token quando nenhum ambiente está ativo. `null` se cancelou. */
export async function chooseBearerEnvironment(): Promise<string | null> {
  const message = `Este request tem um token Bearer. Ele vai para o segredo ${BEARER_SECRET_NAME} de um ambiente (só nesta máquina, fora do git) e o request fica com Bearer {{${BEARER_SECRET_NAME}}}.`;
  if (environments.list.length === 0) {
    const ok = await dialogs.confirm(
      'Guardar token Bearer',
      `${message} Criar o ambiente "${DEFAULT_ENVIRONMENT}" e deixá-lo ativo?`,
      `Criar "${DEFAULT_ENVIRONMENT}"`,
    );
    if (!ok) return null;
    await useEnvironment(DEFAULT_ENVIRONMENT);
    return DEFAULT_ENVIRONMENT;
  }
  const choice = await dialogs.choose(
    'Guardar token Bearer',
    `${message} Nenhum ambiente está ativo: em qual guardar? Ele passa a ser o ambiente ativo.`,
    [
      { id: 'cancel', label: 'Cancelar' },
      ...environments.list.map((name) => ({ id: `env:${name}`, label: name })),
    ],
  );
  if (!choice.startsWith('env:')) return null;
  const name = choice.slice('env:'.length);
  await useEnvironment(name);
  return name;
}

/**
 * Roda a gravação com o ambiente ativo. Se o backend pedir um ambiente
 * (`BEARER_NEEDS_ENVIRONMENT`), pergunta e tenta de novo. `null` se cancelou.
 */
export async function withBearerEnvironment<T>(
  action: (env: string | null) => Promise<T>,
): Promise<T | null> {
  try {
    return await action(environments.active);
  } catch (e) {
    if (!isAppError(e) || e.code !== 'BEARER_NEEDS_ENVIRONMENT') throw e;
    const env = await chooseBearerEnvironment();
    if (!env) return null;
    return action(env);
  }
}

/** Avisos depois de guardar o token e atualização dos nomes conhecidos. */
export async function announceBearer(outcome: BearerOutcome | null | undefined): Promise<void> {
  if (!outcome) return;
  if (outcome.action === 'created')
    toasts.show(`Token guardado como segredo ${BEARER_SECRET_NAME} em "${outcome.env}".`);
  else if (outcome.action === 'updated')
    toasts.show(`Segredo ${BEARER_SECRET_NAME} atualizado em "${outcome.env}".`);
  if (outcome.gitignoreFixed) {
    toasts.show('.gitignore atualizado para proteger os segredos. Inclua essa alteração no próximo push.');
    bearerEvents.onGitignoreFixed?.();
  }
  if (outcome.env === environments.active) await environments.loadKnown();
}
