import type { Diagnostic, RecordSummary, Snapshot, TaskResult, TaskRun } from '$lib/api';
import { diagnosticLocation } from '$lib/diagnostics';
import { recordForDiagnostic } from '$lib/diagnostics/actions';
import { supersededIds } from '$lib/records/supersedes';
import { objectArrayField, stringArrayField, stringField } from '$lib/tasks/result-fields';

export type InboxGroupId =
  | 'running'
  | 'candidates'
  | 'proposals'
  | 'revisions'
  | 'conflicts'
  | 'reports'
  | 'issues';

export type IssueCategory = 'Project' | 'Records' | 'Links' | 'Tags';

export interface CandidateProposal {
  id?: string;
  title: string;
  tags: string[];
  body_markdown: string;
  /** Records the saved candidate supersedes: one, or several for a merge. */
  supersedes?: string[];
  /** Why the decision changed; Dadrock writes it into `## Supersedes`. */
  supersedes_reason?: string;
  confidence?: string;
  warnings: string[];
}

export interface ConflictFinding {
  id: string;
  title: string;
  left_record_id: string;
  right_record_id: string;
  relationship: string;
  conflict_explanation: string;
  why_might_not_be_conflict?: string;
  suggested_actions: string[];
}

interface InboxItemBase {
  id: string;
  title: string;
  subtitle: string;
}

export interface RunningItem extends InboxItemBase {
  kind: 'running';
  run: TaskRun;
}

export interface CandidateItem extends InboxItemBase {
  kind: 'candidate';
  record: RecordSummary;
}

/**
 * Several candidates that each supersede the same current decision: the
 * halves of a split, or competing alternatives. The reviewer decides which to
 * accept together.
 */
export interface SplitItem extends InboxItemBase {
  kind: 'split';
  /** The decisions the candidates supersede, in order of first mention. */
  predecessors: RecordSummary[];
  /** The candidates, newest first. */
  records: RecordSummary[];
}

export interface ProposalItem extends InboxItemBase {
  kind: 'proposal';
  runId: string;
  taskId: string;
  proposal: CandidateProposal;
  summary: string;
  /** The predecessors found in the project, in the proposal's order. */
  supersededRecords?: RecordSummary[];
}

export interface RevisionItem extends InboxItemBase {
  kind: 'revision';
  runId: string;
  recordId: string;
  record?: RecordSummary;
  revisedTitle: string;
  tags: string[];
  body_markdown: string;
  change_explanation: string;
  summary: string;
}

export interface ConflictItem extends InboxItemBase {
  kind: 'conflict';
  runId: string;
  finding: ConflictFinding;
  left?: RecordSummary;
  right?: RecordSummary;
}

export interface ReportItem extends InboxItemBase {
  kind: 'report';
  runId: string;
  taskId: string;
  body_markdown: string;
  summary: string;
}

export interface IssueItem extends InboxItemBase {
  kind: 'issue';
  category: IssueCategory;
  diagnostic: Diagnostic;
  record?: RecordSummary;
}

export type InboxItem =
  | RunningItem
  | CandidateItem
  | SplitItem
  | ProposalItem
  | RevisionItem
  | ConflictItem
  | ReportItem
  | IssueItem;

export type InboxItemKind = InboxItem['kind'];

export interface InboxGroup {
  id: InboxGroupId;
  label: string;
  items: InboxItem[];
}

export interface InboxSource {
  snapshot: Snapshot | undefined;
  runs: TaskRun[];
  results: Record<string, TaskResult | undefined>;
  resolved: Iterable<string>;
}

const GROUP_LABELS: Record<InboxGroupId, string> = {
  running: 'Working',
  candidates: 'Candidates',
  proposals: 'Agent proposals',
  revisions: 'Proposed revisions',
  conflicts: 'Conflicts',
  reports: 'Reports',
  issues: 'Issues',
};

const GROUP_ORDER: InboxGroupId[] = [
  'running',
  'candidates',
  'proposals',
  'revisions',
  'conflicts',
  'reports',
  'issues',
];

/** Project diagnostics that only restate the inbox itself. */
const REDUNDANT_DIAGNOSTIC_CODES = new Set(['candidate_backlog']);

