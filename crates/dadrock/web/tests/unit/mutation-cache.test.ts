import { QueryClient } from '@tanstack/svelte-query';
import { describe, expect, it } from 'vitest';
import { applyMutationResponse } from '../../src/lib/query/mutation-cache';
import type { MutationResponse, RecordLifecycle, RecordSummary, Snapshot } from '../../src/lib/api';

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

function snapshot(records: RecordSummary[]): Snapshot {
  return {
    project: {
      cwd: '/tmp/dadr-mock',
      decisions_dir: '/tmp/dadr-mock/dadr/accepted',
      initialized: true,
      discovery_source: 'directory',
      status: 'Project discovered.',
    },
    records,
    counts: { total: records.length, candidates: 0, accepted: records.length, rejected: 0, superseded: 0 },
    tags: [],
    project_validation: { counts: {}, diagnostics: [] },
    record_validation: { counts: {}, diagnostics: [] },
    link_health: { counts: {}, diagnostics: [] },
    settings: { config: [], templates: [], agent: [], agent_available: false, tags: [], tag_counts: {}, diagnostics: [] },
    command_catalog: {},
  };
}

describe('applyMutationResponse', () => {
  it('sets the snapshot and updates cached lifecycle summaries from it', async () => {
    const queryClient = new QueryClient();
    const lifecycle: RecordLifecycle = {
      record: candidate,
      predecessors: [],
      successors: [],
      diagnostics: [],
    };
    queryClient.setQueryData(['record-lifecycle', candidate.id], lifecycle);
    const response: MutationResponse = {
      message: `accepted candidate ${candidate.id}`,
      changed_paths: [candidate.path, accepted.path],
      focus: {
        selected_id: candidate.id,
        status_filter: 'accepted',
      },
      snapshot: snapshot([accepted]),
    };

    await applyMutationResponse(queryClient, response);

    expect(queryClient.getQueryData(['snapshot'])).toEqual(response.snapshot);
    expect(queryClient.getQueryData(['record-lifecycle', candidate.id])).toEqual({
      record: accepted,
      predecessors: [],
      successors: [],
      diagnostics: [],
    });
  });
});
