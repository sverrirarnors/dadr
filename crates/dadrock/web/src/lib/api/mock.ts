import type {
  CandidateTemplate,
  MutationResponse,
  RecordLifecycle,
  RecordSource,
  RecordSummary,
  Snapshot,
  TaskListing,
  TaskPackage,
  TaskResult,
  TaskRun,
  TaskRunLogEntry,
  ValidationTarget,
} from './schemas';
import type {
  ApplyRecordRevisionRequest,
  CreateCandidateRequest,
  DadrApi,
  DraftCandidateRequest,
  RunTaskRequest,
  SupersedeRecordsRequest,
} from './types';

const fixture: Snapshot = {
  project: {
    cwd: '/tmp/dadr-mock',
    decisions_dir: '/tmp/dadr-mock/dadr/accepted',
    initialized: true,
    discovery_source: 'directory',
    status: 'Project discovered. Commands operate on the local decision records.',
  },
  records: [
    {
      id: '0193fb1f-a3a0-7000-8000-000000000001',
      id_prefix: '0193fb1fa3a0',
      simple_record_id: '0193fb1fa3a070008000000000000001',
      title: 'Use Polars over pandas',
      status: 'accepted',
      superseded: true,
      path: 'dadr/accepted/0001-use-polars.md',
      preview: 'Use Polars for tabular transformations.',
      tags: ['#dadr/data/processing'],
      timestamp_ms: 1730000000000,
    },
    {
      id: '0193fb1f-a3a0-7000-8000-000000000002',
      id_prefix: '0193fb1fa3a0',
      simple_record_id: '0193fb1fa3a070008000000000000002',
      title: 'Adopt pre-registration for studies',
      status: 'candidate',
      superseded: false,
      path: 'dadr/candidates/0002-preregister.md',
      preview: 'Candidate decision for pre-registration practice.',
      tags: ['#dadr/methodology'],
      timestamp_ms: 1730000100000,
    },
    {
      id: '0193fb1f-a3a0-7000-8000-000000000003',
      id_prefix: '0193fb1fa3a0',
      simple_record_id: '0193fb1fa3a070008000000000000003',
      title: 'Use Polars lazy frames',
      status: 'accepted',
      superseded: false,
      path: 'dadr/accepted/0003-use-polars-lazy.md',
      preview: 'Use lazy frames for transformations that can benefit from deferred execution.',
      tags: ['#dadr/data/processing'],
      timestamp_ms: 1730000200000,
    },
    {
      id: '0193fb1f-a3a0-7000-8000-000000000004',
      id_prefix: '0193fb1fa3a0',
      simple_record_id: '0193fb1fa3a070008000000000000004',
      title: 'Use Arrow IPC snapshots',
      status: 'accepted',
      superseded: false,
      path: 'dadr/accepted/0004-use-arrow-ipc.md',
      preview: 'Use Arrow IPC snapshots for portable intermediate data.',
      tags: ['#dadr/data/processing'],
      timestamp_ms: 1730000300000,
    },
  ],
  counts: { total: 4, candidates: 1, accepted: 2, rejected: 0, superseded: 1 },
  tags: [
    { tag: '#dadr/data/processing', display: 'dadr/data/processing', record_count: 3, occurrence_count: 3 },
    { tag: '#dadr/methodology', display: 'dadr/methodology', record_count: 1, occurrence_count: 1 },
  ],
  project_validation: {
    counts: { errors: 0, warnings: 1, infos: 0 },
    diagnostics: [
      {
        severity: 'warning',
        code: 'candidate_backlog',
        message: 'One candidate is waiting for review.',
        path: 'dadr/candidates/0002-preregister.md',
        record_id: '0193fb1f-a3a0-7000-8000-000000000002',
        field: 'status',
        line: null,
        related_paths: [],
      },
    ],
  },
  record_validation: { counts: { errors: 0, warnings: 0, infos: 0 }, diagnostics: [] },
  link_health: {
    counts: { errors: 1, warnings: 0, infos: 0 },
    diagnostics: [
      {
        severity: 'error',
        code: 'broken_markdown_link',
        message: 'A local Markdown link points at a missing note.',
        path: 'dadr/accepted/0001-use-polars.md',
        record_id: '0193fb1f-a3a0-7000-8000-000000000001',
        field: './missing-note.md',
        line: 12,
        related_paths: [],
      },
    ],
  },
  settings: {
    config: [],
    templates: [],
    agent: ['mode: local ACP agent adaptor', 'auto-detected: mock-agent (mock-agent --acp)'],
    agent_available: true,
    tags: [],
    tag_counts: { errors: 0, warnings: 0, infos: 1 },
    diagnostics: [
      {
        severity: 'info',
        code: 'tag_catalog_empty',
        message: 'No project tag catalog was found; tags are inferred from records.',
        path: null,
        record_id: null,
        field: null,
        line: null,
        related_paths: [],
      },
    ],
  },
  command_catalog: {},
};

