import path from 'node:path';
import { expect, test, type Page } from '@playwright/test';

async function openContextEditor(page: Page) {
  await page.locator('[data-task-context-editor]').waitFor({ state: 'visible' });
  await page.locator('[data-task-context-nav]').waitFor({ state: 'visible' });
}

async function selectContextTab(page: Page, name: RegExp | string) {
  await page.locator('[data-task-context-nav]').getByRole('tab', { name }).click();
}

test('tasks route lets users choose a built-in task and run it', async ({ page }) => {
  await page.goto('/tasks');

  await expect(page.locator('[data-workbench-page][data-layout="single"]')).toBeVisible();
  await expect(page.locator('[data-task-workflow-pane]')).toHaveCount(0);
  await expect(page.locator('[data-task-runner-pane]')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Task result workbench' })).toHaveCount(0);
  await expect(page.locator('[data-ui-segmented-tabs]').first()).toBeVisible();
  await expect(page.locator('[data-ui-status-panel]')).toBeVisible();
  await expect(page.getByRole('tab', { name: 'Context' })).toBeVisible();
  await expect(page.getByRole('tab', { name: 'Tasks' })).toHaveCount(0);
  await expect(page.getByRole('tab', { name: 'Results' })).toBeVisible();
  await expect(page.getByRole('tab', { name: 'History' })).toHaveCount(0);
  await expect(page.getByRole('tab', { name: 'Runs' })).toHaveCount(0);
  await expect(page.getByRole('heading', { name: 'Task', level: 2 })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Runner' })).toHaveCount(0);
  await expect(page.locator('[data-ui-button]').filter({ hasText: 'Run task' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Customize task' })).toHaveCount(0);
  await expect(page.getByText('7 available tasks')).toBeVisible();
  await expect(page.getByText('Ready to run a task.')).toBeVisible();
  await expect(page.getByRole('link', { name: 'Task runner' })).toBeVisible();
  await expect(page.getByLabel('Records to audit')).toHaveCount(0);
  await expect(page.getByLabel('Scope')).toHaveCount(0);
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /^System prompt$/ })).toHaveCount(0);
  await expect(page.locator('[data-task-context-nav]')).toBeVisible();

  const chrome = await page.evaluate(() => {
    function borderWidth(selector: string): string | null {
      const element = document.querySelector(selector);
      if (!element) return null;
      return getComputedStyle(element).borderTopWidth;
    }

    return {
      workbenchPane: borderWidth('[data-workbench-pane]'),
      statusPanel: borderWidth('[data-ui-status-panel]'),
      runnerPane: borderWidth('[data-task-runner-pane]'),
      contextEditor: borderWidth('[data-task-context-editor]'),
    };
  });

  expect(chrome).toEqual({
    workbenchPane: '0px',
    statusPanel: '0px',
    runnerPane: '0px',
    contextEditor: '0px',
  });

  const taskSelect = page.getByRole('combobox', { name: 'Task' });
  await taskSelect.selectOption('find-conflicts');
  await expect(taskSelect).toHaveValue('find-conflicts');
  await expect(page.getByText('Scan decisions and candidates for contradictions')).toHaveCount(0);
  await expect(page.getByLabel('Scope')).toHaveCount(0);
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /^Records 0$/ })).toBeVisible();
  await selectContextTab(page, /Records/);
  await expect(page.getByLabel('Scope')).toBeVisible();

  await page.getByRole('button', { name: 'Run task' }).click();

  const statusPanel = page.getByRole('region', { name: 'Task status' });
  await expect(statusPanel).toContainText('agent output');
  await expect(page.getByText(/Loaded result/)).toBeVisible();
  await expect(statusPanel.getByRole('button')).toHaveAttribute('aria-expanded', 'false');
  await expect(page.getByRole('tab', { name: 'Results' })).toHaveAttribute('aria-selected', 'true');
  await expect(page.locator('[data-task-results-panel]')).toBeVisible();
  await expect(page.locator('details[data-ui-stacked-card][data-result-card]').first()).toBeVisible();
  await page.locator('details[data-result-card]').first().locator('summary').click();
  await expect(page.getByText('No conflicts found in mock records.')).toBeVisible();
  await expect(page.locator('[data-task-history-pane]')).toBeVisible();
  await expect(page.getByRole('button', { name: /find-conflicts/ })).toBeVisible();
});

