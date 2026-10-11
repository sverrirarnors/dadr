import { z } from 'zod';

export const recordStatusSchema = z.enum(['candidate', 'accepted', 'rejected', 'superseded']);
export type RecordStatus = z.infer<typeof recordStatusSchema>;

const uuidv7 = z
  .string()
  .regex(
    /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
    'expected UUIDv7',
  );

export const recordSummarySchema = z.object({
  id: uuidv7,
  id_prefix: z.string(),
  simple_record_id: z.string(),
  title: z.string(),
  status: recordStatusSchema,
  superseded: z.boolean(),
  path: z.string(),
  preview: z.string(),
  tags: z.array(z.string()),
  timestamp_ms: z.number().int().nonnegative(),
});
export type RecordSummary = z.infer<typeof recordSummarySchema>;

export const projectInfoSchema = z.object({
  cwd: z.string(),
  decisions_dir: z.string(),
  initialized: z.boolean(),
  discovery_source: z.string(),
  status: z.string(),
});

export const recordCountsSchema = z.object({
  total: z.number().int().nonnegative(),
  candidates: z.number().int().nonnegative(),
  accepted: z.number().int().nonnegative(),
  rejected: z.number().int().nonnegative(),
  superseded: z.number().int().nonnegative(),
});

export const tagSchema = z.object({
  tag: z.string(),
  display: z.string(),
  record_count: z.number().int().nonnegative(),
  occurrence_count: z.number().int().nonnegative(),
});
export type TagSummary = z.infer<typeof tagSchema>;

export const diagnosticSeveritySchema = z.enum(['error', 'warning', 'info']);
export const diagnosticSchema = z.object({
  severity: diagnosticSeveritySchema,
  code: z.string(),
  message: z.string(),
  path: z.string().nullable(),
  record_id: z.string().nullable(),
  field: z.string().nullable(),
  line: z.number().int().nullable(),
  related_paths: z.array(z.string()),
});
export type Diagnostic = z.infer<typeof diagnosticSchema>;

/** A predecessor named in `## Supersedes` that was superseded since. */
export const staleSupersedesSchema = z.object({
  predecessor: recordSummarySchema,
  current_successors: z.array(recordSummarySchema),
});
export type StaleSupersedes = z.infer<typeof staleSupersedesSchema>;

export const recordLifecycleSchema = z.object({
  /** Every record a candidate's `## Supersedes` section names. */
  pending_supersedes: z.array(recordSummarySchema).default([]),
  stale_supersedes: z.array(staleSupersedesSchema).default([]),
  record: recordSummarySchema,
  predecessors: z.array(recordSummarySchema),
  successors: z.array(recordSummarySchema),
  diagnostics: z.array(diagnosticSchema),
});
export type RecordLifecycle = z.infer<typeof recordLifecycleSchema>;

/** One line of a body diff: unchanged, added, or removed. */
export const textDiffLineSchema = z.object({
  kind: z.enum(['context', 'added', 'removed']),
  text: z.string(),
});
export type TextDiffLine = z.infer<typeof textDiffLineSchema>;

/** The difference between two records: title, tags, and the body without `## Supersedes`. */
export const recordDiffSchema = z.object({
  changed: z.boolean(),
  title: z.object({ old: z.string(), new: z.string() }).nullable().default(null),
  tags_added: z.array(z.string()).default([]),
  tags_removed: z.array(z.string()).default([]),
  lines: z.array(textDiffLineSchema).default([]),
});
export type RecordDiff = z.infer<typeof recordDiffSchema>;

/** One record as it appears in a history or a diff. */
export const historyRecordSchema = z.object({
  id: uuidv7,
  short_id: z.string(),
  title: z.string(),
  status: z.enum(['candidate', 'accepted', 'rejected']),
  superseded: z.boolean(),
  /** No record supersedes this one. */
  current: z.boolean(),
  path: z.string(),
  tags: z.array(z.string()).default([]),
});
export type HistoryRecord = z.infer<typeof historyRecordSchema>;

