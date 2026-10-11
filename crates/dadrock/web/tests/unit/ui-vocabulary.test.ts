import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

function text(path: string): string {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

describe('web UI vocabulary', () => {
  it('documents the shared page and component vocabulary', () => {
    const guide = text('../../docs/web-ui-vocabulary.md');

    for (const term of ['WorkbenchPage', 'StatusPanel', 'InlineLinkBadge', 'PanelSection', 'TextInput', 'StackedCard', 'Inbox', 'Ledger', 'Draft']) {
      expect(guide).toContain(term);
    }
  });

  it('keeps the two workbench routes on shared structural primitives', () => {
    const routeExpectations: Array<[string, string[]]> = [
      ['../../src/routes/inbox/+page.svelte', ['ResizableWorkbenchSplit', 'WorkbenchPane', 'ListRow']],
      ['../../src/routes/ledger/+page.svelte', ['ResizableWorkbenchSplit', 'WorkbenchPane', 'ListRow']],
      ['../../src/routes/ledger/[id]/+page.svelte', ['RecordDetailPanel']],
      ['../../src/lib/components/draft/DraftComposer.svelte', ['TextareaInput', 'Button']],
      ['../../src/lib/components/shell/Header.svelte', ['data-draft-button', 'ProjectGear']],
    ];

    for (const [path, expectedTerms] of routeExpectations) {
      const source = text(path);
      for (const term of expectedTerms) {
        expect(source, `${path} should use ${term}`).toContain(term);
      }
    }
  });

  it('keeps split workbench panes side-by-side before the large breakpoint', () => {
    const split = text('../../src/lib/components/workbench/ResizableWorkbenchSplit.svelte');

    expect(split).toContain('md:grid-cols-[var(--workbench-split-left-width)_8px_minmax(0,1fr)]');
    expect(split).toContain('md:flex');
    expect(split).not.toContain('lg:grid-cols-[var(--workbench-split-left-width)_8px_minmax(0,1fr)]');
  });

  it('turns every legacy destination into a redirect', () => {
    for (const route of ['records', 'records/[id]', 'candidates/[id]', 'candidates/new', 'tasks', 'validation', 'tags', 'settings', 'audit', 'links']) {
      const source = text(`../../src/routes/${route}/+page.svelte`);
      expect(source, `${route} should redirect`).toContain('LegacyRedirect');
    }
  });
});
