import { goto } from '$app/navigation';
import { resolve } from '$app/paths';

export type DraftMode = 'new' | 'revise' | 'supersede' | 'merge';

/** Which half of the Draft page is showing: the agent prompt or the form written by hand. */
export type ComposerView = 'agent' | 'manual';

export interface ComposerPrefill {
  mode?: DraftMode;
  recordIds?: string[];
  paths?: string[];
  text?: string;
  /** Open on this view instead of the default for the mode. */
  view?: ComposerView;
}

export interface ComposerShowOptions {
  /** Replace the current history entry instead of pushing one (deep links). */
  replace?: boolean;
}

/**
 * The Draft page is one place shared by every drafting verb. Pages call
 * `show()` with a prefill; it stores the prefill and navigates to `/draft`.
 * The page owns the rest of the form state while it is open.
 */
class ComposerStore {
  open = $state(false);
  mode = $state<DraftMode>('new');
  recordIds = $state<string[]>([]);
  paths = $state<string[]>([]);
  text = $state('');
  view = $state<ComposerView | null>(null);

  show(prefill: ComposerPrefill = {}, options: ComposerShowOptions = {}) {
    this.prefill(prefill);
    this.open = true;
    void goto(resolve('/draft'), options.replace ? { replaceState: true } : {});
  }

  /** Called by the Draft page when it is reached by a plain link. */
  ensureOpen() {
    if (this.open) return;
    this.prefill({});
    this.open = true;
  }

  hide() {
    this.open = false;
  }

  reset() {
    this.hide();
    this.prefill({});
  }

  private prefill(prefill: ComposerPrefill) {
    this.mode = prefill.mode ?? 'new';
    this.recordIds = [...(prefill.recordIds ?? [])];
    this.paths = [...(prefill.paths ?? [])];
    this.text = prefill.text ?? '';
    this.view = prefill.view ?? null;
  }
}

export const composer = new ComposerStore();