/**
 * One edit Git recorded to a version's title or body after it was accepted or
 * rejected. The uncommitted working-tree edit has no commit, author, date, or
 * subject. `diff` comes only with `?patch=1`.
 */
export const recordEditSchema = z.object({
  commit: z.string().nullable().default(null),
  short_commit: z.string().nullable().default(null),
  author: z.string().nullable().default(null),
  /** Author date, ISO 8601. */
  date: z.string().nullable().default(null),
  subject: z.string().nullable().default(null),
  title_changed: z.boolean(),
  body_changed: z.boolean(),
  diff: recordDiffSchema.optional(),
});
export type RecordEdit = z.infer<typeof recordEditSchema>;

/**
 * A version of a decision with its body, without `## Supersedes`, and its text
 * edits since it was accepted or rejected, newest first: empty for
 * candidates, null outside a Git repository.
 */
export const historyVersionSchema = historyRecordSchema.extend({
  body: z.string(),
  edits: z.array(recordEditSchema).nullable().default(null),
});
export type HistoryVersion = z.infer<typeof historyVersionSchema>;

/** One supersession edge, with the reason from the successor's `## Supersedes`. */
export const historyLinkSchema = z.object({
  predecessor: uuidv7,
  successor: uuidv7,
  reason: z.string().nullable().default(null),
  diff: recordDiffSchema.optional(),
});
export type HistoryLink = z.infer<typeof historyLinkSchema>;

/** The versions of one decision, newest first, with links and neighbors. */
export const recordHistorySchema = z.object({
  record: uuidv7,
  versions: z.array(historyVersionSchema),
  links: z.array(historyLinkSchema),
  neighbors: z.array(historyRecordSchema),
});
export type RecordHistory = z.infer<typeof recordHistorySchema>;

/** Two records and the difference from `old` to `new`. */
export const recordDiffResultSchema = z.object({
  old: historyRecordSchema,
  new: historyRecordSchema,
  diff: recordDiffSchema,
});
export type RecordDiffResult = z.infer<typeof recordDiffResultSchema>;

export const issueCountsSchema = z
  .object({})
  .catchall(z.number().int().nonnegative().or(z.string()).or(z.boolean()));

export const diagnosticPanelSchema = z.object({
  counts: issueCountsSchema,
  diagnostics: z.array(diagnosticSchema),
});
export type DiagnosticPanel = z.infer<typeof diagnosticPanelSchema>;

export const validationTargetSchema = z.enum(['project', 'records', 'links', 'tags']);
export type ValidationTarget = z.infer<typeof validationTargetSchema>;

export const settingsSchema = z.object({
  config: z.array(z.string()),
  templates: z.array(z.string()),
  agent: z.array(z.string()),
  /** True when a local ACP agent is configured or auto-detected on PATH. */
  agent_available: z.boolean().default(false),
  tags: z.array(z.string()),
  tag_counts: issueCountsSchema,
  diagnostics: z.array(diagnosticSchema),
});

export const snapshotSchema = z.object({
  project: projectInfoSchema,
  records: z.array(recordSummarySchema),
  counts: recordCountsSchema,
  tags: z.array(tagSchema),
  project_validation: diagnosticPanelSchema,
  record_validation: diagnosticPanelSchema,
  link_health: diagnosticPanelSchema,
  settings: settingsSchema,
  command_catalog: z.unknown(),
});
export type Snapshot = z.infer<typeof snapshotSchema>;

export const recordSourceSchema = z.object({
  id: uuidv7,
  path: z.string(),
  content: z.string(),
});
export type RecordSource = z.infer<typeof recordSourceSchema>;

export const candidateTemplateSchema = z.object({
  name: z.string(),
  source: z.string(),
  path: z.string().nullable(),
  body: z.string(),
});
export type CandidateTemplate = z.infer<typeof candidateTemplateSchema>;

export const taskInputTypeSchema = z.enum([
  'text',
  'textarea',
  'path',
  'select',
  'multiselect',
  'checkbox',
  'number',
]);
export type TaskInputType = z.infer<typeof taskInputTypeSchema>;

