import type { RecordSummary } from '$lib/api';
import type { ComposerPrefill } from '$lib/stores/composer.svelte';
import type { DisplayRecordStatus } from './filters';

export function detailActionRecord(
  lifecycleRecord: RecordSummary | undefined,
  snapshotRecords: RecordSummary[] | undefined,
): RecordSummary | undefined {
  if (!lifecycleRecord) return lifecycleRecord;
  const snapshotRecord = snapshotRecords?.find((record) => record.id === lifecycleRecord.id);
  if (!snapshotRecord) return lifecycleRecord;
  if (lifecycleRecord.status === 'candidate' && snapshotRecord.status !== 'candidate') {
    return snapshotRecord;
  }
  return lifecycleRecord;
}

/** Where the detail panel is shown; the surface decides which verbs apply. */
export type DetailSurface = 'inbox' | 'ledger';

export type DetailAction = 'edit' | 'revise' | 'supersede' | 'reject' | 'accept';

/**
 * How Edit and Revise with agent change a record: candidates change in place;
 * a decision in force changes by drafting a successor on the Draft page.
 */
export type EditMode = 'in-place' | 'successor';

export interface DetailActionGroups {
  /** Verbs that change the text: Edit, Revise with agent. */
  primary: DetailAction[];
  /** Verbs that change the lifecycle: Reject, Accept, Supersede. */
  lifecycle: DetailAction[];
  /** How the primary verbs work, or null when the text cannot change. */
  editMode: EditMode | null;
  /** Accepted and rejected records keep their text but their tags can change in place. */
  editTags: boolean;
}

/**
 * One visible button per verb. Candidates are reviewed in the Inbox and edited
 * in place. A current decision is edited by drafting a successor, and the
 * Ledger offers to supersede it. Archived records (superseded or rejected)
 * never change; only their tags can.
 */
export function detailActions(input: {
  surface: DetailSurface;
  status: DisplayRecordStatus;
  agentAvailable: boolean;
}): DetailActionGroups {
  const { surface, status, agentAvailable } = input;
  const withAgent: DetailAction[] = agentAvailable ? ['edit', 'revise'] : ['edit'];

  if (status === 'candidate') {
    if (surface === 'inbox') {
      return { primary: withAgent, lifecycle: ['reject', 'accept'], editMode: 'in-place', editTags: false };
    }
    return { primary: ['edit'], lifecycle: [], editMode: 'in-place', editTags: false };
  }
  if (status === 'accepted') {
    return {
      primary: withAgent,
      lifecycle: surface === 'ledger' ? ['supersede'] : [],
      editMode: 'successor',
      editTags: true,
    };
  }
  return { primary: [], lifecycle: [], editMode: null, editTags: true };
}

/**
 * The Draft page a successor verb opens for a current decision: Edit writes
 * the successor by hand, Revise with agent asks the agent, and Supersede
 * opens the page in its usual view.
 */
export function successorDraft(
  action: 'edit' | 'revise' | 'supersede',
  recordId: string,
): ComposerPrefill {
  const prefill: ComposerPrefill = { mode: 'supersede', recordIds: [recordId] };
  if (action === 'edit') prefill.view = 'manual';
  if (action === 'revise') prefill.view = 'agent';
  return prefill;
}
