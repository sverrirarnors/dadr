import type { RecordSummary } from '$lib/api';
import { parseListInput } from '$lib/candidates/form';
import { ledgerUrl, type LedgerHref } from '$lib/navigation/routes';
import type { RecordStatusFilter } from '$lib/records/filters';
import { readFrontMatter, splitSource } from '$lib/records/front-matter';
import { mergedSuccessorBody } from '$lib/records/sections';
import { bodyWithoutSupersedes, comparableBody } from '$lib/records/supersedes';
import type { ComposerView, DraftMode } from '$lib/stores/composer.svelte';

export type { ComposerView };

/** The record a revise or supersede composer is pinned to, when it is known. */
export function pinnedRecord(
  mode: DraftMode,
  recordIds: string[],
  records: RecordSummary[],
): RecordSummary | undefined {
  if (mode !== 'revise' && mode !== 'supersede') return undefined;
  const id = recordIds[0];
  if (!id) return undefined;
  return records.find((record) => record.id === id);
}

/** The attached records that exist, in the order they were attached. */
export function attachedRecords(recordIds: string[], records: RecordSummary[]): RecordSummary[] {
  return recordIds
    .map((id) => records.find((record) => record.id === id))
    .filter((record): record is RecordSummary => Boolean(record));
}

/**
 * The page title. Writing a successor of one decision by hand is editing it;
 * several attached decisions are superseded together, as a merge.
 */
export function composerTitle(mode: DraftMode, records: RecordSummary[], view: ComposerView = 'agent'): string {
  const record = records[0];
  switch (mode) {
    case 'revise':
      return record ? `Revise “${record.title}”` : 'Revise a record';
    case 'supersede':
      if (records.length > 1) return `Supersede ${records.length} decisions`;
      if (!record) return 'Supersede a decision';
      return view === 'manual' ? `Edit “${record.title}”` : `Supersede “${record.title}”`;
    case 'merge':
      return 'Merge candidates';
    case 'new':
      return 'Draft a decision';
  }
}

/** Label (and accessible name) of the main textarea. */
export function promptLabel(mode: DraftMode): string {
  switch (mode) {
    case 'revise':
      return 'What should change?';
    case 'supersede':
      return 'What changed, and why?';
    case 'merge':
      return 'Anything the merged decision should emphasize? (optional)';
    case 'new':
      return 'What is the decision? Or say what to look for in the attached files.';
  }
}

/** Default status filter of the "Attach records" picker. */
export function attachRecordsStatus(mode: DraftMode): RecordStatusFilter {
  if (mode === 'merge') return 'candidate';
  if (mode === 'supersede') return 'accepted';
  return 'all';
}

/** The manual form only makes sense when the outcome is a new candidate. */
export function manualFormAvailable(mode: DraftMode): boolean {
  return mode === 'new' || mode === 'supersede';
}

/** The view shown when the composer opens, before the person switches. */
export function initialComposerView(mode: DraftMode, agentAvailable: boolean): ComposerView {
  return !agentAvailable && manualFormAvailable(mode) ? 'manual' : 'agent';
}

export interface SuccessorSource {
  record: RecordSummary;
  /** The record's file content. */
  source: string;
}

export interface SuccessorPrefill {
  title: string;
  /** Space-separated, as the tags field shows them. */
  tags: string;
  body: string;
}

/** A record's body as a successor starts from it: without front matter and without its own `## Supersedes`. */
export function successorStartingBody(source: string): string {
  return bodyWithoutSupersedes(splitSource(source).body).replace(/^(?:[ \t]*\r?\n)+/, '');
}

/**
 * What a successor written by hand starts from. A successor of one decision
 * starts from its exact title, tags, and body, so the diff shows only what
 * the analyst changes. A merge starts untitled, with every predecessor's tags
 * and each section's text from every predecessor.
 */
export function successorPrefill(predecessors: SuccessorSource[]): SuccessorPrefill {
  const tagsOf = ({ record, source }: SuccessorSource) => {
    const tags = readFrontMatter(source).tags;
    return tags.length ? tags : record.tags;
  };
  const [only] = predecessors;
  if (predecessors.length === 1 && only) {
    return { title: only.record.title, tags: tagsOf(only).join(' '), body: successorStartingBody(only.source) };
  }
  const tags: string[] = [];
  for (const predecessor of predecessors) {
    for (const tag of tagsOf(predecessor)) if (!tags.includes(tag)) tags.push(tag);
  }
  return {
    title: '',
    tags: tags.join(' '),
    body: predecessors.length ? mergedSuccessorBody(predecessors.map(({ source }) => successorStartingBody(source))) : '',
  };
}

/** Whether the draft differs from what it started from: title, tags, or body. */
export function successorChanged(prefill: SuccessorPrefill, draft: SuccessorPrefill): boolean {
  if (prefill.title.trim() !== draft.title.trim()) return true;
  if (parseListInput(prefill.tags).join(' ') !== parseListInput(draft.tags).join(' ')) return true;
  return comparableBody(prefill.body) !== comparableBody(draft.body);
}

/** Where "Edit it in the Ledger" goes for a record. */
export function ledgerEditHref(record: RecordSummary): LedgerHref {
  return ledgerUrl({
    id: record.id,
    tab: 'edit',
    archived: record.status !== 'accepted' || record.superseded,
  });
}

/** A readable message for an unknown thrown value. */
export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return 'Unknown error';
}
