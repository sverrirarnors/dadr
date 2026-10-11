import { describe, expect, it } from 'vitest';
import type { RecordSummary } from '../../src/lib/api';
import type { InboxItem, IssueItem } from '../../src/lib/inbox/items';
import { isCurrentDecision, kindBadge, recordLedgerHref, severityVariant } from '../../src/lib/inbox/presentation';

function record(status: RecordSummary['status'], superseded = false): RecordSummary {
  return {
    id: '0193fb1f-a3a0-7000-8000-000000000001',
    id_prefix: '0193fb1fa3a0',
    simple_record_id: '0193fb1fa3a070008000000000000001',
    title: 'Record',
    status,
    superseded,
    path: 'dadr/accepted/0001.md',
    preview: '',
    tags: [],
    timestamp_ms: 0,
  };
}

function issue(severity: IssueItem['diagnostic']['severity']): IssueItem {
  return {
    kind: 'issue',
    id: `issue:${severity}`,
    title: 'Issue',
    subtitle: '',
    category: 'Links',
    diagnostic: { severity, code: 'x', message: 'Issue', path: null, record_id: null, field: null, line: null, related_paths: [] },
  };
}

describe('kindBadge', () => {
  it('labels every inbox item kind', () => {
    const items: Array<[InboxItem['kind'], string]> = [
      ['candidate', 'CAN'],
      ['proposal', 'AGENT'],
      ['revision', 'REV'],
      ['conflict', 'CONF'],
      ['report', 'RPT'],
      ['running', 'RUN'],
    ];
    for (const [kind, label] of items) {
      expect(kindBadge({ kind } as InboxItem).label).toBe(label);
    }
    expect(kindBadge(issue('error'))).toEqual({ label: 'ISS', variant: 'danger' });
    expect(kindBadge(issue('warning'))).toEqual({ label: 'ISS', variant: 'warning' });
    expect(kindBadge(issue('info'))).toEqual({ label: 'ISS', variant: 'neutral' });
  });

  it('maps diagnostic severities to badge variants', () => {
    expect(severityVariant('error')).toBe('danger');
    expect(severityVariant('warning')).toBe('warning');
    expect(severityVariant('info')).toBe('neutral');
  });
});

describe('record links', () => {
  it('sends current decisions to the ledger and everything else to its history', () => {
    expect(recordLedgerHref(record('accepted'))).toBe('/ledger?id=0193fb1f-a3a0-7000-8000-000000000001');
    expect(recordLedgerHref(record('accepted', true))).toBe('/ledger?history=1&id=0193fb1f-a3a0-7000-8000-000000000001');
    expect(recordLedgerHref(record('candidate'))).toBe('/ledger?history=1&id=0193fb1f-a3a0-7000-8000-000000000001');
  });

  it('knows which records are current decisions', () => {
    expect(isCurrentDecision(record('accepted'))).toBe(true);
    expect(isCurrentDecision(record('accepted', true))).toBe(false);
    expect(isCurrentDecision(record('candidate'))).toBe(false);
  });
});