const sources = new Map<string, string>();
const mockRecords = fixture.records;
let nextCandidate = 5;
let initialized = fixture.project.initialized;
let nextTaskRun = 1;
const taskRuns: TaskRun[] = [];
const taskResults = new Map<string, TaskResult>();
const taskLogs = new Map<string, TaskRunLogEntry[]>();
const taskRunPollsUntilSuccess = new Map<string, number>();
const pendingReplacements = new Map<string, string>();
const supersessionEdges = new Map<string, Set<string>>([
  [
    '0193fb1f-a3a0-7000-8000-000000000001',
    new Set(['0193fb1f-a3a0-7000-8000-000000000003']),
  ],
]);

function readMockInitialized(): boolean {
  try {
    const value = globalThis.localStorage?.getItem('dadrock:mock-initialized');
    if (value === 'true') return true;
    if (value === 'false') return false;
  } catch {
    // Ignore unavailable storage in non-browser mock contexts.
  }
  return initialized;
}

function setMockInitialized(value: boolean) {
  initialized = value;
  try {
    globalThis.localStorage?.setItem('dadrock:mock-initialized', String(value));
  } catch {
    // Ignore unavailable storage in non-browser mock contexts.
  }
}

const uninitializedStatus =
  'No Dadrock project found here. Initialize to create dadr/ with records, candidates, templates, tasks, and runs.';

const taskPackages: TaskPackage[] = [
  taskPackage('custom', 'Custom', 'Use a freeform prompt with selected records, files, folders, or notes when no dedicated DADR workflow fits.', 'markdown_report.v1', []),
  taskPackage('context-to-record', 'Context to record', 'Turn supplied context into one candidate DADR when you already know the decision to capture.', 'candidate_draft.v1', [
    { name: 'context', label: 'Context', type: 'textarea', options: [], required: true },
  ]),
  taskPackage('find-conflicts', 'Find conflicting decisions', 'Audit selected DADRs for contradictions, duplicates, or unresolved overlap that need follow-up.', 'conflict_set.v1', [
    {
      name: 'status_scope',
      label: 'Records to scan',
      type: 'select',
      default: 'live',
      options: ['live', 'candidates', 'accepted', 'all'],
      required: false,
    },
  ]),
  taskPackage('code-to-record', 'Code to records', 'Scan selected project files and draft candidate DADRs for consequential choices to review.', 'candidate_batch.v1', [
    { name: 'path', label: 'Path', type: 'path', default: '.', options: [], required: false },
    { name: 'focus', label: 'Focus', type: 'textarea', options: [], required: false },
  ]),
  taskPackage('merge-records', 'Merge records', 'Combine overlapping candidate DADRs into one clearer candidate while leaving source records unchanged.', 'candidate_draft.v1', [
    { name: 'record_ids', label: 'Candidate records', type: 'textarea', options: [], required: true },
    { name: 'merge_goal', label: 'Merge goal', type: 'textarea', options: [], required: false },
  ]),
  taskPackage('edit-record', 'Edit record', 'Revise any existing DADR while preserving its lifecycle status until you choose what to do next.', 'record_revision.v1', [
    { name: 'record_id', label: 'Record', type: 'text', options: [], required: true },
    { name: 'requested_changes', label: 'Requested changes', type: 'textarea', options: [], required: true },
  ]),
  taskPackage('supersede-record', 'Supersede record', 'Draft a replacement candidate for an accepted DADR when the project decision has changed.', 'supersession_proposal.v1', [
    { name: 'record_id', label: 'Record to supersede', type: 'text', options: [], required: true },
    { name: 'reason', label: 'Reason', type: 'textarea', options: [], required: true },
  ]),
  taskPackage('decisions-to-spec', 'Decisions to spec', 'Synthesize accepted DADRs into an implementation, analysis, or reporting spec for downstream work.', 'markdown_report.v1', [
    { name: 'focus', label: 'Spec focus', type: 'textarea', options: [], required: false },
  ]),
];

