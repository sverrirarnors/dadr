import { expect, test, type Page } from '@playwright/test';

const acceptedId = '0193fb1f-a3a0-7000-8000-000000000001';
const candidateId = '0193fb1f-a3a0-7000-8000-000000000002';

async function openRawEditor(page: Page) {
  const editMenu = page.locator('[data-action-menu]').filter({ has: page.getByRole('button', { name: 'Edit' }) });
  await editMenu.getByRole('button', { name: 'Edit' }).click();
  await editMenu.getByRole('menuitem', { name: 'Edit', exact: true }).click();
}

test('record editor resets dirty markdown to the loaded source', async ({ page }) => {
  await page.goto(`/records/${acceptedId}`);
  await openRawEditor(page);
  const editor = page.getByLabel('Raw Markdown');
  const original = await editor.inputValue();

  await editor.fill(`${original}\n\nLocal dirty edit.`);

  await expect(page.getByText('Unsaved')).toBeVisible();
  await page.getByRole('button', { name: 'Cancel' }).click();

  await expect(page).toHaveURL(new RegExp(`/records/${acceptedId}$`));
  await expect(page.getByLabel('Raw Markdown')).not.toBeVisible();
  await expect(page.getByRole('article')).toContainText('Use Polars for tabular transformations.');
  await expect(page.getByText('Unsaved')).not.toBeVisible();
});

test('record editor cancel is available before markdown changes', async ({ page }) => {
  await page.goto(`/records/${acceptedId}`);
  await openRawEditor(page);

  await expect(page.getByRole('button', { name: 'Cancel' })).toBeEnabled();
});

test('record detail switches between rendered and edit modes with a compact supersession line', async ({
  page,
}) => {
  await page.goto(`/records/${acceptedId}`);

  await expect(page.getByRole('heading', { name: 'Use Polars over pandas' }).first()).toBeVisible();
  await expect(page.getByText('Superseded by')).toBeVisible();
  await expect(page.getByRole('link', { name: '0193fb1fa3a0' })).toBeVisible();
  await expect(
    page.getByRole('article').getByRole('heading', { name: 'Use Polars over pandas' }),
  ).toBeVisible();
  await expect(page.getByLabel('Raw Markdown')).not.toBeVisible();
  await expect(page.getByRole('button', { name: 'Rendered' })).not.toBeVisible();
  await expect(page.getByRole('button', { name: 'Raw' })).not.toBeVisible();

  await openRawEditor(page);
  await expect(page).toHaveURL(new RegExp(`/records/${acceptedId}\\?tab=edit$`));
  await expect(page.getByLabel('Raw Markdown')).toBeVisible();
});

test('record detail places tags below the file name', async ({ page }) => {
  await page.goto(`/records/${acceptedId}`);

  const fileName = page.getByText('0001-use-polars.md');
  const tag = page.getByText('#dadr/data/processing');
  await expect(fileName).toBeVisible();
  await expect(tag).toBeVisible();

  const [fileBox, tagBox] = await Promise.all([fileName.boundingBox(), tag.boundingBox()]);
  if (!fileBox || !tagBox) throw new Error('expected file name and tag boxes');
  expect(tagBox.y).toBeGreaterThan(fileBox.y + fileBox.height);
});

test('record detail tags link to tag-scoped record search', async ({ page }) => {
  await page.goto(`/records/${acceptedId}`);

  await expect(page.locator('[data-ui-inline-link-badge]').getByText('#dadr/data/processing')).toBeVisible();
  await page.getByRole('link', { name: '#dadr/data/processing' }).click();

  await expect(page).toHaveURL(/\/records\?scope=tag&q=%23dadr%2Fdata%2Fprocessing$/);
  await expect(page.getByLabel('Search scope')).toHaveValue('tag');
  await expect(page.getByRole('link', { name: /Use Polars over pandas/ })).toBeVisible();
  await expect(page.getByRole('link', { name: /Adopt pre-registration for studies/ })).not.toBeVisible();
});

test('record detail keeps tag metadata close to the view controls', async ({ page }) => {
  await page.goto(`/records/${candidateId}`);

  const tag = page.getByText('#dadr/methodology');
  const controls = page.getByRole('button', { name: 'Edit' });
  await expect(tag).toBeVisible();
  await expect(controls).toBeVisible();

  const [tagBox, controlsBox] = await Promise.all([tag.boundingBox(), controls.boundingBox()]);
  if (!tagBox || !controlsBox) throw new Error('expected tag and control boxes');
  expect(controlsBox.y - (tagBox.y + tagBox.height)).toBeLessThanOrEqual(20);
});

test('record editor saves markdown and keeps it available during SPA navigation', async ({ page }) => {
  await page.goto(`/records/${acceptedId}`);
  await openRawEditor(page);
  const editor = page.getByLabel('Raw Markdown');
  const original = await editor.inputValue();
  const edited = `${original}\n\nSaved through Playwright.`;

  await editor.fill(edited);
  await page.getByRole('button', { name: 'Save' }).click();

  await expect(page.getByText(`saved ${acceptedId}`)).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/records/${acceptedId}$`));
  await expect(page.getByLabel('Raw Markdown')).not.toBeVisible();
  await page.getByRole('link', { name: 'Back to records' }).click();
  await page.getByRole('link', { name: /Use Polars over pandas/ }).click();
  await openRawEditor(page);

  await expect(page.getByLabel('Raw Markdown')).toHaveValue(edited);
});

test('record editor saves with the keyboard shortcut', async ({ page }) => {
  await page.goto(`/records/${candidateId}`);
  await openRawEditor(page);
  const editor = page.getByLabel('Raw Markdown');
  const original = await editor.inputValue();
  const edited = `${original}\n\nSaved with keyboard shortcut.`;

  await editor.fill(edited);
  await page.keyboard.press('Control+S');

  await expect(page.getByText(`saved ${candidateId}`)).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/records/${candidateId}$`));
  await expect(page.getByLabel('Raw Markdown')).not.toBeVisible();
});

test('record editor loads the next record when same-route navigation changes ids while dirty', async ({
  page,
}) => {
  await page.goto(`/records/${acceptedId}`);
  await openRawEditor(page);
  const editor = page.getByLabel('Raw Markdown');
  await editor.fill('dirty markdown for the accepted record');

  await page.evaluate((nextId) => {
    const link = document.createElement('a');
    link.href = `/records/${nextId}?tab=edit`;
    link.textContent = 'Switch to candidate record';
    document.body.append(link);
  }, candidateId);
  await page.getByRole('link', { name: 'Switch to candidate record' }).click();

  await expect(page).toHaveURL(new RegExp(`/records/${candidateId}\\?tab=edit$`));
  await expect(editor).toHaveValue(/Adopt pre-registration for studies/);
  await expect(editor).not.toHaveValue(/dirty markdown for the accepted record/);
});
