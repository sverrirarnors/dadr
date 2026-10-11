import { expect, test, type Page } from '@playwright/test';
import { editorValue, fillEditor } from './editor';

// Default fixture.
const LAZY_ID = '0193fb1f-a3a0-7000-8000-000000000003';
const ARROW_ID = '0193fb1f-a3a0-7000-8000-000000000004';

// History scenario (src/lib/api/mock-records.ts).
const id = (n: number) => `0193fb1f-a3a0-7000-8000-${String(n).padStart(12, '0')}`;
const IMPUTATION = id(1002);
const OUTLIERS = id(2001);
const INCOME = id(2002);
const AGES = id(2003);
const MERGE = id(3003);
const PANDAS = id(4001);
const POLARS = id(4002);
const STALE = id(4003);
const SAS = id(5001);

function useHistoryScenario(page: Page) {
  return page.addInitScript(() => localStorage.setItem('dadrock:mock-scenario', 'history'));
}

function ledgerDetail(page: Page) {
  return page.locator('[data-record-detail-pane]');
}

function inboxDetail(page: Page) {
  return page.locator('[data-inbox-detail]');
}

/** Navigate inside the app, keeping the mock project's state (a reload would reset it). */
async function navigate(page: Page, href: string) {
  await page.evaluate((target) => {
    const link = document.createElement('a');
    link.href = target;
    link.textContent = 'e2e navigation';
    link.dataset.e2eNavigation = '';
    document.body.append(link);
  }, href);
  await page.locator('a[data-e2e-navigation]').last().click();
  await expect(page).toHaveURL((url) => `${url.pathname}${url.search}` === href);
}

function status(page: Page, text: string | RegExp) {
  return page.getByRole('status').filter({ hasText: text });
}

test('Edit drafts a successor that the Inbox reviews as changes, and History keeps both versions', async ({ page }) => {
  await page.goto(`/ledger?id=${ARROW_ID}`);
  await ledgerDetail(page).getByRole('button', { name: 'Edit', exact: true }).click();

  await expect(page).toHaveURL(/\/draft$/);
  const draft = page.locator('[data-draft-composer]');
  await expect(draft.getByRole('heading', { name: 'Edit “Use Arrow IPC snapshots”' })).toBeVisible();
  await expect(page.getByRole('dialog')).toHaveCount(0);
  await expect(draft.getByLabel('Title')).toHaveValue('Use Arrow IPC snapshots');
  await expect(draft.getByLabel('Tags')).toHaveValue('#dadr/data/processing');
  const markdown = draft.getByLabel('Successor Markdown');
  await expect(markdown).toHaveAttribute('contenteditable', 'true');
  const original = await editorValue(markdown);
  expect(original).toContain('## Decision\n\nUse Arrow IPC snapshots for portable intermediate data.');
  expect(original).not.toContain('## Supersedes');

  const reason = draft.getByLabel('Why does this decision change?');
  await expect(reason).toBeFocused();
  const create = draft.getByRole('button', { name: 'Create candidate' });
  await expect(create).toBeDisabled();
  await reason.fill('Parquet files compress better and every tool reads them.');
  await expect(create).toBeDisabled();
  await expect(draft.locator('[data-draft-successor-hint]')).toHaveText('Nothing has changed yet. Edit the text, then save.');

  await fillEditor(
    markdown,
    original.replace('Use Arrow IPC snapshots for portable intermediate data.', 'Use Parquet files for portable intermediate data.'),
  );
  await expect(create).toBeEnabled();
  await create.click();

  await expect(page).toHaveURL(/\/inbox\?item=candidate%3A/);
  await expect(status(page, 'created candidate')).toBeVisible();
  const detail = inboxDetail(page);
  await expect(detail.getByRole('tab', { name: 'Changes' })).toHaveAttribute('aria-selected', 'true');
  await expect(detail.locator('[data-successor-reason]')).toHaveText('Parquet files compress better and every tool reads them.');
  const diff = detail.locator('[data-record-diff]');
  await expect(diff.locator('[data-diff-line="remove"]')).toHaveText([/Use Arrow IPC snapshots for portable intermediate data\./]);
  await expect(diff.locator('[data-diff-line="add"]')).toHaveText([/Use Parquet files for portable intermediate data\./]);
  await expect(diff.locator('[data-record-diff-title]')).toHaveCount(0);

  await detail.getByRole('button', { name: 'Accept' }).click();
  await expect(status(page, 'accepted candidate')).toBeVisible();

  await navigate(page, `/ledger?archived=1&id=${ARROW_ID}`);
  await expect(ledgerDetail(page).locator('[data-record-superseded-by]')).toContainText('Superseded by “Use Arrow IPC snapshots”');
  await expect(ledgerDetail(page).getByRole('button', { name: 'Edit', exact: true })).toHaveCount(0);
  await ledgerDetail(page).locator('[data-record-superseded-by]').getByRole('link').click();

  await ledgerDetail(page).getByRole('tab', { name: /History/ }).click();
  const history = ledgerDetail(page).locator('[data-record-history]');
  await expect(history.locator('[data-history-version]')).toHaveCount(2);
  await expect(history.locator('[data-history-version]').first()).toHaveAttribute('data-history-opened', 'true');
  await expect(history.locator('[data-history-version]').first()).toContainText('current');
  await expect(history.locator('[data-history-reason]')).toHaveText('Parquet files compress better and every tool reads them.');
  await expect(history.locator(`[data-history-version="${ARROW_ID}"]`)).toContainText('Use Arrow IPC snapshots');
  await history.getByText('Changes from “Use Arrow IPC snapshots”').click();
  await expect(history.locator('[data-diff-line="add"]')).toHaveText([/Use Parquet files for portable intermediate data\./]);
});

