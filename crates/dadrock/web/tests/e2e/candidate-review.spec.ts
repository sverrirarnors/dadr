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
  await expect(page.getByRole('button', { name: 'Accept', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Reject', exact: true })).toHaveCount(0);
  await expect(page.getByRole('button', { name: 'Change this decision', exact: true })).toBeVisible();
  const toast = page.getByText(`accepted candidate ${id}`);
  await expect(toast).toBeVisible();
  await expect(toast).not.toBeVisible({ timeout: 4000 });
});

test('missing rationale remains editable and acceptance endorses the saved explanation', async ({ page }) => {
  await page.goto('/candidates/new');
  await page.getByLabel('Title').fill('Explain missing income exclusions');
  await page.getByLabel('Candidate Markdown').fill('## Context\nObserved: rows missing income are excluded.\n\n## Decision\nRationale not documented.');
  await page.getByRole('button', { name: 'Create candidate' }).click();
  await expect(page.getByText('Reason for this choice is missing')).toBeVisible();
  await page.getByRole('button', { name: 'Edit', exact: true }).click();
  await page.getByRole('menuitem', { name: 'Edit', exact: true }).click();
  const editor = page.locator('textarea');
  await editor.fill((await editor.inputValue()).replace('Rationale not documented.', 'Income is required to calculate the outcome.'));
  await expect(page.getByRole('button', { name: 'Accept', exact: true })).toBeDisabled();
  await page.getByRole('button', { name: 'Save', exact: true }).click();
  await expect(page.getByText('Reason for this choice is missing')).toHaveCount(0);
  await page.getByRole('button', { name: 'Accept', exact: true }).click();
  await expect(page.getByRole('button', { name: 'Change this decision' })).toBeVisible();
});

test('replacement acceptance links the original and preserves its explanation', async ({ page }) => {
  const oldId = '0193fb1f-a3a0-7000-8000-000000000003';
  await page.goto(`/records/${oldId}`);
  const original = await page.getByRole('article').textContent();
  await page.getByRole('button', { name: 'Change this decision' }).click();
  await page.getByRole('menuitem', { name: 'Draft a replacement', exact: true }).click();
  await page.getByLabel('Title').fill('Use another method');
  await page.getByRole('button', { name: 'Create candidate' }).click();
  await expect(page.getByText('Proposed replacement')).toBeVisible();
  const replacementId = page.url().split('/').at(-1);
  await page.getByRole('button', { name: 'Accept', exact: true }).click();
  await expect(page.getByText('Supersedes', { exact: true })).toBeVisible();
  await page.locator(`a[href="/records/${oldId}"]`).click();
  await expect(page.getByText('Superseded by', { exact: true })).toBeVisible();
  await expect(page.locator(`a[href="/records/${replacementId}"]`)).toBeVisible();
  await expect(page.getByRole('article')).toHaveText(original ?? '');
});