function snapshotCounts(records: RecordSummary[]): Snapshot['counts'] {
  return records.reduce<Snapshot['counts']>(
    (counts, record) => {
      counts.total += 1;
      if (record.status === 'candidate') counts.candidates += 1;
      else if (record.status === 'accepted' && record.superseded) counts.superseded += 1;
      else if (record.status === 'accepted') counts.accepted += 1;
      else if (record.status === 'rejected') counts.rejected += 1;
      return counts;
    },
    { total: 0, candidates: 0, accepted: 0, rejected: 0, superseded: 0 },
  );
}

function slugify(title: string): string {
  return (
    title
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-|-$/g, '')
      .slice(0, 48) || 'candidate'
  );
}

function newCandidateRecord(title: string, tags: string[], body: string): RecordSummary {
  const suffix = String(nextCandidate).padStart(12, '0');
  const id = `0193fb1f-a3a0-7000-8000-${suffix}`;
  nextCandidate += 1;
  const path = `dadr/candidates/${suffix.slice(-4)}-${slugify(title)}.md`;
  const record: RecordSummary = {
    id,
    id_prefix: '0193fb1fa3a0',
    simple_record_id: id.replaceAll('-', ''),
    title,
    status: 'candidate',
    superseded: false,
    path,
    preview: body.trim() || 'Candidate decision.',
    tags,
    timestamp_ms: 1730000200000 + nextCandidate,
  };
  mockRecords.push(record);
  sources.set(
    id,
    `---\nid: ${id}\ntitle: ${title}\nstatus: candidate\ntags: [${tags
      .map((tag) => `"${tag}"`)
      .join(', ')}]\n---\n\n# ${title}\n\n${body}\n`,
  );
  return record;
}

function taskPackage(
  id: string,
  title: string,
  description: string,
  outputKind: string,
  inputs: TaskPackage['manifest']['inputs'],
  execution: TaskPackage['manifest']['execution'] = 'agent',
): TaskPackage {
  return {
    manifest: {
      id,
      title,
      description,
      version: 1,
      execution,
      input: { mode: 'form' },
      inputs,
      requirements: taskRequirementsFor(id, inputs),
      output: { kind: outputKind },
      mcp_access: id === 'context-to-record' || id === 'decisions-to-spec' ? 'read_only' : 'full',
      tools: [],
    },
    source: {
      kind: 'builtin',
      id: `builtin:${id}`,
    },
    prompt: `Mock prompt for ${title}.`,
  };
}

function taskRequirementsFor(id: string, inputs: TaskPackage['manifest']['inputs']): TaskPackage['manifest']['requirements'] {
  const hasInput = (name: string) => inputs.some((input) => input.name === name);
  const hasRequiredInput = (name: string) =>
    inputs.some((input) => input.name === name && input.required);
  const hasDetailInput = inputs.some((input) =>
    [
      'context',
      'discussion',
      'focus',
      'requested_changes',
      'reason',
      'merge_goal',
      'review_focus',
      'tag_goal',
      'decision_hint',
    ].includes(input.name),
  );
  const hasRequiredDetail = inputs.some(
    (input) =>
      input.required &&
      [
        'context',
        'discussion',
        'requested_changes',
        'reason',
      ].includes(input.name),
  );

  return {
    records: id === 'custom' ? 'many' : hasInput('record_ids') ? 'many' : hasInput('record_id') ? 'one' : 'none',
    record_status:
      id === 'supersede-record' || id === 'decisions-to-spec'
        ? 'accepted'
        : id === 'merge-records'
          ? 'candidate'
          : 'all',
    paths:
      id === 'code-to-record'
        ? 'required'
        : hasRequiredInput('path_or_diff')
          ? 'required'
          : hasInput('path') || hasInput('path_or_diff')
            ? 'optional'
            : 'none',
    notes: id === 'custom' ? 'required' : hasRequiredDetail ? 'required' : hasDetailInput ? 'optional' : 'none',
    scope: id === 'decisions-to-spec' ? 'accepted' : hasInput('status_scope') ? 'live' : 'none',
  };
}

