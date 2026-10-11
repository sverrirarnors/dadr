import { describe, expect, it } from 'vitest';
import type { RecordSummary } from '../../src/lib/api';
import { detailActionRecord, detailActions, successorDraft } from '../../src/lib/records/detail-state';

const candidate: RecordSummary = {
  id: '0193fb1f-a3a0-7000-8000-000000000002',
  id_prefix: '0193fb1fa3a0',
  simple_record_id: '0193fb1fa3a070008000000000000002',
  title: 'Adopt pre-registration for studies',
  status: 'candidate',
  superseded: false,
  path: 'dadr/inbox/preregister.md',
  preview: 'Candidate decision.',
  tags: ['#dadr/methodology'],
  timestamp_ms: 1730000100000,
};

const accepted: RecordSummary = {
  ...candidate,
  status: 'accepted',
  path: 'dadr/ledger/preregister.md',
};

describe('detailActionRecord', () => {
  it('uses the snapshot summary when lifecycle data is stale after acceptance', () => {
    expect(detailActionRecord(candidate, [accepted])).toEqual(accepted);
  });

  it('keeps non-candidate lifecycle data when an older snapshot still shows a candidate', () => {
    expect(detailActionRecord(accepted, [candidate])).toEqual(accepted);
  });

  it('falls back to lifecycle data when the snapshot has not loaded the record', () => {
    expect(detailActionRecord(candidate, [])).toEqual(candidate);
  });
});

describe('detailActions', () => {
  it('lets the inbox edit in place, revise, reject, and accept a candidate', () => {
    expect(detailActions({ surface: 'inbox', status: 'candidate', agentAvailable: true })).toEqual({
      primary: ['edit', 'revise'],
      lifecycle: ['reject', 'accept'],
      revise: 'edit-record',
      correction: false,
    });
  });

  it('hides the agent verb when no agent is available', () => {
    expect(detailActions({ surface: 'inbox', status: 'candidate', agentAvailable: false })).toEqual({
      primary: ['edit'],
      lifecycle: ['reject', 'accept'],
      revise: null,
      correction: false,
    });
    expect(detailActions({ surface: 'ledger', status: 'accepted', agentAvailable: false })).toEqual({
      primary: ['edit'],
      lifecycle: ['supersede'],
      revise: null,
      correction: true,
    });
  });

  it('corrects a decision in force in place, supersedes it, and revises it with the agent as a successor', () => {
    expect(detailActions({ surface: 'ledger', status: 'accepted', agentAvailable: true })).toEqual({
      primary: ['edit', 'revise'],
      lifecycle: ['supersede'],
      revise: 'supersede-record',
      correction: true,
    });
  });

  it('only corrects archived records: Edit, but no Revise or Supersede', () => {
    for (const status of ['superseded', 'rejected'] as const) {
      expect(detailActions({ surface: 'ledger', status, agentAvailable: true })).toEqual({
        primary: ['edit'],
        lifecycle: [],
        revise: null,
        correction: true,
      });
    }
  });

  it('only offers to edit a candidate reached through the ledger', () => {
    expect(detailActions({ surface: 'ledger', status: 'candidate', agentAvailable: true })).toEqual({
      primary: ['edit'],
      lifecycle: [],
      revise: null,
      correction: false,
    });
  });

  it('never offers lifecycle verbs for non-candidates in the inbox', () => {
    expect(detailActions({ surface: 'inbox', status: 'accepted', agentAvailable: true })).toEqual({
      primary: ['edit', 'revise'],
      lifecycle: [],
      revise: 'supersede-record',
      correction: true,
    });
  });
});

describe('successorDraft', () => {
  it('opens the Draft page in supersede mode, by hand for Supersede and with the agent for Revise', () => {
    expect(successorDraft('supersede', 'acc-1')).toEqual({ mode: 'supersede', recordIds: ['acc-1'], view: 'manual' });
    expect(successorDraft('revise', 'acc-1')).toEqual({ mode: 'supersede', recordIds: ['acc-1'], view: 'agent' });
  });
});
