import { expect, test } from '@playwright/test';

const candidateId = '0193fb1f-a3a0-7000-8000-000000000002';

test('records route reads selected records inline in a split view', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/records');

  await expect(page.locator('[data-workbench-page][data-layout="two-pane"]')).toBeVisible();
  await expect(page.locator('[data-workbench-pane]').first()).toBeVisible();
  await expect(page.locator('[data-workbench-document-pane]')).toBeVisible();
  await expect(page.getByLabel('Search scope')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Use Polars over pandas' }).first()).toBeVisible();
  await expect(page.getByRole('article')).toContainText('Use Polars for tabular transformations.');
  await expect(page.getByText('0001-use-polars.md')).toBeVisible();
  await expect(page.locator('[data-record-status-filter]')).toBeVisible();
  await expect(
    page.locator('[data-workbench-pane]').first().locator('[data-action-menu]').getByRole('button', { name: 'New' }),
  ).toBeVisible();

  const statusFilterFits = await page.locator('[data-record-status-filter]').evaluate((element) => {
    return element.scrollWidth <= element.clientWidth + 1;
  });
  expect(statusFilterFits).toBe(true);

  const firstRow = page.getByRole('link', { name: /Use Polars over pandas/ }).first();
  await expect(firstRow).not.toContainText('dadr/accepted/0001-use-polars.md');
  await expect(firstRow).not.toContainText('0193fb1fa3a0');

  await page.getByRole('link', { name: /Adopt pre-registration for studies/ }).click();

  await expect(page).toHaveURL(new RegExp(`/records\\?id=${candidateId}$`));
  await expect(page.getByRole('heading', { name: 'Adopt pre-registration for studies' }).first()).toBeVisible();
  await expect(page.getByRole('article')).toContainText('Candidate decision for pre-registration practice.');
});

test('records split view stays horizontal in VS Code sized webviews', async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 700 });
  await page.goto('/records');

  await expect(page.locator('[data-record-detail-pane]')).toBeVisible();
  await expect(page.getByRole('separator', { name: 'Resize records panes' })).toBeVisible();

  const metrics = await page.evaluate(() => {
    const nav = document.querySelector('[data-workbench-pane]');
    const detail = document.querySelector('[data-record-detail-pane]');
    if (!nav || !detail) throw new Error('missing records panes');
    const navBox = nav.getBoundingClientRect();
    const detailBox = detail.getBoundingClientRect();
    return {
      navX: navBox.x,
      detailX: detailBox.x,
      navY: navBox.y,
      detailY: detailBox.y,
      detailDisplay: getComputedStyle(detail).display,
    };
  });

  expect(metrics.detailDisplay).not.toBe('none');
  expect(metrics.detailX).toBeGreaterThan(metrics.navX + 250);
  expect(Math.abs(metrics.detailY - metrics.navY)).toBeLessThanOrEqual(2);
});

test('records route uses shared action menus for new, edit, and supersede families', async ({ page }) => {
  const candidateId = '0193fb1f-a3a0-7000-8000-000000000002';
  const acceptedId = '0193fb1f-a3a0-7000-8000-000000000003';
  await page.setViewportSize({ width: 1440, height: 900 });

  await page.goto('/records');
  const newMenu = page.locator('[data-workbench-pane]').first().locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'New' }) });
  await newMenu.getByRole('button', { name: 'New' }).click();
  await expect(newMenu.getByRole('menuitem', { name: 'New', exact: true })).toHaveAttribute('href', '/candidates/new');
  await expect(newMenu.getByRole('menuitem', { name: 'New with agent' })).toHaveAttribute('href', '/candidates/new?mode=agent');

  await page.goto(`/records?status=candidate&id=${candidateId}`);
  const editMenu = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Edit' }) });
  await editMenu.getByRole('button', { name: 'Edit' }).click();
  await expect(editMenu.getByRole('menuitem', { name: 'Edit', exact: true })).toBeVisible();
  await expect(editMenu.getByRole('menuitem', { name: 'Edit with agent' })).toHaveAttribute(
    'href',
    `/tasks?task=edit-record&record=${candidateId}`,
  );

  await page.goto(`/records?id=${acceptedId}`);
  const supersedeMenu = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Change this decision' }) });
  await supersedeMenu.getByRole('button', { name: 'Change this decision' }).click();
  await expect(supersedeMenu.getByRole('menuitem', { name: 'Replace with existing decisions' })).toBeVisible();
  await expect(supersedeMenu.getByRole('menuitem', { name: 'Draft a replacement', exact: true })).toHaveAttribute(
    'href',
    `/candidates/new?supersedes=${acceptedId}`,
  );
  await expect(supersedeMenu.getByRole('menuitem', { name: 'Draft a replacement with agent' })).toHaveAttribute(
    'href',
    `/tasks?task=supersede-record&record=${acceptedId}`,
  );
});

