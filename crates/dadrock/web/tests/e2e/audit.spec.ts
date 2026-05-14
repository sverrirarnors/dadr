import { expect, test } from '@playwright/test';

test('audit route renders a project timeline from snapshot data', async ({ page }) => {
  await page.goto('/audit');

  await expect(page).toHaveURL(/\/validation\?tab=audit$/);
  await expect(page.getByText('Project health, links, settings, and audit signals.')).not.toBeVisible();
  await expect(page.getByRole('button', { name: /^Audit \d+$/ })).toHaveAttribute('aria-pressed', 'true');
  await expect(page.getByText('Project discovered. Commands operate on the local decision records.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Records' })).toBeVisible();
  await expect(page.getByText('broken_markdown_link')).toBeVisible();
});
