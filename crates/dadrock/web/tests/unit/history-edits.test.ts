import { describe, expect, it } from 'vitest';
import type { RecordEdit } from '../../src/lib/api';
import { editCountLabel, editDay, editScope, versionEdits } from '../../src/lib/history/edits';

const committed: RecordEdit = {
  commit: 'e3b1c9a4f27d8e6b5a0c1d2e3f4a5b6c7d8e9f01',
  short_commit: 'e3b1c9a',
  author: 'Alice Martin',
  date: '2026-10-01T23:22:05-04:00',
  subject: 'Fix a typo',
  title_changed: false,
  body_changed: true,
};

const working: RecordEdit = {
  commit: null,
  short_commit: null,
  author: null,
  date: null,
  subject: null,
  title_changed: true,
  body_changed: true,
};

describe('versionEdits', () => {
  it('treats no Git and a missing version like a version without edits', () => {
    expect(versionEdits({ edits: null })).toEqual([]);
    expect(versionEdits(undefined)).toEqual([]);
    expect(versionEdits({ edits: [working, committed] })).toEqual([working, committed]);
  });
});

describe('editCountLabel', () => {
  it('counts the edits since the version was accepted or rejected', () => {
    expect(editCountLabel(1, 'accepted')).toBe('Edited once since accepted');
    expect(editCountLabel(2, 'accepted')).toBe('Edited 2 times since accepted');
    expect(editCountLabel(3, 'rejected')).toBe('Edited 3 times since rejected');
  });
});

describe('editDay', () => {
  it('keeps the day Git recorded, in the author’s time zone', () => {
    expect(editDay(committed)).toBe('2026-10-01');
    expect(editDay(working)).toBeNull();
  });
});

describe('editScope', () => {
  it('says whether the title, the body, or both changed', () => {
    expect(editScope(committed)).toBe('body');
    expect(editScope(working)).toBe('title and body');
    expect(editScope({ title_changed: true, body_changed: false })).toBe('title');
  });
});
