import { expect, test, type Page } from '@playwright/test';

const supersededId = '0193fb1f-a3a0-7000-8000-000000000001';
const candidateId = '0193fb1f-a3a0-7000-8000-000000000002';
const lazyFramesId = '0193fb1f-a3a0-7000-8000-000000000003';
const arrowId = '0193fb1f-a3a0-7000-8000-000000000004';

function list(page: Page) {
  return page.locator('[data-ledger-list]');
}

function detail(page: Page) {
  return page.locator('[data-record-detail-pane]');
}

function facet(page: Page) {
  return page.locator('[data-ledger-tag-facet]');
}

test('ledger lists current decisions newest first and counts them', async ({ page }) => {
  await page.goto('/ledger');

  const rows = list(page).getByRole('link');
  await expect(rows).toHaveCount(2);
  await expect(rows.nth(0)).toContainText('Use Arrow IPC snapshots');
  await expect(rows.nth(1)).toContainText('Use Polars lazy frames');
  await expect(rows.nth(0)).toContainText('0193fb1fa3a0');
  await expect(list(page).getByText('Use Polars over pandas')).toHaveCount(0);
  await expect(list(page).getByText('Adopt pre-registration for studies')).toHaveCount(0);
  await expect(page.getByText('2 decisions', { exact: true })).toBeVisible();
  await expect(list(page).getByText('ACC')).toHaveCount(0);
});

test('show archived reveals superseded decisions with a status badge', async ({ page }) => {
  await page.goto('/ledger');

  await page.getByLabel('Show archived').check();

  await expect(page).toHaveURL(/\/ledger\?archived=1$/);
  const row = list(page).getByRole('link', { name: /Use Polars over pandas/ });
  await expect(row).toBeVisible();
  await expect(row.getByText('SUP')).toBeVisible();
  await expect(list(page).getByRole('link')).toHaveCount(3);
  await expect(page.getByText('2 decisions, 1 archived', { exact: true })).toBeVisible();
  await expect(list(page).getByText('Adopt pre-registration for studies')).toHaveCount(0);

  await page.getByLabel('Show archived').uncheck();
  await expect(page).toHaveURL(/\/ledger$/);
  await expect(list(page).getByRole('link')).toHaveCount(2);
});

test('search narrows the list and lives in the url', async ({ page }) => {
  await page.goto('/ledger');

  await page.getByLabel('Search ledger').fill('arrow');

  await expect(page).toHaveURL(/\/ledger\?q=arrow$/);
  await expect(list(page).getByRole('link')).toHaveCount(1);
  await expect(list(page).getByRole('link')).toContainText('Use Arrow IPC snapshots');
  await expect(page.getByLabel('Search ledger')).toBeFocused();

  await page.goto('/ledger?q=lazy');
  await expect(page.getByLabel('Search ledger')).toHaveValue('lazy');
  await expect(list(page).getByRole('link')).toHaveCount(1);
});

test('tag facet filters by prefix and offers a clear control', async ({ page }) => {
  await page.goto('/ledger');

  await expect(facet(page).getByRole('button', { name: 'Clear' })).toHaveCount(0);
  const processing = facet(page).getByRole('button', { name: /^dadr\/data\/processing/ });
  await processing.click();

  await expect(page).toHaveURL(/\/ledger\?tag=%23dadr%2Fdata%2Fprocessing$/);
  await expect(processing).toHaveAttribute('aria-pressed', 'true');
  await expect(facet(page).getByRole('button', { name: 'Clear' })).toBeVisible();
  await expect(list(page).getByRole('link')).toHaveCount(2);

  // The namespace row still filters by prefix.
  await facet(page).getByRole('button', { name: /^dadr\/data / }).click();
  await expect(page).toHaveURL(/\/ledger\?tag=%23dadr%2Fdata$/);
  await expect(list(page).getByRole('link')).toHaveCount(2);

  await facet(page).getByRole('button', { name: /^dadr\/methodology/ }).click();
  await expect(page).toHaveURL(/\/ledger\?tag=%23dadr%2Fmethodology$/);
  await expect(list(page).getByRole('link')).toHaveCount(0);
  await expect(list(page).locator('[data-ui-empty-state]')).toBeVisible();

  await facet(page).getByRole('button', { name: 'Clear' }).click();
  await expect(page).toHaveURL(/\/ledger$/);
  await expect(list(page).getByRole('link')).toHaveCount(2);
  await expect(facet(page).getByRole('button', { name: 'Clear' })).toHaveCount(0);
});

