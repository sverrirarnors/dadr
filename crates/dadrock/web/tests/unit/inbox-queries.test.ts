import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { TaskResult, TaskRun } from '../../src/lib/api';
import {
  clearInboxResultCache,
  fetchInboxRuns,
  hasActiveRun,
  inboxRunsRefetchInterval,
} from '../../src/lib/inbox/queries';

function run(id: string, status: TaskRun['status']): TaskRun {
  return {
    id,
    task_id: 'custom',
    task_source: { kind: 'builtin', id: 'builtin:custom' },
    task_version: 1,
    status,
    created_at: '2026-10-11T10:00:00Z',
    updated_at: '2026-10-11T10:00:00Z',
    run_dir: `/runs/${id}`,
  };
}

describe('fetchInboxRuns', () => {
  beforeEach(() => {
    clearInboxResultCache();
  });

  it('loads results for succeeded runs only and caches them', async () => {
    const runs = [run('ok', 'succeeded'), run('live', 'running'), run('bad', 'failed')];
    const getTaskResult = vi.fn(async (id: string): Promise<TaskResult> => ({ kind: 'markdown_report.v1', id }));
    const api = { listTaskRuns: vi.fn(async () => runs), getTaskResult };

    const first = await fetchInboxRuns(api);
    expect(first.runs).toEqual(runs);
    expect(Object.keys(first.results)).toEqual(['ok']);
    expect(getTaskResult).toHaveBeenCalledTimes(1);
    expect(getTaskResult).toHaveBeenCalledWith('ok');

    const second = await fetchInboxRuns(api);
    expect(second.results.ok).toEqual({ kind: 'markdown_report.v1', id: 'ok' });
    expect(getTaskResult).toHaveBeenCalledTimes(1);
  });

  it('ignores runs whose result cannot be read', async () => {
    const api = {
      listTaskRuns: async () => [run('a', 'succeeded'), run('b', 'succeeded')],
      getTaskResult: async (id: string): Promise<TaskResult> => {
        if (id === 'a') throw new Error('missing');
        return { kind: 'markdown_report.v1' };
      },
    };
    const { results } = await fetchInboxRuns(api);
    expect(results).toEqual({ b: { kind: 'markdown_report.v1' } });
  });
});

describe('polling', () => {
  it('polls only while a run is created or running', () => {
    expect(hasActiveRun([run('a', 'succeeded'), run('b', 'cancelled')])).toBe(false);
    expect(hasActiveRun([run('a', 'created')])).toBe(true);
    expect(inboxRunsRefetchInterval(undefined)).toBe(false);
    expect(inboxRunsRefetchInterval({ runs: [run('a', 'running')], results: {} })).toBe(1000);
    expect(inboxRunsRefetchInterval({ runs: [run('a', 'failed')], results: {} })).toBe(false);
  });
});