/** A record preview as one line of prose: section headings and markers removed. */
export function previewText(preview: string): string {
  return preview
    .replace(/#{1,6}\s*(Context|Decision|Alternatives|Consequences|Open questions)\b:?\s*/gi, ' ')
    .replace(/#{1,6}\s*/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
}

export function candidateItemId(recordId: string): string {
  return `candidate:${recordId}`;
}

/** One item per set of candidates superseding the same decisions. */
export function splitItemId(predecessorIds: string[]): string {
  return `split:${[...predecessorIds].sort().join(',')}`;
}

/** The inbox item that shows a candidate: its own, or the split it belongs to. */
export function itemForCandidate(items: InboxItem[], recordId: string): InboxItem | undefined {
  const id = candidateItemId(recordId);
  return (
    items.find((item) => item.id === id) ??
    items.find((item) => item.kind === 'split' && item.records.some((record) => record.id === recordId))
  );
}

export function runningItemId(runId: string): string {
  return `running:${runId}`;
}

export function proposalItemId(runId: string, proposalId: string): string {
  return `proposal:${runId}:${proposalId}`;
}

export function revisionItemId(runId: string): string {
  return `revision:${runId}`;
}

export function conflictItemId(runId: string, findingId: string): string {
  return `conflict:${runId}:${findingId}`;
}

export function reportItemId(runId: string): string {
  return `report:${runId}`;
}

export function issueItemId(category: IssueCategory, diagnostic: Diagnostic): string {
  return [
    'issue',
    category,
    diagnostic.code,
    diagnostic.path ?? '',
    diagnostic.record_id ?? '',
    diagnostic.field ?? '',
    diagnostic.line ?? '',
    diagnostic.message,
  ].join(':');
}

/** All run-derived item ids for a run, so a cleared run can be forgotten. */
export function isRunItemId(id: string, runId: string): boolean {
  return id.includes(`:${runId}`);
}

export function buildInboxGroups(source: InboxSource): InboxGroup[] {
  const resolved = new Set(source.resolved);
  const records = source.snapshot?.records ?? [];
  const byId = new Map(records.map((record) => [record.id, record]));
  const items: Record<InboxGroupId, InboxItem[]> = {
    running: [],
    candidates: [],
    proposals: [],
    revisions: [],
    conflicts: [],
    reports: [],
    issues: [],
  };

  const runs = [...source.runs].sort((left, right) => right.created_at.localeCompare(left.created_at));
  for (const run of runs) {
    if (run.status === 'created' || run.status === 'running') {
      items.running.push({
        kind: 'running',
        id: runningItemId(run.id),
        title: runningTitle(run.task_id),
        subtitle: 'The agent is working. Results will appear here.',
        run,
      });
      continue;
    }
    if (run.status !== 'succeeded') continue;
    const result = source.results[run.id];
    if (!result) continue;
    for (const item of itemsForResult(run, result, byId)) {
      if (resolved.has(item.id)) continue;
      items[groupForItem(item)].push(item);
    }
  }

  const candidates = records
    .filter((record) => record.status === 'candidate')
    .sort((left, right) => right.timestamp_ms - left.timestamp_ms);
  const splits = groupSuccessorCandidates(candidates, byId);
  const grouped = new Set(splits.flatMap((split) => split.records.map((record) => record.id)));
  const emitted = new Set<string>();
  for (const record of candidates) {
    if (grouped.has(record.id)) {
      const split = splits.find((candidate) => candidate.records.includes(record));
      if (split && !emitted.has(split.id)) {
        emitted.add(split.id);
        items.candidates.push(split);
      }
      continue;
    }
    items.candidates.push({
      kind: 'candidate',
      id: candidateItemId(record.id),
      title: record.title,
      subtitle: candidateSubtitle(record, byId),
      record,
    });
  }

  if (source.snapshot) {
    const panels: Array<[IssueCategory, Diagnostic[]]> = [
      ['Project', source.snapshot.project_validation.diagnostics],
      ['Records', source.snapshot.record_validation.diagnostics],
      ['Links', source.snapshot.link_health.diagnostics],
      ['Tags', source.snapshot.settings.diagnostics],
    ];
    const issues: IssueItem[] = [];
    for (const [category, diagnostics] of panels) {
      for (const diagnostic of diagnostics) {
        if (REDUNDANT_DIAGNOSTIC_CODES.has(diagnostic.code)) continue;
        const id = issueItemId(category, diagnostic);
        if (resolved.has(id)) continue;
        const issue: IssueItem = {
          kind: 'issue',
          id,
          title: diagnostic.message,
          subtitle: [category, diagnosticLocation(diagnostic)].filter(Boolean).join(' · '),
          category,
          diagnostic,
        };
        const record = recordForDiagnostic(diagnostic, records);
        if (record) issue.record = record;
        issues.push(issue);
      }
    }
    issues.sort((left, right) => severityRank(left) - severityRank(right));
    items.issues.push(...issues);
  }

  return GROUP_ORDER.filter((id) => items[id].length > 0).map((id) => ({
    id,
    label: GROUP_LABELS[id],
    items: items[id],
  }));
}

export function flattenInboxGroups(groups: InboxGroup[]): InboxItem[] {
  return groups.flatMap((group) => group.items);
}

export function countActionable(groups: InboxGroup[]): number {
  return groups
    .filter((group) => group.id !== 'running')
    .reduce((total, group) => total + group.items.length, 0);
}

/**
 * Candidates that supersede the same current decision, grouped: each group
 * is connected through shared predecessors and has at least two candidates.
 * Stale candidates, which name a decision superseded since, stay on their own
 * so they can be retargeted.
 */
export function groupSuccessorCandidates(
  candidates: RecordSummary[],
  byId: Map<string, RecordSummary>,
): SplitItem[] {
  const named = new Map<string, RecordSummary[]>();
  for (const candidate of candidates) {
    const predecessors = supersededIds(candidate.preview).map((id) => byId.get(id.toLowerCase()));
    if (!predecessors.length || predecessors.some((record) => !record || !isCurrent(record))) continue;
    named.set(candidate.id, predecessors as RecordSummary[]);
  }

  // Union candidates that share a predecessor.
  const parent = new Map<string, string>();
  const find = (id: string): string => {
    const up = parent.get(id) ?? id;
    if (up === id) return id;
    const root = find(up);
    parent.set(id, root);
    return root;
  };
  const owner = new Map<string, string>();
  for (const [candidateId, predecessors] of named) {
    for (const predecessor of predecessors) {
      const first = owner.get(predecessor.id);
      if (first) parent.set(find(candidateId), find(first));
      else owner.set(predecessor.id, candidateId);
    }
  }

  const components = new Map<string, RecordSummary[]>();
  for (const candidate of candidates) {
    if (!named.has(candidate.id)) continue;
    const root = find(candidate.id);
    components.set(root, [...(components.get(root) ?? []), candidate]);
  }

  return [...components.values()]
    .filter((group) => group.length >= 2)
    .map((group) => {
      const predecessors: RecordSummary[] = [];
      for (const candidate of group) {
        for (const predecessor of named.get(candidate.id) ?? []) {
          if (!predecessors.includes(predecessor)) predecessors.push(predecessor);
        }
      }
      return {
        kind: 'split' as const,
        id: splitItemId(predecessors.map((predecessor) => predecessor.id)),
        title: `Split ${quotedList(predecessors.map((predecessor) => predecessor.title))} into ${group.length} decisions`,
        subtitle: group.map((candidate) => candidate.title).join(' · '),
        predecessors,
        records: group,
      };
    });
}

function isCurrent(record: RecordSummary): boolean {
  return record.status === 'accepted' && !record.superseded;
}

function quotedList(titles: string[]): string {
  const quoted = titles.map((title) => `“${title}”`);
  if (quoted.length <= 1) return quoted.join('');
  return `${quoted.slice(0, -1).join(', ')} and ${quoted.at(-1)}`;
}

/** A successor candidate says what it supersedes; any other shows its first words. */
function candidateSubtitle(record: RecordSummary, byId: Map<string, RecordSummary>): string {
  const predecessors = supersededIds(record.preview)
    .map((id) => byId.get(id.toLowerCase()))
    .filter((predecessor): predecessor is RecordSummary => Boolean(predecessor));
  if (!predecessors.length) return previewText(record.preview);
  const stale = predecessors.some((predecessor) => !isCurrent(predecessor));
  return `Supersedes ${quotedList(predecessors.map((predecessor) => predecessor.title))}${
    stale ? ' (superseded since)' : ''
  }`;
}

function groupForItem(item: InboxItem): InboxGroupId {
  switch (item.kind) {
    case 'running':
      return 'running';
    case 'candidate':
    case 'split':
      return 'candidates';
    case 'proposal':
      return 'proposals';
    case 'revision':
      return 'revisions';
    case 'conflict':
      return 'conflicts';
    case 'report':
      return 'reports';
    case 'issue':
      return 'issues';
  }
}

function severityRank(item: IssueItem): number {
  if (item.diagnostic.severity === 'error') return 0;
  if (item.diagnostic.severity === 'warning') return 1;
  return 2;
}

function runningTitle(taskId: string): string {
  const labels: Record<string, string> = {
    'code-to-record': 'Looking for decisions in your code',
    'context-to-record': 'Drafting a decision',
    'edit-record': 'Revising a record',
    'supersede-record': 'Drafting a successor',
    'merge-records': 'Merging candidates',
    'find-conflicts': 'Checking for conflicting decisions',
    'decisions-to-spec': 'Writing a spec from accepted decisions',
    custom: 'Running your request',
  };
  return labels[taskId] ?? `Running ${taskId}`;
}

function itemsForResult(
  run: TaskRun,
  result: TaskResult,
  byId: Map<string, RecordSummary>,
): InboxItem[] {
  const kind = stringField(result, 'kind');
  switch (kind) {
    case 'candidate_draft.v1': {
      const proposal = proposalFromDraft(result);
      if (!proposal) return [];
      return [
        {
          kind: 'proposal',
          id: proposalItemId(run.id, 'draft'),
          title: proposal.title,
          subtitle: 'Drafted by the agent. Save it to review it as a candidate.',
          runId: run.id,
          taskId: run.task_id,
          proposal,
          summary: '',
        },
      ];
    }
    case 'candidate_batch.v1': {
      const summary = stringField(result, 'summary');
      return candidateArray(result).map((proposal, index) => ({
        kind: 'proposal' as const,
        id: proposalItemId(run.id, proposal.id ?? String(index)),
        title: proposal.title,
        subtitle: proposal.confidence
          ? `Found in your code · confidence ${proposal.confidence}`
          : 'Found in your code',
        runId: run.id,
        taskId: run.task_id,
        proposal,
        summary,
      }));
    }
    case 'supersession_proposal.v1': {
      const body = stringField(result, 'superseding_body_markdown');
      if (!body.trim()) return [];
      const supersedes = stringArrayField(result, 'superseded_record_ids');
      // Results saved before merges were supported name one record.
      const legacy = stringField(result, 'superseded_record_id');
      if (!supersedes.length && legacy) supersedes.push(legacy);
      const supersededRecords = supersedes
        .map((id) => byId.get(id))
        .filter((record): record is RecordSummary => Boolean(record));
      const rationale = stringField(result, 'rationale');
      const proposal: CandidateProposal = {
        title: stringField(result, 'superseding_title') || 'Successor decision',
        tags: stringArrayField(result, 'tags'),
        body_markdown: body,
        warnings: [],
      };
      if (supersedes.length) proposal.supersedes = supersedes;
      // Dadrock writes `## Supersedes` with the rationale as the reason, unless
      // an older result already wrote the section into the body.
      if (rationale && !/^## Supersedes[ \t]*$/m.test(body)) proposal.supersedes_reason = rationale;
      const item: ProposalItem = {
        kind: 'proposal',
        id: proposalItemId(run.id, 'supersession'),
        title: proposal.title,
        subtitle: supersededRecords.length
          ? `Supersedes ${supersededRecords.map((record) => `“${record.title}”`).join(', ')}`
          : 'Successor drafted by the agent',
        runId: run.id,
        taskId: run.task_id,
        proposal,
        summary: [stringField(result, 'summary'), stringField(result, 'rationale')]
          .filter(Boolean)
          .join('\n\n'),
      };
      if (supersededRecords.length) item.supersededRecords = supersededRecords;
      return [item];
    }
    case 'record_revision.v1': {
      const recordId = stringField(result, 'record_id');
      const record = byId.get(recordId);
      if (!recordId || !record) return [];
      const revisedTitle = stringField(result, 'title') || record.title;
      const tags = stringArrayField(result, 'tags');
      return [
        {
          kind: 'revision',
          id: revisionItemId(run.id),
          title: revisedTitle,
          subtitle: `Revision of “${record.title}”`,
          runId: run.id,
          recordId,
          record,
          revisedTitle,
          tags: tags.length ? tags : record.tags,
          body_markdown: stringField(result, 'body_markdown'),
          change_explanation: stringField(result, 'change_explanation'),
          summary: stringField(result, 'summary'),
        },
      ];
    }
    case 'conflict_set.v1': {
      return objectArrayField(result, 'findings').flatMap((raw, index) => {
        const finding = conflictFinding(raw, index);
        const left = byId.get(finding.left_record_id);
        const right = byId.get(finding.right_record_id);
        const item: ConflictItem = {
          kind: 'conflict',
          id: conflictItemId(run.id, finding.id),
          title: finding.title,
          subtitle: [left?.title, right?.title].filter(Boolean).join(' ↔ ') || finding.relationship,
          runId: run.id,
          finding,
        };
        if (left) item.left = left;
        if (right) item.right = right;
        return [item];
      });
    }
    case 'diagnosis_report.v1':
    case 'markdown_report.v1': {
      const body = reportBody(result);
      if (!body.trim()) return [];
      return [
        {
          kind: 'report',
          id: reportItemId(run.id),
          title: stringField(result, 'title') || reportTitle(run.task_id),
          subtitle: 'Report from the agent',
          runId: run.id,
          taskId: run.task_id,
          body_markdown: body,
          summary: typeof result.summary === 'string' ? result.summary : '',
        },
      ];
    }
    default:
      return [];
  }
}

function reportTitle(taskId: string): string {
  return taskId === 'custom' ? 'Agent report' : `Report from ${taskId}`;
}

function reportBody(result: TaskResult): string {
  const body = stringField(result, 'body_markdown');
  if (body.trim()) return body;
  const summary = typeof result.summary === 'string' ? result.summary : '';
  const issues = objectArrayField(result, 'issues').map((issue) => {
    const title = typeof issue.title === 'string' ? issue.title : 'Issue';
    const severity = typeof issue.severity === 'string' ? ` (${issue.severity})` : '';
    const detail = typeof issue.detail === 'string' ? `\n  ${issue.detail}` : '';
    return `- **${title}**${severity}${detail}`;
  });
  return [summary, issues.join('\n')].filter(Boolean).join('\n\n');
}

function proposalFromDraft(result: TaskResult): CandidateProposal | null {
  const body = stringField(result, 'body_markdown');
  if (!body.trim()) return null;
  return {
    title: stringField(result, 'title') || 'Drafted decision',
    tags: stringArrayField(result, 'tags'),
    body_markdown: body,
    warnings: stringArrayField(result, 'warnings'),
  };
}

export function candidateArray(result: TaskResult): CandidateProposal[] {
  return objectArrayField(result, 'candidates').flatMap((candidate) => {
    const title = typeof candidate.title === 'string' ? candidate.title : '';
    const body = typeof candidate.body_markdown === 'string' ? candidate.body_markdown : '';
    if (!title || !body) return [];
    const proposal: CandidateProposal = {
      title,
      body_markdown: body,
      tags: Array.isArray(candidate.tags)
        ? candidate.tags.filter((tag): tag is string => typeof tag === 'string')
        : [],
      warnings: Array.isArray(candidate.warnings)
        ? candidate.warnings.filter((warning): warning is string => typeof warning === 'string')
        : [],
    };
    if (typeof candidate.confidence === 'string') proposal.confidence = candidate.confidence;
    if (typeof candidate.id === 'string') proposal.id = candidate.id;
    return [proposal];
  });
}

function conflictFinding(raw: Record<string, unknown>, index: number): ConflictFinding {
  const text = (key: string) => (typeof raw[key] === 'string' ? (raw[key] as string) : '');
  return {
    id: text('id') || String(index),
    title: text('title') || 'Possible conflict',
    left_record_id: text('left_record_id'),
    right_record_id: text('right_record_id'),
    relationship: text('relationship'),
    conflict_explanation: text('conflict_explanation'),
    ...(text('why_might_not_be_conflict') ? { why_might_not_be_conflict: text('why_might_not_be_conflict') } : {}),
    suggested_actions: Array.isArray(raw.suggested_actions)
      ? raw.suggested_actions.filter((action): action is string => typeof action === 'string')
      : [],
  };
}