function taskResultFor(request: RunTaskRequest): TaskResult {
  const contextLines = [
    request.context?.system_prompt ? `System prompt override: ${request.context.system_prompt}` : '',
    request.context?.instructions ? `Context note: ${request.context.instructions}` : '',
    ...(request.context?.paths ?? []).map((path) => `Path context: ${path}`),
    ...(request.context?.records ?? []).map((record) => `Record context: ${record}`),
    request.context?.text ? `Pasted context: ${request.context.text}` : '',
  ].filter(Boolean);
  const contextSummary = contextLines.length ? `\n\n${contextLines.join('\n')}` : '';
  switch (request.task_id) {
    case 'custom':
      return {
        kind: 'markdown_report.v1',
        title: 'Custom task result',
        summary: 'Mock custom task complete.',
        body_markdown: `## Result\n${request.context?.instructions ?? 'No custom instructions supplied.'}${contextSummary}`,
      };
    case 'find-conflicts':
      return {
        kind: 'conflict_set.v1',
        summary: 'One pair of accepted decisions overlaps.',
        findings: [
          {
            id: 'lazy-vs-ipc',
            title: 'Lazy frames and IPC snapshots overlap',
            left_record_id: '0193fb1f-a3a0-7000-8000-000000000003',
            right_record_id: '0193fb1f-a3a0-7000-8000-000000000004',
            relationship: 'overlap',
            conflict_explanation:
              'Both records describe how intermediate data flows between steps without saying which one wins.',
            suggested_actions: ['Replace one record with a combined decision.'],
          },
        ],
      };
    case 'code-to-record':
      return {
        kind: 'candidate_batch.v1',
        summary: `Found one candidate-worthy decision in the requested code.${contextSummary}`,
        candidates: [
          {
            id: 'missing-data-policy',
            title: 'Use explicit missing data policy',
            tags: ['#kind/sample', '#writer/agent'],
            body_markdown:
              '## Context\nThe analysis code filters rows with missing values.\n\n## Decision\nRecord an explicit missing data policy before downstream modeling.\n\n## Alternatives\nLeave the implicit filter undocumented.\n\n## Consequences\nReviewers can evaluate the sample definition before accepting it.',
            confidence: 'medium',
            warnings: [],
          },
        ],
      };
    case 'context-to-record':
    case 'merge-records':
      return {
        kind: 'candidate_draft.v1',
        title: 'Task drafted candidate',
        tags: ['#writer/agent'],
        body_markdown: `## Context\n${String(request.input.context ?? 'Task context.')}\n\n## Decision\nCreate a candidate from the supplied context.\n\n## Alternatives\nDo not record this decision.\n\n## Consequences\nThe candidate remains reviewable before acceptance.`,
        warnings: [],
      };
    case 'supersede-record':
      return {
        kind: 'supersession_proposal.v1',
        summary: 'Mock supersession proposal.',
        superseded_record_id: String(request.input.record_id ?? ''),
        superseding_title: 'Superseding candidate',
        superseding_body_markdown:
          '## Context\nA previous decision needs replacement.\n\n## Decision\nSupersede the older record.\n\n## Alternatives\nKeep both records active.\n\n## Consequences\nThe ledger has one current decision for this issue.',
        rationale: String(request.input.reason ?? 'No rationale supplied.'),
        suggested_actions: ['Write a candidate and review it.'],
      };
    case 'edit-record':
      return {
        kind: 'record_revision.v1',
        summary: 'Mock revision generated.',
        record_id: String(request.input.record_id ?? ''),
        body_markdown:
          '## Context\nRevision requested.\n\n## Decision\nUpdate the record wording.\n\n## Alternatives\nKeep the previous wording.\n\n## Consequences\nThe record is clearer for review.',
        change_explanation: String(request.input.requested_changes ?? 'No changes supplied.'),
        warnings: [],
      };
    default:
      return {
        kind: 'markdown_report.v1',
        title: 'Task result',
        body_markdown: 'Task completed.',
      };
  }
}