test('tasks results tab uses the shared two-pane shell', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 700 });
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('find-conflicts');
  await page.getByRole('button', { name: 'Run task' }).click();
  await expect(page.getByText(/Loaded result/)).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Task', level: 2 })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Runner' })).toHaveCount(0);

  await expect(page.locator('[data-task-runs-workbench][data-layout="two-pane"]')).toBeVisible();
  await expect(page.locator('[data-resizable-workbench-split="task-runs"]')).toBeVisible();
  await expect(page.locator('[data-task-runs-list-pane]')).toBeVisible();
  await expect(page.locator('[data-task-runs-result-pane]')).toBeVisible();
  const handle = page.getByRole('separator', { name: 'Resize task run panes' });
  await expect(handle).toBeVisible();

  const metrics = await page.evaluate(() => {
    const main = document.querySelector('main');
    const list = document.querySelector('[data-task-runs-list-pane]');
    const result = document.querySelector('[data-task-runs-result-pane]');
    if (!main || !list || !result) throw new Error('missing task run panes');
    return {
      mainOverflowY: getComputedStyle(main).overflowY,
      mainClientHeight: main.clientHeight,
      mainScrollHeight: main.scrollHeight,
      listOverflowY: getComputedStyle(list).overflowY,
      resultOverflowY: getComputedStyle(result).overflowY,
    };
  });

  expect(metrics.mainOverflowY).toBe('hidden');
  expect(metrics.mainScrollHeight).toBe(metrics.mainClientHeight);
  expect(metrics.listOverflowY).toBe('auto');
  expect(metrics.resultOverflowY).toBe('auto');
  await expect(page.locator('[data-task-runs-list-pane] [data-task-history-pane]')).toHaveCSS('border-top-width', '0px');
  await expect(page.locator('[data-task-runs-result-pane] [data-task-results-panel]')).toHaveCSS('border-top-width', '0px');
  await expect(page.locator('[data-task-runs-list-pane]').getByText(/[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}/)).toHaveCount(0);
  const resultPane = page.locator('[data-task-runs-result-pane]');
  await expect(resultPane.getByRole('heading', { name: 'Results' })).toBeVisible();
  await expect(resultPane.locator('[data-task-result-meta]')).toContainText('Report');
  await expect(resultPane.locator('[data-task-result-meta]')).toContainText('succeeded');
  await expect(resultPane.locator('[data-task-result-meta]')).toContainText(/[0-9a-f]{12}/);
  await expect(resultPane.getByRole('region', { name: 'Result overview' })).toHaveCount(0);

  const list = page.locator('[data-task-runs-list-pane]');
  const result = page.locator('[data-task-runs-result-pane]');
  const [beforeList, beforeResult, handleBox] = await Promise.all([
    list.boundingBox(),
    result.boundingBox(),
    handle.boundingBox(),
  ]);
  if (!beforeList || !beforeResult || !handleBox) throw new Error('expected task run pane boxes');

  await page.mouse.move(handleBox.x + handleBox.width / 2, handleBox.y + handleBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(handleBox.x + handleBox.width / 2 + 100, handleBox.y + handleBox.height / 2, { steps: 5 });
  await page.mouse.up();

  const [afterList, afterResult] = await Promise.all([list.boundingBox(), result.boundingBox()]);
  if (!afterList || !afterResult) throw new Error('expected resized task run pane boxes');
  expect(afterList.width).toBeGreaterThan(beforeList.width + 60);
  expect(afterResult.width).toBeLessThan(beforeResult.width - 60);
});

test('tasks route shows all tasks in the task selector without search', async ({ page }) => {
  await page.goto('/tasks');

  const taskSelect = page.getByRole('combobox', { name: 'Task' });
  await expect(page.getByRole('region', { name: 'Task intents' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: /Improve records/ })).toHaveCount(0);
  await expect(page.getByRole('searchbox', { name: 'Search tasks' })).toHaveCount(0);
  await expect(page.locator('[data-task-description]')).toContainText(
    'Use a freeform prompt with selected records, files, folders, or notes when no dedicated DADR workflow fits.',
  );
  await expect(taskSelect.locator('option[value="custom"]')).toHaveCount(1);
  await expect(taskSelect.locator('option[value="improve-candidate-quality"]')).toHaveCount(0);
  await expect(taskSelect.locator('option[value="code-to-record"]')).toHaveCount(1);
  await expect(taskSelect.locator('option[value="prepare-review-brief"]')).toHaveCount(0);
  await taskSelect.selectOption('code-to-record');
  await expect(taskSelect).toHaveValue('code-to-record');
  await expect(page.locator('[data-task-description]')).toContainText(
    'Scan selected project files and draft candidate DADRs for consequential choices to review.',
  );
});

