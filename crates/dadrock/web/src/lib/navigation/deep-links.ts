import { goto } from '$app/navigation';
import { resolve } from '$app/paths';
import { composer, type DraftMode } from '$lib/stores/composer.svelte';
import { legacyRedirectTarget } from './legacy';

/**
 * `?draft=<mode>` on any page opens the Draft page with a prefill. The
 * current history entry is replaced so Back does not reopen it.
 */
export function openComposerFromUrl(url: URL): void {
  const draft = url.searchParams.get('draft');
  if (!draft) return;
  const record = url.searchParams.get('record');
  const records = (url.searchParams.get('records') ?? '').split(',').filter(Boolean);
  const path = url.searchParams.get('path');
  const text = url.searchParams.get('text') ?? '';
  const recordIds = record ? [record] : records;
  composer.show(
    {
      mode: composerMode(draft),
      recordIds,
      paths: path ? [path] : [],
      text,
    },
    { replace: true },
  );
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
