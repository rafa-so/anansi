// Rede de segurança: erro que escapa de um handler (ex.: promise sem catch num
// onclick) vira toast e vai para o console, em vez de sumir em silêncio.
import { errorMessage } from './types';

export function installGlobalErrorHandlers(target: Window, notify: (message: string) => void): () => void {
  const report = (error: unknown) => {
    console.error('[anansi] erro não tratado', error);
    notify(`Erro inesperado: ${errorMessage(error)}`);
  };
  const onRejection = (e: PromiseRejectionEvent) => report(e.reason);
  const onError = (e: ErrorEvent) => report(e.error ?? e.message);
  target.addEventListener('unhandledrejection', onRejection);
  target.addEventListener('error', onError);
  return () => {
    target.removeEventListener('unhandledrejection', onRejection);
    target.removeEventListener('error', onError);
  };
}
