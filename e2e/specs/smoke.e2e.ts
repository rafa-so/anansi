// Cenários de fumaça (Seção 15.4). Rodam em sequência sobre o mesmo workspace temporário.
import fs from 'node:fs';
import path from 'node:path';
import { browser, expect } from '@wdio/globals';
import {
  answerPrompt,
  byTestId,
  menuItem,
  openContextMenu,
  pressCtrl,
  textOf,
  treeNode,
  treePaths,
} from '../support/app.ts';

const root = () => process.env.ANANSI_E2E_WORKSPACE!;

describe('anansi', () => {
  it('abre o workspace da fixture e mostra a árvore', async () => {
    await browser.waitUntil(async () => (await textOf('workspace-name')).includes('Demo'));
    await (await treeNode('store-api')).click();
    await (await treeNode('store-api/products')).waitForDisplayed();
    const paths = await treePaths();
    expect(paths).toEqual(['billing-api', 'store-api', 'store-api/products', 'store-api/health-check.json']);
  });

  it('cria collection, pasta e request e grava os arquivos', async () => {
    await (await byTestId('new-collection')).click();
    await answerPrompt('E2E API');
    await openContextMenu('e2e-api');
    await menuItem('menu-new-folder');
    await answerPrompt('Ações');
    expect(fs.existsSync(path.join(root(), 'e2e-api', 'acoes', '.gitkeep'))).toBe(true);

    await openContextMenu('e2e-api/acoes');
    await menuItem('menu-new-request');
    const url = await byTestId('url');
    await url.setValue('{{baseUrlStore}}/health?full=1');
    await (await byTestId('request-name')).click(); // tira o foco da URL: a query vai para a tabela
    await pressCtrl('s');
    await answerPrompt('Verificar saúde');

    const file = path.join(root(), 'e2e-api', 'acoes', 'verificar-saude.json');
    await browser.waitUntil(() => fs.existsSync(file));
    const saved = JSON.parse(fs.readFileSync(file, 'utf8'));
    expect(saved).toMatchObject({
      name: 'Verificar saúde',
      url: '{{baseUrlStore}}/health',
      query: [{ key: 'full', value: '1' }],
    });
    expect(fs.readFileSync(file, 'utf8')).not.toContain('\r');
    await expect(await byTestId('tab-dirty')).not.toBeExisting();
  });

  it('edita sem salvar, tenta fechar a aba e cancela', async () => {
    await (await byTestId('url')).setValue('{{baseUrlStore}}/outro');
    await expect(await byTestId('tab-dirty')).toBeDisplayed();
    await pressCtrl('w');
    await (await byTestId('dialog-cancel')).click();
    await expect(await byTestId('tab')).toBeDisplayed();
    // Descartar fecha sem gravar.
    await pressCtrl('w');
    await (await byTestId('dialog-discard')).click();
    await expect(await byTestId('tab')).not.toBeExisting();
    const saved = JSON.parse(
      fs.readFileSync(path.join(root(), 'e2e-api', 'acoes', 'verificar-saude.json'), 'utf8'),
    );
    expect(saved.url).toBe('{{baseUrlStore}}/health');
  });

  it('Ctrl+P foca a busca sem abrir a impressão', async () => {
    await pressCtrl('p');
    const search = await byTestId('tree-search');
    await expect(search).toBeFocused();
    await search.setValue('saude');
    await (await treeNode('e2e-api/acoes/verificar-saude.json')).waitForDisplayed();
    expect(await treePaths()).toEqual(['e2e-api', 'e2e-api/acoes', 'e2e-api/acoes/verificar-saude.json']);
    await search.clearValue();
  });
});
