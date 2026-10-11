import type { RecordSummary } from '$lib/api';
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

export interface DetailActionGroups {
  /** Verbs that change the text: Edit, Revise with agent. */
  primary: DetailAction[];
  /** Verbs that change the lifecycle: Reject, Accept, Supersede. */
  lifecycle: DetailAction[];
}

/**
 * One visible button per verb. Candidates are reviewed in the Inbox; the
 * Ledger only ever offers to supersede a current decision.
 */
export function detailActions(input: {
  surface: DetailSurface;
  status: DisplayRecordStatus;
  agentAvailable: boolean;
}): DetailActionGroups {
  const { surface, status, agentAvailable } = input;
  const withAgent: DetailAction[] = agentAvailable ? ['edit', 'revise'] : ['edit'];

  if (status === 'candidate') {
    if (surface === 'inbox') return { primary: withAgent, lifecycle: ['reject', 'accept'] };
    return { primary: ['edit'], lifecycle: [] };
  }
  if (surface === 'ledger' && status === 'accepted') {
    return { primary: withAgent, lifecycle: ['supersede'] };
  }
  return { primary: withAgent, lifecycle: [] };
}
