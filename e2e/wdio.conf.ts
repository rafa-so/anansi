// Testes E2E com WebdriverIO + tauri-driver (Seção 15.4 da spec).
//
// Pré-requisitos no Windows:
//   cargo install tauri-driver --locked
//   msedgedriver.exe com a mesma versão principal do WebView2, no PATH
//   (ou informe o caminho em ANANSI_E2E_NATIVE_DRIVER)
// Rodar: npm run test:e2e  (gera o binário de debug e roda os testes)
import { spawn, type ChildProcess } from 'node:child_process';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { startFakeServer } from './support/server.ts';
import { createWorkspace } from './support/workspace.ts';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const exe = process.platform === 'win32' ? 'anansi.exe' : 'anansi';
const application = path.resolve(HERE, '../src-tauri/target/debug', exe);
const tauriDriver = path.join(
  os.homedir(),
  '.cargo',
  'bin',
  process.platform === 'win32' ? 'tauri-driver.exe' : 'tauri-driver',
);

let driver: ChildProcess | undefined;
let server: Awaited<ReturnType<typeof startFakeServer>> | undefined;

export const config: WebdriverIO.Config = {
  runner: 'local',
  host: '127.0.0.1',
  port: 4444,
  // Ordem fixa: os cenários compartilham o mesmo workspace temporário.
  specs: ['./specs/smoke.e2e.ts', './specs/execucao.e2e.ts'],
  maxInstances: 1,
  capabilities: [
    // @ts-expect-error capacidade própria do tauri-driver
    { 'tauri:options': { application } },
  ],
  logLevel: 'warn',
  reporters: ['spec'],
  framework: 'mocha',
  mochaOpts: { ui: 'bdd', timeout: 60_000 },
  waitforTimeout: 10_000,

  // Servidor falso e workspace temporário; o app lê o workspace das variáveis de ambiente.
  onPrepare: async () => {
    server = await startFakeServer();
    const { root, prefsDir } = createWorkspace(server.url);
    process.env.ANANSI_E2E_WORKSPACE = root;
    process.env.ANANSI_E2E_PREFS_DIR = prefsDir;
    process.env.ANANSI_E2E_SERVER_URL = server.url;
  },

  beforeSession: () => {
    const args = process.env.ANANSI_E2E_NATIVE_DRIVER
      ? ['--native-driver', process.env.ANANSI_E2E_NATIVE_DRIVER]
      : [];
    driver = spawn(tauriDriver, args, { stdio: [null, process.stdout, process.stderr], env: process.env });
  },

  afterSession: () => {
    driver?.kill();
  },

  onComplete: async () => {
    await server?.close();
  },
};
