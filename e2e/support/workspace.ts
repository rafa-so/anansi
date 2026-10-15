// Workspace temporário para os testes E2E, criado a partir da fixture do anansi-core.
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FIXTURE = path.resolve(HERE, '../../src-tauri/crates/anansi-core/tests/fixtures/workspace-basic');

/** Copia a fixture (`_x` vira `.x`) e aponta o ambiente `local` para o servidor falso. */
export function createWorkspace(baseUrl: string): { root: string; prefsDir: string } {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'anansi-e2e-'));
  const root = path.join(tmp, 'minhas-collections');
  copyDir(FIXTURE, root);
  const local = { schemaVersion: 1, variables: { baseUrlStore: baseUrl } };
  fs.writeFileSync(path.join(root, 'environments', 'local.json'), JSON.stringify(local, null, 2) + '\n');
  const prefsDir = path.join(tmp, 'prefs');
  fs.mkdirSync(prefsDir);
  return { root, prefsDir };
}

function copyDir(src: string, dst: string) {
  fs.mkdirSync(dst, { recursive: true });
  for (const entry of fs.readdirSync(src, { withFileTypes: true })) {
    const target = path.join(dst, entry.name.startsWith('_') ? '.' + entry.name.slice(1) : entry.name);
    if (entry.isDirectory()) copyDir(path.join(src, entry.name), target);
    else fs.copyFileSync(path.join(src, entry.name), target);
  }
}
