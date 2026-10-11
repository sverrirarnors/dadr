import { describe, expect, it } from 'vitest';
import {
  recordLifecycleSchema,
  recordSummarySchema,
  snapshotSchema,
} from '../../src/lib/api/schemas';

const emptyPanel = { counts: {}, diagnostics: [] };

function minimalSnapshot(): {
  [key: string]: unknown;
  records: unknown[];
  counts: unknown;
} {
  return {
    project: {
      cwd: '/tmp/proj',
      decisions_dir: '/tmp/proj/dadr/ledger',
      initialized: true,
      discovery_source: 'directory',
      status: 'Project discovered.',
    },
    records: [],
    counts: { total: 0, candidates: 0, accepted: 0, rejected: 0, superseded: 0 },
    tags: [],
    project_validation: emptyPanel,
    record_validation: emptyPanel,
    link_health: emptyPanel,
    settings: {
      config: [],
      templates: [],
      agent: [],
      tags: [],
      tag_counts: {},
      diagnostics: [],
    },
    command_catalog: {},
  };
}

describe('snapshotSchema', () => {
  it('parses a minimal valid snapshot', () => {
    const parsed = snapshotSchema.parse(minimalSnapshot());
    expect(parsed.records).toEqual([]);
  });

  it('parses a snapshot with records', () => {
    const input = minimalSnapshot();
    input.records = [
      {
        id: '0193fb1f-a3a0-7000-8000-000000000000',
        id_prefix: '0193fb1fa3a0',
        simple_record_id: '0193fb1fa3a070008000000000000000',
        title: 'Use Polars over pandas',
        status: 'accepted',
        superseded: false,
        path: 'dadr/ledger/use-polars.md',
        preview: 'preview',
        tags: ['#dadr/data/processing'],
        timestamp_ms: 1730000000000,
      },
    ];
    input.counts = { total: 1, candidates: 0, accepted: 1, rejected: 0, superseded: 0 };
    const parsed = snapshotSchema.parse(input);
    expect(parsed.records[0]?.title).toBe('Use Polars over pandas');
  });

  it('rejects an invalid status', () => {
    const input = minimalSnapshot();
    input.records = [
      {
        id: '0193fb1f-a3a0-7000-8000-000000000000',
        id_prefix: '0193fb1fa3a0',
        simple_record_id: '0193fb1fa3a070008000000000000000',
        title: 'x',
        status: 'invalid',
        superseded: false,
        path: 'x.md',
        preview: 'preview',
        tags: [],
        timestamp_ms: 1730000000000,
      },
    ];
    expect(() => snapshotSchema.parse(input)).toThrow();
  });
});

describe('recordSummarySchema', () => {
  it('requires UUIDv7 format for id', () => {
    expect(() =>
      recordSummarySchema.parse({
        id: 'not-a-uuid',
        id_prefix: 'not-a-uuid',
        simple_record_id: 'notauuid',
        title: 't',
        status: 'accepted',
        superseded: false,
        path: 'p.md',
        preview: 'preview',
        tags: [],
        timestamp_ms: 1730000000000,
      }),
    ).toThrow();
  });
});

describe('recordLifecycleSchema', () => {
  it('parses a record with supersession neighbors and diagnostics', () => {
    const record = {
      id: '0193fb1f-a3a0-7000-8000-000000000001',
      id_prefix: '0193fb1fa3a0',
      simple_record_id: '0193fb1fa3a070008000000000000001',
      title: 'Use Polars over pandas',
      status: 'accepted',
      superseded: true,
      path: 'dadr/archive/use-polars.md',
      preview: 'preview',
      tags: ['#dadr/data/processing'],
      timestamp_ms: 1730000000000,
    };

    const parsed = recordLifecycleSchema.parse({
      record,
      predecessors: [],
      successors: [
        {
          ...record,
          id: '0193fb1f-a3a0-7000-8000-000000000002',
          simple_record_id: '0193fb1fa3a070008000000000000002',
          title: 'Use Polars lazy frames',
          superseded: false,
        },
      ],
      diagnostics: [
        {
          severity: 'warning',
          code: 'missing_section',
          message: 'Missing Consequences section.',
          path: 'dadr/ledger/use-polars.md',
          record_id: '0193fb1f-a3a0-7000-8000-000000000001',
          field: 'Consequences',
          line: null,
          related_paths: [],
        },
      ],
    });

    expect(parsed.successors[0]?.title).toBe('Use Polars lazy frames');
    expect(parsed.diagnostics[0]?.code).toBe('missing_section');
  });
});
