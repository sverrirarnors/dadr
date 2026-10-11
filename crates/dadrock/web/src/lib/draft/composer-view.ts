import type { RecordSummary } from '$lib/api';
import { ledgerUrl, type LedgerHref } from '$lib/navigation/routes';
import type { RecordStatusFilter } from '$lib/records/filters';
import type { DraftMode } from '$lib/stores/composer.svelte';

/** Which half of the composer is showing: the agent prompt or the manual form. */
export type ComposerView = 'agent' | 'manual';

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

export function composerTitle(mode: DraftMode, record: RecordSummary | undefined): string {
  switch (mode) {
    case 'revise':
      return record ? `Revise “${record.title}”` : 'Revise a record';
    case 'supersede':
      return record ? `Replace “${record.title}”` : 'Replace a decision';
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

export interface ManualPrefill {
  title: string;
  tags: string;
}

/** Title and tags the manual form starts with. */
export function manualPrefill(mode: DraftMode, record: RecordSummary | undefined): ManualPrefill {
  if (mode === 'supersede' && record) {
    return { title: `Update: ${record.title}`, tags: record.tags.join(' ') };
  }
  return { title: '', tags: '' };
}

/** Where "Edit it in the Ledger" goes for a record. */
export function ledgerEditHref(record: RecordSummary): LedgerHref {
  return ledgerUrl({
    id: record.id,
    tab: 'edit',
    history: record.status !== 'accepted' || record.superseded,
  });
}

/** A readable message for an unknown thrown value. */
export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return 'Unknown error';
}
