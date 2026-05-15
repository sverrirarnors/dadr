export const candidateInboxHref = '/records?status=candidate' as const;
export const homeHref = candidateInboxHref;

export const primaryNavItems = [
  { id: 'inbox', href: candidateInboxHref, label: 'Inbox', ariaLabel: 'Candidate inbox' },
  { id: 'records', href: '/records', label: 'Records', ariaLabel: 'Browse records' },
  { id: 'tags', href: '/tags', label: 'Tags', ariaLabel: 'Browse tag list' },
  { id: 'tasks', href: '/tasks', label: 'Tasks', ariaLabel: 'Task runner' },
  { id: 'validation', href: '/validation', label: 'Validation', ariaLabel: 'Project validation' },
  { id: 'settings', href: '/settings', label: 'Settings', ariaLabel: 'Project settings' },
] as const;

export type PrimaryNavItem = (typeof primaryNavItems)[number];
export type PrimaryNavId = PrimaryNavItem['id'];
export type PrimaryNavHref = PrimaryNavItem['href'];

export function isPrimaryNavItemActive(item: PrimaryNavItem, url: URL): boolean {
  if (item.id === 'inbox') {
    return url.pathname === '/records' && url.searchParams.get('status') === 'candidate';
  }
  if (item.id === 'records') {
    return url.pathname === '/records' && url.searchParams.get('status') !== 'candidate';
  }

  const path = pathFromHref(item.href);
  return url.pathname === path || url.pathname.startsWith(`${path}/`);
}

function pathFromHref(href: string): string {
  return href.split('?')[0] ?? href;
}
