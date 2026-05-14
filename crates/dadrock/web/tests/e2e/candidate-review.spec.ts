import { expect, test } from '@playwright/test';

const candidateId = '0193fb1f-a3a0-7000-8000-000000000002';

test('candidate review renders candidate source and tabs', async ({ page }) => {
  await page.goto(`/candidates/${candidateId}`);

  await expect(page.locator('[data-workbench-page][data-layout="single"]')).toBeVisible();
  await expect(page.locator('[data-ui-segmented-tabs]')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Adopt pre-registration for studies' }).first()).toBeVisible();
  await expect(page.getByText('dadr/candidates/0002-preregister.md')).toBeVisible();
  await expect(page.getByRole('article')).toContainText('Candidate decision for pre-registration practice.');

  await page.getByRole('tab', { name: 'Raw' }).click();
  await expect(page.getByText(`id: ${candidateId}`)).toBeVisible();
});

test('candidate review accepts a newly created candidate', async ({ page }) => {
  await page.goto('/candidates/new');
  const title = `Review accept ${Date.now()}`;
  await page.getByLabel('Title').fill(title);
  await page.getByLabel('Tags').fill('#dadr/review');
  await page.getByLabel('Candidate Markdown').fill(`## Context\nReview test.\n\n## Decision\nAccept ${title}.`);
  await page.getByRole('button', { name: 'Create candidate' }).click();
  await expect(page).toHaveURL(/\/records\/(0193fb1f-a3a0-7000-8000-[0-9a-f]{12})$/);
  const id = page.url().split('/').at(-1);
  if (!id) throw new Error('expected created candidate id in URL');

  await page.evaluate((candidateId) => {
    const link = document.createElement('a');
    link.href = `/candidates/${candidateId}`;
    link.textContent = 'Open candidate review';
    document.body.append(link);
  }, id);
  await page.getByRole('link', { name: 'Open candidate review' }).click();
  await expect(page.locator('[data-ui-button]').getByText('Reject candidate')).toBeVisible();
  await expect(page.locator('[data-ui-button]').getByText('Accept candidate')).toBeVisible();
  await page.getByRole('button', { name: 'Accept candidate' }).click();

  await expect(page).toHaveURL(new RegExp(`/records/${id}$`));
  const toast = page.getByText(`accepted candidate ${id}`);
  await expect(toast).toBeVisible();
  await expect(toast).not.toBeVisible({ timeout: 4000 });
});