function createMockTaskRun(request: RunTaskRequest): TaskRun {
  const suffix = String(nextTaskRun).padStart(12, '0');
  nextTaskRun += 1;
  const id = `0193fb1f-a3a0-7000-8000-${suffix}`;
  const now = new Date().toISOString();
  const task = taskPackages.find((candidate) => candidate.manifest.id === request.task_id) ?? taskPackages[0];
  if (!task) {
    throw new Error('mock task catalog is empty');
  }
  const slowTaskMode = request.context?.instructions?.trim();
  const slowTaskShouldComplete = request.task_id === 'custom' && slowTaskMode === 'complete-slow';
  const isSlowTask = request.task_id === 'custom' && (slowTaskMode === 'slow' || slowTaskMode === 'complete-slow');
  const run: TaskRun = {
    id,
    task_id: request.task_id,
    task_source: task.source,
    task_version: task.manifest.version,
    status: isSlowTask ? 'running' : 'succeeded',
    created_at: now,
    updated_at: now,
    run_dir: `/tmp/dadr-mock/dadr/runs/${request.task_id}/${id}`,
  };
  taskRuns.push(run);
  taskLogs.set(id, [
    { timestamp: now, message: 'Starting local ACP agent: mock-agent' },
    { timestamp: now, message: 'ACP session started.' },
    { timestamp: now, message: 'Prompt sent; waiting for agent output.' },
    ...(run.status === 'succeeded'
      ? [
          { timestamp: now, message: 'agent: mock task response received' },
          { timestamp: now, message: 'Task succeeded.' },
        ]
      : [{ timestamp: now, message: 'agent: mock long-running task started' }]),
  ]);
  if (run.status === 'succeeded') {
    taskResults.set(id, taskResultFor(request));
  } else if (slowTaskShouldComplete) {
    taskRunPollsUntilSuccess.set(id, 2);
    taskResults.set(id, taskResultFor(request));
  }
  return run;
}

function advanceMockTaskRuns() {
  for (const [id, remaining] of taskRunPollsUntilSuccess) {
    const run = taskRuns.find((item) => item.id === id);
    if (!run || run.status !== 'running') {
      taskRunPollsUntilSuccess.delete(id);
      continue;
    }
    if (remaining > 0) {
      taskRunPollsUntilSuccess.set(id, remaining - 1);
      continue;
    }
    const now = new Date().toISOString();
    run.status = 'succeeded';
    run.updated_at = now;
    taskLogs.set(id, [
      ...(taskLogs.get(id) ?? []),
      { timestamp: now, message: 'agent: mock diagnosis response received' },
      { timestamp: now, message: 'Task succeeded.' },
    ]);
    taskRunPollsUntilSuccess.delete(id);
  }
}

export class MockApi implements DadrApi {
  async getSnapshot(): Promise<Snapshot> {
    const currentInitialized = readMockInitialized();
    return JSON.parse(
      JSON.stringify({
        ...fixture,
        project: {
          ...fixture.project,
          initialized: currentInitialized,
          status: currentInitialized
            ? 'Project discovered. Commands operate on the local decision records.'
            : uninitializedStatus,
        },
        records: mockRecords,
        counts: snapshotCounts(mockRecords),
      }),
    ) as Snapshot;
  }

  async initializeProject(): Promise<MutationResponse> {
    setMockInitialized(true);
    return {
      message: 'created 8 paths, reused 0 paths',
      changed_paths: [
        'dadr',
        'dadr/accepted',
        'dadr/candidates',
        'dadr/rejected',
        'dadr/templates',
        'dadr/tasks',
        'dadr/runs',
        'dadr/runs/.gitignore',
      ],
      snapshot: await this.getSnapshot(),
    };
  }

  async getRecordSource(id: string): Promise<RecordSource> {
    const record = mockRecords.find((item) => item.id === id) ?? mockRecords[0];
    if (!record) {
      throw new Error(`mock record ${id} not found`);
    }
    return {
      id: record.id,
      path: record.path,
      content:
        sources.get(record.id) ??
        `---\nid: ${record.id}\ntitle: ${record.title}\nstatus: ${record.status}\n---\n\n# ${record.title}\n\n${record.preview}\n`,
    };
  }

