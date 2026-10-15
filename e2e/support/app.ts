// Atalhos para os testes E2E.
import { $, browser } from '@wdio/globals';

export const byTestId = (id: string) => $(`[data-testid="${id}"]`);

/** Texto do elemento (o WebKitWebDriver às vezes devolve vazio no getText). */
export async function textOf(testid: string): Promise<string> {
  const el = await byTestId(testid);
  await el.waitForExist();
  return browser.execute((e) => e.textContent ?? '', el);
}

export function treeNode(path: string) {
  return $(`[data-testid="tree-node"][data-path="${path}"]`);
}

export async function treePaths(): Promise<string[]> {
  return browser.execute(() =>
    [...document.querySelectorAll<HTMLElement>('[data-testid="tree-node"]')].map(
      (el) => el.dataset.path ?? '',
    ),
  );
}

/** Responde a um pedido de texto do diálogo da interface. */
export async function answerPrompt(text: string) {
  const input = await byTestId('dialog-input');
  await input.waitForDisplayed();
  await input.setValue(text);
  await (await byTestId('dialog-ok')).click();
  await input.waitForDisplayed({ reverse: true });
}

/** Abre o menu de contexto de um nó (evento disparado direto, igual em todos os WebDrivers). */
export async function openContextMenu(path: string) {
  const node = await treeNode(path);
  await node.waitForDisplayed();
  await browser.execute((el) => {
    const rect = el.getBoundingClientRect();
    el.dispatchEvent(
      new MouseEvent('contextmenu', {
        bubbles: true,
        cancelable: true,
        clientX: rect.left + 20,
        clientY: rect.bottom,
      }),
    );
  }, node);
}

export async function menuItem(testid: string) {
  const item = await byTestId(testid);
  await item.waitForDisplayed();
  await item.click();
}

export async function pressCtrl(key: string) {
  await browser.keys(['Control', key]);
}
