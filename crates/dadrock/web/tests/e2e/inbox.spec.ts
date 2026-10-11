import { expect, test, type Page } from '@playwright/test';

const CANDIDATE_ID = '0193fb1f-a3a0-7000-8000-000000000002';
const POLARS_ID = '0193fb1f-a3a0-7000-8000-000000000001';
const CANDIDATE = /Adopt pre-registration for studies/;
const BROKEN_LINK = /A local Markdown link points at a missing note/;
const TAG_INFO = /No project tag catalog was found/;
const CONFLICT = /Lazy frames and IPC snapshots overlap/;

// Matches issueItemId('Links', <broken_markdown_link diagnostic>) in src/lib/inbox/items.ts.
const BROKEN_LINK_ISSUE_ID =
  'issue:Links:broken_markdown_link:dadr/archive/0001-use-polars.md:' +
  `${POLARS_ID}:./missing-note.md:12:A local Markdown link points at a missing note.`;

function group(page: Page, id: string) {
  return page.locator(`[data-inbox-group="${id}"]`);
}

function detail(page: Page) {
  return page.locator('[data-inbox-detail]');
}

function selectedRow(page: Page) {
  return page.locator('[data-inbox-list] [aria-current="page"]');
}

test('lists candidates and issues from the project snapshot', async ({ page }) => {
  await page.goto('/inbox');

  await expect(group(page, 'candidates').getByRole('heading', { name: 'Candidates' })).toBeVisible();
  await expect(group(page, 'candidates').getByRole('link', { name: CANDIDATE })).toBeVisible();
  await expect(group(page, 'issues').getByRole('heading', { name: 'Issues' })).toBeVisible();
  await expect(group(page, 'issues').getByRole('link', { name: BROKEN_LINK })).toBeVisible();
  await expect(group(page, 'issues').getByRole('link', { name: TAG_INFO })).toBeVisible();
  await expect(page.locator('[data-inbox-count]')).toHaveText('1');
  await expect(page.getByText('3 to deal with')).toBeVisible();
});

test('selecting the candidate shows its record detail', async ({ page }) => {
  await page.goto('/inbox');

  await group(page, 'candidates').getByRole('link', { name: CANDIDATE }).click();
  await expect(page).toHaveURL(new RegExp(`/inbox\\?item=candidate%3A${CANDIDATE_ID}$`));
  await expect(selectedRow(page)).toContainText('Adopt pre-registration for studies');
  await expect(detail(page).getByRole('article')).toContainText(
    'Candidate decision for pre-registration practice.',
  );
});

test('rejecting the candidate removes it and moves the selection on', async ({ page }) => {
  await page.goto(`/inbox?item=candidate%3A${CANDIDATE_ID}`);

  await expect(detail(page).getByRole('article')).toContainText('Candidate decision for pre-registration practice.');
  await detail(page).getByRole('button', { name: 'Reject' }).click();

  await expect(group(page, 'candidates')).toHaveCount(0);
  await expect(page.getByRole('link', { name: CANDIDATE })).toHaveCount(0);
  await expect(selectedRow(page)).toContainText('A local Markdown link points at a missing note.');
  await expect(page.locator('[data-inbox-count]')).toHaveCount(0);
});

test('dismissing an informational issue hides it across reloads', async ({ page }) => {
  await page.goto('/inbox');

  await group(page, 'issues').getByRole('link', { name: TAG_INFO }).click();
  await expect(detail(page).getByRole('heading', { name: TAG_INFO })).toBeVisible();
  await detail(page).getByRole('button', { name: 'Dismiss' }).click();

  await expect(group(page, 'issues').getByRole('link', { name: TAG_INFO })).toHaveCount(0);
  await expect(page.getByRole('status').filter({ hasText: 'Dismissed the issue' })).toBeVisible();

  await page.reload();
  await expect(group(page, 'issues').getByRole('link', { name: BROKEN_LINK })).toBeVisible();
  await expect(group(page, 'issues').getByRole('link', { name: TAG_INFO })).toHaveCount(0);
});

test('an error issue opens its record in the ledger editor and cannot be dismissed', async ({ page }) => {
  await page.goto('/inbox');

  await group(page, 'issues').getByRole('link', { name: BROKEN_LINK }).click();
  const pane = detail(page);
  await expect(pane.getByRole('heading', { name: BROKEN_LINK })).toBeVisible();
  await expect(pane.getByText('broken_markdown_link')).toBeVisible();
  await expect(pane.getByRole('link', { name: 'Open record' })).toHaveAttribute(
    'href',
    new RegExp(`^/ledger\\?.*id=${POLARS_ID}.*tab=edit`),
  );
  await expect(pane.getByRole('button', { name: 'Fix with agent' })).toBeVisible();
  await expect(pane.getByRole('button', { name: 'Dismiss' })).toHaveCount(0);
  await expect(pane.getByText('cannot be dismissed')).toBeVisible();
});

