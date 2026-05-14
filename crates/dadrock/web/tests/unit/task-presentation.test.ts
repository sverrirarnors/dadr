import { describe, expect, it } from 'vitest';
import {
  describeTaskRequirements,
  outputKindLabel,
  resultFamilyForKind,
  taskIntentFor,
  taskIntents,
  type TaskPresentationInput,
} from '../../src/lib/tasks/presentation';

function task(id: string, kind: string): TaskPresentationInput {
  return {
    manifest: {
      id,
      inputs: [],
      output: { kind },
    },
  };
}

describe('task presentation model', () => {
  it('groups task manifests by user intent instead of raw output type', () => {
    expect(taskIntents.map((intent) => intent.id)).toEqual([
      'all',
      'create',
      'improve',
      'find',
      'lifecycle',
      'report',
    ]);

    expect(taskIntentFor(task('code-to-record', 'candidate_batch.v1'))).toBe('create');
    expect(taskIntentFor(task('edit-record', 'record_revision.v1'))).toBe('improve');
    expect(taskIntentFor(task('find-conflicts', 'conflict_set.v1'))).toBe('find');
    expect(taskIntentFor(task('supersede-record', 'supersession_proposal.v1'))).toBe('lifecycle');
    expect(taskIntentFor(task('custom', 'markdown_report.v1'))).toBe('report');
  });

  it('labels result output kinds as user-facing families', () => {
    expect(outputKindLabel('candidate_draft.v1')).toBe('Candidate');
    expect(outputKindLabel('candidate_batch.v1')).toBe('Candidate batch');
    expect(outputKindLabel('record_revision.v1')).toBe('Revision');
    expect(outputKindLabel('diagnosis_report.v1')).toBe('Report');
    expect(outputKindLabel('conflict_set.v1')).toBe('Report');
    expect(outputKindLabel('markdown_report.v1')).toBe('Report');
    expect(resultFamilyForKind('supersession_proposal.v1')).toBe('Supersession');
  });

  it('prefers explicit manifest requirements over field-name fallback', () => {
    const requirements = describeTaskRequirements({
      manifest: {
        id: 'custom-review',
        output: { kind: 'markdown_report.v1' },
        inputs: [],
        requirements: {
          records: 'many',
          record_status: 'accepted',
          paths: 'required',
          notes: 'optional',
          scope: 'accepted',
        },
      },
    });

    expect(requirements).toEqual({
      records: 'many',
      recordStatus: 'accepted',
      paths: 'required',
      notes: 'optional',
      scope: 'accepted',
    });
  });

  it('derives task requirements for legacy manifests', () => {
    const requirements = describeTaskRequirements({
      manifest: {
        id: 'edit-record',
        output: { kind: 'record_revision.v1' },
        inputs: [
          { name: 'record_id', label: 'Candidate record', type: 'text', required: true, options: [] },
          { name: 'requested_changes', label: 'Requested changes', type: 'textarea', required: true, options: [] },
        ],
      },
    });

    expect(requirements.records).toBe('one');
    expect(requirements.recordStatus).toBe('all');
    expect(requirements.paths).toBe('none');
    expect(requirements.notes).toBe('required');
  });
});
