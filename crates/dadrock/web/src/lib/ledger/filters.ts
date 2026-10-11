import type { RecordSummary } from '$lib/api';
import { filterRecords } from '$lib/records/filters';

/** URL-backed filters for the Ledger list. */
export interface LedgerFilters {
  /** Also show superseded and rejected decisions. Candidates never appear here. */
  history: boolean;
  q: string;
  tag: string | null;
}

export interface LedgerCounts {
  /** Accepted decisions that have not been superseded. */
  current: number;
  /** Superseded and rejected decisions. */
  history: number;
}

export function readLedgerFilters(params: URLSearchParams): LedgerFilters {
  return {
    history: params.get('history') === '1',
    q: params.get('q') ?? '',
    tag: params.get('tag') || null,
  };
}

/** Lower-case, no leading '#', no surrounding whitespace. */
export function normalizeLedgerTag(tag: string): string {
  return tag.trim().replace(/^#+/, '').toLowerCase();
}

export function isCurrentDecision(record: RecordSummary): boolean {
  return record.status === 'accepted' && !record.superseded;
}

export function isHistoricalDecision(record: RecordSummary): boolean {
  if (record.status === 'rejected' || record.status === 'superseded') return true;
  return record.status === 'accepted' && record.superseded;
}

/** True when the record carries the tag itself or a tag nested under it. */
export function recordHasTag(record: RecordSummary, tag: string): boolean {
  const selected = normalizeLedgerTag(tag);
  if (!selected) return true;
  return record.tags.some((candidate) => {
    const value = normalizeLedgerTag(candidate);
    return value === selected || value.startsWith(`${selected}/`);
  });
}

/** Records shown by the Ledger list, newest first. */
export function ledgerRecords(records: RecordSummary[], filters: LedgerFilters): RecordSummary[] {
  const tag = filters.tag;
  const visible = records.filter((record) =>
    filters.history ? isCurrentDecision(record) || isHistoricalDecision(record) : isCurrentDecision(record),
  );
  const searched = filterRecords(visible, { status: 'all', query: filters.q, scope: 'all' });
  const tagged = tag ? searched.filter((record) => recordHasTag(record, tag)) : searched;
  return [...tagged].sort((left, right) => right.timestamp_ms - left.timestamp_ms);
}

export function ledgerCounts(records: RecordSummary[]): LedgerCounts {
  return records.reduce<LedgerCounts>(
    (counts, record) => {
      if (isCurrentDecision(record)) counts.current += 1;
      else if (isHistoricalDecision(record)) counts.history += 1;
      return counts;
    },
    { current: 0, history: 0 },
  );
}

/** "N decisions", plus ", M in history" while history is shown. */
export function ledgerSubtitle(counts: LedgerCounts, history: boolean): string {
  const decisions = `${counts.current} ${counts.current === 1 ? 'decision' : 'decisions'}`;
  return history ? `${decisions}, ${counts.history} in history` : decisions;
}