test('Revise with agent and Supersede draft successors with the agent, several decisions at once', async ({ page }) => {
  await page.goto(`/ledger?id=${ARROW_ID}`);
  await ledgerDetail(page).getByRole('button', { name: 'Revise with agent' }).click();
  const draft = page.locator('[data-draft-composer]');
  await expect(draft.getByRole('heading', { name: 'Supersede “Use Arrow IPC snapshots”' })).toBeVisible();
  await draft.getByLabel('What changed, and why?').fill('Parquet instead.');
  await expect(draft.locator('[data-draft-intent]')).toHaveText('Draft a successor for “Use Arrow IPC snapshots”');

  // Attaching a second current decision turns it into a merge.
  await draft.getByRole('button', { name: 'Attach records' }).click();
  const picker = page.getByRole('dialog', { name: 'Attach records' });
  await expect(picker.getByLabel('Select Use Polars over pandas')).toHaveCount(0);
  await picker.getByLabel('Select Use Polars lazy frames').check();
  await picker.getByRole('button', { name: 'Attach 1 record' }).click();
  await expect(draft.getByRole('heading', { name: 'Supersede 2 decisions' })).toBeVisible();
  await expect(draft.locator('[data-draft-intent]')).toHaveText('Draft one successor for 2 decisions');
  await draft.getByRole('button', { name: 'Send to agent' }).click();

  await expect(page).toHaveURL(/\/inbox$/);
  await expect(
    page.locator('[data-inbox-group="proposals"]').getByText('Supersedes “Use Arrow IPC snapshots”, “Use Polars lazy frames”'),
  ).toBeVisible();
});

test('a merge written by hand opens on the section view', async ({ page }) => {
  await page.goto(`/ledger?draft=supersede&records=${LAZY_ID},${ARROW_ID}`);
  const draft = page.locator('[data-draft-composer]');
  await draft.getByRole('button', { name: 'Write it by hand' }).click();
  await expect(draft.getByRole('heading', { name: 'Supersede 2 decisions' })).toBeVisible();
  await expect(draft.getByLabel('Title')).toHaveValue('');
  const markdown = draft.getByLabel('Successor Markdown');
  await expect.poll(() => editorValue(markdown)).toContain('Use lazy frames for transformations');
  expect(await editorValue(markdown)).toContain('Use Arrow IPC snapshots for portable intermediate data.');
  await draft.getByLabel('Title').fill('One data pipeline');
  await draft.getByLabel('Why does this decision change?').fill('Both describe the same pipeline.');
  await draft.getByRole('button', { name: 'Create candidate' }).click();

  await expect(page).toHaveURL(/\/inbox\?item=candidate%3A/);
  const detail = inboxDetail(page);
  await expect(detail.getByLabel('Compare with')).toHaveValue('sections');
  await expect(detail.locator('[data-section-compare-section]').first()).toHaveAttribute('data-section-compare-section', 'Decision');
});

