import { expect, test } from '@playwright/test';

test('validation route renders all diagnostic panels from the snapshot', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 700 });
  await page.goto('/validation');

  await expect(page.locator('[data-workbench-page][data-layout="two-pane"]')).toBeVisible();
  await expect(page.locator('[data-resizable-workbench-split="validation"]')).toBeVisible();
  await expect(page.locator('[data-validation-nav-pane]')).toHaveCSS('border-top-width', '1px');
  await expect(page.locator('[data-validation-detail-pane]')).toHaveCSS('border-top-width', '1px');
  await expect(page.getByRole('separator', { name: 'Resize validation panes' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Issue queue' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Issue detail' })).toBeVisible();
  await expect(page.locator('[data-validation-issue-list]')).toBeVisible();
  await expect(page.locator('[data-ui-segmented-tabs]')).toHaveCount(0);
  await expect(page.getByRole('tab', { name: 'Summary' })).toHaveCount(0);
  await expect(page.getByText('Project health, links, settings, and audit signals.')).not.toBeVisible();
  await expect(page.getByRole('button', { name: /^Project \d+$/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /^Records \d+$/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /^Links \d+$/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /^Settings \d+$/ })).toBeVisible();
  await expect(page.getByRole('button', { name: /^Audit \d+$/ })).toBeVisible();

  await expect(page.locator('[data-validation-issue-list]').getByRole('button', { name: /candidate_backlog/ })).toBeVisible();
  const brokenLinkIssue = page.locator('[data-validation-issue-list]').getByRole('button', { name: /broken_markdown_link/ });
  await expect(brokenLinkIssue).toContainText('Links');
  await expect(brokenLinkIssue).not.toContainText('A local Markdown link points at a missing note.');
  await page.locator('[data-validation-issue-list]').getByRole('button', { name: /broken_markdown_link/ }).click();
  const detailPane = page.locator('[data-validation-detail-pane]');
  await expect(detailPane).toContainText('broken_markdown_link');
  await expect(detailPane).toContainText('0001-use-polars.md - line 12');
  await expect(detailPane).not.toContainText('dadr/accepted/0001-use-polars.md');

  const resizeHandle = page.getByRole('separator', { name: 'Resize validation panes' });
  const beforeResize = await page.evaluate(() => {
    const nav = document.querySelector('[data-validation-nav-pane]');
    const detail = document.querySelector('[data-validation-detail-pane]');
    if (!nav || !detail) throw new Error('missing validation panes');
    return {
      navWidth: nav.getBoundingClientRect().width,
      detailWidth: detail.getBoundingClientRect().width,
    };
  });
  const handleBox = await resizeHandle.boundingBox();
  if (!handleBox) throw new Error('missing validation resize handle');
  await page.mouse.move(handleBox.x + handleBox.width / 2, handleBox.y + handleBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(handleBox.x + handleBox.width / 2 + 96, handleBox.y + handleBox.height / 2);
  await page.mouse.up();
  const afterResize = await page.evaluate(() => {
    const nav = document.querySelector('[data-validation-nav-pane]');
    const detail = document.querySelector('[data-validation-detail-pane]');
    if (!nav || !detail) throw new Error('missing validation panes');
    return {
      navWidth: nav.getBoundingClientRect().width,
      detailWidth: detail.getBoundingClientRect().width,
    };
  });
  expect(afterResize.navWidth).toBeGreaterThan(beforeResize.navWidth + 50);
  expect(afterResize.detailWidth).toBeLessThan(beforeResize.detailWidth - 50);

  const metrics = await page.evaluate(() => {
    const main = document.querySelector('main');
    const nav = document.querySelector('[data-validation-nav-pane]');
    const list = document.querySelector('[data-validation-issue-list]');
    const detail = document.querySelector('[data-validation-detail-pane]');
    if (!main || !nav || !list || !detail) throw new Error('missing validation panes');
    return {
      mainOverflowY: getComputedStyle(main).overflowY,
      mainClientHeight: main.clientHeight,
      mainScrollHeight: main.scrollHeight,
      navOverflowY: getComputedStyle(nav).overflowY,
      listOverflowY: getComputedStyle(list).overflowY,
      detailOverflowY: getComputedStyle(detail).overflowY,
    };
  });

  expect(metrics.mainOverflowY).toBe('hidden');
  expect(metrics.mainScrollHeight).toBe(metrics.mainClientHeight);
  expect(metrics.navOverflowY).toBe('hidden');
  expect(metrics.listOverflowY).toBe('auto');
  expect(metrics.detailOverflowY).toBe('auto');

  await page.getByRole('button', { name: /^Links \d+$/ }).click();
  await expect(page).toHaveURL(/\/validation\?tab=links$/);
  await expect(page.locator('[data-validation-issue-list]').getByRole('button', { name: /broken_markdown_link/ })).toBeVisible();

  await page.getByRole('button', { name: /^Settings \d+$/ }).click();
  await expect(page.locator('[data-validation-issue-list]').getByRole('button', { name: /tag_catalog_empty/ })).toBeVisible();
});

