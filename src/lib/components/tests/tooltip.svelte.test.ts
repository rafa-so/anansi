// Tooltip de variáveis (RF-13 da spec do Bearer Token).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/svelte';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import VariableTooltip from '../VariableTooltip.svelte';
import KeyValueTable from '../KeyValueTable.svelte';
import { environments } from '../../state/environments.svelte';
import { ui } from '../../state/ui.svelte';
import type { Peek } from '../../types';

const writeText = vi.hoisted(() => vi.fn());
vi.mock('@tauri-apps/plugin-clipboard-manager', () => ({ writeText, readText: vi.fn() }));

const peeks: Record<string, Peek> = {
  baseUrl: { kind: 'variable', value: 'https://hml.api', empty: false },
  bearer_token: { kind: 'secret', masked: 'eyJh••••••a9Fz', empty: false },
  vazio: { kind: 'secret', masked: '', empty: true },
  nada: { kind: 'missing', empty: true },
};
let reveals: string[];

beforeEach(() => {
  reveals = [];
  writeText.mockReset();
  mockIPC((cmd, args) => {
    const a = args as { key: string };
    if (cmd === 'env_peek') return peeks[a.key];
    if (cmd === 'env_reveal_secret') {
      reveals.push(a.key);
      return 'eyJhbGciOiJIUzI1a9Fz';
    }
    throw new Error(`comando não esperado: ${cmd}`);
  });
  environments.active = 'hml';
  ui.environmentsOpen = false;
  ui.environmentsTarget = null;
});

afterEach(() => {
  cleanup();
  clearMocks();
});

describe('VariableTooltip', () => {
  it('mostra o valor da variável e o segredo só mascarado', async () => {
    render(VariableTooltip, { names: ['baseUrl', 'bearer_token'] });
    const values = await screen.findAllByTestId('var-tooltip-value');
    expect(values.map((v) => v.textContent)).toEqual(['https://hml.api', 'eyJh••••••a9Fz']);
    expect(reveals).toEqual([]);
  });

  it('"Mostrar" busca o valor real do segredo só quando pedido', async () => {
    render(VariableTooltip, { names: ['bearer_token'] });
    await fireEvent.click(await screen.findByTestId('var-tooltip-reveal'));
    expect(await screen.findByText('eyJhbGciOiJIUzI1a9Fz')).toBeInTheDocument();
    expect(reveals).toEqual(['bearer_token']);
  });

  it('"Copiar" copia o valor sem exibi-lo', async () => {
    render(VariableTooltip, { names: ['bearer_token'] });
    await fireEvent.click(await screen.findByTestId('var-tooltip-copy'));
    await vi.waitFor(() => expect(writeText).toHaveBeenCalledWith('eyJhbGciOiJIUzI1a9Fz'));
    expect(screen.getByTestId('var-tooltip-value')).toHaveTextContent('eyJh••••••a9Fz');
  });

  it('segredo vazio ou nome desconhecido oferece "Definir", que abre Ambientes', async () => {
    const onclose = vi.fn();
    render(VariableTooltip, { names: ['vazio', 'nada'], onclose });
    expect(await screen.findAllByTestId('var-tooltip-missing')).toHaveLength(2);
    await fireEvent.click(screen.getAllByTestId('var-tooltip-define')[0]);
    expect(ui.environmentsOpen).toBe(true);
    expect(ui.environmentsTarget).toBe('vazio');
    expect(onclose).toHaveBeenCalled();
  });

  it('sem ambiente ativo, avisa', () => {
    environments.active = null;
    render(VariableTooltip, { names: ['baseUrl'] });
    expect(screen.getByText('Nenhum ambiente ativo.')).toBeInTheDocument();
  });
});

describe('tooltip nos campos da tabela', () => {
  it('abre ao focar um valor com {{nome}} e fecha com Esc', async () => {
    render(KeyValueTable, {
      items: [{ key: 'Authorization', value: 'Bearer {{bearer_token}}', enabled: true }],
      label: 'Headers',
      known: new Set(['bearer_token']),
    });
    const input = screen.getByDisplayValue('Bearer {{bearer_token}}');
    await fireEvent.focus(input);
    const tooltip = await screen.findByTestId('var-tooltip', {}, { timeout: 2000 });
    expect(input.getAttribute('aria-describedby')).toBe(tooltip.id);
    await fireEvent.keyDown(input, { key: 'Escape' });
    expect(screen.queryByTestId('var-tooltip')).toBeNull();
  });

  it('campo sem {{nome}} não abre tooltip', async () => {
    render(KeyValueTable, {
      items: [{ key: 'Accept', value: 'application/json', enabled: true }],
      label: 'Headers',
      known: new Set<string>(),
    });
    await fireEvent.focus(screen.getByDisplayValue('application/json'));
    await new Promise((r) => setTimeout(r, 400));
    expect(screen.queryByTestId('var-tooltip')).toBeNull();
  });
});
