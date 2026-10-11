const STORAGE_KEY = 'dadrock.inbox.resolved.v1';

function readStored(): string[] {
  try {
    const raw = globalThis.localStorage?.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    return Array.isArray(parsed) ? parsed.filter((id): id is string => typeof id === 'string') : [];
  } catch {
    return [];
  }
}

function writeStored(ids: string[]) {
  try {
    globalThis.localStorage?.setItem(STORAGE_KEY, JSON.stringify(ids));
  } catch {
    // Storage is a convenience; the inbox still works without it.
  }
}

/**
 * Inbox items that the person has dealt with but that the backend cannot
 * remove on its own: dismissed diagnostics, discarded agent proposals, and
 * proposals already saved as candidates. Keyed by inbox item id.
 */
class ResolvedItemsStore {
  ids = $state<string[]>(readStored());

  has(id: string): boolean {
    return this.ids.includes(id);
  }

  add(...ids: string[]) {
    const next = [...this.ids];
    for (const id of ids) {
      if (!next.includes(id)) next.push(id);
    }
    this.ids = next;
    writeStored(this.ids);
  }

  remove(id: string) {
    this.ids = this.ids.filter((candidate) => candidate !== id);
    writeStored(this.ids);
  }

  clear() {
    this.ids = [];
    writeStored(this.ids);
  }
}

export const resolvedItems = new ResolvedItemsStore();
