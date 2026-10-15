<script lang="ts">
  // Wrapper do CodeMirror 6: body do request e visualizador de resposta.
  // Destaca {{nome}}: conhecido em azul, não definido em vermelho (Seção 6.4).
  import { onMount } from 'svelte';
  import { Compartment, EditorState, RangeSetBuilder } from '@codemirror/state';
  import {
    Decoration,
    EditorView,
    ViewPlugin,
    keymap,
    lineNumbers,
    highlightActiveLine,
    placeholder as cmPlaceholder,
    type DecorationSet,
    type ViewUpdate,
  } from '@codemirror/view';
  import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
  import {
    bracketMatching,
    foldGutter,
    syntaxHighlighting,
    defaultHighlightStyle,
  } from '@codemirror/language';
  import { json } from '@codemirror/lang-json';
  import { findPlaceholders } from '../variables';

  type Props = {
    value: string;
    language?: 'json' | 'text';
    readonly?: boolean;
    /** Nomes conhecidos do ambiente ativo; `null` desliga o destaque. */
    known?: ReadonlySet<string> | null;
    placeholder?: string;
    ariaLabel: string;
    testid?: string;
  };

  let {
    value = $bindable(),
    language = 'text',
    readonly = false,
    known = null,
    placeholder = '',
    ariaLabel,
    testid,
  }: Props = $props();

  let host = $state<HTMLDivElement | null>(null);
  let view: EditorView | null = null;
  const langSlot = new Compartment();
  const varsSlot = new Compartment();
  const readonlySlot = new Compartment();

  function variableHighlighter(names: ReadonlySet<string> | null) {
    if (!names) return [];
    const ok = Decoration.mark({ class: 'cm-var cm-var-ok' });
    const missing = Decoration.mark({ class: 'cm-var cm-var-missing' });
    const build = (v: EditorView): DecorationSet => {
      const builder = new RangeSetBuilder<Decoration>();
      for (const { from, to } of v.visibleRanges) {
        const text = v.state.doc.sliceString(from, to);
        for (const p of findPlaceholders(text)) {
          builder.add(from + p.start, from + p.end, names.has(p.name) ? ok : missing);
        }
      }
      return builder.finish();
    };
    return ViewPlugin.fromClass(
      class {
        decorations: DecorationSet;
        constructor(v: EditorView) {
          this.decorations = build(v);
        }
        update(u: ViewUpdate) {
          if (u.docChanged || u.viewportChanged) this.decorations = build(u.view);
        }
      },
      { decorations: (p) => p.decorations },
    );
  }

  const theme = EditorView.theme({
    '&': { height: '100%', fontSize: '12.5px', backgroundColor: 'var(--bg)', color: 'var(--text)' },
    '.cm-scroller': { fontFamily: 'var(--mono)', lineHeight: '1.5' },
    '.cm-gutters': { backgroundColor: 'var(--bg-soft)', color: 'var(--text-muted)', border: 'none' },
    '.cm-activeLine': { backgroundColor: 'transparent' },
    '&.cm-focused .cm-activeLine': { backgroundColor: 'var(--bg-soft)' },
    '.cm-content': { caretColor: 'var(--text)' },
    '&.cm-focused': { outline: '2px solid var(--focus)', outlineOffset: '-1px' },
  });

  onMount(() => {
    view = new EditorView({
      parent: host!,
      state: EditorState.create({
        doc: value,
        extensions: [
          lineNumbers(),
          foldGutter(),
          history(),
          bracketMatching(),
          highlightActiveLine(),
          syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
          keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
          EditorView.lineWrapping,
          EditorState.tabSize.of(2),
          cmPlaceholder(placeholder),
          theme,
          EditorView.contentAttributes.of({
            'aria-label': ariaLabel,
            ...(testid ? { 'data-testid': testid } : {}),
          }),
          langSlot.of(language === 'json' ? json() : []),
          varsSlot.of(variableHighlighter(known)),
          readonlySlot.of([EditorState.readOnly.of(readonly), EditorView.editable.of(!readonly)]),
          EditorView.updateListener.of((u) => {
            if (u.docChanged) value = u.state.doc.toString();
          }),
        ],
      }),
    });
    return () => view?.destroy();
  });

  // Valor trocado por fora (ex.: recarregar do disco, nova resposta).
  $effect(() => {
    const next = value;
    if (view && next !== view.state.doc.toString()) {
      view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: next } });
    }
  });

  $effect(() => {
    view?.dispatch({ effects: langSlot.reconfigure(language === 'json' ? json() : []) });
  });

  $effect(() => {
    view?.dispatch({ effects: varsSlot.reconfigure(variableHighlighter(known)) });
  });
</script>

<div class="code-editor" bind:this={host}></div>

<style>
  .code-editor {
    height: 100%;
    min-height: 0;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .code-editor :global(.cm-editor) {
    height: 100%;
  }
  .code-editor :global(.cm-var-ok) {
    color: var(--m-put);
    font-weight: 600;
  }
  .code-editor :global(.cm-var-missing) {
    color: var(--danger);
    font-weight: 600;
    text-decoration: wavy underline;
  }
</style>
