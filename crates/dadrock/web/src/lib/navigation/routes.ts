export const inboxHref = '/inbox' as const;
export const ledgerHref = '/ledger' as const;
export const draftHref = '/draft' as const;
export const homeHref = inboxHref;

export const primaryNavItems = [
  { id: 'inbox', href: inboxHref, label: 'Inbox', ariaLabel: 'Inbox' },
  { id: 'ledger', href: ledgerHref, label: 'Ledger', ariaLabel: 'Ledger' },
  { id: 'draft', href: draftHref, label: 'Draft', ariaLabel: 'Draft' },
] as const;

export type PrimaryNavItem = (typeof primaryNavItems)[number];
export type PrimaryNavId = PrimaryNavItem['id'];
export type PrimaryNavHref = PrimaryNavItem['href'];

export function isPrimaryNavItemActive(item: PrimaryNavItem, url: URL): boolean {
  return url.pathname === item.href || url.pathname.startsWith(`${item.href}/`);
}

export type LedgerHref = '/ledger' | `/ledger?${string}` | `/ledger/${string}`;
export type InboxHref = '/inbox' | `/inbox?${string}`;

export interface LedgerQuery {
  id?: string | null | undefined;
  q?: string | null | undefined;
  tag?: string | null | undefined;
  history?: boolean | undefined;
  tab?: 'edit' | null | undefined;
}

/** Builds a Ledger URL; empty values are omitted so URLs stay canonical. */
export function ledgerUrl(query: LedgerQuery = {}): LedgerHref {
  const params = new URLSearchParams();
  if (query.q) params.set('q', query.q);
  if (query.tag) params.set('tag', query.tag);
  if (query.history) params.set('history', '1');
  if (query.id) params.set('id', query.id);
  if (query.tab) params.set('tab', query.tab);
  const search = params.toString();
  return (search ? `/ledger?${search}` : '/ledger') as LedgerHref;
}

export function ledgerRecordUrl(id: string): `/ledger/${string}` {
  return `/ledger/${encodeURIComponent(id)}`;
}

export interface InboxQuery {
  q?: string | null | undefined;
}

export function inboxUrl(itemId?: string | null, query: InboxQuery = {}): InboxHref {
  const params = new URLSearchParams();
  if (query.q) params.set('q', query.q);
  if (itemId) params.set('item', itemId);
  const search = params.toString();
  return (search ? `/inbox?${search}` : '/inbox') as InboxHref;
}