test('tasks route presents context in one editable surface', async ({ page }) => {
  const recordId = '0193fb1f-a3a0-7000-8000-000000000002';
  await page.goto(`/tasks?task=edit-record&record=${recordId}`);

  await expect(page.getByRole('combobox', { name: 'Task' })).toHaveValue('edit-record');
  await expect(page.locator('[data-task-context-section="details"]').getByText('Record', { exact: true })).toHaveCount(0);
  await expect(page.getByRole('region', { name: 'Run context' })).toHaveCount(0);
  await expect(page.locator('[data-task-context-editor]')).toContainText('Context');
  await expect(page.locator('[data-task-context-editor]')).not.toContainText('No context');
  await expect(page.locator('[data-task-context-editor]')).not.toContainText('1 item');
  await openContextEditor(page);
  await expect(page.locator('[data-task-context-editor]')).toBeVisible();
  await expect(page.locator('[data-task-context-nav]')).toBeVisible();
  await expect(page.locator('[data-task-context-layout]')).toBeVisible();
  await expect(page.locator('[data-task-context-left-pane]')).toBeVisible();
  await expect(page.locator('[data-task-context-right-pane]')).toBeVisible();
  await expect(page.locator('[data-task-context-section="details"]')).toBeVisible();
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /^System prompt$/ })).toHaveCount(0);
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /Prompt/ })).toBeVisible();
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /Files & Directories/ })).toBeVisible();
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /^Directories/ })).toHaveCount(0);
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /Details/ })).toHaveCount(0);
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /^Notes/ })).toHaveCount(0);
  await expect(page.locator('[data-task-context-section="files"]')).toHaveCount(0);
  await expect(page.locator('[data-task-context-section="directories"]')).toHaveCount(0);
  await expect(page.locator('[data-task-context-section="records"]')).toHaveCount(0);
  await expect(page.locator('[data-task-context-section="instructions"]')).toHaveCount(0);
  await expect(page.getByText('Add specific files for the task to inspect.')).toHaveCount(0);
  await expect(page.getByText('Add instructions that should apply only to this run.')).toHaveCount(0);
  await page.getByLabel('Instructions and notes').fill('Keep the decision narrow.\n\nReviewer wants less scope creep.');
  await expect(page.getByLabel('Files and folders')).toHaveCount(0);
  await expect(page.getByLabel('Related records')).toHaveCount(0);
  await expect(page.getByLabel('Pasted context')).toHaveCount(0);
  await selectContextTab(page, /Files & Directories/);
  await page.getByLabel('File path').fill('analysis/model.R');
  await page.getByRole('button', { name: 'Use file path' }).click();

  await expect(page.getByRole('region', { name: 'Run context' })).toHaveCount(0);
  await expect(page.locator('[data-task-context-editor]')).not.toContainText('3 items');
  await expect(page.getByRole('button', { name: 'Remove path analysis/model.R' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Select local files', exact: true })).toBeVisible();
});

test('tasks context editor uses a bordered two-pane layout', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('find-conflicts');
  await openContextEditor(page);

  const metrics = await page.evaluate(() => {
    const editor = document.querySelector('[data-task-context-editor]');
    const layout = document.querySelector('[data-task-context-layout]');
    const left = document.querySelector('[data-task-context-left-pane]');
    const right = document.querySelector('[data-task-context-right-pane]');
    if (!editor || !layout || !left || !right) throw new Error('missing context panes');
    const editorBox = editor.getBoundingClientRect();
    const leftBox = left.getBoundingClientRect();
    const rightBox = right.getBoundingClientRect();
    return {
      editorBorder: getComputedStyle(editor).borderTopWidth,
      layoutBorder: getComputedStyle(layout).borderTopWidth,
      leftBorderRight: getComputedStyle(left).borderRightWidth,
      rightBorderLeft: getComputedStyle(right).borderLeftWidth,
      leftX: leftBox.x,
      rightX: rightBox.x,
      editorLeft: editorBox.x,
      editorRight: editorBox.right,
    };
  });

  expect(metrics.editorBorder).toBe('0px');
  expect(metrics.layoutBorder).toBe('1px');
  expect(metrics.leftBorderRight).toBe('1px');
  expect(metrics.rightBorderLeft).toBe('0px');
  expect(metrics.leftX).toBeGreaterThanOrEqual(metrics.editorLeft);
  expect(metrics.rightX).toBeGreaterThan(metrics.leftX + 150);
  expect(metrics.rightX).toBeLessThan(metrics.editorRight);
});

