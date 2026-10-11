import type {
  CandidateTemplate,
  MutationResponse,
  RecordDiffResult,
  RecordHistory,
  RecordLifecycle,
  RecordSource,
  Snapshot,
  TaskListing,
  TaskPackage,
  TaskResult,
  TaskRun,
  TaskRunLogEntry,
  ValidationTarget,
} from './schemas';

export interface CreateCandidateRequest {
  /** One record, or several for a merge, that the new candidate supersedes. */
  supersedes?: string | string[] | undefined;
  /** Why the decision changed; written into `## Supersedes` after the links. */
  supersedes_reason?: string | undefined;
  title: string;
  tags: string[];
  body_markdown: string;
}

export interface ApplyRecordRevisionRequest {
  record_id: string;
  title: string;
  tags: string[];
  body_markdown: string;
}

export interface SupersedeRecordsRequest {
  from: string;
  to: string[];
}

export interface DraftCandidateRequest {
  prompt: string;
  prompt_packs: string[];
  write: boolean;
}

export type TaskInputValue = string | number | boolean | string[];

export interface TaskContext {
  system_prompt?: string;
  instructions?: string;
  paths?: string[];
  records?: string[];
  text?: string;
}

export interface RunTaskRequest {
  task_id: string;
  input: Record<string, TaskInputValue>;
  context?: TaskContext;
}

export interface DadrApi {
  getSnapshot(): Promise<Snapshot>;
  initializeProject(): Promise<MutationResponse>;
  getRecordSource(id: string): Promise<RecordSource>;
  getRecordLifecycle(id: string): Promise<RecordLifecycle>;
  /** The versions of the decision containing `id`; with `patch`, each link carries its diff. */
  getRecordHistory(id: string, options?: { patch?: boolean }): Promise<RecordHistory>;
  /** The difference from `against` (old) to `id` (new). */
  getRecordDiff(id: string, against: string): Promise<RecordDiffResult>;
  saveRecordSource(id: string, content: string): Promise<MutationResponse>;
  runValidation(target: ValidationTarget): Promise<MutationResponse>;
  getCandidateTemplate(): Promise<CandidateTemplate>;
  createCandidate(request: CreateCandidateRequest): Promise<MutationResponse>;
  applyRecordRevision(request: ApplyRecordRevisionRequest): Promise<MutationResponse>;
  supersedeRecords(request: SupersedeRecordsRequest): Promise<MutationResponse>;
  draftCandidate(request: DraftCandidateRequest): Promise<MutationResponse>;
  /** Accept a candidate, and with `withIds` more candidates in the same operation (a split). */
  acceptCandidate(id: string, withIds?: string[]): Promise<MutationResponse>;
  rejectCandidate(id: string): Promise<MutationResponse>;
  listTasks(): Promise<TaskListing>;
  getTask(id: string): Promise<TaskPackage>;
  listTaskRuns(): Promise<TaskRun[]>;
  getTaskResult(id: string): Promise<TaskResult>;
  getTaskLog(id: string): Promise<TaskRunLogEntry[]>;
  runTask(request: RunTaskRequest): Promise<TaskRun>;
  cancelTaskRun(id: string): Promise<TaskRun>;
  clearTaskRuns(): Promise<MutationResponse>;
}
