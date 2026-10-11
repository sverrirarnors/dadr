<script lang="ts">
  import { EditorState } from '@codemirror/state';
  import { EditorView } from '@codemirror/view';
  import { untrack } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  import { markdownEditorExtensions } from '$lib/editor/markdown-extensions';
  import { cn } from '$lib/utils';

  /** A host element that also exposes the view, for browser tests. */
  type EditorHost = HTMLDivElement & { cmView?: EditorView };

  type Props = Omit<HTMLAttributes<HTMLDivElement>, 'children'> & {
    value: string;
    /** Accessible name of the editable area; `getByLabel` finds it. */
    ariaLabel: string;
    placeholder?: string;
    /** Ctrl/Cmd+S with the caret in the editor. */
    onsave?: () => void;
    autofocus?: boolean;
    readOnly?: boolean;
    class?: string;
  };

  let {
    value = $bindable(''),
    ariaLabel,
    placeholder = '',
    onsave,
    autofocus = false,
    readOnly = false,
    class: className,
    ...rest
  }: Props = $props();

  let host = $state<EditorHost | null>(null);
  let view: EditorView | null = null;
  // True while an outside value is being pushed in, so the listener does not echo it back.
  let applying = false;

  $effect(() => {
    const element = host;
    if (!element) return;
    const initial = untrack(() => value);
    const created = new EditorView({
      state: EditorState.create({
        doc: initial,
        extensions: [
          ...markdownEditorExtensions({
            ariaLabel,
            placeholder,
            readOnly,
            onsave: () => onsave?.(),
          }),
          EditorView.updateListener.of((update) => {
            if (update.docChanged && !applying) {
              value = update.state.doc.toString();
            }
          }),
        ],
      }),
      parent: element,
    });
    view = created;
    element.cmView = created;
    if (autofocus) created.focus();
    return () => {
      created.destroy();
      if (view === created) view = null;
      delete element.cmView;
    };
  });

  // Values that change outside the editor (another record loads, Cancel resets) flow in.
  $effect(() => {
    const next = value;
    const current = view;
    if (!current) return;
    const doc = current.state.doc.toString();
    if (doc === next) return;
    applying = true;
    try {
      current.dispatch({ changes: { from: 0, to: doc.length, insert: next } });
    } finally {
      applying = false;
    }
  });
</script>

<div
  bind:this={host}
  data-markdown-editor
  class={cn('markdown-editor overflow-hidden rounded-md border border-border bg-surface focus-within:ring-2 focus-within:ring-accent', className)}
  {...rest}
></div>
