import type { RecordStatus, RecordSummary } from '$lib/api';

export type RecordStatusFilter = RecordStatus | 'all';
export type DisplayRecordStatus = RecordStatus;
export type RecordSearchScope = 'all' | 'title' | 'file' | 'tag' | 'id';

export const statusOptions: Array<{ value: RecordStatusFilter; label: string }> = [
  { value: 'all', label: 'All' },
  { value: 'candidate', label: 'Candidates' },
  { value: 'accepted', label: 'Accepted' },
  { value: 'rejected', label: 'Rejected' },
  { value: 'superseded', label: 'Superseded' },
];

export const searchScopeOptions: Array<{ value: RecordSearchScope; label: string }> = [
  { value: 'all', label: 'All' },
  { value: 'title', label: 'Title' },
  { value: 'file', label: 'File' },
  { value: 'tag', label: 'Tag' },
  { value: 'id', label: 'ID' },
];

export function recordSearchText(record: RecordSummary): string {
  return [
    record.id,
    record.id_prefix,
    record.simple_record_id,
    record.title,
    record.path,
    record.preview,
    ...record.tags,
  ]
    .join(' ')
    .toLowerCase();
}

export function displayRecordStatus(record: RecordSummary): DisplayRecordStatus {
  return record.status === 'accepted' && record.superseded ? 'superseded' : record.status;
}

export function recordStatusVariant(record: RecordSummary): DisplayRecordStatus {
  return displayRecordStatus(record);
}

export function filterRecords(
  records: RecordSummary[],
  filters: { status: RecordStatusFilter; query: string; scope?: RecordSearchScope },
): RecordSummary[] {
  const query = filters.query.trim().toLowerCase();
  const scope = filters.scope ?? 'all';
  return records.filter((record) => {
    if (filters.status === 'superseded') {
      if (!(record.status === 'accepted' && record.superseded)) return false;
    } else if (filters.status === 'accepted') {
      if (!(record.status === 'accepted' && !record.superseded)) return false;
    } else if (filters.status !== 'all' && record.status !== filters.status) {
      return false;
    }
    if (!query) return true;
    if (scope === 'tag') return tagMatches(record, query);
    return scopedSearchText(record, scope).includes(query);
  });
}

function tagMatches(record: RecordSummary, query: string): boolean {
  const normalizedQuery = normalizeTagSearchValue(query);
  return record.tags.some((tag) => {
    const rawTag = tag.toLowerCase();
    return rawTag.includes(query) || normalizeTagSearchValue(rawTag).includes(normalizedQuery);
  });
}

function normalizeTagSearchValue(value: string): string {
  return value.trim().toLowerCase().replace(/^#+/, '');
}

function scopedSearchText(record: RecordSummary, scope: RecordSearchScope): string {
  switch (scope) {
    case 'title':
      return record.title.toLowerCase();
    case 'file':
      return record.path.toLowerCase();
    case 'tag':
      return record.tags.join(' ').toLowerCase();
    case 'id':
      return [record.id, record.id_prefix, record.simple_record_id].join(' ').toLowerCase();
    case 'all':
      return recordSearchText(record);
  }
}
