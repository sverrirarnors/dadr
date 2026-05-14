import { expect, test } from '@playwright/test';

test('links route renders link health diagnostics', async ({ page }) => {
  await page.goto('/links');

  await expect(page).toHaveURL(/\/validation\?tab=links$/);
  await expect(page.getByRole('button', { name: /^Links \d+$/ })).toHaveAttribute('aria-pressed', 'true');
  const detailPane = page.locator('[data-validation-detail-pane]');
  await expect(detailPane.getByRole('heading', { name: 'broken_markdown_link' })).toBeVisible();
  await expect(detailPane).toContainText('0001-use-polars.md');
  await expect(detailPane).toContainText('./missing-note.md');
});

test('links route can refresh link validation', async ({ page }) => {
  await page.goto('/links');

  await page.getByRole('button', { name: 'Run link validation' }).click();

  await expect(page.getByRole('status').filter({ hasText: 'links validation finished' })).toBeVisible();
});