test('tasks route opens context editor directly from the edit context button', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('find-conflicts');
  await openContextEditor(page);

  await expect(page.locator('[data-task-context-nav]')).toBeVisible();
  await expect(page.locator('[data-task-context-section="details"]')).toBeVisible();
  await expect(page.locator('[data-task-context-section="instructions"]')).toHaveCount(0);
  await expect(page.getByLabel('Scope')).toHaveCount(0);
  await selectContextTab(page, /Records/);
  await expect(page.getByLabel('Scope')).toBeVisible();

  const [editorBox, inputBox] = await Promise.all([
    page.locator('[data-task-context-editor]').boundingBox(),
    page.getByLabel('Scope').boundingBox(),
  ]);
  if (!editorBox || !inputBox) {
    throw new Error('expected context editor and task input boxes');
  }
  expect(editorBox.y).toBeLessThan(inputBox.y);
});

test('tasks route context shell is not collapsible', async ({ page }) => {
  await page.goto('/tasks');

  await openContextEditor(page);
  await expect(page.locator('[data-task-context-section="details"]')).toBeVisible();
  await expect(page.locator('[data-task-context-editor]').getByRole('button', { name: /Context/ })).toHaveCount(0);
});

test('tasks route scrolls at the page level', async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 420 });
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('find-conflicts');
  await openContextEditor(page);

  const metrics = await page.evaluate(() => {
    const main = document.querySelector('main');
    if (!main) throw new Error('missing main');
    main.scrollTop = 200;
    return {
      mainOverflowY: getComputedStyle(main).overflowY,
      mainClientHeight: main.clientHeight,
      mainScrollHeight: main.scrollHeight,
      mainScrollTop: main.scrollTop,
    };
  });

  expect(metrics.mainOverflowY).toBe('auto');
  expect(metrics.mainScrollHeight).toBeGreaterThan(metrics.mainClientHeight);
  expect(metrics.mainScrollTop).toBeGreaterThan(0);
});

test('tasks route scrolls through the page container', async ({ page }) => {
  await page.setViewportSize({ width: 1000, height: 420 });
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('find-conflicts');
  await openContextEditor(page);

  const scrollTop = await page.locator('main').evaluate((element) => {
    element.scrollBy(0, 700);
    return element.scrollTop;
  });
  expect(scrollTop).toBeGreaterThan(0);
});

test('tasks route shows code-to-record candidate proposals without UI write controls', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('code-to-record');
  await expect(page.getByLabel('Path')).toHaveCount(0);
  await expect(page.getByLabel('Focus')).toHaveCount(0);
  await page.getByLabel('Instructions and notes').fill('Find consequential data cleaning decisions.');
  await page.getByRole('button', { name: 'Run task' }).click();

  await expect(page.getByRole('heading', { name: 'Use explicit missing data policy' })).toBeVisible();
  const candidateCard = page.locator('details[data-result-card]').filter({ hasText: 'Use explicit missing data policy' });
  await expect(page.getByRole('heading', { name: 'Review actions' })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Write selected candidates' })).toHaveCount(0);
  await expect(page.getByRole('checkbox', { name: 'Select Use explicit missing data policy' })).toHaveCount(0);
  await expect(candidateCard.getByRole('button', { name: 'Write candidate' })).toHaveCount(0);
  await expect(page.getByText('Wrote 1 candidate from code-to-record.')).toHaveCount(0);

  await expect(page.locator('[data-task-result-meta]')).toContainText('Candidate batch');
});

test('tasks route shows review actions for record revisions', async ({ page }) => {
  const recordId = '0193fb1f-a3a0-7000-8000-000000000002';
  await page.goto(`/tasks?task=edit-record&record=${recordId}`);
  await page.getByLabel('Instructions and notes').fill('Clarify the alternatives and consequences.');

  await page.getByRole('button', { name: 'Run task' }).click();

  await expect(page.getByRole('heading', { name: 'Review revision' })).toBeVisible();
  await expect(page.getByText('Mock revision generated.')).toBeVisible();
  await expect(page.getByRole('link', { name: 'Open record' })).toHaveAttribute(
    'href',
    `/records?id=${recordId}`,
  );
  await expect(page.getByLabel('Revised record markdown')).toHaveValue(/## Alternatives/);
});

test('tasks route can apply a record revision result', async ({ page }) => {
  const recordId = '0193fb1f-a3a0-7000-8000-000000000002';
  await page.goto(`/tasks?task=edit-record&record=${recordId}`);
  await page.getByLabel('Instructions and notes').fill('Tighten the decision wording.');

  await page.getByRole('button', { name: 'Run task' }).click();

  await expect(page.getByRole('heading', { name: 'Review revision' })).toBeVisible();
  await expect(page.getByLabel('Revision title')).toHaveValue('Adopt pre-registration for studies');
  await expect(page.getByLabel('Revision tags')).toHaveValue('#dadr/methodology');
  await page.getByRole('button', { name: 'Apply revision' }).click();

  await expect(page).toHaveURL(`/records/${recordId}`);
  await expect(page.getByRole('article')).toContainText('Update the record wording.');
});

test('tasks route sends additional context with a task run', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('code-to-record');
  await openContextEditor(page);
  await selectContextTab(page, /Prompt/);
  await page.getByLabel('Instructions and notes').fill('Focus on MCP task APIs.');
  await selectContextTab(page, /Files & Directories/);
  await page.getByLabel('File path').fill('crates/dadrock/src/mcp.rs');
  await page.getByRole('button', { name: 'Use file path' }).click();
  await selectContextTab(page, /Records/);
  await page.getByRole('button', { name: 'Add records' }).click();
  const dialog = page.getByRole('dialog', { name: 'Add records' });
  await dialog.getByLabel('Search scope').selectOption('title');
  await dialog.getByRole('searchbox', { name: 'Search records' }).fill('Use Polars over pandas');
  await dialog.getByRole('checkbox', { name: 'Select Use Polars over pandas' }).check();
  await dialog.getByRole('button', { name: 'Add 1 record' }).click();

  await page.getByRole('button', { name: 'Run task' }).click();

  await page.locator('details[data-result-card]').filter({ hasText: 'Summary' }).locator('summary').click();
  await expect(page.getByText('Context note: Focus on MCP task APIs.')).toBeVisible();
  await expect(page.getByText('Path context: crates/dadrock/src/mcp.rs')).toBeVisible();
});

