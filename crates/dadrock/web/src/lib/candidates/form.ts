import type { MutationResponse } from '$lib/api';
import { inboxUrl, type InboxHref } from '$lib/navigation/routes';
import { candidateItemId } from '$lib/inbox/items';

export function parseListInput(input: string): string[] {
  return input
    .split(/[\s,]+/)
    .map((item) => item.trim())
    .filter(Boolean);
}

/** Where to go after a candidate is created: straight to it in the Inbox. */
export function candidateDestination(response: MutationResponse): InboxHref {
  const id = response.focus?.selected_id;
  return inboxUrl(id ? candidateItemId(id) : null);
}
