import type { DadrApi, TaskResult, TaskRun } from '$lib/api';

export const inboxRunsQueryKey = ['inbox-runs'] as const;

export interface InboxRuns {
  runs: TaskRun[];
  results: Record<string, TaskResult | undefined>;
}

type InboxRunsApi = Pick<DadrApi, 'listTaskRuns' | 'getTaskResult'>;

/** A succeeded run's result never changes, so each one is fetched once per session. */
const resultCache = new Map<string, TaskResult>();

/**
 * Lists agent runs and loads the result of every succeeded run in parallel.
 * A run whose result cannot be read is simply left without one.
 */
export async function fetchInboxRuns(api: InboxRunsApi): Promise<InboxRuns> {
  const runs = await api.listTaskRuns();
  const results: Record<string, TaskResult | undefined> = {};
  await Promise.all(
    runs
      .filter((run) => run.status === 'succeeded')
      .map(async (run) => {
        const result = await loadResult(api, run.id);
        if (result) results[run.id] = result;
      }),
  );
  return { runs, results };
}

async function loadResult(api: InboxRunsApi, runId: string): Promise<TaskResult | undefined> {
  const cached = resultCache.get(runId);
  if (cached) return cached;
  try {
    const result = await api.getTaskResult(runId);
    resultCache.set(runId, result);
    return result;
  } catch {
    return undefined;
  }
}

export function isActiveRun(run: TaskRun): boolean {
  return run.status === 'created' || run.status === 'running';
}

export function hasActiveRun(runs: TaskRun[]): boolean {
  return runs.some(isActiveRun);
}

/** Poll every second while the agent is working, otherwise stay quiet. */
export function inboxRunsRefetchInterval(data: InboxRuns | undefined): number | false {
  return data && hasActiveRun(data.runs) ? 1000 : false;
}

export function clearInboxResultCache(): void {
  resultCache.clear();
}