test('selecting a row shows the record and its history', async ({ page }) => {
  await page.goto('/ledger?archived=1');

  await list(page).getByRole('link', { name: /Use Polars lazy frames/ }).click();

  await expect(page).toHaveURL(new RegExp(`/ledger\\?archived=1&id=${lazyFramesId}$`));
  await expect(detail(page).getByRole('heading', { name: 'Use Polars lazy frames' }).first()).toBeVisible();
  await expect(detail(page).getByText('use-polars-lazy.md')).toBeVisible();
  await expect(detail(page).getByRole('link', { name: '#dadr/data/processing' })).toHaveAttribute(
    'href',
    '/ledger?tag=%23dadr%2Fdata%2Fprocessing',
  );
  await expect(detail(page).getByRole('button', { name: 'Edit', exact: true })).toBeVisible();
  await expect(detail(page).getByRole('button', { name: 'Revise with agent' })).toBeVisible();
  await expect(detail(page).getByRole('button', { name: 'Supersede' })).toBeVisible();
  await expect(detail(page).locator('[data-action-menu]')).toHaveCount(0);
  await expect(detail(page).locator('[data-record-supersession]')).toHaveCount(0);

  await detail(page).getByRole('tab', { name: /History/ }).click();
  const history = detail(page).locator('[data-record-history]');
  await expect(history.locator('[data-history-version]')).toHaveCount(2);
  await expect(history.locator('[data-history-opened]')).toContainText('Use Polars lazy frames');
  await expect(history.locator('[data-history-opened]')).toContainText('current');
  await expect(history.getByText('No reason recorded.')).toBeVisible();
  const predecessor = history.getByRole('link', { name: 'Use Polars over pandas' });
  await expect(predecessor).toHaveAttribute('href', `/ledger?archived=1&id=${supersededId}`);
  await predecessor.click();

  await expect(page).toHaveURL(new RegExp(`/ledger\\?archived=1&id=${supersededId}$`));
  await expect(detail(page).getByRole('heading', { name: 'Use Polars over pandas' }).first()).toBeVisible();
  await expect(detail(page).getByText('use-polars.md')).toBeVisible();
  const supersededBy = detail(page).locator('[data-record-superseded-by]');
  await expect(supersededBy).toContainText('Superseded by');
  await expect(supersededBy.getByRole('link', { name: '“Use Polars lazy frames”' })).toHaveAttribute(
    'href',
    `/ledger?archived=1&id=${lazyFramesId}`,
  );
  // An archived record is corrected in place, never superseded or revised.
  await expect(detail(page).getByRole('button', { name: 'Supersede' })).toHaveCount(0);
  await expect(detail(page).getByRole('button', { name: 'Edit', exact: true })).toBeVisible();
  await expect(detail(page).getByRole('button', { name: 'Revise with agent' })).toHaveCount(0);
  await expect(list(page).getByRole('link', { name: /Use Polars over pandas/ })).toHaveAttribute('aria-current', 'page');
});

test('supersede opens the Draft page for that decision', async ({ page }) => {
  await page.goto(`/ledger?id=${arrowId}`);

  await detail(page).getByRole('button', { name: 'Supersede' }).click();

  await expect(page).toHaveURL(/\/draft$/);
  await expect(page.locator('[data-draft-composer]').getByRole('heading', { name: 'Supersede “Use Arrow IPC snapshots”' })).toBeVisible();
});

test('narrow screens open records on their own page', async ({ page }) => {
  await page.setViewportSize({ width: 375, height: 700 });
  await page.goto('/ledger');

  await expect(detail(page)).toBeHidden();
  await list(page).getByRole('link', { name: /Use Arrow IPC snapshots/ }).click();

  await expect(page).toHaveURL(new RegExp(`/ledger/${arrowId}$`));
  await expect(page.getByRole('link', { name: 'Back to ledger' })).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Use Arrow IPC snapshots' }).first()).toBeVisible();
  await expect(page.getByRole('button', { name: 'Supersede' })).toBeVisible();

  await page.getByRole('link', { name: 'Back to ledger' }).click();
  await expect(page).toHaveURL(new RegExp(`/ledger\\?id=${arrowId}$`));
  await expect(list(page).getByRole('link', { name: /Use Arrow IPC snapshots/ })).toBeVisible();
});

test('a candidate deep link points back to the inbox', async ({ page }) => {
  await page.goto(`/ledger?id=${candidateId}`);

  await expect(detail(page).getByText('This is a candidate')).toBeVisible();
  await expect(detail(page).getByRole('link', { name: 'Review it in the Inbox.' })).toHaveAttribute(
    'href',
    `/inbox?item=candidate%3A${candidateId}`,
  );
  await expect(detail(page).getByRole('button', { name: 'Edit' })).toBeVisible();
  await expect(detail(page).getByRole('button', { name: 'Accept' })).toHaveCount(0);
  await expect(detail(page).getByRole('button', { name: 'Reject' })).toHaveCount(0);
  await expect(detail(page).getByRole('button', { name: 'Supersede' })).toHaveCount(0);
  await expect(list(page).getByText('Adopt pre-registration for studies')).toHaveCount(0);
});

test('an uninitialized project shows the initialize prompt instead of the ledger', async ({ page }) => {
  await page.addInitScript(() => {
    localStorage.setItem('dadrock:mock-initialized', 'false');
  });

  await page.goto('/ledger');

  await expect(page.getByRole('heading', { name: 'Initialize Dadrock here?' })).toBeVisible();
  await expect(list(page)).toHaveCount(0);
});
