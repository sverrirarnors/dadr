import { expect, test } from '@playwright/test';

test('preview launch prompts to initialize an uninitialized project', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('dadrock:mock-initialized', 'false');
  });

  await page.goto('/');

  await expect(page).toHaveURL(/\/records\?status=candidate$/);
  await expect(page.getByRole('heading', { name: 'Initialize Dadrock here?' })).toBeVisible();
  await expect(page.getByText('No Dadrock project was found in')).toBeVisible();
  await expect(page.getByText('/tmp/dadr-mock', { exact: true })).toHaveCount(2);
  await expect(page.getByText('dadr/accepted/')).toBeVisible();
  await expect(page.getByText('dadr/rejected/')).toBeVisible();
  await expect(page.getByText('dadr/runs/')).toBeVisible();

  await page.getByRole('button', { name: 'Initialize here' }).click();

  await expect(page.getByRole('status').filter({ hasText: 'created 8 paths, reused 0 paths' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Initialize Dadrock here?' })).toHaveCount(0);
  await expect(page.getByRole('link', { name: /Adopt pre-registration for studies/ })).toBeVisible();
});
