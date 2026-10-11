import { expect, test, type Page } from '@playwright/test';
import { editorValue, fillEditor } from './editor';

const LAZY_FRAMES_ID = '0193fb1f-a3a0-7000-8000-000000000003';
const PREREGISTRATION_ID = '0193fb1f-a3a0-7000-8000-000000000002';
const ARROW_ID = '0193fb1f-a3a0-7000-8000-000000000004';

function draftPage(page: Page) {
  return page.locator('[data-draft-composer]');
}

async function openDraft(page: Page) {
  await page.locator('[data-app-header]').getByRole('link', { name: 'Draft' }).click();
  await expect(page).toHaveURL(/\/draft$/);
  await expect(draftPage(page)).toBeVisible();
}

test('the header Draft link opens the full-page composer and the intent follows the text', async ({ page }) => {
  await page.goto('/inbox');
  await openDraft(page);

  const draft = draftPage(page);
  await expect(draft.getByRole('heading', { name: 'Draft a decision', level: 1 })).toBeVisible();
  await expect(page.locator('[data-app-header]').getByRole('link', { name: 'Draft' })).toHaveAttribute('aria-current', 'page');
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(draft.locator('[data-draft-intent]')).toHaveText('Describe the decision, or attach files to scan.');
  const send = draft.getByRole('button', { name: 'Send to agent' });
  await expect(send).toBeDisabled();

  const prompt = draft.getByLabel('What is the decision? Or say what to look for in the attached files.');
  await expect(prompt).toBeFocused();
  await prompt.fill('We winsorize income at the 99th percentile.');
  await expect(draft.locator('[data-draft-intent]')).toHaveText('Draft one decision from your notes');
  await expect(send).toBeEnabled();
});

test('attaching a folder scans it and sending lands in the Inbox', async ({ page }) => {
  await page.goto('/ledger');
  await openDraft(page);
  const draft = draftPage(page);

  await draft.getByRole('button', { name: 'Attach files or folders' }).click();
  await draft.getByLabel('Path').fill('analysis');
  await draft.getByRole('button', { name: 'Add', exact: true }).click();
  await expect(draft.getByRole('button', { name: 'Remove analysis' })).toBeVisible();
  await expect(draft.locator('[data-draft-intent]')).toHaveText('Look for decisions in 1 path');

  await draft.getByRole('button', { name: 'Send to agent' }).click();
  await expect(page).toHaveURL(/\/inbox$/);
  await expect(page.getByRole('status').filter({ hasText: 'Sent to the agent' })).toBeVisible();
  await expect(draft).toHaveCount(0);
  await expect(page.getByText('Use explicit missing data policy')).toBeVisible();
});

