import { recordSearchText } from '$lib/records/filters';
import type { InboxGroup, InboxItem } from './items';

/** The text a search box matches against for one inbox item. */
export function inboxItemSearchText(item: InboxItem): string {
  const parts: string[] = [item.title, item.subtitle];
  switch (item.kind) {
    case 'candidate':
      parts.push(recordSearchText(item.record));
      break;
    case 'split':
      parts.push(...[...item.predecessors, ...item.records].map(recordSearchText));
      break;
    case 'proposal':
      parts.push(...item.proposal.tags, item.proposal.body_markdown);
      break;
    case 'revision':
      parts.push(...item.tags, item.change_explanation);
      if (item.record) parts.push(recordSearchText(item.record));
      break;
    case 'conflict':
      parts.push(item.finding.conflict_explanation, item.finding.relationship);
      if (item.left) parts.push(recordSearchText(item.left));
      if (item.right) parts.push(recordSearchText(item.right));
      break;
    case 'report':
      parts.push(item.body_markdown);
      break;
    case 'issue':
      parts.push(item.diagnostic.code, item.diagnostic.path ?? '', item.category);
      if (item.record) parts.push(recordSearchText(item.record));
      break;
    case 'running':
      parts.push(item.run.task_id);
      break;
  }
  return parts.join(' ').toLowerCase();
}

/** Keeps the items whose text contains the query; drops groups left empty. */
export function filterInboxGroups(groups: InboxGroup[], query: string): InboxGroup[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return groups;
  return groups
    .map((group) => ({
      ...group,
      items: group.items.filter((item) => inboxItemSearchText(item).includes(normalized)),
    }))
    .filter((group) => group.items.length > 0);
}
