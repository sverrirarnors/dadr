import { expect, test } from '@playwright/test';

test('default route redirects to inbox and chrome renders', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveURL(/\/records\?status=candidate$/);
  await expect(page.locator('[data-app-shell]')).toBeVisible();
  await expect(page.locator('[data-app-header]')).toBeVisible();
  await expect(page.getByLabel('Search scope')).toBeVisible();
  await expect(page.getByLabel('Toggle sidebar')).not.toBeVisible();
  await expect(page.locator('aside')).toHaveCount(0);
  await expect(page.locator('header').getByRole('link', { name: 'Browse records' })).toBeVisible();
  await expect(page.getByLabel('Open command palette')).not.toBeVisible();
  await expect(page.getByLabel(/theme/i)).toBeVisible();
});

test('command palette opens with keyboard shortcut and navigates', async ({ page }) => {
  await page.goto('/inbox');
  await page.locator('body').click();
  await page.keyboard.press('Control+K');
  await expect(page.getByRole('dialog', { name: 'Command palette' })).toBeVisible();
  await page.getByPlaceholder('Search commands...').fill('records');
  await page.getByRole('button', { name: 'Go to Records' }).click();
  await expect(page).toHaveURL(/\/records$/);
});

test('command palette searches records', async ({ page }) => {
  await page.goto('/inbox');
  await page.locator('body').click();
  await page.keyboard.press('Control+K');
  await page.getByPlaceholder('Search commands...').fill('polars');
  await page.getByRole('button', { name: /Use Polars over pandas/ }).click();
  await expect(page).toHaveURL(/\/records\?id=0193fb1f-a3a0-7000-8000-000000000001$/);
});

test('command palette runs actions', async ({ page }) => {
  await page.goto('/inbox');
  await page.locator('body').click();
  await page.keyboard.press('Control+K');
  await page.getByPlaceholder('Search commands...').fill('toggle theme');
  await page.getByRole('button', { name: 'Toggle Theme' }).click();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'light');

  await page.keyboard.press('Control+K');
  await page.getByPlaceholder('Search commands...').fill('run validation');
  await page.getByRole('button', { name: 'Run Validation' }).click();
  await expect(page.getByRole('status').filter({ hasText: 'project validation finished' })).toBeVisible();
});

test('theme toggle cycles and persists', async ({ page }) => {
  await page.goto('/inbox');
  const html = page.locator('html');
  await expect(html).toHaveAttribute('data-theme', 'system');
  await page.getByLabel(/theme/i).click();
  await expect(html).toHaveAttribute('data-theme', 'light');
  await page.reload();
  await expect(html).toHaveAttribute('data-theme', 'light');
});

test('navbar icons remain the only page selector', async ({ page }) => {
  await page.goto('/inbox');

  await expect(page.locator('aside')).toHaveCount(0);
  await expect(page.getByLabel('Toggle sidebar')).not.toBeVisible();
  await expect(page.getByRole('navigation', { name: 'Primary navigation' })).toHaveCount(1);
  const taskLink = page.locator('header').getByRole('link', { name: 'Task runner' });
  await expect(taskLink).toBeVisible();
  await taskLink.hover();
  await expect(taskLink.getByText('Tasks')).toBeVisible();

  await page.keyboard.press('Control+Backslash');

  await expect(page.locator('aside')).toHaveCount(0);
  await expect(taskLink).toBeVisible();
  await expect.poll(() => page.evaluate(() => localStorage.getItem('dadr.sidebar'))).toBeNull();
});
