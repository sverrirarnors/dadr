import type { Diagnostic, RecordSummary } from '$lib/api';
import { ledgerUrl, type LedgerHref } from '$lib/navigation/routes';
import type { InboxItem } from './items';

export type BadgeVariant =
  | 'neutral'
  | 'accent'
  | 'success'
  | 'warning'
  | 'danger'
  | 'candidate'
  | 'accepted'
  | 'rejected'
  | 'superseded';

export interface KindBadge {
  label: string;
  variant: BadgeVariant;
}

/** The short uppercase tag shown in front of every inbox row. */
export function kindBadge(item: InboxItem): KindBadge {
  switch (item.kind) {
    case 'candidate':
      return { label: 'CAN', variant: 'candidate' };
    case 'split':
      return { label: 'SPLIT', variant: 'candidate' };
    case 'proposal':
      return { label: 'AGENT', variant: 'accent' };
    case 'revision':
      return { label: 'REV', variant: 'accent' };
    case 'conflict':
      return { label: 'CONF', variant: 'warning' };
    case 'report':
      return { label: 'RPT', variant: 'neutral' };
    case 'issue':
      return { label: 'ISS', variant: severityVariant(item.diagnostic.severity) };
    case 'running':
      return { label: 'RUN', variant: 'accent' };
  }
}

export function severityVariant(severity: Diagnostic['severity']): BadgeVariant {
  if (severity === 'error') return 'danger';
  if (severity === 'warning') return 'warning';
  return 'neutral';
}

/** Where a record lives in the Ledger: current decisions up front, everything else under Show archived. */
export function recordLedgerHref(record: RecordSummary): LedgerHref {
  return ledgerUrl({ id: record.id, archived: record.status !== 'accepted' || record.superseded });
}

export function isCurrentDecision(record: RecordSummary): boolean {
  return record.status === 'accepted' && !record.superseded;
}

export function formatStartedAt(iso: string): string {
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return iso;
  return date.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' });
}
