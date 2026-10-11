import { expect, test } from '@playwright/test';

test('the project button opens a popover with the project facts and actions', async ({ page }) => {
  await page.goto('/inbox');
  const header = page.locator('[data-app-header]');
  const button = header.getByRole('button', { name: 'Project' });
  await expect(button).toHaveText(/dadr-mock/);
  await expect(button).toHaveAttribute('aria-expanded', 'false');

  await button.click();
  await expect(button).toHaveAttribute('aria-expanded', 'true');
  const popover = page.getByRole('dialog', { name: 'Project' });
  await expect(popover).toBeVisible();
  await expect(popover.getByText('/tmp/dadr-mock', { exact: true })).toBeVisible();
  await expect(popover.getByText('mock-agent (mock-agent --acp)')).toBeVisible();
  await expect(popover.getByText('directory', { exact: true })).toBeVisible();
  await expect(popover.getByRole('button', { name: 'Initialize project' })).toHaveCount(0);
  await expect(popover.getByText('Validation runs on every load. Findings land in the Inbox.')).toBeVisible();

  await popover.getByRole('button', { name: 'Validate now' }).click();
  await expect(page.getByRole('status').filter({ hasText: 'project validation finished' })).toBeVisible();

  await page.keyboard.press('Escape');
  await expect(popover).toHaveCount(0);
  await expect(button).toHaveAttribute('aria-expanded', 'false');
});

test('clearing the agent history reports it and clicking outside closes the popover', async ({ page }) => {
  await page.goto('/inbox');
  const button = page.locator('[data-app-header]').getByRole('button', { name: 'Project' });
  await button.click();
  const popover = page.getByRole('dialog', { name: 'Project' });
  await popover.getByRole('button', { name: 'Clear agent history' }).click();
  await expect(page.getByRole('status').filter({ hasText: 'Task history cleared.' })).toBeVisible();

  await page.locator('main').click({ position: { x: 10, y: 10 } });
  await expect(popover).toHaveCount(0);
});

test('an uninitialized project can be initialized from the popover', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('dadrock:mock-initialized', 'false');
  });
  await page.goto('/inbox');
  await page.locator('[data-app-header]').getByRole('button', { name: 'Project' }).click();
  const popover = page.getByRole('dialog', { name: 'Project' });
  await popover.getByRole('button', { name: 'Initialize project' }).click();
  await expect(page.getByRole('status').filter({ hasText: 'created 8 paths, reused 0 paths' })).toBeVisible();
  await expect(popover.getByRole('button', { name: 'Initialize project' })).toHaveCount(0);
  await expect(page.locator('[data-app-header]').getByRole('link', { name: 'Draft' })).not.toHaveAttribute('aria-disabled', 'true');
});
