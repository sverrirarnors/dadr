import { inboxUrl, ledgerRecordUrl, ledgerUrl } from './routes';

/**
 * Maps URLs from the previous six-destination UI onto Inbox, Ledger, and the
 * Draft composer. Returns null when the URL is not a legacy route.
 */
export function legacyRedirectTarget(url: URL): string | null {
  const path = url.pathname.replace(/\/+$/, '') || '/';
  const params = url.searchParams;

  if (path === '/') return inboxUrl();

  if (path === '/records') {
    const id = params.get('id');
    if (params.get('status') === 'candidate') {
      return inboxUrl(id ? `candidate:${id}` : null);
    }
    const tab = params.get('tab') === 'edit' ? 'edit' : null;
    const scope = params.get('scope');
    const q = params.get('q');
    if (scope === 'tag' && q) return ledgerUrl({ tag: q, id });
    return ledgerUrl({ q, id, tab, history: Boolean(params.get('status')) });
  }

  const recordMatch = path.match(/^\/records\/([^/]+)$/);
  if (recordMatch?.[1]) return ledgerRecordUrl(decodeURIComponent(recordMatch[1]));

  if (path === '/candidates/new') {
    const supersedes = params.get('supersedes');
    if (supersedes) return withDraft(ledgerUrl({ id: supersedes }), 'supersede', { record: supersedes });
    return withDraft(inboxUrl(), 'new');
  }

  const candidateMatch = path.match(/^\/candidates\/([^/]+)$/);
  if (candidateMatch?.[1]) return inboxUrl(`candidate:${decodeURIComponent(candidateMatch[1])}`);

  if (path === '/tasks') {
    const task = params.get('task');
    const record = params.get('record');
    if (task === 'edit-record' && record) return withDraft(ledgerUrl({ id: record }), 'revise', { record });
    if (task === 'supersede-record' && record) {
      return withDraft(ledgerUrl({ id: record }), 'supersede', { record });
    }
    if (task === 'code-to-record') return withDraft(inboxUrl(), 'code', { path: params.get('path') });
    if (params.get('tab') === 'runs') return inboxUrl();
    return withDraft(inboxUrl(), 'new', { text: params.get('instructions'), path: params.get('path') });
  }

  if (path === '/validation' || path === '/audit' || path === '/links' || path === '/settings') {
    return inboxUrl();
  }

  if (path === '/tags') return ledgerUrl();

  return null;
}

export type DraftDeepLinkMode = 'new' | 'code' | 'revise' | 'supersede' | 'merge';

function withDraft(
  base: string,
  mode: DraftDeepLinkMode,
  extra: { record?: string | null; path?: string | null; text?: string | null } = {},
): string {
  const [pathname, search = ''] = base.split('?');
  const params = new URLSearchParams(search);
  params.set('draft', mode);
  if (extra.record) params.set('record', extra.record);
  if (extra.path) params.set('path', extra.path);
  if (extra.text) params.set('text', extra.text);
  return `${pathname}?${params.toString()}`;
}
