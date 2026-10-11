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
        request: { input: { record_id: 'acc-1', reason: 'Moved to DuckDB' } },
      },
    });
  });

  it('refuses to merge fewer than two candidates', () => {
    expect(resolveDraftIntent({ mode: 'merge', text: '', paths: [], recordIds: ['cand-1', 'acc-1'] }, records)).toEqual({
      ok: false,
      reason: 'Attach at least two candidates to merge.',
    });
  });
});
