import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { composer, type DraftMode } from '$lib/stores/composer.svelte';
import { legacyRedirectTarget } from './legacy';

/**
 * `?draft=<mode>` opens the composer from a link, then the parameter is
 * removed so a reload does not reopen it.
 */
export function openComposerFromUrl(url: URL): void {
  const draft = url.searchParams.get('draft');
  if (!draft) return;
  const record = url.searchParams.get('record');
  const records = (url.searchParams.get('records') ?? '').split(',').filter(Boolean);
  const path = url.searchParams.get('path');
  const text = url.searchParams.get('text') ?? '';
  const recordIds = record ? [record] : records;
  const mode = composerMode(draft);
  composer.show({
    mode,
    recordIds,
    paths: path ? [path] : [],
    text,
  });
  const next = new URL(url);
  for (const key of ['draft', 'record', 'records', 'path', 'text']) next.searchParams.delete(key);
  void goto(resolve(`${next.pathname}${next.search}` as '/inbox'), {
    replaceState: true,
    noScroll: true,
    keepFocus: true,
  });
}

function composerMode(value: string): DraftMode {
  if (value === 'revise' || value === 'supersede' || value === 'merge') return value;
  return 'new';
}

/** Navigates a legacy URL to its new home. Returns false when there is none. */
export function redirectLegacyUrl(url: URL): boolean {
  const target = legacyRedirectTarget(url);
  if (!target) return false;
  void goto(resolve(target as '/inbox'), { replaceState: true });
  return true;
}
