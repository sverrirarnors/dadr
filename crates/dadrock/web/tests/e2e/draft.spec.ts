import { expect, test, type Page } from '@playwright/test';

const LAZY_FRAMES_ID = '0193fb1f-a3a0-7000-8000-000000000003';
const PREREGISTRATION_ID = '0193fb1f-a3a0-7000-8000-000000000002';

function draftDialog(page: Page) {
  return page.getByRole('dialog', { name: 'Draft' });
}

test('the header Draft button opens the composer and the intent follows the text', async ({ page }) => {
  await page.goto('/inbox');
  await page.locator('[data-app-header]').getByRole('button', { name: 'Draft' }).click();

  const dialog = draftDialog(page);
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole('heading', { name: 'Draft a decision' })).toBeVisible();
  await expect(dialog.locator('[data-draft-intent]')).toHaveText(
    'Describe the decision, or attach files to scan.',
  );
  const send = dialog.getByRole('button', { name: 'Send to agent' });
  await expect(send).toBeDisabled();

  const prompt = dialog.getByLabel(
    'What is the decision? Or say what to look for in the attached files.',
  );
  await expect(prompt).toBeFocused();
  await prompt.fill('We winsorize income at the 99th percentile.');
  await expect(dialog.locator('[data-draft-intent]')).toHaveText('Draft one decision from your notes');
  await expect(send).toBeEnabled();
});

test('attaching a folder scans it and sending lands in the Inbox', async ({ page }) => {
  await page.goto('/ledger');
  await page.locator('[data-app-header]').getByRole('button', { name: 'Draft' }).click();
  const dialog = draftDialog(page);

  await dialog.getByRole('button', { name: 'Attach files or folders' }).click();
  await dialog.getByLabel('Path').fill('analysis');
  await dialog.getByRole('button', { name: 'Add', exact: true }).click();
  await expect(dialog.getByRole('button', { name: 'Remove analysis' })).toBeVisible();
  await expect(dialog.locator('[data-draft-intent]')).toHaveText('Look for decisions in 1 path');

  await dialog.getByRole('button', { name: 'Send to agent' }).click();
  await expect(page).toHaveURL(/\/inbox$/);
  await expect(page.getByRole('status').filter({ hasText: 'Sent to the agent' })).toBeVisible();
  await expect(dialog).toHaveCount(0);
  await expect(page.getByText('Use explicit missing data policy')).toBeVisible();
});

test('"Write it by hand" creates a candidate and opens it in the Inbox', async ({ page }) => {
  await page.goto('/inbox');
  await page.locator('[data-app-header]').getByRole('button', { name: 'Draft' }).click();
  const dialog = draftDialog(page);

  await dialog.getByRole('button', { name: 'Write it by hand' }).click();
  const markdown = dialog.getByLabel('Candidate Markdown');
  await expect(markdown).toHaveValue(/## Context/);
  const create = dialog.getByRole('button', { name: 'Create candidate' });
  await expect(create).toBeDisabled();

  await dialog.getByLabel('Title').fill('Manual decision');
  await markdown.fill('## Context\nx\n\n## Decision\ny');
  await expect(create).toBeEnabled();
  await create.click();

  await expect(page).toHaveURL(/\/inbox\?item=candidate%3A/);
  await expect(page.getByRole('status').filter({ hasText: 'created candidate' })).toBeVisible();
  await expect(dialog).toHaveCount(0);
});

test('?draft=new opens the composer and the URL forgets the parameter', async ({ page }) => {
  await page.goto('/inbox?draft=new');
  await expect(draftDialog(page)).toBeVisible();
  await expect(draftDialog(page).getByRole('heading', { name: 'Draft a decision' })).toBeVisible();
  await expect(page).toHaveURL(/\/inbox$/);
  await expect(page).not.toHaveURL(/draft=/);
});

test('a supersede deep link can replace the decision with existing ones', async ({ page }) => {
  await page.goto(`/ledger?id=${LAZY_FRAMES_ID}&draft=supersede&record=${LAZY_FRAMES_ID}`);
  const dialog = draftDialog(page);
  await expect(dialog.getByRole('heading', { name: 'Replace “Use Polars lazy frames”' })).toBeVisible();
  await expect(page).not.toHaveURL(/draft=/);

  await dialog.getByRole('button', { name: 'Replace with existing decisions instead' }).click();
  const picker = page.getByRole('dialog', { name: 'Replace with existing decisions' });
  await expect(picker).toBeVisible();
  await expect(picker.getByLabel('Select Use Polars lazy frames')).toHaveCount(0);
  await picker.getByLabel('Select Use Arrow IPC snapshots').check();
  await picker.getByRole('button', { name: 'Save' }).click();

  await expect(picker).toHaveCount(0);
  await expect(dialog).toHaveCount(0);
  await expect(page).toHaveURL(new RegExp(`/ledger\\?history=1&id=${LAZY_FRAMES_ID}$`));
  await expect(page.getByRole('status').filter({ hasText: 'record superseded' })).toBeVisible();
});

test('a revise deep link sends the requested change to the agent', async ({ page }) => {
  await page.goto(`/ledger?id=${PREREGISTRATION_ID}&draft=revise&record=${PREREGISTRATION_ID}`);
  const dialog = draftDialog(page);
  await expect(
    dialog.getByRole('heading', { name: 'Revise “Adopt pre-registration for studies”' }),
  ).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Attach records' })).toHaveCount(0);
  await expect(dialog.getByRole('button', { name: 'Remove Adopt pre-registration for studies' })).toHaveCount(0);
  await expect(dialog.getByRole('link', { name: 'Edit it in the Ledger' })).toHaveAttribute(
    'href',
    `/ledger?history=1&id=${PREREGISTRATION_ID}&tab=edit`,
  );

  const send = dialog.getByRole('button', { name: 'Send to agent' });
  await expect(send).toBeDisabled();
  await expect(dialog.locator('[data-draft-intent]')).toHaveText('Describe what should change.');
  await dialog.getByLabel('What should change?').fill('Tighten the wording of the decision.');
  await expect(dialog.locator('[data-draft-intent]')).toHaveText(
    'Revise “Adopt pre-registration for studies”',
  );
  await send.click();

  await expect(page).toHaveURL(/\/inbox$/);
  await expect(page.getByRole('status').filter({ hasText: 'Sent to the agent' })).toBeVisible();
});

test('Escape and the backdrop close the composer', async ({ page }) => {
  await page.goto('/inbox');
  await page.locator('[data-app-header]').getByRole('button', { name: 'Draft' }).click();
  const dialog = draftDialog(page);
  await expect(dialog).toBeVisible();
  await page.keyboard.press('Escape');
  await expect(dialog).toHaveCount(0);

  await page.locator('[data-app-header]').getByRole('button', { name: 'Draft' }).click();
  await expect(dialog).toBeVisible();
  await page.locator('[data-draft-composer]').click({ position: { x: 8, y: 300 } });
  await expect(dialog).toHaveCount(0);
});

test('the sheet fills a phone screen', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 700 });
  await page.goto('/inbox');
  await page.locator('[data-app-header]').getByRole('button', { name: 'Draft' }).click();
  const dialog = draftDialog(page);
  await expect(dialog).toBeVisible();
  await expect(dialog.getByRole('button', { name: 'Send to agent' })).toBeVisible();
  // The sheet slides in; poll until it has settled against the left edge.
  await expect
    .poll(async () => {
      const box = await dialog.boundingBox();
      return box ? [Math.round(box.x), Math.round(box.width)] : null;
    })
    .toEqual([0, 375]);
});
