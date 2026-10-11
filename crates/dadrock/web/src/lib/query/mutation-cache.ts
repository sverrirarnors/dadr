import type { QueryClient, QueryKey } from '@tanstack/svelte-query';
import type { MutationResponse, RecordLifecycle, RecordSummary } from '$lib/api';

export async function applyMutationResponse(
  queryClient: QueryClient,
  response: MutationResponse,
): Promise<void> {
  queryClient.setQueryData(['snapshot'], response.snapshot);
  reconcileCachedLifecycles(queryClient, response.snapshot.records);

  const affectedRecordIds = recordIdsForMutationResponse(response);
  await Promise.all([
    queryClient.invalidateQueries({ queryKey: ['snapshot'] }),
    queryClient.invalidateQueries({ queryKey: ['record-lifecycle'] }),
    // Histories and diffs span several records, so any write can change them.
    queryClient.invalidateQueries({ queryKey: ['record-history'] }),
    queryClient.invalidateQueries({ queryKey: ['record-diff'] }),
    ...affectedRecordIds.map((recordId) =>
      queryClient.invalidateQueries({ queryKey: ['record-source', recordId] }),
    ),
  ]);
}

function reconcileCachedLifecycles(queryClient: QueryClient, records: RecordSummary[]) {
  const summaries = new Map(records.map((record) => [record.id, record]));
  for (const query of queryClient.getQueryCache().findAll({ queryKey: ['record-lifecycle'] })) {
    const recordId = recordIdFromLifecycleKey(query.queryKey);
    if (!recordId) continue;
    queryClient.setQueryData<RecordLifecycle>(query.queryKey, (current) =>
      reconcileLifecycle(current, summaries),
    );
  }
}

function reconcileLifecycle(
  lifecycle: RecordLifecycle | undefined,
  summaries: Map<string, RecordSummary>,
): RecordLifecycle | undefined {
  if (!lifecycle) return lifecycle;
  return {
    ...lifecycle,
    record: summaries.get(lifecycle.record.id) ?? lifecycle.record,
    predecessors: lifecycle.predecessors.map((record) => summaries.get(record.id) ?? record),
    successors: lifecycle.successors.map((record) => summaries.get(record.id) ?? record),
  };
}

function recordIdsForMutationResponse(response: MutationResponse): string[] {
  const changedPaths = new Set(response.changed_paths);
  const ids = new Set<string>();
  if (response.focus?.selected_id) {
    ids.add(response.focus.selected_id);
  }
  for (const record of response.snapshot.records) {
    if (changedPaths.has(record.path)) {
      ids.add(record.id);
    }
  }
  return [...ids];
}

function recordIdFromLifecycleKey(queryKey: QueryKey): string | null {
  return queryKey[0] === 'record-lifecycle' && typeof queryKey[1] === 'string'
    ? queryKey[1]
    : null;
}
