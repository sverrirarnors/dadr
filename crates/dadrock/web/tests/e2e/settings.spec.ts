import { expect, test } from '@playwright/test';

test('settings route shows project and configuration state', async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 420 });
  await page.goto('/settings');

  await expect(page.locator('[data-workbench-page][data-layout="single"]')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Settings inspector' })).toHaveCount(0);
  await expect(page.locator('[data-ui-key-value-grid]')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Settings', exact: true })).toHaveCount(0);
  await expect(page.locator('[data-workbench-pane]')).toHaveCSS('border-top-width', '0px');
  await expect(page.getByText('/tmp/dadr-mock', { exact: true })).toBeVisible();
  await expect(page.getByText('/tmp/dadr-mock/dadr/accepted', { exact: true })).toBeVisible();
  await expect(page.getByText('Project discovered. Commands operate on the local decision records.')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Agent' })).toBeVisible();
  await expect(page.getByText('tag_catalog_empty')).toBeVisible();

  const metrics = await page.evaluate(() => {
    const main = document.querySelector('main');
    const pane = document.querySelector('[data-settings-pane]');
    const content = document.querySelector('[data-settings-content]');
    if (!main || !pane || !content) throw new Error('missing settings shell');
    return {
      mainOverflowY: getComputedStyle(main).overflowY,
      mainClientHeight: main.clientHeight,
      mainScrollHeight: main.scrollHeight,
      paneOverflowY: getComputedStyle(pane).overflowY,
      contentOverflowY: getComputedStyle(content).overflowY,
    };
  });

  expect(metrics.mainOverflowY).toBe('hidden');
  expect(metrics.mainScrollHeight).toBe(metrics.mainClientHeight);
  expect(metrics.paneOverflowY).toBe('hidden');
  expect(metrics.contentOverflowY).toBe('auto');
});
