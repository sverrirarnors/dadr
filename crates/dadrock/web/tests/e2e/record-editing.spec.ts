import { expect, test, type Page } from '@playwright/test';
import { editorValue, fillEditor } from './editor';

const supersededId = '0193fb1f-a3a0-7000-8000-000000000001';
const candidateId = '0193fb1f-a3a0-7000-8000-000000000002';
const acceptedId = '0193fb1f-a3a0-7000-8000-000000000003';

async function openEditor(page: Page) {
  await page.getByRole('button', { name: 'Edit' }).click();
  await expect(page.getByLabel('Raw Markdown')).toBeVisible();
}

test('the raw editor is CodeMirror with the candidate loaded', async ({ page }) => {
  await page.goto(`/ledger/${candidateId}`);
  await openEditor(page);
  const editor = page.getByLabel('Raw Markdown');
  await expect(editor).toHaveAttribute('contenteditable', 'true');
  await expect(page.locator('[data-record-editor-pane] .cm-editor')).toBeVisible();
  expect(await editorValue(editor)).toContain(`id: ${candidateId}`);
  await expect(editor).toBeFocused();
});

test('record editor resets dirty markdown to the loaded source', async ({ page }) => {
  await page.goto(`/ledger/${candidateId}`);
  await openEditor(page);
  const editor = page.getByLabel('Raw Markdown');
  const original = await editorValue(editor);

  await fillEditor(editor, `${original}\n\nLocal dirty edit.`);

  await expect(page.getByText('Unsaved')).toBeVisible();
  await page.getByRole('button', { name: 'Cancel' }).click();

  await expect(page).toHaveURL(new RegExp(`/ledger/${candidateId}$`));
  await expect(page.getByLabel('Raw Markdown')).not.toBeVisible();
  await expect(page.getByRole('article')).toContainText('Candidate decision for pre-registration practice.');
  await expect(page.getByText('Unsaved')).not.toBeVisible();
});

test('accepted and archived records open the raw editor with a note on corrections', async ({ page }) => {
  await page.goto(`/ledger/${acceptedId}?tab=edit`);
  await expect(page.getByRole('heading', { name: 'Use Polars lazy frames' }).first()).toBeVisible();
  await expect(page.getByLabel('Raw Markdown')).toBeVisible();
  expect(await editorValue(page.getByLabel('Raw Markdown'))).toContain('status: accepted');
  const note = page.locator('[data-record-correction-note]');
  await expect(note).toContainText('stay in this record and show in its History');
  await expect(note).toContainText('To change the decision, use Supersede.');

  await page.goto(`/ledger/${supersededId}?tab=edit`);
  await expect(page.getByRole('heading', { name: 'Use Polars over pandas' }).first()).toBeVisible();
  await expect(page.getByLabel('Raw Markdown')).toBeVisible();
  await expect(note).toContainText('supersede the decision now in force');
});

