// Execução, ambientes, cURL e mudanças no disco (Seção 15.4, cenários 3, 4, 6 e 8).
// Roda depois do smoke.e2e.ts, no mesmo workspace temporário (ordem em wdio.conf.ts).
import fs from 'node:fs';
import path from 'node:path';
import { $$, browser, expect } from '@wdio/globals';
import { byTestId, menuItem, openContextMenu, pressCtrl, textOf, treeNode } from '../support/app.ts';

const root = () => process.env.ANANSI_E2E_WORKSPACE!;

async function openRequest(collection: string, requestPath: string) {
  const node = await treeNode(requestPath);
  if (!(await node.isExisting())) await (await treeNode(collection)).click();
  await (await treeNode(requestPath)).click();
  await (await byTestId('url')).waitForDisplayed();
}

async function setUrl(value: string) {
  const url = await byTestId('url');
  await url.setValue(value);
  await (await byTestId('request-name')).click();
}

describe('execução de requests', () => {
  before(async () => {
    await browser.waitUntil(async () => (await textOf('workspace-name')).includes('Demo'));
    await (await byTestId('env-select')).selectByAttribute('value', 'local');
  });

  it('envia com Ctrl+Enter usando o ambiente ativo e mostra a resposta', async () => {
    await openRequest('store-api', 'store-api/health-check.json');
    await pressCtrl('Enter');
    const status = await byTestId('response-status');
    await status.waitForDisplayed();
    expect(await textOf('response-status')).toContain('200 OK');
    expect(await textOf('response-body')).toContain('"status": "ok"');
  });

  it('avisa variáveis não definidas antes de enviar', async () => {
    await setUrl('{{naoExiste}}/health');
    await expect(await byTestId('unresolved')).toBeDisplayed();
    await (await byTestId('send')).click();
    await (await byTestId('dialog-cancel')).click();
    await setUrl('{{baseUrlStore}}/health');
  });

  it('cancela uma requisição lenta', async () => {
    await setUrl('{{baseUrlStore}}/slow');
    await (await byTestId('send')).click();
    await (await byTestId('cancel-send')).waitForDisplayed();
    await browser.keys(['Escape']);
    await (await byTestId('response-error')).waitForDisplayed();
    expect(await textOf('response-error')).toContain('Requisição cancelada');
    await setUrl('{{baseUrlStore}}/health');
  });

  it('recarrega a aba quando o arquivo muda no disco', async () => {
    // Descarta a edição de URL dos cenários anteriores.
    await pressCtrl('w');
    const discard = await byTestId('dialog-discard');
    if (await discard.isExisting()) await discard.click();
    await openRequest('store-api', 'store-api/health-check.json');

    const file = path.join(root(), 'store-api', 'health-check.json');
    const data = JSON.parse(fs.readFileSync(file, 'utf8'));
    data.url = '{{baseUrlStore}}/health?alterado=1';
    fs.writeFileSync(file, JSON.stringify(data, null, 2) + '\n');
    await browser.waitUntil(
      async () => (await (await byTestId('url')).getValue()) === '{{baseUrlStore}}/health?alterado=1',
      {
        timeout: 10_000,
        timeoutMsg: 'a aba não foi recarregada',
      },
    );

    // Com edição pendente, aparece o aviso em vez de recarregar.
    await (await byTestId('request-name')).setValue('Editado no app');
    data.url = '{{baseUrlStore}}/health?alterado=2';
    fs.writeFileSync(file, JSON.stringify(data, null, 2) + '\n');
    await (await byTestId('disk-changed')).waitForDisplayed({ timeout: 10_000 });
    await (await byTestId('disk-reload')).click();
    expect(await (await byTestId('url')).getValue()).toBe('{{baseUrlStore}}/health?alterado=2');
  });

  it('importa um cURL do Postman', async () => {
    await (await byTestId('import-curl')).click();
    const text = await byTestId('curl-text');
    await text.setValue(
      "curl --location --globoff '{{baseUrlStore}}/echo' \\\n--header 'x-api-key;' \\\n--header 'Content-Type: application/json' \\\n--data '{\"Id\": \"12345678\"}'",
    );
    await (await byTestId('curl-preview')).waitForDisplayed();
    await (await byTestId('curl-confirm')).click();
    await browser.waitUntil(() => fs.existsSync(path.join(root(), 'store-api', 'post-echo.json')));
    expect(fs.readFileSync(path.join(root(), 'store-api', 'post-echo.body.json'), 'utf8')).toBe(
      '{"Id": "12345678"}',
    );
    await pressCtrl('Enter');
    await (await byTestId('response-status')).waitForDisplayed();
    expect(await textOf('response-body')).toContain('12345678');
  });

  it('cadastra uma variável na tela de ambientes', async () => {
    await (await byTestId('open-environments')).click();
    await (await byTestId('environments-dialog')).waitForDisplayed();
    await (await byTestId('var-add')).click();
    const keys = await $$('[data-testid="var-key"]').getElements();
    const values = await $$('[data-testid="var-value"]').getElements();
    await keys[keys.length - 1].setValue('novaVariavel');
    await values[values.length - 1].setValue('123');
    await (await byTestId('env-save')).click();
    await browser.waitUntil(() =>
      fs.readFileSync(path.join(root(), 'environments', 'local.json'), 'utf8').includes('novaVariavel'),
    );
    await browser.keys(['Escape']);
  });

  it('copia como cURL pelo menu da árvore', async () => {
    await openContextMenu('store-api/health-check.json');
    await menuItem('menu-curl-bash');
    const toasts = () =>
      browser.execute(() => [...document.querySelectorAll('.toast')].map((e) => e.textContent ?? ''));
    await browser
      .waitUntil(async () => (await toasts()).some((t) => t.includes('cURL copiado')), {
        timeoutMsg: 'aviso de cópia não apareceu',
      })
      .catch(async (e) => {
        throw new Error(`${e.message}; avisos: ${JSON.stringify(await toasts())}`);
      });
  });
});