test('checking for conflicts adds a conflict that can be dismissed', async ({ page }) => {
  await page.goto('/inbox');

  await page.getByRole('button', { name: 'Check for conflicts' }).click();

  const conflicts = group(page, 'conflicts');
  await expect(conflicts.getByRole('heading', { name: 'Conflicts' })).toBeVisible();
  await conflicts.getByRole('link', { name: CONFLICT }).click();

  const pane = detail(page);
  await expect(pane.getByRole('heading', { name: CONFLICT })).toBeVisible();
  await expect(pane.getByRole('button', { name: /^Supersede “/ })).toHaveCount(2);
  await expect(pane.getByRole('button', { name: 'Supersede “Use Polars lazy frames”' })).toBeVisible();
  await expect(pane.getByRole('button', { name: 'Supersede “Use Arrow IPC snapshots”' })).toBeVisible();
  await expect(pane.getByText('Both records describe how intermediate data flows')).toBeVisible();

  await pane.getByRole('button', { name: 'Dismiss' }).click();
  await expect(conflicts).toHaveCount(0);
  await expect(page.getByRole('link', { name: CONFLICT })).toHaveCount(0);
  await expect(
    page.getByRole('status').filter({ hasText: 'Dismissed “Lazy frames and IPC snapshots overlap”' }),
  ).toBeVisible();
});

test('j and k move the selection through the list', async ({ page }) => {
  await page.goto('/inbox');

  await expect(selectedRow(page)).toContainText('Adopt pre-registration for studies');
  await page.keyboard.press('j');
  await expect(selectedRow(page)).toContainText('A local Markdown link points at a missing note.');
  await expect(page).toHaveURL(/item=issue%3ALinks%3Abroken_markdown_link/);
  await page.keyboard.press('j');
  await expect(selectedRow(page)).toContainText('No project tag catalog was found');
  await page.keyboard.press('k');
  await expect(selectedRow(page)).toContainText('A local Markdown link points at a missing note.');
  await page.keyboard.press('ArrowUp');
  await expect(selectedRow(page)).toContainText('Adopt pre-registration for studies');
});

test('clearing every item reaches inbox zero', async ({ page }) => {
  // Errors cannot be dismissed from the UI; the fixture's broken link stays
  // until the record is fixed, so it is marked resolved up front here.
  await page.addInitScript((id) => {
    localStorage.setItem('dadrock.inbox.resolved.v1', JSON.stringify([id]));
  }, BROKEN_LINK_ISSUE_ID);
  await page.goto('/inbox');

  await expect(group(page, 'issues').getByRole('link', { name: BROKEN_LINK })).toHaveCount(0);
  await detail(page).getByRole('button', { name: 'Reject' }).click();
  await expect(group(page, 'candidates')).toHaveCount(0);

  await expect(detail(page).getByRole('heading', { name: TAG_INFO })).toBeVisible();
  await detail(page).getByRole('button', { name: 'Dismiss' }).click();

  await expect(page.getByRole('heading', { name: 'Inbox zero' })).toBeVisible();
  await expect(page.getByText('Nothing needs your attention.')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Draft a decision' })).toBeVisible();
  await expect(page.getByRole('link', { name: 'Browse the Ledger' })).toHaveAttribute('href', '/ledger');
});

test('narrow screens show the list, then the detail with a way back', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 700 });
  await page.goto('/inbox');

  await expect(group(page, 'candidates').getByRole('link', { name: CANDIDATE })).toBeVisible();
  await expect(page.locator('[data-inbox-detail-pane]')).toBeHidden();

  await group(page, 'candidates').getByRole('link', { name: CANDIDATE }).click();
  await expect(page).toHaveURL(new RegExp(`/inbox\\?item=candidate%3A${CANDIDATE_ID}$`));
  await expect(page.getByRole('link', { name: 'Back to inbox' })).toBeVisible();
  await expect(detail(page).getByRole('article')).toContainText('Candidate decision for pre-registration practice.');
  await expect(page.locator('[data-inbox-list]')).toHaveCount(0);

  await page.getByRole('link', { name: 'Back to inbox' }).click();
  await expect(page).toHaveURL(/\/inbox$/);
  await expect(group(page, 'candidates').getByRole('link', { name: CANDIDATE })).toBeVisible();
});

test('the search box narrows the queue and the URL remembers it', async ({ page }) => {
  await page.goto('/inbox');
  const search = page.getByLabel('Search inbox');
  await expect(search).toBeVisible();

  await search.fill('polars');
  await expect(page).toHaveURL(/\/inbox\?q=polars$/);
  await expect(group(page, 'candidates')).toHaveCount(0);
  await expect(group(page, 'issues').getByRole('link', { name: BROKEN_LINK })).toBeVisible();
  await expect(group(page, 'issues').getByRole('link', { name: TAG_INFO })).toHaveCount(0);
  await expect(selectedRow(page)).toContainText('missing note');

  await search.fill('zzz-nothing');
  await expect(page.getByText('Nothing in the inbox matches “zzz-nothing”.')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Inbox zero' })).toHaveCount(0);

  await search.fill('');
  await expect(page).toHaveURL(/\/inbox$/);
  await expect(group(page, 'candidates').getByRole('link', { name: CANDIDATE })).toBeVisible();
});
