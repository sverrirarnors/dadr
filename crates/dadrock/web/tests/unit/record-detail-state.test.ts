import { describe, expect, it } from 'vitest';
import type { RecordSummary } from '../../src/lib/api';
import { detailActionRecord } from '../../src/lib/records/detail-state';

const candidate: RecordSummary = {
  id: '0193fb1f-a3a0-7000-8000-000000000002',
  id_prefix: '0193fb1fa3a0',
  simple_record_id: '0193fb1fa3a070008000000000000002',
  title: 'Adopt pre-registration for studies',
  status: 'candidate',
  superseded: false,
  path: 'dadr/candidates/0002-preregister.md',
  preview: 'Candidate decision.',
  tags: ['#dadr/methodology'],
  timestamp_ms: 1730000100000,
};

const accepted: RecordSummary = {
  ...candidate,
  status: 'accepted',
  path: 'dadr/accepted/0002-preregister.md',
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