  async getRecordLifecycle(id: string): Promise<RecordLifecycle> {
    const record = mockRecords.find((item) => item.id === id) ?? mockRecords[0];
    if (!record) {
      throw new Error(`mock record ${id} not found`);
    }
    const predecessors = mockRecords.filter((candidate) =>
      supersessionEdges.get(candidate.id)?.has(record.id),
    );
    const successors = mockRecords.filter((candidate) =>
      supersessionEdges.get(record.id)?.has(candidate.id),
    );
    return JSON.parse(
      JSON.stringify({
        record,
        predecessors,
        successors,
        pending_supersedes: mockRecords.find((item) => item.id === pendingReplacements.get(record.id)) ?? null,
        diagnostics: [],
      }),
    ) as RecordLifecycle;
  }

  async saveRecordSource(id: string, content: string): Promise<MutationResponse> {
    const record = mockRecords.find((item) => item.id === id) ?? mockRecords[0];
    if (!record) {
      throw new Error(`mock record ${id} not found`);
    }
    sources.set(record.id, content);
    return {
      message: `saved ${record.id}`,
      changed_paths: [record.path],
      focus: {
        selected_id: record.id,
        status_filter: record.status,
      },
      snapshot: await this.getSnapshot(),
    };
  }

  async runValidation(target: ValidationTarget): Promise<MutationResponse> {
    return {
      message: `${target} validation finished`,
      changed_paths: [],
      snapshot: await this.getSnapshot(),
    };
  }

  async getCandidateTemplate(): Promise<CandidateTemplate> {
    return {
      name: 'candidate',
      source: 'built_in',
      path: null,
      body: '## Context\n\nDescribe the decision context.\n\n## Decision\n\nState the proposed decision.\n\n## Alternatives\n\nList rejected options.\n\n## Consequences\n\nDescribe tradeoffs.\n',
    };
  }

  async createCandidate(request: CreateCandidateRequest): Promise<MutationResponse> {
    const record = newCandidateRecord(request.title, request.tags, request.body_markdown);
    if (request.supersedes) pendingReplacements.set(record.id, request.supersedes);
    return {
      message: `created candidate ${record.id}`,
      changed_paths: [record.path],
      focus: {
        selected_id: record.id,
        status_filter: 'candidate',
      },
      snapshot: await this.getSnapshot(),
    };
  }

  async applyRecordRevision(request: ApplyRecordRevisionRequest): Promise<MutationResponse> {
    const record = mockRecords.find((item) => item.id === request.record_id);
    if (!record) {
      throw new Error(`mock record ${request.record_id} not found`);
    }
    record.title = request.title;
    record.tags = request.tags;
    record.preview = request.body_markdown.trim() || record.preview;
    sources.set(
      record.id,
      `---\nid: ${record.id}\ntitle: ${request.title}\nstatus: ${record.status}\ntags: [${request.tags
        .map((tag) => `"${tag}"`)
        .join(', ')}]\n---\n\n# ${request.title}\n\n${request.body_markdown}\n`,
    );
    return {
      message: `applied revision ${record.id}`,
      changed_paths: [record.path],
      focus: {
        selected_id: record.id,
        status_filter: record.status,
      },
      snapshot: await this.getSnapshot(),
    };
  }

  async supersedeRecords(request: SupersedeRecordsRequest): Promise<MutationResponse> {
    const oldRecord = mockRecords.find((record) => record.id === request.from);
    if (!oldRecord) {
      throw new Error(`mock record ${request.from} not found`);
    }
    const successors = request.to.map((recordId) => {
      const successor = mockRecords.find((record) => record.id === recordId);
      if (!successor) {
        throw new Error(`mock record ${recordId} not found`);
      }
      return successor;
    });
    oldRecord.superseded = true;
    const edges = supersessionEdges.get(oldRecord.id) ?? new Set<string>();
    for (const successor of successors) {
      edges.add(successor.id);
    }
    supersessionEdges.set(oldRecord.id, edges);
    return {
      message: 'record superseded',
      changed_paths: [oldRecord.path, ...successors.map((record) => record.path)],
      focus: {
        selected_id: oldRecord.id,
        status_filter: 'superseded',
      },
      snapshot: await this.getSnapshot(),
    };
  }

