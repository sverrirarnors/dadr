import type { InboxItemKind } from './items';

export type InboxKeyAction = 'next' | 'previous' | 'primary' | 'secondary' | 'edit' | 'open';
export type InboxButtonAction = Exclude<InboxKeyAction, 'next' | 'previous' | 'open'>;

type KeyLike = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'altKey'>;

/**
 * Single-key shortcuts for the inbox: j/k move, a acts, r rejects or
 * dismisses, e edits, o opens the record in the Ledger.
 */
export function inboxKeyAction(event: KeyLike): InboxKeyAction | null {
  if (event.ctrlKey || event.metaKey || event.altKey) return null;
  switch (event.key) {
    case 'j':
    case 'ArrowDown':
      return 'next';
    case 'k':
    case 'ArrowUp':
      return 'previous';
    case 'a':
      return 'primary';
    case 'r':
      return 'secondary';
    case 'e':
      return 'edit';
    case 'o':
      return 'open';
    default:
      return null;
  }
}

/** True when keystrokes belong to the focused control rather than the inbox. */
export function isTypingTarget(element: Element | null): boolean {
  if (!element) return false;
  const tag = element.tagName;
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true;
  if (element instanceof HTMLElement && element.isContentEditable) return true;
  const editable = element.closest('[contenteditable]')?.getAttribute('contenteditable');
  return editable !== undefined && editable !== null && editable !== 'false';
}

/** The item one step away, clamped to the list; the first item when nothing is selected. */
export function neighbourItemId(ids: string[], currentId: string | null, step: 1 | -1): string | null {
  if (ids.length === 0) return null;
  const index = currentId ? ids.indexOf(currentId) : -1;
  if (index === -1) return ids[0] ?? null;
  const next = Math.min(ids.length - 1, Math.max(0, index + step));
  return ids[next] ?? null;
}

/**
 * The button a shortcut presses inside the detail pane. Candidate records are
 * rendered by the shared record panel, which marks its buttons with
 * `data-record-action`; every other detail marks its own with `data-inbox-action`.
 */
export function actionSelector(kind: InboxItemKind, action: InboxButtonAction): string {
  if (kind === 'candidate') {
    const recordAction = { primary: 'accept', secondary: 'reject', edit: 'edit' }[action];
    return `[data-record-action="${recordAction}"]`;
  }
  return `[data-inbox-action="${action}"]`;
}
