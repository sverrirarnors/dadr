import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

function text(path: string): string {
  return readFileSync(new URL(path, import.meta.url), 'utf8');
}

describe('web UI vocabulary', () => {
  it('documents the shared page and component vocabulary', () => {
    const guide = text('../../docs/web-ui-vocabulary.md');

    expect(guide).toContain('WorkbenchPage');
    expect(guide).toContain('SegmentedTabs');
    expect(guide).toContain('StatusPanel');
    expect(guide).toContain('InlineLinkBadge');
    expect(guide).toContain('PanelSection');
    expect(guide).toContain('TextInput');
    expect(guide).toContain('StackedCard');
  });

  it('keeps core workbench routes on shared structural primitives', () => {
    const routeExpectations = [
      ['../../src/routes/tasks/+page.svelte', ['WorkbenchPage', 'SegmentedTabs', 'StatusPanel']],
      ['../../src/routes/validation/+page.svelte', ['ResizableWorkbenchSplit', 'WorkbenchPane', 'DiagnosticPanel']],
      ['../../src/routes/settings/+page.svelte', ['WorkbenchPage', 'KeyValueGrid']],
      ['../../src/routes/records/+page.svelte', ['ResizableWorkbenchSplit', 'WorkbenchPane', 'ListRow']],
      ['../../src/routes/candidates/[id]/+page.svelte', ['WorkbenchPage', 'SegmentedTabs', 'Button']],
      ['../../src/routes/candidates/new/+page.svelte', ['WorkbenchPage', 'PanelSection', 'TextInput', 'TextareaInput']],
      ['../../src/lib/components/tasks/TaskRunnerPane.svelte', ['data-task-runner-pane']],
      ['../../src/routes/tasks/+page.svelte', ['data-task-context-editor', 'data-task-context-layout']],
      ['../../src/lib/components/tasks/TaskResultsPanel.svelte', ['PanelSection']],
      ['../../src/lib/components/tasks/TaskFollowUpComposer.svelte', ['PanelSection', 'TextareaInput']],
    ];

    for (const [path, expectedTerms] of routeExpectations) {
      const source = text(path as string);
      for (const term of expectedTerms as string[]) {
        expect(source, `${path} should use ${term}`).toContain(term);
      }
    }
  });

  it('keeps split workbench panes side-by-side before the large breakpoint', () => {
    const split = text('../../src/lib/components/workbench/ResizableWorkbenchSplit.svelte');
    const recordsRoute = text('../../src/routes/records/+page.svelte');

    expect(split).toContain('md:grid-cols-[var(--workbench-split-left-width)_8px_minmax(0,1fr)]');
    expect(split).toContain('md:flex');
    expect(split).not.toContain('lg:grid-cols-[var(--workbench-split-left-width)_8px_minmax(0,1fr)]');
    expect(recordsRoute).toContain('md:flex');
    expect(recordsRoute).not.toContain('data-record-detail-pane class="hidden min-h-0 min-w-0 flex-col p-4 lg:flex');
  });

  it('keeps candidate creation content in a bounded scroll container', () => {
    const candidateNewRoute = text('../../src/routes/candidates/new/+page.svelte');

    expect(candidateNewRoute).toContain('<WorkbenchPage layout="single" class="h-full">');
    expect(candidateNewRoute).toContain('data-candidate-new-pane class="h-full max-w-4xl overflow-auto"');
  });
});
