export type DraftMode = 'new' | 'revise' | 'supersede' | 'merge';

export interface ComposerPrefill {
  mode?: DraftMode;
  recordIds?: string[];
  paths?: string[];
  text?: string;
}

/**
 * The Draft composer is one sheet shared by every page. Pages open it with a
 * prefill; the composer owns the rest of the form state while it is visible.
 */
class ComposerStore {
  open = $state(false);
  mode = $state<DraftMode>('new');
  recordIds = $state<string[]>([]);
  paths = $state<string[]>([]);
  text = $state('');

  show(prefill: ComposerPrefill = {}) {
    this.mode = prefill.mode ?? 'new';
    this.recordIds = [...(prefill.recordIds ?? [])];
    this.paths = [...(prefill.paths ?? [])];
    this.text = prefill.text ?? '';
    this.open = true;
  }

  hide() {
    this.open = false;
  }

  reset() {
    this.hide();
    this.mode = 'new';
    this.recordIds = [];
    this.paths = [];
    this.text = '';
  }
}

export const composer = new ComposerStore();
