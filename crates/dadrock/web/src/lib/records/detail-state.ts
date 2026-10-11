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
 * What Revise with agent asks for: `edit-record` revises a candidate in place,
 * `supersede-record` drafts a successor of a decision in force. Agents never
 * edit accepted text.
 */
export type ReviseTask = 'edit-record' | 'supersede-record';

export interface DetailActionGroups {
  /** Verbs that change the text: Edit, Revise with agent. */
  primary: DetailAction[];
  /** Verbs that change the lifecycle: Reject, Accept, Supersede. */
  lifecycle: DetailAction[];
  /** The task Revise with agent runs, or null when it is not offered. */
  revise: ReviseTask | null;
  /**
   * Edit corrects a decided record (accepted, superseded, or rejected) in
   * place: the editor says that corrections show in History and that changing
   * the decision means superseding it.
   */
  correction: boolean;
}

/**
 * One visible button per verb. Edit works in place on every record: candidates
 * freely, decided records for corrections. Candidates are reviewed in the
 * Inbox and revised by the agent in place. A decision in force is superseded,
 * by hand or by the agent, in the Ledger. Archived records (superseded or
 * rejected) are only corrected.
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
      return {
        primary: withAgent,
        lifecycle: ['reject', 'accept'],
        revise: agentAvailable ? 'edit-record' : null,
        correction: false,
      };
    }
    return { primary: ['edit'], lifecycle: [], revise: null, correction: false };
  }
  if (status === 'accepted') {
    return {
      primary: withAgent,
      lifecycle: surface === 'ledger' ? ['supersede'] : [],
      revise: agentAvailable ? 'supersede-record' : null,
      correction: true,
    };
  }
  return { primary: ['edit'], lifecycle: [], revise: null, correction: true };
}

/**
 * The Draft page a successor verb opens for a decision in force: Supersede
 * writes the successor by hand, starting from the record's exact text, and
 * Revise with agent asks the agent.
 */
export function successorDraft(action: 'revise' | 'supersede', recordId: string): ComposerPrefill {
  return { mode: 'supersede', recordIds: [recordId], view: action === 'revise' ? 'agent' : 'manual' };
}
