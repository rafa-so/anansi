// Liga o tooltip de variáveis aos campos com {{nome}} (inputs) e ao CodeMirror.
import { mount, unmount } from 'svelte';
import { hoverTooltip, type Tooltip } from '@codemirror/view';
import VariableTooltip from './components/VariableTooltip.svelte';
import { findPlaceholders } from './variables';

const OPEN_DELAY_MS = 300;
const CLOSE_DELAY_MS = 200;
let seq = 0;

function namesIn(text: string): string[] {
  return [...new Set(findPlaceholders(text).map((p) => p.name))];
}

/**
 * Ação Svelte para `<input>`: ao passar o mouse ou focar um campo com `{{nome}}`,
 * mostra o tooltip abaixo dele. Fecha ao sair, com `Esc` ou ao perder o foco.
 */
export function variableTooltip(node: HTMLInputElement, enabled: boolean = true) {
  let active = enabled;
  let openTimer: ReturnType<typeof setTimeout> | undefined;
  let closeTimer: ReturnType<typeof setTimeout> | undefined;
  let host: HTMLDivElement | null = null;
  let component: ReturnType<typeof mount> | null = null;
  const id = `var-tooltip-${++seq}`;

  function close() {
    clearTimeout(openTimer);
    clearTimeout(closeTimer);
    if (component) void unmount(component);
    component = null;
    host?.remove();
    host = null;
    node.removeAttribute('aria-describedby');
  }

  function open() {
    const names = namesIn(node.value);
    if (!active || names.length === 0) return;
    close();
    const rect = node.getBoundingClientRect();
    host = document.createElement('div');
    host.style.position = 'fixed';
    host.style.left = `${Math.max(4, rect.left)}px`;
    host.style.top = `${rect.bottom + 4}px`;
    host.style.zIndex = '1000';
    host.addEventListener('mouseenter', () => clearTimeout(closeTimer));
    host.addEventListener('mouseleave', scheduleClose);
    host.addEventListener('keydown', onKeydown);
    host.addEventListener('focusout', (e) => {
      if (!host?.contains(e.relatedTarget as Node | null) && e.relatedTarget !== node) scheduleClose();
    });
    document.body.appendChild(host);
    component = mount(VariableTooltip, { target: host, props: { names, id, onclose: close } });
    node.setAttribute('aria-describedby', id);
  }

  function scheduleOpen() {
    clearTimeout(closeTimer);
    if (host) return;
    clearTimeout(openTimer);
    openTimer = setTimeout(open, OPEN_DELAY_MS);
  }

  function scheduleClose() {
    clearTimeout(openTimer);
    clearTimeout(closeTimer);
    closeTimer = setTimeout(close, CLOSE_DELAY_MS);
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape' && host) {
      e.stopPropagation();
      close();
      node.focus();
    }
  }

  function onBlur(e: FocusEvent) {
    if (host?.contains(e.relatedTarget as Node | null)) return;
    scheduleClose();
  }

  node.addEventListener('mouseenter', scheduleOpen);
  node.addEventListener('mouseleave', scheduleClose);
  node.addEventListener('focus', scheduleOpen);
  node.addEventListener('blur', onBlur);
  node.addEventListener('keydown', onKeydown);
  node.addEventListener('input', close);

  return {
    update(next: boolean = true) {
      active = next;
      if (!active) close();
    },
    destroy() {
      close();
      node.removeEventListener('mouseenter', scheduleOpen);
      node.removeEventListener('mouseleave', scheduleClose);
      node.removeEventListener('focus', scheduleOpen);
      node.removeEventListener('blur', onBlur);
      node.removeEventListener('keydown', onKeydown);
      node.removeEventListener('input', close);
    },
  };
}

/** Extensão do CodeMirror: tooltip ao passar o mouse sobre um `{{nome}}` do body. */
export function variableHoverTooltip() {
  return hoverTooltip(
    (view, pos): Tooltip | null => {
      const line = view.state.doc.lineAt(pos);
      const hit = findPlaceholders(line.text).find(
        (p) => line.from + p.start <= pos && pos <= line.from + p.end,
      );
      if (!hit) return null;
      return {
        pos: line.from + hit.start,
        end: line.from + hit.end,
        above: false,
        create: () => {
          const dom = document.createElement('div');
          const component = mount(VariableTooltip, { target: dom, props: { names: [hit.name] } });
          return { dom, destroy: () => void unmount(component) };
        },
      };
    },
    { hoverTime: OPEN_DELAY_MS },
  );
}
