import { describe, expect, it } from 'vitest';
import type { RecordSummary } from '../../src/lib/api';
import { resolveDraftIntent } from '../../src/lib/draft/intent';

function record(id: string, status: RecordSummary['status'], title = id): RecordSummary {
  return {
    id,
    id_prefix: id.slice(0, 12),
    simple_record_id: id,
    title,
    status,
    superseded: false,
    path: `dadr/${status}/${id}.md`,
    preview: '',
    tags: [],
    timestamp_ms: 0,
  };
}

const records = [
  record('acc-1', 'accepted', 'Use Polars'),
  record('cand-1', 'candidate', 'First draft'),
  record('cand-2', 'candidate', 'Second draft'),
];

describe('resolveDraftIntent', () => {
  it('turns notes alone into one candidate', () => {
    const result = resolveDraftIntent({ mode: 'new', text: 'We winsorize income.', paths: [], recordIds: [] }, records);
    expect(result).toMatchObject({
      ok: true,
      intent: { taskId: 'context-to-record', request: { task_id: 'context-to-record', input: { context: 'We winsorize income.' } } },
    });
  });

  it('scans code when paths are attached, using the notes as focus', () => {
    const result = resolveDraftIntent({ mode: 'new', text: 'cleaning', paths: ['R/', 'py/x.py'], recordIds: [] }, records);
    expect(result).toMatchObject({
      ok: true,
      intent: {
        taskId: 'code-to-record',
        request: { input: { path: 'R/', focus: 'cleaning' }, context: { paths: ['R/', 'py/x.py'], instructions: 'cleaning' } },
      },
    });
  });

  it('merges when two or more candidates are attached and nothing else', () => {
    const result = resolveDraftIntent({ mode: 'new', text: '', paths: [], recordIds: ['cand-1', 'cand-2'] }, records);
    expect(result).toMatchObject({
      ok: true,
      intent: { taskId: 'merge-records', request: { input: { record_ids: 'cand-1\ncand-2' } } },
    });
  });

  it('asks for something to work from', () => {
    expect(resolveDraftIntent({ mode: 'new', text: '  ', paths: [], recordIds: [] }, records)).toEqual({
      ok: false,
      reason: 'Describe the decision, or attach files to scan.',
    });
  });

  it('revises the attached record', () => {
    const result = resolveDraftIntent({ mode: 'revise', text: 'Tighten wording', paths: [], recordIds: ['cand-1'] }, records);
    expect(result).toMatchObject({
      ok: true,
      intent: {
        taskId: 'edit-record',
        label: 'Revise “First draft”',
        request: { input: { record_id: 'cand-1', requested_changes: 'Tighten wording' }, context: { records: ['cand-1'] } },
      },
    });
    expect(resolveDraftIntent({ mode: 'revise', text: '', paths: [], recordIds: ['cand-1'] }, records)).toMatchObject({ ok: false });
  });

  it('drafts a successor for the attached decision', () => {
    const result = resolveDraftIntent({ mode: 'supersede', text: 'Moved to DuckDB', paths: [], recordIds: ['acc-1'] }, records);
    expect(result).toMatchObject({
      ok: true,
      intent: {
        taskId: 'supersede-record',
        label: 'Draft a successor for “Use Polars”',
        request: { input: { record_ids: 'acc-1', requested_changes: 'Moved to DuckDB' } },
      },
    });
  });

  it('drafts one successor for several decisions, a merge', () => {
    const result = resolveDraftIntent(
      { mode: 'supersede', text: 'One rule', paths: [], recordIds: ['acc-1', 'acc-2'] },
      [...records, { ...records[0]!, id: 'acc-2', title: 'Use Arrow', status: 'accepted' }],
    );
    expect(result).toMatchObject({
      ok: true,
      intent: {
        taskId: 'supersede-record',
        label: 'Draft one successor for 2 decisions',
        request: { input: { record_ids: 'acc-1\nacc-2' }, context: { records: ['acc-1', 'acc-2'] } },
      },
    });
  });

  it('refuses to supersede an archived record', () => {
    const archived = { ...records[0]!, id: 'acc-9', title: 'Old rule', status: 'accepted' as const, superseded: true };
    expect(
      resolveDraftIntent({ mode: 'supersede', text: 'x', paths: [], recordIds: ['acc-9'] }, [...records, archived]),
    ).toEqual({ ok: false, reason: '“Old rule” is not a decision in force; supersede the current decision instead.' });
  });

  it('revises a decision in force by drafting a successor, and never an archived one', () => {
    expect(
      resolveDraftIntent({ mode: 'revise', text: 'Use DuckDB', paths: [], recordIds: ['acc-1'] }, records),
    ).toMatchObject({ ok: true, intent: { taskId: 'supersede-record', request: { input: { record_ids: 'acc-1' } } } });
    const rejected = { ...records[0]!, id: 'rej-1', title: 'Use SAS', status: 'rejected' as const };
    expect(
      resolveDraftIntent({ mode: 'revise', text: 'x', paths: [], recordIds: ['rej-1'] }, [...records, rejected]),
    ).toEqual({ ok: false, reason: '“Use SAS” is archived; correct it with Edit in the Ledger.' });
  });

  it('refuses to merge fewer than two candidates', () => {
    expect(resolveDraftIntent({ mode: 'merge', text: '', paths: [], recordIds: ['cand-1', 'acc-1'] }, records)).toEqual({
      ok: false,
      reason: 'Attach at least two candidates to merge.',
    });
  });
});
