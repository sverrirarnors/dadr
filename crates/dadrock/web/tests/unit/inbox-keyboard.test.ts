import { describe, expect, it } from 'vitest';
import { actionSelector, inboxKeyAction, isTypingTarget, neighbourItemId } from '../../src/lib/inbox/keyboard';

function key(key: string, modifiers: Partial<Pick<KeyboardEvent, 'ctrlKey' | 'metaKey' | 'altKey'>> = {}) {
  return { key, ctrlKey: false, metaKey: false, altKey: false, ...modifiers };
}

describe('inboxKeyAction', () => {
  it('maps vim-style and arrow keys to inbox actions', () => {
    expect(inboxKeyAction(key('j'))).toBe('next');
    expect(inboxKeyAction(key('ArrowDown'))).toBe('next');
    expect(inboxKeyAction(key('k'))).toBe('previous');
    expect(inboxKeyAction(key('ArrowUp'))).toBe('previous');
    expect(inboxKeyAction(key('a'))).toBe('primary');
    expect(inboxKeyAction(key('r'))).toBe('secondary');
    expect(inboxKeyAction(key('e'))).toBe('edit');
    expect(inboxKeyAction(key('o'))).toBe('open');
  });

  it('ignores other keys and chords', () => {
    expect(inboxKeyAction(key('x'))).toBeNull();
    expect(inboxKeyAction(key('J'))).toBeNull();
    expect(inboxKeyAction(key('j', { ctrlKey: true }))).toBeNull();
    expect(inboxKeyAction(key('a', { metaKey: true }))).toBeNull();
    expect(inboxKeyAction(key('r', { altKey: true }))).toBeNull();
  });
});

describe('isTypingTarget', () => {
  it('recognises form controls and editable regions', () => {
    expect(isTypingTarget(null)).toBe(false);
    expect(isTypingTarget(document.createElement('input'))).toBe(true);
    expect(isTypingTarget(document.createElement('textarea'))).toBe(true);
    expect(isTypingTarget(document.createElement('select'))).toBe(true);
    expect(isTypingTarget(document.createElement('button'))).toBe(false);
    expect(isTypingTarget(document.body)).toBe(false);

    const editable = document.createElement('div');
    editable.setAttribute('contenteditable', 'true');
    const child = document.createElement('span');
    editable.append(child);
    expect(isTypingTarget(editable)).toBe(true);
    expect(isTypingTarget(child)).toBe(true);

    const notEditable = document.createElement('div');
    notEditable.setAttribute('contenteditable', 'false');
    expect(isTypingTarget(notEditable)).toBe(false);
  });
});

describe('neighbourItemId', () => {
  const ids = ['a', 'b', 'c'];

  it('steps through the list and clamps at both ends', () => {
    expect(neighbourItemId(ids, 'a', 1)).toBe('b');
    expect(neighbourItemId(ids, 'b', -1)).toBe('a');
    expect(neighbourItemId(ids, 'c', 1)).toBe('c');
    expect(neighbourItemId(ids, 'a', -1)).toBe('a');
  });

  it('falls back to the first item when nothing matches', () => {
    expect(neighbourItemId(ids, null, 1)).toBe('a');
    expect(neighbourItemId(ids, 'gone', -1)).toBe('a');
    expect(neighbourItemId([], 'a', 1)).toBeNull();
  });
});

describe('actionSelector', () => {
  it('targets the record panel buttons for candidates', () => {
    expect(actionSelector('candidate', 'primary')).toBe('[data-record-action="accept"]');
    expect(actionSelector('candidate', 'secondary')).toBe('[data-record-action="reject"]');
    expect(actionSelector('candidate', 'edit')).toBe('[data-record-action="edit"]');
  });

  it('targets inbox detail buttons for everything else', () => {
    expect(actionSelector('proposal', 'primary')).toBe('[data-inbox-action="primary"]');
    expect(actionSelector('issue', 'secondary')).toBe('[data-inbox-action="secondary"]');
    expect(actionSelector('revision', 'edit')).toBe('[data-inbox-action="edit"]');
  });
});