test('records route offers edit with agent for accepted and rejected records', async ({ page }) => {
  const acceptedId = '0193fb1f-a3a0-7000-8000-000000000003';
  await page.setViewportSize({ width: 1440, height: 900 });

  await page.goto(`/records?status=candidate&id=${candidateId}`);
  await page.getByRole('button', { name: 'Reject', exact: true }).click();
  await expect(page).toHaveURL(/\/records\?status=candidate$/);

  for (const recordId of [acceptedId, candidateId]) {
    await page.goto(`/records?id=${recordId}`);
    const editMenu = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Edit' }) });
    await editMenu.getByRole('button', { name: 'Edit' }).click();
    await expect(editMenu.getByRole('menuitem', { name: 'Edit', exact: true })).toBeVisible();
    await expect(editMenu.getByRole('menuitem', { name: 'Edit with agent' })).toHaveAttribute(
      'href',
      `/tasks?task=edit-record&record=${recordId}`,
    );
  }
});

test('records route shows candidate actions in the detail pane', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/records?status=candidate&id=${candidateId}`);

  await expect(page.getByRole('heading', { name: 'Adopt pre-registration for studies' }).first()).toBeVisible();
  await expect(page.getByRole('button', { name: 'Accept', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Reject', exact: true })).toBeVisible();
});

test('records route updates detail actions after accepting a candidate', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/records?status=candidate&id=${candidateId}`);
  await page.getByRole('button', { name: 'Accept', exact: true }).click();

  await expect(page).toHaveURL(`/records?id=${candidateId}`);
  await expect(page.getByRole('button', { name: 'Accept', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Reject', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Change this decision', exact: true })).toBeVisible();
});

test('records route links candidate records to the revise task', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/records?status=candidate&id=${candidateId}`);

  const menu = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Edit' }) });
  await menu.getByRole('button', { name: 'Edit' }).click();
  const action = menu.getByRole('menuitem', { name: 'Edit with agent' });
  await expect(action).toBeVisible();
  await expect(action.locator('.lucide-sparkles')).toBeVisible();
  await expect(action).toHaveAttribute('href', `/tasks?task=edit-record&record=${candidateId}`);

  await action.click();

  await expect(page).toHaveURL(new RegExp(`/tasks\\?task=edit-record&record=${candidateId}$`));
});

test('records route links accepted records to the supersession task', async ({ page }) => {
  const acceptedId = '0193fb1f-a3a0-7000-8000-000000000003';
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/records?id=${acceptedId}`);

  const menu = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Change this decision' }) });
  await menu.getByRole('button', { name: 'Change this decision' }).click();
  const action = menu.getByRole('menuitem', { name: 'Draft a replacement with agent' });
  await expect(action).toBeVisible();
  await expect(action.locator('.lucide-sparkles')).toBeVisible();
  await expect(action).toHaveAttribute('href', `/tasks?task=supersede-record&record=${acceptedId}`);

  await action.click();

  await expect(page).toHaveURL(new RegExp(`/tasks\\?task=supersede-record&record=${acceptedId}$`));
});

test('records route groups edit and supersede actions consistently', async ({ page }) => {
  const candidateId = '0193fb1f-a3a0-7000-8000-000000000002';
  const acceptedId = '0193fb1f-a3a0-7000-8000-000000000003';
  await page.setViewportSize({ width: 1440, height: 900 });

  await page.goto(`/records?status=candidate&id=${candidateId}`);
  const editMenuButton = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Edit' }) }).getByRole('button', { name: 'Edit' });
  await expect(editMenuButton).toBeVisible();
  await editMenuButton.click();
  await expect(page.getByRole('menuitem', { name: 'Edit', exact: true })).toBeVisible();
  await expect(page.getByRole('menuitem', { name: 'Edit with agent' })).toBeVisible();

  await page.goto(`/records?id=${acceptedId}`);
  const supersedeMenuButton = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Change this decision' }) }).getByRole('button', { name: 'Change this decision' });
  await expect(supersedeMenuButton).toBeVisible();
  await supersedeMenuButton.click();
  const supersedeExisting = page.getByRole('menuitem', { name: 'Replace with existing decisions' });
  const supersedeNew = page.getByRole('menuitem', { name: 'Draft a replacement', exact: true });
  const supersedeAgent = page.getByRole('menuitem', { name: 'Draft a replacement with agent' });
  await expect(supersedeNew).toHaveAttribute('href', `/candidates/new?supersedes=${acceptedId}`);
  await expect(supersedeExisting).toBeVisible();
  await expect(supersedeAgent).toBeVisible();
});

