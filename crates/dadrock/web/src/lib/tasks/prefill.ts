export type TaskPrefillTaskId = 'custom' | 'edit-record';

export interface TaskPrefillParams {
  recordId?: string | null | undefined;
  path?: string | null | undefined;
  instructions?: string | null | undefined;
}

export function taskPrefillHref(taskId: TaskPrefillTaskId, prefill: TaskPrefillParams = {}): `/tasks?${string}` {
  const params = new URLSearchParams();
  params.set('task', taskId);
  if (prefill.recordId) params.set('record', prefill.recordId);
  if (prefill.path) params.set('path', prefill.path);
  if (prefill.instructions) params.set('instructions', prefill.instructions);
  return `/tasks?${params.toString()}` as `/tasks?${string}`;
}