export const taskInputSchema = z.object({
  name: z.string(),
  label: z.string(),
  type: taskInputTypeSchema,
  default: z.string().optional(),
  options: z.array(z.string()).default([]),
  required: z.boolean().default(false),
});
export type TaskInput = z.infer<typeof taskInputSchema>;

export const taskRequirementsSchema = z
  .object({
    records: z.enum(['none', 'one', 'one_or_more', 'many']).default('none'),
    record_status: z.enum(['all', 'candidate', 'accepted']).default('all'),
    paths: z.enum(['none', 'optional', 'required']).default('none'),
    notes: z.enum(['none', 'optional', 'required']).default('none'),
    scope: z.enum(['none', 'live', 'candidates', 'accepted', 'all']).default('none'),
  })
  .default({
    records: 'none',
    record_status: 'all',
    paths: 'none',
    notes: 'none',
    scope: 'none',
  });
export type TaskRequirements = z.infer<typeof taskRequirementsSchema>;

export const taskManifestSchema = z.object({
  id: z.string(),
  title: z.string(),
  description: z.string(),
  version: z.number().int().nonnegative(),
  execution: z.literal('agent').default('agent'),
  input: z.object({ mode: z.string() }).default({ mode: 'form' }),
  inputs: z.array(taskInputSchema).default([]),
  requirements: taskRequirementsSchema,
  output: z.object({ kind: z.string() }),
  mcp_access: z.enum(['full', 'read_only']).default('full'),
  tools: z.array(z.object({ name: z.string(), mode: z.string() })).default([]),
});
export type TaskManifest = z.infer<typeof taskManifestSchema>;

export const taskPackageSchema = z.object({
  manifest: taskManifestSchema,
  source: z.object({
    kind: z.enum(['project', 'user', 'builtin']),
    id: z.string(),
    path: z.string().optional(),
  }),
  prompt: z.string(),
});
export type TaskPackage = z.infer<typeof taskPackageSchema>;

export const taskDiagnosticSchema = z.object({
  code: z.string(),
  message: z.string(),
  source_id: z.string().optional(),
  path: z.string().optional(),
});
export type TaskDiagnostic = z.infer<typeof taskDiagnosticSchema>;

export const taskListingSchema = z.object({
  tasks: z.array(taskPackageSchema),
  diagnostics: z.array(taskDiagnosticSchema),
});
export type TaskListing = z.infer<typeof taskListingSchema>;

export const taskRunSchema = z.object({
  id: uuidv7,
  task_id: z.string(),
  task_source: z.object({
    kind: z.enum(['project', 'user', 'builtin']),
    id: z.string(),
    path: z.string().optional(),
  }),
  task_version: z.number().int().nonnegative(),
  status: z.enum(['created', 'running', 'succeeded', 'failed', 'cancelled']),
  created_at: z.string(),
  updated_at: z.string(),
  run_dir: z.string(),
});
export type TaskRun = z.infer<typeof taskRunSchema>;

export const taskRunLogEntrySchema = z.object({
  timestamp: z.string(),
  message: z.string(),
});
export type TaskRunLogEntry = z.infer<typeof taskRunLogEntrySchema>;

export const taskContextSchema = z.object({
  system_prompt: z.string().optional(),
  instructions: z.string().optional(),
  paths: z.array(z.string()).optional(),
  records: z.array(z.string()).optional(),
  text: z.string().optional(),
});

export const taskResultSchema = z.record(z.string(), z.unknown());
export type TaskResult = z.infer<typeof taskResultSchema>;

export const mutationFocusSchema = z.object({
  selected_id: z.string().nullable(),
  status_filter: z.string().nullable(),
});

export const mutationResponseSchema = z.object({
  message: z.string(),
  changed_paths: z.array(z.string()),
  focus: mutationFocusSchema.optional(),
  snapshot: snapshotSchema,
});
export type MutationResponse = z.infer<typeof mutationResponseSchema>;

export function commandEnvelopeSchema<T extends z.ZodTypeAny>(data: T) {
  return z.object({
    command: z.string(),
    ok: z.boolean(),
    data,
    diagnostics: z.array(z.unknown()).default([]),
    changed_paths: z.array(z.string()).default([]),
  });
}
