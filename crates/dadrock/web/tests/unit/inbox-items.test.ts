import { describe, expect, it } from 'vitest';
import type { RecordSummary, Snapshot, TaskRun } from '../../src/lib/api';
import { buildInboxGroups, countActionable, flattenInboxGroups, previewText } from '../../src/lib/inbox/items';

function record(id: string, status: RecordSummary['status'], title: string, timestamp_ms = 0): RecordSummary {
  return {
    id,
    id_prefix: id.slice(0, 12),
    simple_record_id: id,
    title,
    status,
    superseded: false,
    path: `dadr/${status}/${id}.md`,
    preview: `${title} preview`,
    tags: [],
    timestamp_ms,
  };
}

const emptyPanel = { counts: {}, diagnostics: [] };
const snapshot: Snapshot = {
  project: { cwd: '/p', decisions_dir: '/p/dadr/ledger', initialized: true, discovery_source: 'directory', status: '' },
  records: [
    record('acc-1', 'accepted', 'Use Polars'),
    record('cand-old', 'candidate', 'Older candidate', 10),
    record('cand-new', 'candidate', 'Newer candidate', 20),
  ],
  counts: { total: 3, candidates: 2, accepted: 1, rejected: 0, superseded: 0 },
  tags: [],
  project_validation: {
    counts: {},
    diagnostics: [
      { severity: 'warning', code: 'candidate_backlog', message: 'Two candidates wait.', path: null, record_id: null, field: null, line: null, related_paths: [] },
    ],
  },
  record_validation: emptyPanel,
  link_health: {
    counts: {},
    diagnostics: [
      { severity: 'error', code: 'broken_link', message: 'Broken link.', path: 'dadr/ledger/acc-1.md', record_id: 'acc-1', field: null, line: 3, related_paths: [] },
    ],
  },
  settings: {
    config: [], templates: [], agent: [], agent_available: true, tags: [], tag_counts: {},
    diagnostics: [
      { severity: 'info', code: 'tag_catalog_empty', message: 'No tag catalog.', path: null, record_id: null, field: null, line: null, related_paths: [] },
    ],
  },
  command_catalog: {},
};

function run(id: string, task_id: string, status: TaskRun['status'], created_at: string): TaskRun {
  return { id, task_id, task_source: { kind: 'builtin', id: `builtin:${task_id}` }, task_version: 1, status, created_at, updated_at: created_at, run_dir: `/runs/${id}` };
}