test('tasks route prefills edit-record from query parameters', async ({ page }) => {
  const recordId = '0193fb1f-a3a0-7000-8000-000000000002';
  await page.goto(`/tasks?task=edit-record&record=${recordId}`);

  await expect(page.getByRole('combobox', { name: 'Task' })).toHaveValue('edit-record');
  await expect(page.locator('[data-task-context-section="details"]').getByText('Record', { exact: true })).toHaveCount(0);
  await expect(page.getByLabel('Requested changes')).toHaveCount(0);
  await expect(page.getByRole('region', { name: 'Task source record' })).toContainText(
    'Adopt pre-registration for studies',
  );
  await expect(page.getByRole('link', { name: 'Back to record' })).toHaveAttribute(
    'href',
    `/records?id=${recordId}`,
  );
  await expect(page.locator('[data-task-context-editor]')).not.toContainText('1 item');
  await openContextEditor(page);
  await selectContextTab(page, /Records/);
  await expect(page.getByText('Attach records that should shape this task run.')).toHaveCount(0);
  await expect(page.locator('[data-record-picker]')).toBeVisible();
  await expect(page.locator('[data-record-picker-selected]')).toContainText('Adopt pre-registration for studies');
  await expect(page.getByRole('button', { name: 'Remove Adopt pre-registration for studies' })).toBeVisible();
});

test('tasks route prefills supersede-record from query parameters', async ({ page }) => {
  const recordId = '0193fb1f-a3a0-7000-8000-000000000003';
  await page.goto(`/tasks?task=supersede-record&record=${recordId}`);

  await expect(page.getByRole('combobox', { name: 'Task' })).toHaveValue('supersede-record');
  await expect(page.getByLabel('Record to supersede')).toHaveCount(0);
  await expect(page.getByLabel('Reason')).toHaveCount(0);
  await openContextEditor(page);
  await selectContextTab(page, /Records/);
  await expect(page.getByRole('button', { name: 'Remove Use Polars lazy frames' })).toBeVisible();
});

test('tasks route prefills instructions and path context from query parameters', async ({ page }) => {
  const recordId = '0193fb1f-a3a0-7000-8000-000000000001';
  const instructions = 'Fix validation issue broken_markdown_link.';
  await page.goto(
    `/tasks?task=custom&record=${recordId}&path=dadr/accepted/0001-use-polars.md&instructions=${encodeURIComponent(instructions)}`,
  );

  await expect(page.getByRole('combobox', { name: 'Task' })).toHaveValue('custom');
  await expect(page.getByLabel('Instructions and notes')).toHaveValue(instructions);
  await expect(page.getByRole('button', { name: 'Run task' })).toBeEnabled();

  await openContextEditor(page);
  await selectContextTab(page, /Files & Directories/);
  await expect(page.getByRole('button', { name: 'Remove path dadr/accepted/0001-use-polars.md' })).toBeVisible();
  await selectContextTab(page, /Records/);
  await expect(page.getByRole('button', { name: 'Remove Use Polars over pandas' })).toBeVisible();
});

