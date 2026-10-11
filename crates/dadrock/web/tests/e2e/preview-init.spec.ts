import { expect, test } from '@playwright/test';

test('preview launch prompts to initialize an uninitialized project', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('dadrock:mock-initialized', 'false');
  });

  await page.goto('/');

  await expect(page).toHaveURL(/\/inbox$/);
  await expect(page.getByRole('heading', { name: 'Initialize Dadrock here?' })).toBeVisible();
  await expect(page.getByText('No Dadrock project was found in')).toBeVisible();
  await expect(page.getByText('/tmp/dadr-mock', { exact: true })).toHaveCount(2);
  await expect(page.getByText('dadr/inbox/')).toBeVisible();
  await expect(page.getByText('dadr/ledger/')).toBeVisible();
  await expect(page.getByText('dadr/archive/')).toBeVisible();
  await expect(page.getByText('dadr/runs/')).toBeVisible();

  const draft = page.locator('[data-app-header]').getByRole('link', { name: 'Draft' });
  await expect(draft).toHaveAttribute('aria-disabled', 'true');

  await page.getByRole('button', { name: 'Initialize here' }).click();

  await expect(page.getByRole('status').filter({ hasText: 'created 8 paths, reused 0 paths' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Initialize Dadrock here?' })).toHaveCount(0);
  await expect(page.getByText('Adopt pre-registration for studies').first()).toBeVisible();
  await expect(draft).not.toHaveAttribute('aria-disabled', 'true');
});
