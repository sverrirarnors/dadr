import type { RecordSummary, RunTaskRequest, TaskContext } from '$lib/api';
import type { DraftMode } from '$lib/stores/composer.svelte';

export type { DraftMode };

export interface DraftComposition {
  mode: DraftMode;
  text: string;
  paths: string[];
  recordIds: string[];
}

export type DraftTaskId =
  | 'context-to-record'
  | 'code-to-record'
  | 'edit-record'
  | 'supersede-record'
  | 'merge-records';

export interface DraftIntent {
  taskId: DraftTaskId;
  /** Short, human description of what will happen when the draft is sent. */
  label: string;
  request: RunTaskRequest;
}

export type DraftIntentResult = { ok: true; intent: DraftIntent } | { ok: false; reason: string };

/**
 * Decides which built-in task a composition maps to. The composer never shows
 * the task catalog; what you attached and where you opened it decide.
 */
export function resolveDraftIntent(
  composition: DraftComposition,
  records: RecordSummary[],
): DraftIntentResult {
  const text = composition.text.trim();
  const paths = unique(composition.paths.map((path) => path.trim()).filter(Boolean));
  const recordIds = unique(composition.recordIds.filter(Boolean));
  const byId = new Map(records.map((record) => [record.id, record]));
  const attached = recordIds.map((id) => byId.get(id)).filter((record): record is RecordSummary => Boolean(record));

  switch (composition.mode) {
    case 'revise': {
      const record = attached[0];
      if (!record) return fail('Choose the record to revise.');
      if (!text) return fail('Describe what should change.');
      return ok('edit-record', `Revise “${record.title}”`, {
        task_id: 'edit-record',
        input: { record_id: record.id, requested_changes: text },
        context: context({ records: [record.id], paths }),
      });
    }
    case 'supersede': {
      const record = attached[0];
      if (!record) return fail('Choose the decision to supersede.');
      if (!text) return fail('Explain what changed and why.');
      return ok('supersede-record', `Draft a successor for “${record.title}”`, {
        task_id: 'supersede-record',
        input: { record_ids: record.id, requested_changes: text },
        context: context({ records: [record.id], paths }),
      });
    }
    case 'merge': {
      const candidates = attached.filter((record) => record.status === 'candidate');
      if (candidates.length < 2) return fail('Attach at least two candidates to merge.');
      return ok('merge-records', `Merge ${candidates.length} candidates into one`, {
        task_id: 'merge-records',
        input: {
          record_ids: candidates.map((record) => record.id).join('\n'),
          ...(text ? { merge_goal: text } : {}),
        },
        context: context({ records: candidates.map((record) => record.id) }),
      });
    }
    case 'new': {
      if (paths.length) {
        const noun = paths.length === 1 ? 'path' : 'paths';
        return ok('code-to-record', `Look for decisions in ${paths.length} ${noun}`, {
          task_id: 'code-to-record',
          input: { path: paths[0] as string, ...(text ? { focus: text } : {}) },
          context: context({ paths, records: recordIds, instructions: text }),
        });
      }
      const candidates = attached.filter((record) => record.status === 'candidate');
      if (candidates.length >= 2 && candidates.length === attached.length) {
        return ok('merge-records', `Merge ${candidates.length} candidates into one`, {
          task_id: 'merge-records',
          input: {
            record_ids: candidates.map((record) => record.id).join('\n'),
            ...(text ? { merge_goal: text } : {}),
          },
          context: context({ records: candidates.map((record) => record.id) }),
        });
      }
      if (!text) return fail('Describe the decision, or attach files to scan.');
      return ok('context-to-record', 'Draft one decision from your notes', {
        task_id: 'context-to-record',
        input: { context: text },
        context: context({ records: recordIds }),
      });
    }
  }
}

interface LooseRunTaskRequest {
  task_id: string;
  input: RunTaskRequest['input'];
  context?: TaskContext | undefined;
}

function ok(taskId: DraftTaskId, label: string, request: LooseRunTaskRequest): DraftIntentResult {
  const cleaned: RunTaskRequest = { task_id: request.task_id, input: request.input };
  if (request.context) cleaned.context = request.context;
  return { ok: true, intent: { taskId, label, request: cleaned } };
}

function fail(reason: string): DraftIntentResult {
  return { ok: false, reason };
}

function context(fields: { records?: string[]; paths?: string[]; instructions?: string }): TaskContext | undefined {
  const result: TaskContext = {};
  if (fields.records?.length) result.records = fields.records;
  if (fields.paths?.length) result.paths = fields.paths;
  if (fields.instructions?.trim()) result.instructions = fields.instructions.trim();
  return Object.keys(result).length ? result : undefined;
}

function unique(values: string[]): string[] {
  return Array.from(new Set(values));
}