test('tasks route lets users search and add records as context', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('code-to-record');
  await openContextEditor(page);
  await selectContextTab(page, /Records/);
  await expect(page.locator('[data-record-picker]')).toBeVisible();
  await page.getByRole('button', { name: 'Add records' }).click();

  const dialog = page.getByRole('dialog', { name: 'Add records' });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByLabel('Search scope')).toBeVisible();
  await expect(dialog.locator('[data-record-status-filter]')).toBeVisible();
  await dialog.getByLabel('Search scope').selectOption('title');
  await dialog.getByRole('searchbox', { name: 'Search records' }).fill('Use Polars');

  await dialog.getByRole('checkbox', { name: 'Select Use Polars over pandas' }).check();
  await dialog.getByRole('checkbox', { name: 'Select Use Polars lazy frames' }).check();
  await dialog.getByRole('button', { name: 'Add 2 records' }).click();

  await expect(page.getByLabel('Related records')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Remove Use Polars over pandas' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Remove Use Polars lazy frames' })).toBeVisible();
  await expect(page.locator('[data-task-context-editor]')).not.toContainText('1 item');
});

test('tasks route scrolls record picker results inside the overlay', async ({ page }) => {
  await page.goto('/tasks');

  await openContextEditor(page);
  await selectContextTab(page, /Records/);
  await page.getByRole('button', { name: 'Add records' }).click();

  const metrics = await page.evaluate(() => {
    const dialog = document.querySelector('[data-record-picker-dialog]');
    const results = document.querySelector('[data-record-picker-results]');
    if (!dialog || !results) throw new Error('missing record picker overlay');
    return {
      dialogPosition: getComputedStyle(dialog).position,
      resultsOverflowY: getComputedStyle(results).overflowY,
      resultsClientHeight: results.clientHeight,
      dialogClientHeight: dialog.clientHeight,
    };
  });

  expect(metrics.dialogPosition).toBe('fixed');
  expect(metrics.resultsOverflowY).toBe('auto');
  expect(metrics.resultsClientHeight).toBeLessThan(metrics.dialogClientHeight);
});

test('tasks route places select controls next to their labels', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('find-conflicts');
  await openContextEditor(page);
  await selectContextTab(page, /Records/);

  const labelText = page.getByText('Scope', { exact: true });
  const select = page.getByLabel('Scope');
  const [labelBox, selectBox] = await Promise.all([labelText.boundingBox(), select.boundingBox()]);
  if (!labelBox || !selectBox) {
    throw new Error('expected select label and control');
  }

  expect(Math.abs(labelBox.y - selectBox.y)).toBeLessThanOrEqual(8);
  expect(selectBox.x).toBeGreaterThan(labelBox.x + labelBox.width);
});

test('tasks route normalizes freeform task fields into details notes', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('context-to-record');
  await expect(page.getByRole('textbox', { name: 'Context' })).toHaveCount(0);
  await expect(page.getByLabel('Instructions and notes')).toBeVisible();
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /Prompt Required/ })).toBeVisible();
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /^Files & Directories 0$/ })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Run task' })).toBeDisabled();
  await expect(page.locator('[data-task-run-requirement-message]')).toHaveCount(0);

  await page.getByLabel('Instructions and notes').fill('Use the API migration notes as candidate context.');
  await expect(page.getByRole('button', { name: 'Run task' })).toBeEnabled();
  await expect(page.locator('[data-task-run-requirement-message]')).toHaveCount(0);
});

test('tasks route uses Files for path-like task fields', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('code-to-record');
  await expect(page.getByLabel('Path')).toHaveCount(0);
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /^Files & Directories 0$/ })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Run task' })).toBeEnabled();
  await expect(page.locator('[data-task-run-requirement-message]')).toHaveCount(0);

  await selectContextTab(page, /Files & Directories/);
  await page.getByLabel('File path').fill('crates/dadrock/src/tasks/runner.rs');
  await page.getByRole('button', { name: 'Use file path' }).click();
  await expect(page.getByRole('button', { name: 'Run task' })).toBeEnabled();
});

test('tasks route explains missing record requirements', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('edit-record');
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /Records Required/ })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Run task' })).toBeDisabled();
  await expect(page.locator('[data-task-run-requirement-message]')).toHaveCount(0);
});

test('tasks route supports typed directory path context without suggestion add buttons', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('code-to-record');
  await expect(page.getByRole('combobox', { name: 'Task' })).toHaveValue('code-to-record');
  await openContextEditor(page);
  await selectContextTab(page, /Files & Directories/);
  await expect(page.locator('[data-task-context-nav]').getByRole('tab', { name: /^Directories/ })).toHaveCount(0);
  await expect(page.getByLabel('File path')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Select local folder', exact: true })).toBeVisible();
  await page.getByLabel('Directory path').fill('dadr/accepted');
  await expect(page.locator('[data-task-path-picker]').getByRole('button', { name: 'Add file' })).toHaveCount(0);
  await expect(page.locator('[data-task-path-picker]').getByRole('button', { name: 'Add folder' })).toHaveCount(0);
  await page.getByRole('button', { name: 'Use directory path' }).click();

  await expect(page.getByRole('button', { name: 'Remove path dadr/accepted' })).toBeVisible();
  await expect(page.locator('[data-task-context-editor]')).not.toContainText('1 item');
});