test('a merge candidate compares section by section, or against one predecessor', async ({ page }) => {
  await useHistoryScenario(page);
  await page.goto(`/inbox?item=candidate%3A${MERGE}`);

  const detail = inboxDetail(page);
  await expect(detail.getByRole('tab', { name: 'Changes' })).toHaveAttribute('aria-selected', 'true');
  await expect(detail.locator('[data-successor-reason]')).toContainText('The two rules disagreed at the tails');
  const compare = detail.getByLabel('Compare with');
  await expect(compare).toHaveValue('sections');
  const sections = detail.locator('[data-section-compare-section]');
  await expect(sections).toHaveCount(4);
  const decision = sections.first();
  await expect(decision).toHaveAttribute('data-section-compare-section', 'Decision');
  await expect(decision).toContainText('Trim income above the 95th percentile.');
  await expect(decision).toContainText('Cap recorded ages at 99.');
  await expect(decision.locator('[data-section-compare-new]')).toContainText(
    'Winsorize every continuous covariate at the 99th percentile.',
  );

  await compare.selectOption({ label: 'Cap ages at 99' });
  await expect(detail.locator('[data-section-compare]')).toHaveCount(0);
  await expect(detail.locator('[data-record-diff-title]')).toContainText('Cap ages at 99');
  await expect(detail.locator('[data-record-diff-title]')).toContainText('One outlier rule for all covariates');
  await expect(detail.locator('[data-diff-line="remove"]').filter({ hasText: 'Cap recorded ages at 99.' })).toBeVisible();
});

test('candidates that supersede the same decision are one item and are accepted together', async ({ page }) => {
  await useHistoryScenario(page);
  await page.goto('/inbox');

  const candidates = page.locator('[data-inbox-group="candidates"]');
  const split = candidates.getByRole('link', { name: /Split “Outlier rule for covariates” into 2 decisions/ });
  await expect(split).toBeVisible();
  await expect(candidates.getByRole('link', { name: /^CAN Winsorize income at p99/ })).toHaveCount(0);
  await split.click();

  const detail = inboxDetail(page);
  await expect(detail.locator('[data-split-candidate]')).toHaveCount(2);
  const income = detail.locator(`[data-split-candidate="${INCOME}"]`);
  await expect(income).toContainText('Income and age need different rules.');
  await expect(income.locator('[data-diff-line="add"]').filter({ hasText: 'Winsorize income at the 99th percentile.' })).toBeVisible();
  await expect(detail.locator(`[data-split-candidate="${AGES}"]`)).toContainText('Ages above 110 are data entry errors');

  await detail.getByRole('button', { name: 'Accept selected' }).click();
  await expect(status(page, 'accepted 2 candidates')).toBeVisible();
  await expect(split).toHaveCount(0);

  await navigate(page, `/ledger?archived=1&id=${OUTLIERS}`);
  const supersededBy = ledgerDetail(page).locator('[data-record-superseded-by]');
  await expect(supersededBy).toContainText('Winsorize income at p99');
  await expect(supersededBy).toContainText('Drop implausible ages');

  await navigate(page, `/ledger?id=${INCOME}&tab=history`);
  const history = ledgerDetail(page).locator('[data-record-history]');
  await expect(history.locator('[data-history-version]')).toHaveCount(2);
  await expect(history.locator('[data-history-neighbor="split"]')).toContainText('Also became “Drop implausible ages”');
});

test('competing alternatives: accept one, then reject the other', async ({ page }) => {
  await useHistoryScenario(page);
  await page.goto(`/inbox?item=candidate%3A${AGES}`);

  const detail = inboxDetail(page);
  await expect(detail.locator(`[data-split-candidate="${AGES}"]`)).toBeVisible();
  await detail.getByLabel('Select Drop implausible ages').uncheck();
  await expect(detail.getByText('1 of 2 selected')).toBeVisible();
  await detail.getByRole('button', { name: 'Accept selected' }).click();
  await expect(status(page, `accepted candidate ${INCOME}`)).toBeVisible();

  // The other one now names a decision that moved on.
  const remaining = page.locator('[data-inbox-group="candidates"]').getByRole('link', { name: /Drop implausible ages/ });
  await expect(remaining).toContainText('(superseded since)');
  await remaining.click();
  await expect(inboxDetail(page).getByText('Predecessor already superseded')).toBeVisible();
  await inboxDetail(page).getByRole('button', { name: 'Reject' }).click();
  await expect(status(page, 'rejected candidate')).toBeVisible();
  await expect(page.locator('[data-inbox-group="candidates"]').getByRole('link', { name: /Drop implausible ages/ })).toHaveCount(0);
});

