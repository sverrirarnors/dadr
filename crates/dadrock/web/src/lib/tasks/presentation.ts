export type TaskIntentId = 'all' | 'create' | 'improve' | 'find' | 'lifecycle' | 'report';

export interface TaskPresentationInput {
  manifest: {
    id: string;
    inputs?: TaskPresentationField[];
    requirements?: Partial<TaskManifestRequirements>;
    output: {
      kind: string;
    };
  };
}

export interface TaskPresentationField {
  name: string;
  label: string;
  type: string;
  required?: boolean;
  options?: string[];
}

export type RecordRequirement = 'none' | 'one' | 'many';
export type RecordStatusRequirement = 'all' | 'candidate' | 'accepted';
export type PathRequirement = 'none' | 'optional' | 'required';
export type NoteRequirement = 'none' | 'optional' | 'required';
export type ScopeRequirement = 'none' | 'live' | 'candidates' | 'accepted' | 'all';

export interface TaskManifestRequirements {
  records: RecordRequirement;
  record_status: RecordStatusRequirement;
  paths: PathRequirement;
  notes: NoteRequirement;
  scope: ScopeRequirement;
}

export interface TaskRequirements {
  records: RecordRequirement;
  recordStatus: RecordStatusRequirement;
  paths: PathRequirement;
  notes: NoteRequirement;
  scope: ScopeRequirement;
}

export interface TaskRequirementState {
  recordCount: number;
  pathCount: number;
  hasNotes: boolean;
}

export interface TaskIntent {
  id: TaskIntentId;
  label: string;
  description: string;
}

export const taskIntents: TaskIntent[] = [
  {
    id: 'all',
    label: 'All tasks',
    description: 'Every available workflow.',
  },
  {
    id: 'create',
    label: 'Create records',
    description: 'Turn context, code, or discussion into candidate DADRs.',
  },
  {
    id: 'improve',
    label: 'Improve records',
    description: 'Revise existing candidates, tags, and record wording.',
  },
  {
    id: 'find',
    label: 'Find problems',
    description: 'Audit the ledger for conflicts, stale records, and quality issues.',
  },
  {
    id: 'lifecycle',
    label: 'Lifecycle changes',
    description: 'Propose supersession or replacement direction.',
  },
  {
    id: 'report',
    label: 'Reports',
    description: 'Produce review briefs and read-only summaries.',
  },
];

export function taskIntentFor(task: TaskPresentationInput): TaskIntentId {
  const id = task.manifest.id;
  const kind = task.manifest.output.kind;

  if (id === 'supersede-record' || kind === 'supersession_proposal.v1') return 'lifecycle';
  if (kind === 'markdown_report.v1') return 'report';
  if (kind === 'record_revision.v1') return 'improve';
  if (kind === 'diagnosis_report.v1' || kind === 'conflict_set.v1') return 'find';
  if (kind === 'candidate_draft.v1' || kind === 'candidate_batch.v1') return 'create';
  return 'report';
}

export function outputKindLabel(kind: string): string {
  switch (kind) {
    case 'candidate_draft.v1':
      return 'Candidate';
    case 'candidate_batch.v1':
      return 'Candidate batch';
    case 'record_revision.v1':
      return 'Revision';
    case 'diagnosis_report.v1':
      return 'Report';
    case 'conflict_set.v1':
      return 'Report';
    case 'supersession_proposal.v1':
      return 'Supersession';
    case 'markdown_report.v1':
      return 'Report';
    default:
      return 'Report';
  }
}

export function resultFamilyForKind(kind: string): string {
  return outputKindLabel(kind);
}

export function describeTaskRequirements(task: TaskPresentationInput): TaskRequirements {
  const explicit = task.manifest.requirements;
  if (explicit) {
    return {
      records: explicit.records ?? 'none',
      recordStatus: explicit.record_status ?? defaultRecordStatusForTask(task.manifest.id),
      paths: explicit.paths ?? 'none',
      notes: explicit.notes ?? 'none',
      scope: explicit.scope ?? 'none',
    };
  }

  const inputs = task.manifest.inputs ?? [];
  const recordInput = inputs.find((input) => input.name === 'record_id' || input.name === 'record_ids');
  const pathInputs = inputs.filter((input) => input.name === 'path' || input.name === 'path_or_diff');
  const detailInputs = inputs.filter(isDetailInput);
  const primaryDetail = detailInputs.find((input) => input.required) ?? detailInputs[0];

  return {
    records: recordInput?.name === 'record_ids' ? 'many' : recordInput ? 'one' : 'none',
    recordStatus: defaultRecordStatusForTask(task.manifest.id),
    paths: pathInputs.some((input) => input.required) ? 'required' : pathInputs.length ? 'optional' : 'none',
    notes: primaryDetail?.required ? 'required' : primaryDetail ? 'optional' : 'none',
    scope: inputs.some((input) => input.name === 'status_scope') ? 'live' : 'none',
  };
}

export function taskRequirementTabLabel(
  task: TaskPresentationInput,
  tab: 'details' | 'files' | 'records',
  state: TaskRequirementState,
): string {
  const requirements = describeTaskRequirements(task);
  if (tab === 'details') return requirementStatusLabel(requirements.notes, state.hasNotes ? 1 : 0);
  if (tab === 'files') return requirementStatusLabel(requirements.paths, state.pathCount);
  if (requirements.records === 'none') return '0';
  return state.recordCount ? String(state.recordCount) : 'Required';
}

export function defaultRecordStatusForTask(taskId: string): RecordStatusRequirement {
  if (taskId === 'merge-records') {
    return 'candidate';
  }
  if (taskId === 'supersede-record') return 'accepted';
  return 'all';
}

function requirementStatusLabel(requirement: PathRequirement | NoteRequirement, count: number): string {
  if (requirement === 'required') return count ? String(count) : 'Required';
  if (requirement === 'optional') return String(count);
  return '0';
}

function isDetailInput(input: TaskPresentationField): boolean {
  return [
    'context',
    'discussion',
    'focus',
    'requested_changes',
    'reason',
    'merge_goal',
    'review_focus',
    'tag_goal',
    'decision_hint',
  ].includes(input.name);
}
