import { describe, expect, it } from 'vitest';
import type { RecordSummary } from '../../src/lib/api';
import {
  isCurrentDecision,
  isArchivedDecision,
  ledgerCounts,
  ledgerRecords,
  ledgerSubtitle,
  normalizeLedgerTag,
  readLedgerFilters,
  recordHasTag,
} from '../../src/lib/ledger/filters';

function record(overrides: Partial<RecordSummary> & Pick<RecordSummary, 'id' | 'title'>): RecordSummary {
  const suffix = overrides.id.slice(-4);
  const archived = overrides.superseded || (overrides.status && overrides.status !== 'accepted');
  return {
    id_prefix: '0193fb1fa3a0',
    simple_record_id: overrides.id.replaceAll('-', ''),
    status: 'accepted',
    superseded: false,
    path: `dadr/${archived ? 'archive' : 'ledger'}/${suffix}-${overrides.title.toLowerCase().replaceAll(' ', '-')}.md`,
    preview: '',
    tags: [],
    timestamp_ms: 0,
    ...overrides,
  };
}

const current = record({
  id: '0193fb1f-a3a0-7000-8000-000000000003',
  title: 'Use Polars lazy frames',
  tags: ['#dadr/data/processing'],
  timestamp_ms: 1730000200000,
});
const newest = record({
  id: '0193fb1f-a3a0-7000-8000-000000000004',
  title: 'Use Arrow IPC snapshots',
  tags: ['#dadr/data/processing', '#kind/sample'],
  timestamp_ms: 1730000300000,
});
const superseded = record({
  id: '0193fb1f-a3a0-7000-8000-000000000001',
  title: 'Use Polars over pandas',
  superseded: true,
  tags: ['#dadr/data/processing'],
  timestamp_ms: 1730000000000,
});
const rejected = record({
  id: '0193fb1f-a3a0-7000-8000-000000000005',
  title: 'Keep manual spreadsheet merge',
  status: 'rejected',
  path: 'dadr/archive/spreadsheet.md',
  tags: ['#dadr/data'],
  timestamp_ms: 1730000400000,
});
const candidate = record({
  id: '0193fb1f-a3a0-7000-8000-000000000002',
  title: 'Adopt pre-registration for studies',
  status: 'candidate',
  path: 'dadr/inbox/preregister.md',
  tags: ['#dadr/methodology'],
  timestamp_ms: 1730000100000,
});
const explicitlySuperseded = record({
  id: '0193fb1f-a3a0-7000-8000-000000000006',
  title: 'Old status enum value',
  status: 'superseded',
  timestamp_ms: 1730000500000,
});

const records = [superseded, candidate, current, newest, rejected, explicitlySuperseded];

function titles(result: RecordSummary[]): string[] {
  return result.map((item) => item.title);
}

