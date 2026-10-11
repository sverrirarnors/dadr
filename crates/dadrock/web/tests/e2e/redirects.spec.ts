import { expect, test } from '@playwright/test';

const cases: Array<[string, RegExp]> = [
  ['/records', /\/ledger$/],
  ['/records?status=candidate', /\/inbox$/],
  ['/records?status=candidate&id=0193fb1f-a3a0-7000-8000-000000000002', /\/inbox\?item=candidate%3A0193fb1f-a3a0-7000-8000-000000000002$/],
  ['/records?id=0193fb1f-a3a0-7000-8000-000000000003&tab=edit', /\/ledger\?id=0193fb1f-a3a0-7000-8000-000000000003&tab=edit$/],
  ['/records/0193fb1f-a3a0-7000-8000-000000000003', /\/ledger\/0193fb1f-a3a0-7000-8000-000000000003$/],
  ['/candidates/0193fb1f-a3a0-7000-8000-000000000002', /\/inbox\?item=candidate%3A0193fb1f-a3a0-7000-8000-000000000002$/],
  ['/inbox', /\/inbox$/],
  ['/tags', /\/ledger$/],
  ['/validation', /\/inbox$/],
  ['/validation?tab=links', /\/inbox$/],
  ['/audit', /\/inbox$/],
  ['/links', /\/inbox$/],
  ['/settings', /\/inbox$/],
  ['/tasks?tab=runs', /\/inbox$/],
];

for (const [from, to] of cases) {
  test(`legacy ${from} lands on ${to}`, async ({ page }) => {
    await page.goto(from);
    await expect(page).toHaveURL(to);
    await expect(page.locator('[data-app-header]')).toBeVisible();
  });
}

test('legacy drafting routes open the composer and drop the draft parameter', async ({ page }) => {
  await page.goto('/candidates/new');
  await expect(page.getByRole('dialog', { name: 'Draft' })).toBeVisible();
  await expect(page).toHaveURL(/\/inbox$/);

  await page.goto('/tasks?task=edit-record&record=0193fb1f-a3a0-7000-8000-000000000002');
  await expect(page.getByRole('dialog', { name: 'Draft' })).toBeVisible();
  await expect(page).toHaveURL(/\/ledger\?id=0193fb1f-a3a0-7000-8000-000000000002$/);
});
