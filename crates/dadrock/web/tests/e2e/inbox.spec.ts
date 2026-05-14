import { expect, test } from '@playwright/test';

test('inbox lists candidate records from the project snapshot', async ({ page }) => {
  await page.goto('/inbox');

  await expect(page).toHaveURL(/\/records\?status=candidate$/);
  await expect(page.getByLabel('Search scope')).toBeVisible();
  await expect(page.getByRole('link', { name: /Adopt pre-registration for studies/ })).toBeVisible();
  await expect(page.getByText('0002-preregister.md')).toBeVisible();
  await expect(page.getByRole('link', { name: /Use Polars over pandas/ })).not.toBeVisible();
});

test('inbox candidate rows and actions navigate to candidate workflows', async ({ page }) => {
  await page.goto('/inbox');

  await expect(page).toHaveURL(/\/records\?status=candidate$/);
  await page.getByRole('link', { name: /Adopt pre-registration for studies/ }).click();
  await expect(page).toHaveURL(/\/records\?status=candidate&id=0193fb1f-a3a0-7000-8000-000000000002$/);
  await expect(page.getByRole('article')).toContainText('Candidate decision for pre-registration practice.');

  const newMenu = page.locator('[data-workbench-pane]').first().locator('[data-action-menu]');
  await newMenu.getByRole('button', { name: 'New' }).click();
  await newMenu.getByRole('menuitem', { name: 'New', exact: true }).click();
  await expect(page).toHaveURL(/\/candidates\/new$/);
});
