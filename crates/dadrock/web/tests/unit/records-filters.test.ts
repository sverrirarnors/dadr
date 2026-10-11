import { describe, expect, it } from 'vitest';
import type { RecordSummary } from '../../src/lib/api';
import {
  displayRecordStatus,
  filterRecords,
  recordSearchText,
  recordStatusVariant,
  searchScopeOptions,
  statusOptions,
} from '../../src/lib/records/filters';

const records: RecordSummary[] = [
  {
    id: '0193fb1f-a3a0-7000-8000-000000000001',
    id_prefix: '0193fb1fa3a0',
    simple_record_id: '0193fb1fa3a070008000000000000001',
    title: 'Use Polars over pandas',
    status: 'accepted',
    superseded: false,
    path: 'dadr/ledger/use-polars.md',
    preview: 'Use Polars for tabular transforms.',
    tags: ['#dadr/data/processing'],
    timestamp_ms: 1730000000000,
  },
  {
    id: '0193fb1f-a3a0-7000-8000-000000000002',
    id_prefix: '0193fb1fa3a0',
    simple_record_id: '0193fb1fa3a070008000000000000002',
    title: 'Draft reproducibility checklist',
    status: 'candidate',
    superseded: false,
    path: 'dadr/inbox/repro-checklist.md',
    preview: 'Candidate checklist.',
    tags: ['#dadr/methodology'],
    timestamp_ms: 1730000100000,
  },
  {
    id: '0193fb1f-a3a0-7000-8000-000000000003',
    id_prefix: '0193fb1fa3a0',
    simple_record_id: '0193fb1fa3a070008000000000000003',
    title: 'Reject manual spreadsheet merge',
    status: 'rejected',
    superseded: false,
    path: 'dadr/archive/spreadsheet-merge.md',
    preview: 'Rejected merge workflow.',
    tags: ['#dadr/data/processing'],
    timestamp_ms: 1730000200000,
  },
  {
    id: '0193fb1f-a3a0-7000-8000-000000000004',
    id_prefix: '0193fb1fa3a0',
    simple_record_id: '0193fb1fa3a070008000000000000004',
    title: 'Supersede broad transform rule',
    status: 'accepted',
    superseded: true,
    path: 'dadr/archive/superseded-transform.md',
    preview: 'Old transform rule.',
    tags: ['#dadr/data/processing'],
    timestamp_ms: 1730000300000,
  },
  {
    id: '0193fb1f-a3a0-7000-8000-000000000005',
    id_prefix: '0193fb1fa3a0',
    simple_record_id: '0193fb1fa3a070008000000000000005',
    title: 'Sample tagged record',
    status: 'rejected',
    superseded: false,
    path: 'dadr/archive/sample.md',
    preview: 'Sample tag workflow.',
    tags: ['kind/sample'],
    timestamp_ms: 1730000400000,
  },
];

describe('statusOptions', () => {
  it('contains all supported record list status filters', () => {
    expect(statusOptions.map((option) => option.value)).toEqual([
      'all',
      'candidate',
      'accepted',
      'rejected',
      'superseded',
    ]);
  });
});

describe('searchScopeOptions', () => {
  it('contains all supported record search scopes', () => {
    expect(searchScopeOptions.map((option) => option.value)).toEqual(['all', 'title', 'file', 'tag', 'id']);
  });
});

describe('recordSearchText', () => {
  it('includes title, path, tags, id, and id prefix', () => {
    const text = recordSearchText(records[0]!);
    expect(text).toContain('use polars over pandas');
    expect(text).toContain('dadr/ledger/use-polars.md');
    expect(text).toContain('#dadr/data/processing');
    expect(text).toContain('0193fb1f-a3a0-7000-8000-000000000001');
    expect(text).toContain('0193fb1fa3a0');
  });
});

describe('filterRecords', () => {
  it('centralizes displayed lifecycle status', () => {
    expect(displayRecordStatus(records[0]!)).toBe('accepted');
    expect(displayRecordStatus(records[3]!)).toBe('superseded');
    expect(recordStatusVariant(records[3]!)).toBe('superseded');
  });

  it('filters by status', () => {
    expect(filterRecords(records, { status: 'candidate', query: '' }).map((record) => record.title)).toEqual([
      'Draft reproducibility checklist',
    ]);
    expect(filterRecords(records, { status: 'accepted', query: '' }).map((record) => record.title)).toEqual([
      'Use Polars over pandas',
    ]);
    expect(filterRecords(records, { status: 'superseded', query: '' }).map((record) => record.title)).toEqual([
      'Supersede broad transform rule',
    ]);
  });

  it('matches query text against title, path, tags, and ids', () => {
    expect(filterRecords(records, { status: 'all', query: 'polars', scope: 'all' })).toHaveLength(1);
    expect(filterRecords(records, { status: 'all', query: 'repro-checklist', scope: 'all' })).toHaveLength(1);
    expect(filterRecords(records, { status: 'all', query: '#dadr/methodology', scope: 'all' })).toHaveLength(1);
    expect(filterRecords(records, { status: 'all', query: '000000000003', scope: 'all' })).toHaveLength(1);
  });

  it('limits query matching to the requested search scope', () => {
    expect(filterRecords(records, { status: 'all', query: 'polars', scope: 'title' })).toHaveLength(1);
    expect(filterRecords(records, { status: 'all', query: 'ledger/use-polars', scope: 'title' })).toHaveLength(0);
    expect(filterRecords(records, { status: 'all', query: 'ledger/use-polars', scope: 'file' })).toHaveLength(1);
    expect(filterRecords(records, { status: 'all', query: '#dadr/methodology', scope: 'tag' })).toHaveLength(1);
    expect(filterRecords(records, { status: 'all', query: '000000000003', scope: 'id' })).toHaveLength(1);
  });

  it('matches tag searches with or without a leading hash', () => {
    expect(filterRecords(records, { status: 'all', query: '#dadr/data/processing', scope: 'tag' })).toHaveLength(3);
    expect(filterRecords(records, { status: 'all', query: 'dadr/data/processing', scope: 'tag' })).toHaveLength(3);
    expect(filterRecords(records, { status: 'all', query: '#kind/sample', scope: 'tag' })).toHaveLength(1);
  });

  it('combines status and query filters', () => {
    expect(filterRecords(records, { status: 'accepted', query: 'processing', scope: 'all' })).toHaveLength(1);
    expect(filterRecords(records, { status: 'candidate', query: 'processing', scope: 'all' })).toHaveLength(0);
  });

  it('trims and lowercases query text', () => {
    expect(filterRecords(records, { status: 'all', query: '  POLARS  ', scope: 'all' })).toHaveLength(1);
  });
});