test('tasks route accepts a file from the system file picker', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('code-to-record');
  await expect(page.getByRole('combobox', { name: 'Task' })).toHaveValue('code-to-record');
  await openContextEditor(page);
  await selectContextTab(page, /Files & Directories/);
  await page.locator('input[aria-label="Select local files from system dialog"]').setInputFiles({
    name: 'context-note.md',
    mimeType: 'text/markdown',
    buffer: Buffer.from('# Context\n'),
  });

  await expect(page.getByRole('button', { name: 'Remove path context-note.md' })).toBeVisible();
  await expect(page.locator('[data-task-context-editor]')).not.toContainText('1 item');
});

test('tasks route accepts a folder from the system folder picker', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('code-to-record');
  await expect(page.getByRole('combobox', { name: 'Task' })).toHaveValue('code-to-record');
  await openContextEditor(page);
  await selectContextTab(page, /Files & Directories/);
  await page
    .locator('input[aria-label="Select local folder from system dialog"]')
    .setInputFiles(path.resolve('tests/fixtures/context-folder'));

  await expect(page.getByRole('button', { name: 'Remove path context-folder' })).toBeVisible();
  await expect(page.locator('[data-task-context-editor]')).not.toContainText('1 item');
});

test('tasks route defaults record picker status for record-scoped tasks', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('edit-record');
  await openContextEditor(page);
  await selectContextTab(page, /Records/);

  await expect(page.locator('[data-record-picker]')).toBeVisible();
  await page.getByRole('button', { name: 'Add records' }).click();
  const dialog = page.getByRole('dialog', { name: 'Add records' });
  await expect(dialog.getByLabel('Search scope')).toBeVisible();
  await expect(dialog.getByRole('button', { name: /^All$/ })).toHaveAttribute('aria-pressed', 'true');
  await expect(dialog.getByRole('listitem').filter({ hasText: 'Adopt pre-registration for studies' })).toBeVisible();
});

test('tasks results keep record return actions after contextual runs', async ({ page }) => {
  const recordId = '0193fb1f-a3a0-7000-8000-000000000002';
  await page.goto(`/tasks?task=edit-record&record=${recordId}`);

  await page.getByLabel('Instructions and notes').fill('Strengthen the alternatives section.');
  await page.getByRole('button', { name: 'Run task' }).click();

  await expect(page.getByRole('heading', { name: 'Mock revision generated.' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Back to record' })).toHaveAttribute(
    'href',
    `/records?id=${recordId}`,
  );
  await page.getByRole('button', { name: 'Run another task for this record' }).first().click();
  await expect(page.getByRole('tab', { name: 'Context' })).toHaveAttribute('aria-selected', 'true');
  await expect(page.getByRole('combobox', { name: 'Task' })).toHaveValue('edit-record');
});

test('candidate batch results use the same folded card stack', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('code-to-record');
  await page.getByRole('button', { name: 'Run task' }).click();

  await expect(page.getByText('Wrote 1 candidate from code-to-record.')).toHaveCount(0);
  await expect(page.locator('[data-task-result-meta]')).toContainText('Candidate batch');
  await expect(page.getByRole('region', { name: 'Result overview' })).toHaveCount(0);
  const summaryCard = page.locator('details[data-result-card]').filter({ hasText: 'Summary' });
  const candidateCard = page.locator('details[data-result-card]').filter({ hasText: 'Use explicit missing data policy' });
  await expect(summaryCard).toHaveCount(1);
  await expect(candidateCard).toHaveCount(1);
  await summaryCard.locator('summary').click();
  await expect(summaryCard).toHaveAttribute('open', '');
  await expect(candidateCard).not.toHaveAttribute('open', '');
  await expect(page.getByText('Record an explicit missing data policy before downstream modeling.')).not.toBeVisible();

  await candidateCard.locator('summary').click();
  await expect(summaryCard).not.toHaveAttribute('open', '');
  await expect(candidateCard).toHaveAttribute('open', '');
  await expect(page.getByText('Record an explicit missing data policy before downstream modeling.')).toBeVisible();
  await expect(candidateCard.getByRole('button', { name: 'Write candidate' })).toHaveCount(0);
  await expect(candidateCard.getByRole('button', { name: 'Refine candidate' })).toBeVisible();
});

test('tasks route can refine one candidate batch item with a note', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('code-to-record');
  await page.getByRole('button', { name: 'Run task' }).click();

  await expect(page.getByRole('heading', { name: 'Use explicit missing data policy' })).toBeVisible();
  const candidateCard = page.locator('details[data-result-card]').filter({ hasText: 'Use explicit missing data policy' });
  await candidateCard.locator('summary').click();
  await candidateCard.getByRole('button', { name: 'Refine candidate' }).click();
  await expect(page.locator('[data-task-follow-up-composer]')).toBeVisible();
  await page.getByLabel('Follow-up instructions').fill('Make the policy narrower.');
  await page.getByRole('button', { name: 'Run follow-up' }).click();

  await expect(page.getByRole('heading', { name: 'Task drafted candidate' })).toBeVisible();
  await expect(page.getByLabel('Draft candidate markdown')).toHaveValue(/Make the policy narrower\./);
});