test('a stale successor is retargeted at the decision now in force', async ({ page }) => {
  await useHistoryScenario(page);
  await page.goto(`/inbox?item=candidate%3A${STALE}`);

  const detail = inboxDetail(page);
  const warning = detail.locator(`[data-stale-supersedes="${PANDAS}"]`);
  await expect(warning).toContainText('“Use pandas for data frames” was superseded after this candidate was drafted');
  await expect(warning).toContainText('“Use Polars for data frames”');
  await expect(detail.getByRole('button', { name: 'Accept' })).toBeDisabled();
  await expect(detail.locator('[data-record-diff-title]')).toContainText('Use pandas for data frames');

  await warning.getByRole('button', { name: 'Retarget to “Use Polars for data frames”' }).click();
  await expect(status(page, 'Now supersedes “Use Polars for data frames”')).toBeVisible();
  await expect(detail.locator('[data-stale-supersedes]')).toHaveCount(0);
  await expect(detail.getByRole('button', { name: 'Accept' })).toBeEnabled();
  await expect(detail.locator('[data-record-diff-title]')).toContainText('Use Polars for data frames');
  await expect(detail.locator('[data-successor-reason]')).toHaveText('Arrow dtypes fix the memory blow-up.');

  await detail.getByRole('button', { name: 'Accept' }).click();
  await expect(status(page, `accepted candidate ${STALE}`)).toBeVisible();
  await navigate(page, `/ledger?id=${STALE}&tab=history`);
  const history = ledgerDetail(page).locator('[data-record-history]');
  await expect(history.locator('[data-history-version]')).toHaveCount(3);
  await expect(history.locator(`[data-history-version="${POLARS}"]`)).toBeVisible();
});

test('archived records have no Edit or Revise', async ({ page }) => {
  await useHistoryScenario(page);
  await page.goto(`/ledger?archived=1&id=${PANDAS}`);
  const detail = ledgerDetail(page);
  await expect(detail.getByRole('heading', { name: 'Use pandas for data frames' }).first()).toBeVisible();
  await expect(detail.locator('[data-record-superseded-by]').getByRole('link', { name: '“Use Polars for data frames”' })).toBeVisible();
  for (const name of ['Edit', 'Revise with agent', 'Supersede']) {
    await expect(detail.getByRole('button', { name, exact: true })).toHaveCount(0);
  }
  await expect(detail.getByRole('button', { name: 'Edit tags' })).toBeVisible();

  await page.goto(`/ledger?archived=1&id=${SAS}`);
  await expect(detail.getByRole('heading', { name: 'Use SAS for estimation' }).first()).toBeVisible();
  await expect(detail.locator('[data-record-rejected-notice]')).toBeVisible();
  for (const name of ['Edit', 'Revise with agent', 'Supersede']) {
    await expect(detail.getByRole('button', { name, exact: true })).toHaveCount(0);
  }
});

test('tags of an accepted decision are edited in place', async ({ page }) => {
  await useHistoryScenario(page);
  await page.goto(`/ledger?id=${IMPUTATION}`);
  const detail = ledgerDetail(page);

  await detail.getByRole('button', { name: 'Edit tags' }).click();
  const tags = detail.getByLabel('Tags', { exact: true });
  await expect(tags).toHaveValue('topic/missing-data kind/transformation');
  await expect(tags).toBeFocused();
  await tags.fill('topic/missing-data kind/transformation topic/imputation');
  await detail.getByRole('button', { name: 'Save tags' }).click();

  await expect(status(page, 'Tags saved')).toBeVisible();
  await expect(detail.getByRole('link', { name: 'topic/imputation' })).toBeVisible();
  await expect(detail.getByLabel('Tags', { exact: true })).toHaveCount(0);
  await expect(detail.getByRole('article')).toContainText('Impute income with chained equations');
});