test('"Write it by hand" creates a candidate and opens it in the Inbox', async ({ page }) => {
  await page.goto('/inbox');
  await openDraft(page);
  const draft = draftPage(page);

  await draft.getByRole('button', { name: 'Write it by hand' }).click();
  const markdown = draft.getByLabel('Candidate Markdown');
  await expect(markdown).toHaveAttribute('contenteditable', 'true');
  await expect.poll(() => editorValue(markdown)).toMatch(/## Context/);
  const create = draft.getByRole('button', { name: 'Create candidate' });
  await expect(create).toBeDisabled();

  await draft.getByLabel('Title').fill('Manual decision');
  await fillEditor(markdown, '## Context\nx\n\n## Decision\ny');
  await expect(create).toBeEnabled();
  await create.click();

  await expect(page).toHaveURL(/\/inbox\?item=candidate%3A/);
  await expect(page.getByRole('status').filter({ hasText: 'created candidate' })).toBeVisible();
  await expect(draft).toHaveCount(0);
});

test('?draft=new lands on the Draft page without the parameter', async ({ page }) => {
  await page.goto('/inbox?draft=new');
  await expect(page).toHaveURL(/\/draft$/);
  await expect(draftPage(page).getByRole('heading', { name: 'Draft a decision' })).toBeVisible();
});

test('a supersede deep link can supersede the decision with existing ones', async ({ page }) => {
  await page.goto(`/ledger?id=${LAZY_FRAMES_ID}&draft=supersede&record=${LAZY_FRAMES_ID}`);
  await expect(page).toHaveURL(/\/draft$/);
  const draft = draftPage(page);
  await expect(draft.getByRole('heading', { name: 'Supersede “Use Polars lazy frames”' })).toBeVisible();

  await draft.getByRole('button', { name: 'Supersede with existing decisions instead' }).click();
  const picker = page.getByRole('dialog', { name: 'Supersede with existing decisions' });
  await expect(picker).toBeVisible();
  await expect(picker.getByLabel('Select Use Polars lazy frames')).toHaveCount(0);
  await picker.getByLabel('Select Use Arrow IPC snapshots').check();
  await picker.getByRole('button', { name: 'Save' }).click();

  await expect(picker).toHaveCount(0);
  await expect(page).toHaveURL(new RegExp(`/ledger\\?archived=1&id=${LAZY_FRAMES_ID}$`));
  await expect(page.getByRole('status').filter({ hasText: 'record superseded' })).toBeVisible();
});

test('a successor written by hand starts with Supersedes and accepting it archives the predecessor', async ({ page }) => {
  await page.goto(`/ledger?id=${ARROW_ID}&draft=supersede&record=${ARROW_ID}`);
  const draft = draftPage(page);
  await expect(draft.getByRole('heading', { name: 'Supersede “Use Arrow IPC snapshots”' })).toBeVisible();

  await draft.getByRole('button', { name: 'Write it by hand' }).click();
  await expect(draft.getByRole('heading', { name: 'Edit “Use Arrow IPC snapshots”' })).toBeVisible();
  await expect(draft.getByText('Saving drafts a successor candidate in the Inbox.')).toBeVisible();
  await expect(draft.getByLabel('Title')).toHaveValue('Use Arrow IPC snapshots');
  await draft.getByLabel('Title').fill('Use Arrow IPC snapshots everywhere');
  await draft.getByLabel('Why does this decision change?').fill('Every step reads the snapshots now.');
  await fillEditor(
    draft.getByLabel('Successor Markdown'),
    '## Context\nc\n\n## Decision\nd\n\n## Alternatives\na\n\n## Consequences\nq',
  );
  await draft.getByRole('button', { name: 'Create candidate' }).click();

  await expect(page).toHaveURL(/\/inbox\?item=candidate%3A/);
  const detail = page.locator('[data-inbox-detail]');
  await expect(detail.getByText('Proposed successor')).toBeVisible();
  await expect(detail.getByRole('tab', { name: 'Changes' })).toHaveAttribute('aria-selected', 'true');
  await expect(detail.locator('[data-successor-reason]')).toHaveText('Every step reads the snapshots now.');
  await detail.getByRole('tab', { name: 'Document' }).click();
  const body = detail.getByRole('article');
  await expect(body.locator('h2').first()).toHaveText('Supersedes');
  await expect(body.getByRole('button', { name: 'Use Arrow IPC snapshots' })).toBeVisible();

  await detail.getByRole('button', { name: 'Accept' }).click();
  await expect(page.getByRole('status').filter({ hasText: 'accepted candidate' })).toBeVisible();

  await page.locator('[data-app-header]').getByRole('link', { name: 'Ledger' }).click();
  const list = page.locator('[data-ledger-list]');
  await expect(list.getByRole('link', { name: /Use Arrow IPC snapshots everywhere/ })).toBeVisible();
  await expect(list.getByRole('link', { name: /^Use Arrow IPC snapshots 0193/ })).toHaveCount(0);
  await page.getByLabel('Show archived').check();
  await expect(list.getByRole('link', { name: /^sup Use Arrow IPC snapshots 0193/ })).toBeVisible();
});

test('a revise deep link sends the requested change to the agent', async ({ page }) => {
  await page.goto(`/ledger?id=${PREREGISTRATION_ID}&draft=revise&record=${PREREGISTRATION_ID}`);
  await expect(page).toHaveURL(/\/draft$/);
  const draft = draftPage(page);
  await expect(draft.getByRole('heading', { name: 'Revise “Adopt pre-registration for studies”' })).toBeVisible();
  await expect(draft.getByRole('button', { name: 'Attach records' })).toHaveCount(0);
  await expect(draft.getByRole('button', { name: 'Remove Adopt pre-registration for studies' })).toHaveCount(0);
  await expect(draft.getByRole('link', { name: 'Edit it in the Ledger' })).toHaveAttribute(
    'href',
    `/ledger?archived=1&id=${PREREGISTRATION_ID}&tab=edit`,
  );

  const send = draft.getByRole('button', { name: 'Send to agent' });
  await expect(send).toBeDisabled();
  await expect(draft.locator('[data-draft-intent]')).toHaveText('Describe what should change.');
  await draft.getByLabel('What should change?').fill('Tighten the wording of the decision.');
  await expect(draft.locator('[data-draft-intent]')).toHaveText('Revise “Adopt pre-registration for studies”');
  await send.click();

  await expect(page).toHaveURL(/\/inbox$/);
  await expect(page.getByRole('status').filter({ hasText: 'Sent to the agent' })).toBeVisible();
});

test('Close and Escape leave the Draft page', async ({ page }) => {
  await page.goto('/inbox');
  await openDraft(page);
  await page.keyboard.press('Escape');
  await expect(page).toHaveURL(/\/inbox$/);
  await expect(draftPage(page)).toHaveCount(0);

  await openDraft(page);
  await draftPage(page).getByRole('button', { name: 'Close' }).click();
  await expect(page).toHaveURL(/\/inbox$/);

  // Opened from a record, Close returns to that record.
  await page.goto(`/ledger?id=${LAZY_FRAMES_ID}`);
  await page.locator('[data-record-detail-pane]').getByRole('button', { name: 'Supersede' }).click();
  await expect(page).toHaveURL(/\/draft$/);
  await draftPage(page).getByRole('button', { name: 'Close' }).click();
  await expect(page).toHaveURL(new RegExp(`/ledger\\?id=${LAZY_FRAMES_ID}$`));
});

test('the Draft page is usable on a phone', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 700 });
  await page.goto('/inbox');
  await openDraft(page);
  const draft = draftPage(page);
  await expect(draft.getByRole('button', { name: 'Send to agent' })).toBeVisible();
  const box = await draft.boundingBox();
  expect(box?.width).toBeLessThanOrEqual(375);
});