test('records route can supersede with existing accepted records', async ({ page }) => {
  const acceptedId = '0193fb1f-a3a0-7000-8000-000000000003';
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/records?id=${acceptedId}`);

  const menu = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Change this decision' }) });
  await menu.getByRole('button', { name: 'Change this decision' }).click();
  await menu.getByRole('menuitem', { name: 'Replace with existing decisions' }).click();

  const dialog = page.getByRole('dialog', { name: 'Replace with existing decisions' });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Cancel' })).toHaveCount(1);
  await expect(dialog.getByLabel('Search scope')).toBeVisible();
  await expect(dialog.getByRole('button', { name: /Accepted/ })).toHaveAttribute('aria-pressed', 'true');
  await dialog.getByLabel('Search scope').selectOption('title');
  await dialog.getByRole('searchbox', { name: 'Search records' }).fill('Arrow IPC snapshots');
  await dialog.getByRole('checkbox', { name: 'Select Use Arrow IPC snapshots' }).check();
  await dialog.getByRole('button', { name: 'Save' }).click();

  await expect(page.getByText('record superseded')).toBeVisible();
  await expect(page).toHaveURL(`/records?status=superseded&id=${acceptedId}`);
});

test('records route starts a new candidate from supersede with new record', async ({ page }) => {
  const acceptedId = '0193fb1f-a3a0-7000-8000-000000000003';
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/records?id=${acceptedId}`);

  const menu = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Change this decision' }) });
  await menu.getByRole('button', { name: 'Change this decision' }).click();
  await menu.getByRole('menuitem', { name: 'Draft a replacement', exact: true }).click();

  await expect(page).toHaveURL(`/candidates/new?supersedes=${acceptedId}`);
  await expect(page.getByRole('textbox', { name: 'Title' })).toHaveValue(/Use Polars lazy frames/);
  await expect(page.getByRole('textbox', { name: 'Tags' })).toHaveValue(/#dadr\/data\/processing/);
  await expect(page.getByLabel('Candidate Markdown')).toHaveValue(/supersede Use Polars lazy frames/);
});

test('shared record picker focuses search and closes without committing', async ({ page }) => {
  const acceptedId = '0193fb1f-a3a0-7000-8000-000000000003';
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/records?id=${acceptedId}`);

  const menu = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Change this decision' }) });
  await menu.getByRole('button', { name: 'Change this decision' }).click();
  await menu.getByRole('menuitem', { name: 'Replace with existing decisions' }).click();

  const dialog = page.getByRole('dialog', { name: 'Replace with existing decisions' });
  const search = dialog.getByRole('searchbox', { name: 'Search records' });
  await expect(search).toBeFocused();

  await page.keyboard.press('Escape');
  await expect(dialog).not.toBeVisible();
  await expect(page.getByText('record superseded')).not.toBeVisible();

  await menu.getByRole('button', { name: 'Change this decision' }).click();
  await menu.getByRole('menuitem', { name: 'Replace with existing decisions' }).click();
  await expect(dialog).toBeVisible();
  await page.mouse.click(12, 12);
  await expect(dialog).not.toBeVisible();
  await expect(page.getByText('record superseded')).not.toBeVisible();
});

test('records route aligns candidate actions with the detail view controls', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/records?status=candidate&id=${candidateId}`);

  const edit = page.getByRole('button', { name: 'Edit' });
  const accept = page.getByRole('button', { name: 'Accept', exact: true });
  const reject = page.getByRole('button', { name: 'Reject', exact: true });
  await expect(edit).toBeVisible();
  await expect(accept).toBeVisible();
  await expect(reject).toBeVisible();
  await expect(page.locator('[data-workbench-action-bar]')).toHaveCSS('border-top-width', '0px');

  const [editBox, acceptBox, rejectBox] = await Promise.all([
    edit.boundingBox(),
    accept.boundingBox(),
    reject.boundingBox(),
  ]);
  if (!editBox || !acceptBox || !rejectBox) throw new Error('expected toolbar control boxes');
  expect(Math.abs(acceptBox.y - editBox.y)).toBeLessThanOrEqual(2);
  expect(Math.abs(rejectBox.y - editBox.y)).toBeLessThanOrEqual(2);
});

