import { expect, test } from '@playwright/test';

test('tags route renders the snapshot tag hierarchy', async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 260 });
  await page.goto('/tags');

  await expect(page.locator('[data-workbench-page][data-layout="single"]')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Tag explorer' })).toHaveCount(0);
  await expect(page.getByRole('heading', { name: 'Tags' }).nth(1)).toBeVisible();
  await expect(page.locator('[data-workbench-pane]')).toHaveCSS('border-top-width', '0px');
  await expect(page.getByText('Browse tag namespaces and jump into filtered records.')).not.toBeVisible();
  await expect(page.getByRole('link', { name: 'View records tagged #dadr', exact: true })).toBeVisible();
  await expect(page.getByRole('link', { name: /#dadr\/data\/processing/ })).toBeVisible();
  await expect(page.getByRole('link', { name: /#dadr\/methodology/ })).toBeVisible();
  await expect(page.getByText('2 tags')).toBeVisible();

  const metrics = await page.evaluate(() => {
    const main = document.querySelector('main');
    const pane = document.querySelector('[data-tags-shell-pane]');
    const list = document.querySelector('[data-tags-pane]');
    if (!main || !pane || !list) throw new Error('missing tags shell');
    return {
      mainOverflowY: getComputedStyle(main).overflowY,
      mainClientHeight: main.clientHeight,
      mainScrollHeight: main.scrollHeight,
      paneOverflowY: getComputedStyle(pane).overflowY,
      listOverflowY: getComputedStyle(list).overflowY,
      listClientHeight: list.clientHeight,
      paneClientHeight: pane.clientHeight,
    };
  });

  expect(metrics.mainOverflowY).toBe('hidden');
  expect(metrics.mainScrollHeight).toBe(metrics.mainClientHeight);
  expect(metrics.paneOverflowY).toBe('hidden');
  expect(metrics.listOverflowY).toBe('auto');
  expect(metrics.listClientHeight).toBeLessThan(metrics.paneClientHeight);
});

test('tag links navigate to the records list filtered by tag', async ({ page }) => {
  await page.goto('/tags');

  await page.getByRole('link', { name: /#dadr\/data\/processing/ }).click();

  await expect(page).toHaveURL(/\/records\?scope=tag&q=%23dadr%2Fdata%2Fprocessing$/);
  await expect(page.getByLabel('Search scope')).toBeVisible();
  await expect(page.getByLabel('Search scope')).toHaveValue('tag');
  await expect(page.getByRole('link', { name: /Use Polars over pandas/ })).toBeVisible();
  await expect(page.getByRole('link', { name: /Adopt pre-registration for studies/ })).not.toBeVisible();
});

test('tags route uses a compact nested one-column list', async ({
  page,
}) => {
  await page.setViewportSize({ width: 1100, height: 800 });
  await page.goto('/tags');
  await expect(page.getByRole('link', { name: 'View records tagged #dadr', exact: true })).toBeVisible();

  const wideColumns = await page.locator('[data-tags-pane]').evaluate((element) => {
    return getComputedStyle(element).gridTemplateColumns.split(' ').length;
  });
  expect(wideColumns).toBe(1);

  const parent = page.getByRole('link', { name: 'View records tagged #dadr', exact: true });
  const child = page.getByRole('link', { name: /#dadr\/data\/processing/ });
  const namespace = parent.getByText('namespace');
  await expect(namespace).toBeVisible();

  const [parentBox, parentLabelBox, parentCountBox, childLabelBox] = await Promise.all([
    parent.boundingBox(),
    parent.getByText('#dadr', { exact: true }).boundingBox(),
    parent.locator('[data-tag-count]').boundingBox(),
    child.getByText('#dadr/data/processing').boundingBox(),
  ]);
  if (!parentBox || !parentLabelBox || !parentCountBox || !childLabelBox) {
    throw new Error('expected tag row boxes');
  }
  expect(childLabelBox.x).toBeGreaterThan(parentLabelBox.x);
  expect(parentCountBox.x - (parentLabelBox.x + parentLabelBox.width)).toBeLessThanOrEqual(120);
  expect(parentBox.height).toBeLessThanOrEqual(36);

  await page.setViewportSize({ width: 640, height: 800 });
  const narrowColumns = await page.locator('[data-tags-pane]').evaluate((element) => {
    return getComputedStyle(element).gridTemplateColumns.split(' ').length;
  });
  expect(narrowColumns).toBe(1);
});

test('tags route scrolls long tag lists inside the pane', async ({ page }) => {
  await page.setViewportSize({ width: 1100, height: 260 });
  await page.goto('/tags');
  await expect(page.getByRole('link', { name: 'View records tagged #dadr', exact: true })).toBeVisible();

  const metrics = await page.locator('[data-tags-pane]').evaluate((list) => {
    const firstRow = list.querySelector('a');
    if (!firstRow) throw new Error('expected tag row');

    for (let index = 0; index < 80; index += 1) {
      const row = firstRow.cloneNode(true) as HTMLElement;
      const label = row.querySelector('.truncate');
      if (label) label.textContent = `#overflow/tag/${String(index).padStart(2, '0')}`;
      list.appendChild(row);
    }

    return {
      clientHeight: list.clientHeight,
      scrollHeight: list.scrollHeight,
    };
  });

  expect(metrics.scrollHeight).toBeGreaterThan(metrics.clientHeight);

  await page.locator('[data-tags-pane]').hover();
  await page.mouse.wheel(0, 600);

  await expect
    .poll(async () => page.locator('[data-tags-pane]').evaluate((list) => list.scrollTop))
    .toBeGreaterThan(0);
  await expect
    .poll(async () => page.evaluate(() => document.scrollingElement?.scrollTop ?? 0))
    .toBe(0);
});