describe('ledgerRecords', () => {
  it('shows only current accepted decisions by default, newest first', () => {
    expect(titles(ledgerRecords(records, { archived: false, q: '', tag: null }))).toEqual([
      'Use Arrow IPC snapshots',
      'Use Polars lazy frames',
    ]);
  });

  it('adds archived decisions (superseded and rejected), but never candidates', () => {
    const shown = titles(ledgerRecords(records, { archived: true, q: '', tag: null }));
    expect(shown).toEqual([
      'Old status enum value',
      'Keep manual spreadsheet merge',
      'Use Arrow IPC snapshots',
      'Use Polars lazy frames',
      'Use Polars over pandas',
    ]);
    expect(shown).not.toContain('Adopt pre-registration for studies');
  });

  it('matches the query against title, path, tag, and id', () => {
    const all = { archived: true, tag: null };
    expect(titles(ledgerRecords(records, { ...all, q: 'arrow' }))).toEqual(['Use Arrow IPC snapshots']);
    expect(titles(ledgerRecords(records, { ...all, q: 'archive/spreadsheet' }))).toEqual([
      'Keep manual spreadsheet merge',
    ]);
    expect(titles(ledgerRecords(records, { ...all, q: '#kind/sample' }))).toEqual(['Use Arrow IPC snapshots']);
    expect(titles(ledgerRecords(records, { ...all, q: '000000000001' }))).toEqual(['Use Polars over pandas']);
    expect(ledgerRecords(records, { ...all, q: '  ARROW  ' })).toHaveLength(1);
  });

  it('keeps the query from surfacing candidates', () => {
    expect(ledgerRecords(records, { archived: true, q: 'pre-registration', tag: null })).toHaveLength(0);
  });

  it('filters by exact tag or tag prefix, case-insensitively, with or without a hash', () => {
    const base = { archived: true, q: '' };
    expect(titles(ledgerRecords(records, { ...base, tag: '#dadr/data/processing' }))).toEqual([
      'Use Arrow IPC snapshots',
      'Use Polars lazy frames',
      'Use Polars over pandas',
    ]);
    expect(titles(ledgerRecords(records, { ...base, tag: 'dadr/data' }))).toEqual([
      'Keep manual spreadsheet merge',
      'Use Arrow IPC snapshots',
      'Use Polars lazy frames',
      'Use Polars over pandas',
    ]);
    expect(titles(ledgerRecords(records, { ...base, tag: '#DADR/Data' }))).toHaveLength(4);
    expect(ledgerRecords(records, { ...base, tag: '#dadr/dat' })).toHaveLength(0);
    expect(ledgerRecords(records, { ...base, tag: '#kind' })).toHaveLength(1);
  });

  it('combines archived, query, and tag', () => {
    expect(titles(ledgerRecords(records, { archived: false, q: 'polars', tag: '#dadr/data' }))).toEqual([
      'Use Polars lazy frames',
    ]);
    expect(titles(ledgerRecords(records, { archived: true, q: 'polars', tag: '#dadr/data' }))).toEqual([
      'Use Polars lazy frames',
      'Use Polars over pandas',
    ]);
  });

  it('does not mutate the input order', () => {
    const copy = [...records];
    ledgerRecords(records, { archived: true, q: '', tag: null });
    expect(records).toEqual(copy);
  });
});

describe('recordHasTag', () => {
  it('treats an empty tag as a match', () => {
    expect(recordHasTag(current, '')).toBe(true);
    expect(recordHasTag(current, '#')).toBe(true);
  });

  it('matches nested tags under a namespace', () => {
    expect(recordHasTag(current, '#dadr')).toBe(true);
    expect(recordHasTag(current, 'DADR/DATA/')).toBe(false);
    expect(recordHasTag(current, '#dadr/methodology')).toBe(false);
  });
});

describe('normalizeLedgerTag', () => {
  it('strips hashes and whitespace and lower-cases', () => {
    expect(normalizeLedgerTag('  ##Dadr/Data ')).toBe('dadr/data');
  });
});

describe('decision status helpers', () => {
  it('separates current decisions from archived ones', () => {
    expect(isCurrentDecision(current)).toBe(true);
    expect(isCurrentDecision(superseded)).toBe(false);
    expect(isArchivedDecision(superseded)).toBe(true);
    expect(isArchivedDecision(rejected)).toBe(true);
    expect(isArchivedDecision(explicitlySuperseded)).toBe(true);
    expect(isArchivedDecision(candidate)).toBe(false);
    expect(isArchivedDecision(current)).toBe(false);
  });

  it('counts current and archived decisions separately', () => {
    expect(ledgerCounts(records)).toEqual({ current: 2, archived: 3 });
    expect(ledgerCounts([])).toEqual({ current: 0, archived: 0 });
  });

  it('formats the pane subtitle', () => {
    expect(ledgerSubtitle({ current: 2, archived: 3 }, false)).toBe('2 decisions');
    expect(ledgerSubtitle({ current: 2, archived: 3 }, true)).toBe('2 decisions, 3 archived');
    expect(ledgerSubtitle({ current: 1, archived: 0 }, true)).toBe('1 decision, 0 archived');
  });
});

describe('readLedgerFilters', () => {
  it('reads the url parameters with canonical defaults', () => {
    expect(readLedgerFilters(new URLSearchParams(''))).toEqual({ archived: false, q: '', tag: null });
    expect(readLedgerFilters(new URLSearchParams('archived=1&q=polars&tag=%23dadr%2Fdata'))).toEqual({
      archived: true,
      q: 'polars',
      tag: '#dadr/data',
    });
    expect(readLedgerFilters(new URLSearchParams('archived=0&tag=')).archived).toBe(false);
    expect(readLedgerFilters(new URLSearchParams('tag=')).tag).toBeNull();
  });
});