test('tasks route lets users cancel a running agent task', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('custom');
  await page.getByLabel('Instructions and notes').fill('slow');
  const runButton = page.getByRole('button', { name: 'Run task' });
  await expect(runButton).toBeEnabled();
  await runButton.click();

  const statusPanel = page.getByRole('region', { name: 'Task status' });
  await expect(statusPanel).toContainText('Task custom is still running. Agent output is updating.');
  await expect(statusPanel.getByRole('button', { name: /Show agent output/ })).toHaveAttribute(
    'aria-expanded',
    'false',
  );
  await expect(page.getByText('Starting local ACP agent')).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Cancel task' })).toBeVisible();

  await page.getByRole('button', { name: 'Cancel task' }).click();

  await expect(page.getByText('Task custom cancelled.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Cancel task' })).not.toBeVisible();
  await page.getByRole('tab', { name: 'Results' }).click();
  await expect(
    page
      .locator('section')
      .filter({ has: page.getByRole('heading', { name: 'Result history' }) })
      .getByRole('button', { name: /custom/ }),
  ).toContainText('cancelled');
});

test('tasks results tab shows an in-progress state for running runs', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('custom');
  await page.getByLabel('Instructions and notes').fill('slow');
  await page.getByRole('button', { name: 'Run task' }).click();

  await expect(page.getByText('Task custom is still running. Agent output is updating.')).toBeVisible();
  await page.getByRole('tab', { name: 'Results' }).click();
  await page.getByRole('button', { name: /custom/ }).first().click();

  const resultPane = page.locator('[data-task-runs-result-pane]');
  await expect(resultPane).toContainText('Task custom is still running.');
  await expect(resultPane).toContainText('Results will appear here when the run finishes.');
  await expect(resultPane).not.toContainText('has no result');
  await expect(page.getByRole('region', { name: 'Task status' })).toContainText(
    'Task custom is still running.',
  );
});

test('tasks history can be cleared', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('find-conflicts');
  await page.getByRole('button', { name: 'Run task' }).click();
  await expect(page.getByText(/Loaded result/)).toBeVisible();

  await page.getByRole('tab', { name: 'Results' }).click();
  await page.getByRole('button', { name: /find-conflicts/ }).click();
  await expect(page.getByRole('region', { name: 'Task status' }).getByRole('button')).toHaveAttribute(
    'aria-expanded',
    'false',
  );
  await expect(page.getByText('Starting local ACP agent')).toHaveCount(0);
  await expect(page.getByRole('button', { name: /find-conflicts/ })).toBeVisible();
  await page.getByRole('button', { name: 'Clear history' }).click();

  await expect(page.getByText('No task results yet.')).toBeVisible();
  await expect(page.getByRole('region', { name: 'Task status' })).toContainText('Task history cleared.');
});

test('tasks route notices when a running agent task completes', async ({ page }) => {
  await page.goto('/tasks');

  await page.getByRole('combobox', { name: 'Task' }).selectOption('custom');
  await page.getByLabel('Instructions and notes').fill('complete-slow');
  const runButton = page.getByRole('button', { name: 'Run task' });
  await expect(runButton).toBeEnabled();
  await runButton.click();

  await expect(page.getByText('Task custom is still running. Agent output is updating.')).toBeVisible();
  await expect(page.getByText(/Loaded result/)).toBeVisible();
  await expect(page.getByRole('tab', { name: 'Results' })).toHaveAttribute('aria-selected', 'true');
  await page.locator('details[data-result-card]').first().locator('summary').click();
  await expect(page.getByText('complete-slow', { exact: true })).toBeVisible();

  await page.getByRole('tab', { name: 'Results' }).click();
  await expect(page.getByRole('button', { name: /custom/ }).first()).toContainText('succeeded');
});
