import { describe, expect, it } from 'vitest';
import type { RecordSummary, Snapshot, TaskRun } from '../../src/lib/api';
import {
  buildInboxGroups,
  countActionable,
  flattenInboxGroups,
  groupSuccessorCandidates,
  itemForCandidate,
  previewText,
} from '../../src/lib/inbox/items';

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
      'r-sup': { kind: 'supersession_proposal.v1', superseded_record_ids: ['acc-1'], superseding_title: 'Use DuckDB', superseding_body_markdown: '## Decision\nDuckDB', rationale: 'faster' },
      'r-con': { kind: 'conflict_set.v1', summary: '', findings: [{ id: 'f1', title: 'Overlap', left_record_id: 'acc-1', right_record_id: 'cand-old', relationship: 'overlap', conflict_explanation: 'both', suggested_actions: [] }] },
      'r-rep': { kind: 'markdown_report.v1', title: 'Audit', body_markdown: '## Findings\nnone' },
    };
    const groups = buildInboxGroups({ snapshot, runs, results, resolved: [] });
    expect(groups.map((group) => group.id)).toEqual(['running', 'candidates', 'proposals', 'revisions', 'conflicts', 'reports', 'issues']);
    const items = flattenInboxGroups(groups);
    expect(items.find((item) => item.kind === 'running')).toMatchObject({ id: 'running:r-live', title: 'Drafting a decision' });
    const proposals = items.filter((item) => item.kind === 'proposal');
    expect(proposals.map((item) => item.id)).toEqual(['proposal:r-sup:supersession', 'proposal:r-batch:a']);
    expect(proposals[0]).toMatchObject({
      proposal: { supersedes: ['acc-1'], supersedes_reason: 'faster', title: 'Use DuckDB' },
      supersededRecords: [{ id: 'acc-1' }],
    });
    expect(items.find((item) => item.kind === 'revision')).toMatchObject({ id: 'revision:r-rev', recordId: 'cand-old', revisedTitle: 'Older candidate', body_markdown: '## Decision\nnew' });
    expect(items.find((item) => item.kind === 'conflict')).toMatchObject({ id: 'conflict:r-con:f1', left: { id: 'acc-1' }, right: { id: 'cand-old' } });
    expect(items.find((item) => item.kind === 'report')).toMatchObject({ id: 'report:r-rep', title: 'Audit' });
    expect(countActionable(groups)).toBe(2 + 2 + 1 + 1 + 1 + 2);
  });

  it('maps merge proposals and older single-predecessor results', () => {
    const merged = { ...snapshot, records: [...snapshot.records, record('acc-2', 'accepted', 'Use Arrow')] };
    const runs = [
      run('r-merge', 'supersede-record', 'succeeded', '2026-10-11T10:00:00Z'),
      run('r-old', 'supersede-record', 'succeeded', '2026-10-11T09:00:00Z'),
    ];
    const results = {
      'r-merge': { kind: 'supersession_proposal.v1', superseded_record_ids: ['acc-1', 'acc-2'], superseding_title: 'One engine', superseding_body_markdown: '## Decision\nDuckDB', rationale: 'One engine is simpler.' },
      'r-old': { kind: 'supersession_proposal.v1', superseded_record_id: 'acc-1', superseding_title: 'Use DuckDB', superseding_body_markdown: '## Supersedes\n\n[[acc-1|Use Polars]]\n\n## Decision\nDuckDB', rationale: 'faster' },
    };
    const proposals = flattenInboxGroups(buildInboxGroups({ snapshot: merged, runs, results, resolved: [] })).filter(
      (item) => item.kind === 'proposal',
    );
    expect(proposals[0]).toMatchObject({
      subtitle: 'Supersedes “Use Polars”, “Use Arrow”',
      proposal: { supersedes: ['acc-1', 'acc-2'], supersedes_reason: 'One engine is simpler.' },
      supersededRecords: [{ id: 'acc-1' }, { id: 'acc-2' }],
    });
    // The older body already carries its `## Supersedes` section, so no reason is sent.
    expect(proposals[1]).toMatchObject({ proposal: { supersedes: ['acc-1'] } });
    expect(proposals[1]?.kind === 'proposal' ? proposals[1].proposal.supersedes_reason : 'not a proposal').toBeUndefined();
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

describe('successor candidates', () => {
  const P1 = '0193fb1f-a3a0-7000-8000-000000002001';
  const P2 = '0193fb1f-a3a0-7000-8000-000000003001';
  const P3 = '0193fb1f-a3a0-7000-8000-000000003002';
  const OLD = '0193fb1f-a3a0-7000-8000-000000004001';
  const NOW = '0193fb1f-a3a0-7000-8000-000000004002';

  function successor(id: string, title: string, predecessors: string[], timestamp_ms: number): RecordSummary {
    return {
      ...record(id, 'candidate', title, timestamp_ms),
      preview: `## Supersedes\n\n${predecessors.map((p) => `[[${p}|x]]`).join('\n')}\n\nWhy.\n\n## Decision\n\n${title}.`,
    };
  }

  const project: Snapshot = {
    ...snapshot,
    records: [
      record(P1, 'accepted', 'Outlier rule'),
      record(P2, 'accepted', 'Trim income'),
      record(P3, 'accepted', 'Cap ages'),
      { ...record(OLD, 'accepted', 'Use pandas'), superseded: true },
      record(NOW, 'accepted', 'Use Polars'),
      successor('half-1', 'Winsorize income', [P1], 30),
      successor('half-2', 'Drop implausible ages', [P1], 20),
      successor('merge', 'One outlier rule', [P2, P3], 40),
      successor('stale-1', 'Use pandas 2', [OLD], 50),
      successor('stale-2', 'Use pandas with Arrow', [OLD], 10),
      record('plain', 'candidate', 'Plain candidate', 5),
    ],
    project_validation: emptyPanel,
    link_health: emptyPanel,
    settings: { ...snapshot.settings, diagnostics: [] },
  };

  it('shows candidates that supersede the same decision as one split item', () => {
    const items = flattenInboxGroups(buildInboxGroups({ snapshot: project, runs: [], results: {}, resolved: [] }));
    expect(items.map((item) => item.id)).toEqual([
      'candidate:stale-1',
      'candidate:merge',
      `split:${P1}`,
      'candidate:stale-2',
      'candidate:plain',
    ]);
    const split = items.find((item) => item.kind === 'split');
    expect(split).toMatchObject({
      title: 'Split “Outlier rule” into 2 decisions',
      subtitle: 'Winsorize income · Drop implausible ages',
      predecessors: [{ id: P1 }],
      records: [{ id: 'half-1' }, { id: 'half-2' }],
    });
    expect(itemForCandidate(items, 'half-2')?.id).toBe(`split:${P1}`);
    expect(itemForCandidate(items, 'merge')?.id).toBe('candidate:merge');
    expect(itemForCandidate(items, 'missing')).toBeUndefined();
  });

  it('says what a successor supersedes, and when that decision moved on', () => {
    const items = flattenInboxGroups(buildInboxGroups({ snapshot: project, runs: [], results: {}, resolved: [] }));
    expect(items.find((item) => item.id === 'candidate:merge')?.subtitle).toBe('Supersedes “Trim income” and “Cap ages”');
    expect(items.find((item) => item.id === 'candidate:stale-1')?.subtitle).toBe('Supersedes “Use pandas” (superseded since)');
    expect(items.find((item) => item.id === 'candidate:plain')?.subtitle).toBe('Plain candidate preview');
  });

  it('groups candidates connected through a shared predecessor', () => {
    const chained = {
      ...project,
      records: [
        ...project.records.filter((entry) => !entry.id.startsWith('half')),
        successor('also-trim', 'Trim at p90', [P2], 60),
      ],
    };
    const groups = groupSuccessorCandidates(
      chained.records.filter((entry) => entry.status === 'candidate'),
      new Map(chained.records.map((entry) => [entry.id, entry])),
    );
    expect(groups).toHaveLength(1);
    expect(groups[0]).toMatchObject({
      id: `split:${[P2, P3].sort().join(',')}`,
      title: 'Split “Trim income” and “Cap ages” into 2 decisions',
    });
  });
});
