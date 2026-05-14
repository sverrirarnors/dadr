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
});