test('validation two-pane layout stays horizontal in VS Code sized webviews', async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 700 });
  await page.goto('/validation');

  await expect(page.getByRole('separator', { name: 'Resize validation panes' })).toBeVisible();
  const metrics = await page.evaluate(() => {
    const nav = document.querySelector('[data-validation-nav-pane]');
    const detail = document.querySelector('[data-validation-detail-pane]');
    if (!nav || !detail) throw new Error('missing validation panes');
    const navBox = nav.getBoundingClientRect();
    const detailBox = detail.getBoundingClientRect();
    return {
      navX: navBox.x,
      detailX: detailBox.x,
      navY: navBox.y,
      detailY: detailBox.y,
    };
  });

  expect(metrics.detailX).toBeGreaterThan(metrics.navX + 250);
  expect(Math.abs(metrics.detailY - metrics.navY)).toBeLessThanOrEqual(2);
});

test('validation route can run a validation target and reports the result', async ({ page }) => {
  await page.goto('/validation?tab=links');

  await page.getByRole('button', { name: 'Run link validation' }).click();

  const toast = page.getByRole('status').filter({ hasText: 'links validation finished' });
  await expect(toast).toBeVisible();
  await expect(page.locator('[data-validation-detail-pane]')).not.toContainText('links validation finished');
  await expect(toast).toHaveCount(0, { timeout: 5000 });
});

test('audit issue detail links record diagnostics to manual and agent fixes', async ({ page }) => {
  const recordId = '0193fb1f-a3a0-7000-8000-000000000001';
  await page.goto('/validation?tab=links');

  const detailPane = page.locator('[data-validation-detail-pane]');
  const issueHeader = detailPane.locator('header');

  await expect(issueHeader.getByRole('link', { name: 'Open record' })).toHaveAttribute(
    'href',
    `/records?id=${recordId}&tab=edit`,
  );
  await expect(issueHeader.getByRole('link', { name: 'Fix record with agent' })).toHaveAttribute(
    'href',
    new RegExp(`/tasks\\?task=edit-record&record=${recordId}&path=dadr%2Faccepted%2F0001-use-polars\\.md&instructions=`),
  );
  await expect(issueHeader.getByRole('link', { name: 'Investigate with agent' })).toHaveAttribute(
    'href',
    new RegExp(`/tasks\\?task=custom&record=${recordId}&path=dadr%2Faccepted%2F0001-use-polars\\.md&instructions=`),
  );
  await expect(
    detailPane.locator('article').filter({ hasText: 'broken_markdown_link' }).getByRole('link', {
      name: 'Fix record with agent',
    }),
  ).toBeVisible();
});

test('audit and links routes redirect into validation tabs', async ({ page }) => {
  await page.goto('/links');
  await expect(page).toHaveURL(/\/validation\?tab=links$/);
  await expect(page.getByRole('button', { name: /^Links \d+$/ })).toHaveAttribute('aria-pressed', 'true');

  await page.goto('/audit');
  await expect(page).toHaveURL(/\/validation\?tab=audit$/);
  await expect(page.getByRole('button', { name: /^Audit \d+$/ })).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByRole('button', { name: /^Links \d+$/ })).toBeVisible();
});
