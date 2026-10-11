import type { Locator } from '@playwright/test';

type EditorHost = HTMLElement & {
  cmView?: {
    state: { doc: { toString(): string; length: number } };
    dispatch(spec: { changes: { from: number; to: number; insert: string } }): void;
    focus(): void;
  };
};

function hostOf(element: Element): EditorHost {
  const host = element.closest('[data-markdown-editor]') as EditorHost | null;
  if (!host?.cmView) throw new Error('no CodeMirror view behind this element');
  return host;
}

/** The text in a CodeMirror editor found by its accessible label. */
export function editorValue(editor: Locator): Promise<string> {
  return editor.evaluate((element) => {
    const host = element.closest('[data-markdown-editor]') as EditorHost | null;
    if (!host?.cmView) throw new Error('no CodeMirror view behind this element');
    return host.cmView.state.doc.toString();
  });
}

/** Replaces the whole document, the way a person pasting over a selection would. */
export async function fillEditor(editor: Locator, text: string): Promise<void> {
  await editor.evaluate((element, next) => {
    const host = element.closest('[data-markdown-editor]') as EditorHost | null;
    if (!host?.cmView) throw new Error('no CodeMirror view behind this element');
    const view = host.cmView;
    view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: next } });
    view.focus();
  }, text);
}

export { hostOf };
