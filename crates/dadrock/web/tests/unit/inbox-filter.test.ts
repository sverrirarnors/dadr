import { describe, expect, it } from 'vitest';
import { filterInboxGroups, inboxItemSearchText } from '../../src/lib/inbox/filter';
import type { RecordSummary } from '../../src/lib/api';
import type { InboxGroup, InboxItem } from '../../src/lib/inbox/items';

function record(id: string, title: string, tags: string[] = []): RecordSummary {
  return {
    id,
    id_prefix: id.slice(0, 12),
    simple_record_id: id,
    title,
    status: 'candidate',
    superseded: false,
    path: `dadr/inbox/${id}.md`,
    preview: '',
    tags,
    timestamp_ms: 0,
  };
}

const candidate: InboxItem = {
  kind: 'candidate',
  id: 'candidate:c1',
  title: 'Use Polars lazy frames',
  subtitle: 'Deferred execution',
  record: record('c1', 'Use Polars lazy frames', ['#kind/transformation']),
};
const issue: InboxItem = {
  kind: 'issue',
  id: 'issue:x',
  title: 'A local Markdown link points at a missing note.',
  subtitle: 'Links',
  category: 'Links',
  diagnostic: { severity: 'error', code: 'broken_markdown_link', message: 'A local Markdown link points at a missing note.', path: 'dadr/ledger/0001.md', record_id: null, field: null, line: 3, related_paths: [] },
};
const groups: InboxGroup[] = [
  { id: 'candidates', label: 'Candidates', items: [candidate] },
  { id: 'issues', label: 'Issues', items: [issue] },
];

describe('filterInboxGroups', () => {
  it('returns the groups untouched for an empty query', () => {
    expect(filterInboxGroups(groups, '  ')).toBe(groups);
  });

  it('matches titles, tags, paths, and diagnostic codes case-insensitively and drops empty groups', () => {
    expect(filterInboxGroups(groups, 'POLARS').map((group) => group.id)).toEqual(['candidates']);
    expect(filterInboxGroups(groups, '#kind/transformation').map((group) => group.id)).toEqual(['candidates']);
    expect(filterInboxGroups(groups, 'broken_markdown').map((group) => group.id)).toEqual(['issues']);
    expect(filterInboxGroups(groups, '0001.md').map((group) => group.id)).toEqual(['issues']);
    expect(filterInboxGroups(groups, 'nothing here')).toEqual([]);
  });

  it('includes the subtitle in the searchable text', () => {
    expect(inboxItemSearchText(candidate)).toContain('deferred execution');
  });
});