describe('buildInboxGroups', () => {
  it('lists candidates newest first and issues by severity, skipping the backlog notice', () => {
    const groups = buildInboxGroups({ snapshot, runs: [], results: {}, resolved: [] });
    expect(groups.map((group) => group.id)).toEqual(['candidates', 'issues']);
    expect(groups[0]?.items.map((item) => item.title)).toEqual(['Newer candidate', 'Older candidate']);
    expect(groups[1]?.items.map((item) => item.title)).toEqual(['Broken link.', 'No tag catalog.']);
    expect(groups[1]?.items[0]).toMatchObject({ kind: 'issue', category: 'Links', record: { id: 'acc-1' } });
    expect(countActionable(groups)).toBe(4);
  });

  it('turns agent results into proposals, revisions, conflicts, and reports', () => {
    const runs = [
      run('r-batch', 'code-to-record', 'succeeded', '2026-10-11T10:00:00Z'),
      run('r-rev', 'edit-record', 'succeeded', '2026-10-11T10:01:00Z'),
      run('r-sup', 'supersede-record', 'succeeded', '2026-10-11T10:02:00Z'),
      run('r-con', 'find-conflicts', 'succeeded', '2026-10-11T10:03:00Z'),
      run('r-rep', 'custom', 'succeeded', '2026-10-11T10:04:00Z'),
      run('r-live', 'context-to-record', 'running', '2026-10-11T10:05:00Z'),
      run('r-fail', 'context-to-record', 'failed', '2026-10-11T10:06:00Z'),
    ];
    const results = {
      'r-batch': {
        kind: 'candidate_batch.v1',
        summary: 'Scanned R/',
        candidates: [
          { id: 'a', title: 'Drop missing rows', tags: ['#kind/sample'], body_markdown: '## Context\nx', confidence: 'high', warnings: [] },
          { id: 'b', title: '', tags: [], body_markdown: 'ignored: no title', warnings: [] },
        ],
      },
      'r-rev': { kind: 'record_revision.v1', record_id: 'cand-old', body_markdown: '## Decision\nnew', change_explanation: 'Clearer', summary: 'rev' },
      'r-sup': { kind: 'supersession_proposal.v1', superseded_record_id: 'acc-1', superseding_title: 'Use DuckDB', superseding_body_markdown: '## Decision\nDuckDB', rationale: 'faster' },
      'r-con': { kind: 'conflict_set.v1', summary: '', findings: [{ id: 'f1', title: 'Overlap', left_record_id: 'acc-1', right_record_id: 'cand-old', relationship: 'overlap', conflict_explanation: 'both', suggested_actions: [] }] },
      'r-rep': { kind: 'markdown_report.v1', title: 'Audit', body_markdown: '## Findings\nnone' },
    };
    const groups = buildInboxGroups({ snapshot, runs, results, resolved: [] });
    expect(groups.map((group) => group.id)).toEqual(['running', 'candidates', 'proposals', 'revisions', 'conflicts', 'reports', 'issues']);
    const items = flattenInboxGroups(groups);
    expect(items.find((item) => item.kind === 'running')).toMatchObject({ id: 'running:r-live', title: 'Drafting a decision' });
    const proposals = items.filter((item) => item.kind === 'proposal');
    expect(proposals.map((item) => item.id)).toEqual(['proposal:r-sup:supersession', 'proposal:r-batch:a']);
    expect(proposals[0]).toMatchObject({ proposal: { supersedes: 'acc-1', title: 'Use DuckDB' }, supersededRecord: { id: 'acc-1' } });
    expect(items.find((item) => item.kind === 'revision')).toMatchObject({ id: 'revision:r-rev', recordId: 'cand-old', revisedTitle: 'Older candidate', body_markdown: '## Decision\nnew' });
    expect(items.find((item) => item.kind === 'conflict')).toMatchObject({ id: 'conflict:r-con:f1', left: { id: 'acc-1' }, right: { id: 'cand-old' } });
    expect(items.find((item) => item.kind === 'report')).toMatchObject({ id: 'report:r-rep', title: 'Audit' });
    expect(countActionable(groups)).toBe(2 + 2 + 1 + 1 + 1 + 2);
  });

  it('hides resolved items', () => {
    const runs = [run('r-batch', 'code-to-record', 'succeeded', '2026-10-11T10:00:00Z')];
    const results = {
      'r-batch': { kind: 'candidate_batch.v1', summary: '', candidates: [{ id: 'a', title: 'Keep', tags: [], body_markdown: 'x', warnings: [] }, { id: 'b', title: 'Gone', tags: [], body_markdown: 'y', warnings: [] }] },
    };
    const resolved = ['proposal:r-batch:b', 'issue:Links:broken_link:dadr/ledger/acc-1.md:acc-1::3:Broken link.'];
    const items = flattenInboxGroups(buildInboxGroups({ snapshot, runs, results, resolved }));
    expect(items.filter((item) => item.kind === 'proposal').map((item) => item.title)).toEqual(['Keep']);
    expect(items.filter((item) => item.kind === 'issue').map((item) => item.title)).toEqual(['No tag catalog.']);
  });

  it('returns no groups for an empty, healthy project', () => {
    const clean: Snapshot = { ...snapshot, records: [snapshot.records[0]!], project_validation: emptyPanel, link_health: emptyPanel, settings: { ...snapshot.settings, diagnostics: [] } };
    expect(buildInboxGroups({ snapshot: clean, runs: [], results: {}, resolved: [] })).toEqual([]);
  });
});

describe('previewText', () => {
  it('turns a record preview into one line without heading markers', () => {
    expect(previewText('## Context Island may confound species differences.')).toBe('Island may confound species differences.');
    expect(previewText('## Context\nA reviewer asked.\n\n## Decision\nAdd one outcome.')).toBe('A reviewer asked. Add one outcome.');
    expect(previewText('Plain preview.')).toBe('Plain preview.');
  });
});
