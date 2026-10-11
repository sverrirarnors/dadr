import { expect, test } from '@playwright/test';

test('default route redirects to the inbox and the chrome renders', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveURL(/\/inbox$/);
  await expect(page.locator('[data-app-shell]')).toBeVisible();
  const header = page.locator('[data-app-header]');
  await expect(header).toBeVisible();
  await expect(header.getByRole('link', { name: 'Inbox' })).toHaveAttribute('aria-current', 'page');
  await expect(header.getByRole('link', { name: 'Ledger' })).toBeVisible();
  await expect(header.getByRole('link', { name: 'Draft' })).toBeVisible();
  await expect(header.getByRole('button', { name: 'Project' })).toBeVisible();
  for (const name of ['Inbox', 'Ledger', 'Draft']) {
    await expect(header.getByRole('link', { name }).locator('svg')).toHaveCount(1);
  }
  // All three destinations sit left of the theme toggle.
  const [draftBox, themeBox] = await Promise.all([
    header.getByRole('link', { name: 'Draft' }).boundingBox(),
    page.getByLabel(/theme/i).boundingBox(),
  ]);
  if (!draftBox || !themeBox) throw new Error('expected header boxes');
  expect(draftBox.x + draftBox.width).toBeLessThan(themeBox.x);
  await expect(header.getByRole('link', { name: /Tags|Tasks|Tools|Validation|Settings|More/ })).toHaveCount(0);
  await expect(page.getByLabel(/theme/i)).toBeVisible();
});

test('the inbox count badge reflects waiting candidates', async ({ page }) => {
  await page.goto('/inbox');
  await expect(page.locator('[data-inbox-count]')).toHaveText('1');
});

test('navigation keeps both destinations visible on a phone', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 700 });
  await page.goto('/inbox');
  const header = page.locator('[data-app-header]');
  await expect(header.getByRole('link', { name: 'Inbox' })).toBeVisible();
  await expect(header.getByRole('link', { name: 'Ledger' })).toBeVisible();
  await expect(header.getByRole('link', { name: 'Draft' })).toBeVisible();
  await header.getByRole('link', { name: 'Ledger' }).click();
  await expect(page).toHaveURL(/\/ledger$/);
});

test('command palette opens with the keyboard and navigates', async ({ page }) => {
  await page.goto('/inbox');
  await page.locator('body').click();
  await page.keyboard.press('Control+K');
  await expect(page.getByRole('dialog', { name: 'Command palette' })).toBeVisible();
  await page.getByPlaceholder('Search commands...').fill('ledger');
  await page.getByRole('button', { name: 'Go to Ledger' }).click();
  await expect(page).toHaveURL(/\/ledger$/);
});

test('command palette opens records in the ledger, including archived ones', async ({ page }) => {
  await page.goto('/inbox');
  await page.locator('body').click();
  await page.keyboard.press('Control+K');
  await page.getByPlaceholder('Search commands...').fill('polars over');
  await page.getByRole('button', { name: /Use Polars over pandas/ }).click();
  await expect(page).toHaveURL(/\/ledger\?archived=1&id=0193fb1f-a3a0-7000-8000-000000000001$/);
});

test('command palette opens the Draft page', async ({ page }) => {
  await page.goto('/ledger');
  await page.locator('body').click();
  await page.keyboard.press('Control+K');
  const palette = page.getByRole('dialog', { name: 'Command palette' });
  await palette.getByPlaceholder('Search commands...').fill('draft');
  await palette.getByRole('button', { name: 'Go to Draft' }).click();
  await expect(page).toHaveURL(/\/draft$/);
  await expect(page.locator('[data-draft-composer]')).toBeVisible();
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
