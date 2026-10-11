import type { HistoryVersion, RecordEdit } from '$lib/api';

/**
 * A version's text edits as the History view lists them: Git's edits to its
 * title or body since it was accepted or rejected, newest first. Empty for
 * candidates, for unedited records, and outside a Git repository.
 */
export function versionEdits(version: Pick<HistoryVersion, 'edits'> | undefined): RecordEdit[] {
  return version?.edits ?? [];
}

/** "Edited once since accepted", "Edited 3 times since rejected". */
export function editCountLabel(count: number, status: HistoryVersion['status']): string {
  const since = status === 'rejected' ? 'rejected' : 'accepted';
  return count === 1 ? `Edited once since ${since}` : `Edited ${count} times since ${since}`;
}

/** The day of an edit as Git recorded it (the author's date, `YYYY-MM-DD`), or null for the working tree. */
export function editDay(edit: Pick<RecordEdit, 'date'>): string | null {
  return edit.date ? edit.date.slice(0, 10) : null;
}

/** What an edit changed: "title", "body", or "title and body". */
export function editScope(edit: Pick<RecordEdit, 'title_changed' | 'body_changed'>): string {
  if (edit.title_changed && edit.body_changed) return 'title and body';
  return edit.title_changed ? 'title' : 'body';
}
