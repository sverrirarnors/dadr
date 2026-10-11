import { expect, test } from '@playwright/test';

test('manual candidate creation loads the template and opens the new record', async ({ page }) => {
  await page.goto('/candidates/new');

  await expect(page.locator('[data-workbench-page][data-layout="single"]')).toBeVisible();
  await expect(page.getByText('Candidate editor')).toHaveCount(0);
  await expect(page.getByText('Write a proposed decision, then review and accept it.')).toHaveCount(1);
  await expect(page.locator('[data-ui-panel-section]')).toBeVisible();
  await expect(page.locator('[data-ui-text-input]').first()).toBeVisible();
  await expect(page.locator('[data-ui-textarea-input]').first()).toBeVisible();
  await expect(page.locator('[data-ui-button]').getByText('Create candidate')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'New Candidate' })).toBeVisible();
  await expect(page.getByLabel('Candidate Markdown')).toHaveValue(/## Context/);

  const title = `Manual candidate ${Date.now()}`;
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Tags').fill('#dadr/data/processing #kind/test');
  await page.getByLabel('Candidate Markdown').fill(`## Context\nManual test.\n\n## Decision\nCreate ${title}.`);
  await page.getByRole('button', { name: 'Create candidate' }).click();

  await expect(page).toHaveURL(/\/records\/0193fb1f-a3a0-7000-8000-[0-9a-f]{12}$/);
  await expect(page.getByRole('article')).toContainText(title);
});

test('agent drafting uses the common task workflow', async ({ page }) => {
  await page.goto('/candidates/new');
  await page.getByRole('link', { name: 'Draft a decision with an agent' }).click();
  await expect(page).toHaveURL(/tasks\?task=context-to-record$/);
});

test('legacy agent creation link redirects to the common workflow', async ({ page }) => {
  await page.goto('/candidates/new?mode=agent');
  await expect(page).toHaveURL(/tasks\?task=context-to-record$/);
});

test('manual candidate creation scrolls to the submit action on short viewports', async ({ page }) => {
  await page.setViewportSize({ width: 900, height: 520 });
  await page.goto('/candidates/new');

  const pane = page.locator('[data-candidate-new-pane]');
  await expect(pane).toBeVisible();

  await pane.evaluate((element) => {
    element.scrollTop = element.scrollHeight;
  });

  await expect(page.getByRole('button', { name: 'Create candidate' })).toBeInViewport();
});
