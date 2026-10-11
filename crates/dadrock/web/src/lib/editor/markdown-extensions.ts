import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { markdown } from '@codemirror/lang-markdown';
import { yamlFrontmatter } from '@codemirror/lang-yaml';
import { bracketMatching, HighlightStyle, indentOnInput, syntaxHighlighting } from '@codemirror/language';
import { highlightSelectionMatches, searchKeymap } from '@codemirror/search';
import { EditorState, type Extension } from '@codemirror/state';
import {
  drawSelection,
  dropCursor,
  EditorView,
  highlightActiveLine,
  highlightSpecialChars,
  keymap,
  placeholder as placeholderExtension,
} from '@codemirror/view';
import { tags } from '@lezer/highlight';

/** DADR records: YAML front matter followed by Markdown. */
export const recordLanguage = (): Extension => yamlFrontmatter({ content: markdown() });

/** Colours come from the app's design tokens so light, dark, and VS Code themes all apply. */
export const markdownHighlighting = syntaxHighlighting(
  HighlightStyle.define([
    { tag: [tags.heading, tags.heading1, tags.heading2, tags.heading3], color: 'var(--color-fg)', fontWeight: '650' },
    { tag: tags.strong, fontWeight: '650' },
    { tag: tags.emphasis, fontStyle: 'italic' },
    { tag: [tags.link, tags.url], color: 'var(--color-accent)', textDecoration: 'underline' },
    { tag: tags.monospace, color: 'var(--color-accent)' },
    { tag: [tags.processingInstruction, tags.punctuation, tags.separator, tags.meta], color: 'var(--color-fg-subtle)' },
    { tag: tags.quote, color: 'var(--color-fg-muted)', fontStyle: 'italic' },
    { tag: [tags.propertyName, tags.keyword, tags.atom], color: 'var(--color-fg-muted)' },
    { tag: [tags.string, tags.number, tags.bool], color: 'var(--color-fg)' },
    { tag: tags.comment, color: 'var(--color-fg-subtle)', fontStyle: 'italic' },
    { tag: tags.invalid, textDecoration: 'underline wavy' },
  ]),
);

export const editorTheme = EditorView.theme({
  '&': {
    height: '100%',
    backgroundColor: 'var(--color-surface)',
    color: 'var(--color-fg)',
    fontSize: '0.75rem',
    lineHeight: '1.5rem',
  },
  '&.cm-focused': { outline: 'none' },
  '.cm-scroller': {
    overflow: 'auto',
    fontFamily: 'var(--font-mono)',
  },
  '.cm-content': {
    padding: '0.75rem 1rem',
    caretColor: 'var(--color-fg)',
  },
  '.cm-line': { padding: '0' },
  '.cm-activeLine': { backgroundColor: 'color-mix(in oklab, var(--color-fg) 4%, transparent)' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--color-fg)' },
  '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, ::selection': {
    backgroundColor: 'var(--color-accent-subtle)',
  },
  '.cm-selectionMatch': { backgroundColor: 'color-mix(in oklab, var(--color-accent) 18%, transparent)' },
  '.cm-matchingBracket': { backgroundColor: 'color-mix(in oklab, var(--color-accent) 18%, transparent)' },
  '.cm-placeholder': { color: 'var(--color-fg-subtle)' },
  '.cm-panels': {
    backgroundColor: 'var(--color-bg)',
    color: 'var(--color-fg)',
    borderColor: 'var(--color-border)',
  },
  '.cm-panel input, .cm-panel button': {
    fontFamily: 'var(--font-sans)',
    fontSize: '0.75rem',
    borderRadius: 'var(--radius)',
    border: '1px solid var(--color-border)',
    backgroundColor: 'var(--color-surface)',
    color: 'var(--color-fg)',
  },
  '.cm-searchMatch': { backgroundColor: 'color-mix(in oklab, var(--color-warning) 30%, transparent)' },
  '.cm-searchMatch.cm-searchMatch-selected': {
    backgroundColor: 'color-mix(in oklab, var(--color-warning) 55%, transparent)',
  },
});

export interface MarkdownEditorOptions {
  ariaLabel: string;
  placeholder?: string | undefined;
  /** Ctrl/Cmd+S inside the editor. */
  onsave?: (() => void) | undefined;
  readOnly?: boolean | undefined;
}

/** Everything a record editor needs, in precedence order. */
export function markdownEditorExtensions(options: MarkdownEditorOptions): Extension[] {
  const attributes: Record<string, string> = { 'aria-label': options.ariaLabel, spellcheck: 'false' };
  return [
    EditorView.contentAttributes.of(attributes),
    EditorView.editable.of(!options.readOnly),
    EditorState.readOnly.of(Boolean(options.readOnly)),
    history(),
    drawSelection(),
    dropCursor(),
    highlightActiveLine(),
    highlightSpecialChars(),
    bracketMatching(),
    closeBrackets(),
    indentOnInput(),
    highlightSelectionMatches(),
    recordLanguage(),
    markdownHighlighting,
    editorTheme,
    EditorView.lineWrapping,
    ...(options.placeholder ? [placeholderExtension(options.placeholder)] : []),
    keymap.of([
      { key: 'Mod-s', preventDefault: true, run: () => (options.onsave?.(), true) },
      ...closeBracketsKeymap,
      indentWithTab,
      ...defaultKeymap,
      ...historyKeymap,
      ...searchKeymap,
    ]),
  ];
}