  async draftCandidate(request: DraftCandidateRequest): Promise<MutationResponse> {
    const record = newCandidateRecord(
      'Agent draft candidate',
      ['#dadr/agent/draft'],
      `## Context\n${request.prompt}\n\n## Decision\nAgent draft recommends creating a candidate for review.\n\n## Alternatives\nContinue without an agent-assisted draft.\n\n## Consequences\nHumans still review the candidate before acceptance.\n`,
    );
    return {
      message: `drafted candidate ${record.id}`,
      changed_paths: [record.path],
      focus: {
        selected_id: record.id,
        status_filter: 'candidate',
      },
      snapshot: await this.getSnapshot(),
    };
  }

  async acceptCandidate(id: string): Promise<MutationResponse> {
    return this.candidateAction(id, 'accepted');
  }

  async rejectCandidate(id: string): Promise<MutationResponse> {
    return this.candidateAction(id, 'rejected');
  }

  async listTasks(): Promise<TaskListing> {
    return {
      tasks: JSON.parse(JSON.stringify(taskPackages)) as TaskPackage[],
      diagnostics: [],
    };
  }

  async getTask(id: string): Promise<TaskPackage> {
    const task = taskPackages.find((candidate) => candidate.manifest.id === id);
    if (!task) {
      throw new Error(`mock task ${id} not found`);
    }
    return JSON.parse(JSON.stringify(task)) as TaskPackage;
  }

  async listTaskRuns(): Promise<TaskRun[]> {
    advanceMockTaskRuns();
    return JSON.parse(JSON.stringify(taskRuns)) as TaskRun[];
  }

  async getTaskResult(id: string): Promise<TaskResult> {
    const result = taskResults.get(id);
    if (!result) {
      throw new Error(`mock task run ${id} has no result`);
    }
    return JSON.parse(JSON.stringify(result)) as TaskResult;
  }

  async getTaskLog(id: string): Promise<TaskRunLogEntry[]> {
    return JSON.parse(JSON.stringify(taskLogs.get(id) ?? [])) as TaskRunLogEntry[];
  }

  async runTask(request: RunTaskRequest): Promise<TaskRun> {
    return JSON.parse(JSON.stringify(createMockTaskRun(request))) as TaskRun;
  }

  async cancelTaskRun(id: string): Promise<TaskRun> {
    const run = taskRuns.find((item) => item.id === id);
    if (!run) {
      throw new Error(`mock task run ${id} not found`);
    }
    if (run.status === 'created' || run.status === 'running') {
      const now = new Date().toISOString();
      run.status = 'cancelled';
      run.updated_at = now;
      taskRunPollsUntilSuccess.delete(id);
      taskLogs.set(id, [
        ...(taskLogs.get(id) ?? []),
        { timestamp: now, message: 'Cancellation requested.' },
        { timestamp: now, message: 'Task cancelled.' },
      ]);
    }
    return JSON.parse(JSON.stringify(run)) as TaskRun;
  }

  async clearTaskHistory(): Promise<MutationResponse> {
    taskRuns.splice(0, taskRuns.length);
    taskResults.clear();
    taskLogs.clear();
    taskRunPollsUntilSuccess.clear();
    return {
      message: 'Task history cleared.',
      changed_paths: ['/tmp/dadr-mock/dadr/runs'],
      snapshot: await this.getSnapshot(),
    };
  }

  private async candidateAction(
    id: string,
    status: 'accepted' | 'rejected',
  ): Promise<MutationResponse> {
    const record = mockRecords.find((item) => item.id === id);
    if (!record) {
      throw new Error(`mock record ${id} not found`);
    }
    if (status === 'accepted' && pendingReplacements.has(id)) {
      const old = mockRecords.find((item) => item.id === pendingReplacements.get(id));
      if (!old || old.status !== 'accepted' || old.superseded) throw new Error('Previous decision is no longer current');
      old.superseded = true;
      supersessionEdges.set(old.id, new Set([id]));
      pendingReplacements.delete(id);
    }
    record.status = status;
    record.path =
      status === 'accepted'
        ? record.path.replace('dadr/candidates/', 'dadr/accepted/')
        : record.path.replace('dadr/candidates/', 'dadr/rejected/');
    return {
      message: `${status === 'accepted' ? 'accepted' : 'rejected'} candidate ${record.id}`,
      changed_paths: [record.path],
      focus: {
        selected_id: record.id,
        status_filter: status,
      },
      snapshot: await this.getSnapshot(),
    };
  }
}