test('record editor shows Cancel and a disabled Save before markdown changes', async ({ page }) => {
  await page.goto(`/ledger/${candidateId}`);

  await expect(page.getByRole('button', { name: 'Edit' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Supersede' })).toHaveCount(0);
  await expect(page.locator('[data-action-menu]')).toHaveCount(0);

  await openEditor(page);

  await expect(page.getByRole('button', { name: 'Cancel' })).toBeEnabled();
  await expect(page.getByRole('button', { name: 'Save' })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Edit' })).toHaveCount(0);
});

test('record detail places tags below the file name and links them to the ledger', async ({ page }) => {
  await page.goto(`/ledger/${supersededId}`);

  const fileName = page.getByText('0001-use-polars.md');
  const tag = page.getByRole('link', { name: '#dadr/data/processing' });
  await expect(fileName).toBeVisible();
  await expect(tag).toBeVisible();

  const [fileBox, tagBox] = await Promise.all([fileName.boundingBox(), tag.boundingBox()]);
  if (!fileBox || !tagBox) throw new Error('expected file name and tag boxes');
  expect(tagBox.y).toBeGreaterThan(fileBox.y + fileBox.height);

  await tag.click();
  await expect(page).toHaveURL(/\/ledger\?tag=%23dadr%2Fdata%2Fprocessing$/);
  await expect(page.locator('[data-ledger-list]').getByRole('link')).toHaveCount(2);
});

test('record editor saves markdown and keeps it available during SPA navigation', async ({ page }) => {
  await page.goto(`/ledger/${candidateId}`);
  await openEditor(page);
  const editor = page.getByLabel('Raw Markdown');
  const original = await editorValue(editor);
  const edited = `${original}\n\nSaved through Playwright.`;

  await fillEditor(editor, edited);
  await page.getByRole('button', { name: 'Save' }).click();

  await expect(page.getByRole('status').filter({ hasText: `saved ${candidateId}` })).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/ledger/${candidateId}$`));
  await expect(page.getByLabel('Raw Markdown')).not.toBeVisible();
  await expect(page.getByText('Saved', { exact: true })).toBeVisible();
  await expect(page.getByRole('article')).toContainText('Saved through Playwright.');

  await page.getByRole('link', { name: 'Back to ledger' }).click();
  await expect(page).toHaveURL(new RegExp(`/ledger\\?id=${candidateId}$`));
  await page.locator('[data-record-detail-pane]').getByRole('button', { name: 'Edit' }).click();

  await expect.poll(() => editorValue(page.getByLabel('Raw Markdown'))).toBe(edited);
});

test('record editor saves with the keyboard shortcut', async ({ page }) => {
  await page.goto(`/ledger/${candidateId}`);
  await openEditor(page);
  const editor = page.getByLabel('Raw Markdown');
  const original = await editorValue(editor);

  await fillEditor(editor, `${original}\n\nSaved with keyboard shortcut.`);
  await page.keyboard.press('Control+S');

  await expect(page.getByRole('status').filter({ hasText: `saved ${candidateId}` })).toBeVisible();
  await expect(page).toHaveURL(new RegExp(`/ledger/${candidateId}$`));
  await expect(page.getByLabel('Raw Markdown')).not.toBeVisible();
});

test('record editor loads the next record when same-route navigation changes ids while dirty', async ({
  page,
}) => {
  // The history scenario has two candidates: the halves of a split.
  await page.addInitScript(() => localStorage.setItem('dadrock:mock-scenario', 'history'));
  const first = '0193fb1f-a3a0-7000-8000-000000002002';
  const second = '0193fb1f-a3a0-7000-8000-000000002003';
  await page.goto(`/ledger/${first}`);
  await openEditor(page);
  await fillEditor(page.getByLabel('Raw Markdown'), 'dirty markdown for the first candidate');

  await page.evaluate((nextId) => {
    const link = document.createElement('a');
    link.href = `/ledger/${nextId}?tab=edit`;
    link.textContent = 'Switch to the other candidate';
    document.body.append(link);
  }, second);
  await page.getByRole('link', { name: 'Switch to the other candidate' }).click();

  await expect(page).toHaveURL(new RegExp(`/ledger/${second}\\?tab=edit$`));
  await expect.poll(() => editorValue(page.getByLabel('Raw Markdown'))).toMatch(/Drop implausible ages/);
  expect(await editorValue(page.getByLabel('Raw Markdown'))).not.toMatch(/dirty markdown for the first candidate/);
});

test('a tab=edit deep link opens the editor and leaving clears it', async ({ page }) => {
  await page.goto(`/ledger?id=${candidateId}&tab=edit`);

  const detail = page.locator('[data-record-detail-pane]');
  await expect(detail.getByLabel('Raw Markdown')).toBeVisible();
  await expect(detail.getByRole('button', { name: 'Edit' })).toHaveCount(0);

  await detail.getByRole('button', { name: 'Cancel' }).click();

  await expect(page).toHaveURL(new RegExp(`/ledger\\?id=${candidateId}$`));
  await expect(detail.getByLabel('Raw Markdown')).not.toBeVisible();
  await expect(detail.getByRole('button', { name: 'Edit' })).toBeVisible();

  // Switching records leaves edit mode.
  await detail.getByRole('button', { name: 'Edit' }).click();
  await expect(detail.getByLabel('Raw Markdown')).toBeVisible();
  await page.locator('[data-ledger-list]').getByRole('link', { name: /Use Arrow IPC snapshots/ }).click();
  await expect(detail.getByRole('heading', { name: 'Use Arrow IPC snapshots' }).first()).toBeVisible();
  await expect(detail.getByLabel('Raw Markdown')).not.toBeVisible();
});

test('typing in the search box keeps the raw editor open', async ({ page }) => {
  await page.goto(`/ledger?id=${acceptedId}`);
  const detail = page.locator('[data-record-detail-pane]');
  await detail.getByRole('button', { name: 'Edit', exact: true }).click();
  const editor = detail.getByLabel('Raw Markdown');
  const original = await editorValue(editor);
  await fillEditor(editor, `${original}\nStill editing.\n`);

  await page.getByLabel('Search ledger').fill('lazy');

  await expect(page).toHaveURL(new RegExp(`/ledger\\?q=lazy&id=${acceptedId}$`));
  await expect(editor).toBeVisible();
  expect(await editorValue(editor)).toContain('Still editing.');
});