test('records route keeps shell fixed and scrolls within panes', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 700 });
  await page.goto('/records');

  await expect(page.getByLabel('Search scope')).toBeVisible();

  const metrics = await page.evaluate(() => {
    const main = document.querySelector('main');
    const list = document.querySelector('[data-records-list-pane]');
    const detail = document.querySelector('[data-record-detail-pane]');
    if (!main || !list || !detail) throw new Error('missing records panes');
    const article = document.querySelector('[data-record-rendered-pane]');
    if (!article) throw new Error('missing rendered content pane');
    return {
      mainOverflowY: getComputedStyle(main).overflowY,
      mainClientHeight: main.clientHeight,
      mainScrollHeight: main.scrollHeight,
      listOverflowY: getComputedStyle(list).overflowY,
      detailOverflowY: getComputedStyle(detail).overflowY,
      renderedOverflowY: getComputedStyle(article).overflowY,
    };
  });

  expect(metrics.mainOverflowY).toBe('hidden');
  expect(metrics.mainScrollHeight).toBe(metrics.mainClientHeight);
  expect(metrics.listOverflowY).toBe('auto');
  expect(metrics.detailOverflowY).toBe('hidden');
  expect(metrics.renderedOverflowY).toBe('auto');
});

test('records route scrolls within the edit pane instead of the detail shell', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 700 });
  await page.goto('/records');
  const editMenu = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Edit' }) });
  await editMenu.getByRole('button', { name: 'Edit' }).click();
  await editMenu.getByRole('menuitem', { name: 'Edit', exact: true }).click();

  const metrics = await page.evaluate(() => {
    const detail = document.querySelector('[data-record-detail-pane]');
    const editor = document.querySelector('[data-record-editor-pane]');
    if (!detail || !editor) throw new Error('missing edit panes');
    return {
      detailOverflowY: getComputedStyle(detail).overflowY,
      editorOverflowY: getComputedStyle(editor).overflowY,
      editorClientHeight: editor.clientHeight,
      detailClientHeight: detail.clientHeight,
    };
  });

  expect(metrics.detailOverflowY).toBe('hidden');
  expect(metrics.editorOverflowY).toBe('auto');
  expect(metrics.editorClientHeight).toBeLessThan(metrics.detailClientHeight);
});

test('records route lets users drag resize the split panes', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/records');

  await expect(page.locator('[data-resizable-workbench-split="records"]')).toBeVisible();
  const list = page.locator('[data-records-list-pane]');
  const detail = page.locator('[data-record-detail-pane]');
  const handle = page.getByRole('separator', { name: 'Resize records panes' });
  await expect(handle).toBeVisible();

  const [beforeList, beforeDetail, handleBox] = await Promise.all([
    list.boundingBox(),
    detail.boundingBox(),
    handle.boundingBox(),
  ]);
  if (!beforeList || !beforeDetail || !handleBox) throw new Error('expected records pane boxes');

  await page.mouse.move(handleBox.x + handleBox.width / 2, handleBox.y + handleBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(handleBox.x + handleBox.width / 2 + 120, handleBox.y + handleBox.height / 2, {
    steps: 5,
  });
  await page.mouse.up();

  const [afterList, afterDetail] = await Promise.all([list.boundingBox(), detail.boundingBox()]);
  if (!afterList || !afterDetail) throw new Error('expected resized records pane boxes');
  expect(afterList.width).toBeGreaterThan(beforeList.width + 80);
  expect(afterDetail.width).toBeLessThan(beforeDetail.width - 80);
});

test('records route opens full-screen detail on narrow screens', async ({ page }) => {
  await page.setViewportSize({ width: 600, height: 700 });
  await page.goto('/records');

  await expect(page.getByLabel('Search scope')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Use Polars over pandas' }).first()).not.toBeVisible();

  const listPane = page.locator('[data-workbench-pane]').first();
  const listBox = await listPane.boundingBox();
  if (!listBox) throw new Error('expected records list pane box');
  expect(listBox.width).toBeGreaterThan(560);

  await page.getByRole('link', { name: /Use Polars over pandas/ }).click();

  await expect(page).toHaveURL(/\/records\/0193fb1f-a3a0-7000-8000-000000000001$/);
  const backLink = page.getByRole('link', { name: 'Back to records' });
  await expect(backLink).toBeVisible();
  await expect(backLink.locator('.lucide-arrow-left')).toBeVisible();
  await expect(page.getByRole('article')).toContainText('Use Polars for tabular transformations.');
});

test('records search scope limits matching fields', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/records');

  await page.getByLabel('Search scope').selectOption('title');
  await page.getByPlaceholder('Search title, path, tag, or id').fill('0001-use-polars');
  await expect(page.getByRole('link', { name: /Use Polars over pandas/ })).not.toBeVisible();

  await page.getByLabel('Search scope').selectOption('file');
  await expect(page.getByRole('link', { name: /Use Polars over pandas/ })).toBeVisible();
});
