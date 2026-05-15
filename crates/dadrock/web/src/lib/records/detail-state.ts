import type { RecordSummary } from '$lib/api';

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
