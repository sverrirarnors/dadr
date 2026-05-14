# Record Task Deeplinks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let record detail surfaces offer status-aware task actions that route to `/tasks` with the selected task, record input, and task context prefilled.

**Architecture:** Keep `/tasks` as the only task runner and result surface. `RecordDetailPanel.svelte` emits plain navigation links to `/tasks`, while the tasks page reads query parameters and applies them after task discovery loads. The URL is the handoff boundary; long user prompts stay on `/tasks`.

**Tech Stack:** Svelte 5, SvelteKit routing helpers, TanStack Svelte Query, Playwright e2e tests, Bun web tooling.

---

### Task 1: Cover Record Action Links

**Files:**
- Modify: `crates/dadrock/web/tests/e2e/records-split.spec.ts`

- [x] **Step 1: Write failing tests**

Add e2e coverage that the shared record detail panel shows candidate and accepted-record task links and routes through `/tasks`.

```ts
test('records route links candidate records to the revise task', async ({ page }) => {
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/records?status=candidate&id=${candidateId}`);

  const action = page.getByRole('link', { name: 'Revise with agent' });
  await expect(action).toBeVisible();
  await expect(action).toHaveAttribute('href', `/tasks?task=edit-record&record=${candidateId}`);

  await action.click();

  await expect(page).toHaveURL(new RegExp(`/tasks\\?task=edit-record&record=${candidateId}$`));
});

test('records route links accepted records to the supersession task', async ({ page }) => {
  const acceptedId = '0193fb1f-a3a0-7000-8000-000000000003';
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto(`/records?id=${acceptedId}`);

  const action = page.getByRole('link', { name: 'Suggest supersession' });
  await expect(action).toBeVisible();
  await expect(action).toHaveAttribute('href', `/tasks?task=supersede-record&record=${acceptedId}`);

  await action.click();

  await expect(page).toHaveURL(new RegExp(`/tasks\\?task=supersede-record&record=${acceptedId}$`));
});
```

- [x] **Step 2: Verify red**

Run:

```bash
bun run test:e2e -- records-split.spec.ts
```

Expected: the two new tests fail because no task action links are present.

### Task 2: Cover Task Route Prefill

**Files:**
- Modify: `crates/dadrock/web/tests/e2e/tasks.spec.ts`

- [x] **Step 1: Write failing tests**

Add tests that `/tasks` applies `task` and `record` query parameters to the existing form and context UI.

```ts
test('tasks route prefills edit-record from query parameters', async ({ page }) => {
  const recordId = '0193fb1f-a3a0-7000-8000-000000000002';
  await page.goto(`/tasks?task=edit-record&record=${recordId}`);

  await expect(page.getByRole('combobox', { name: 'Task' })).toHaveValue('edit-record');
  await expect(page.getByLabel('Candidate record')).toHaveValue(recordId);
  await expect(page.getByLabel('Requested changes')).toHaveValue('');
  await page.getByRole('button', { name: 'Additional context' }).click();
  await expect(page.getByLabel('Related records')).toHaveValue(recordId);
});

test('tasks route prefills supersede-record from query parameters', async ({ page }) => {
  const recordId = '0193fb1f-a3a0-7000-8000-000000000003';
  await page.goto(`/tasks?task=supersede-record&record=${recordId}`);

  await expect(page.getByRole('combobox', { name: 'Task' })).toHaveValue('supersede-record');
  await expect(page.getByLabel('Record to supersede')).toHaveValue(recordId);
  await expect(page.getByLabel('Reason')).toHaveValue('');
  await page.getByRole('button', { name: 'Additional context' }).click();
  await expect(page.getByLabel('Related records')).toHaveValue(recordId);
});
```

- [x] **Step 2: Verify red**

Run:

```bash
bun run test:e2e -- tasks.spec.ts
```

Expected: the new tests fail because the tasks page ignores `task` and `record` query parameters.

### Task 3: Implement Record Task Links

**Files:**
- Modify: `crates/dadrock/web/src/lib/components/records/RecordDetailPanel.svelte`

- [x] **Step 1: Add task link helpers and icon**

Add `Sparkles` to the lucide import and add a `taskLink` helper:

```ts
import { ArrowLeft, Check, Sparkles, X } from 'lucide-svelte';

function taskLink(taskId: string, recordId: string): '/tasks' | `/tasks?${string}` {
  return `/tasks?task=${encodeURIComponent(taskId)}&record=${encodeURIComponent(recordId)}`;
}
```

- [x] **Step 2: Render status-aware links**

In the record toolbar, render links that route to `/tasks`:

```svelte
{#if isCandidate}
  <a
    class="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-bg px-2.5 text-sm font-medium text-fg-muted transition-colors hover:bg-surface hover:text-fg"
    href={resolve(taskLink('edit-record', lifecycle.data.record.id))}
  >
    <Sparkles class="h-3.5 w-3.5" aria-hidden="true" />
    <span>Revise with agent</span>
  </a>
{:else if displayStatus(lifecycle.data.record) === 'accepted'}
  <a
    class="inline-flex h-8 items-center gap-1.5 rounded-md border border-border bg-bg px-2.5 text-sm font-medium text-fg-muted transition-colors hover:bg-surface hover:text-fg"
    href={resolve(taskLink('supersede-record', lifecycle.data.record.id))}
  >
    <Sparkles class="h-3.5 w-3.5" aria-hidden="true" />
    <span>Suggest supersession</span>
  </a>
{/if}
```

- [x] **Step 3: Verify record tests pass**

Run:

```bash
bun run test:e2e -- records-split.spec.ts
```

Expected: record split tests pass.

### Task 4: Implement Task Query Prefill

**Files:**
- Modify: `crates/dadrock/web/src/routes/tasks/+page.svelte`

- [x] **Step 1: Add prefill tracking state**

Add state near the task state declarations:

```ts
let appliedPrefillKey = $state('');
```

- [x] **Step 2: Apply query prefill after task discovery**

Add an effect after task selection initialization:

```ts
$effect(() => {
  if (!tasks.length) return;
  const taskId = page.url.searchParams.get('task') ?? '';
  const recordId = page.url.searchParams.get('record') ?? '';
  const key = `${taskId}:${recordId}`;
  if (!taskId || appliedPrefillKey === key) return;
  const task = tasks.find((candidate) => candidate.manifest.id === taskId);
  if (!task) return;
  applyTaskPrefill(task, recordId);
  appliedPrefillKey = key;
});
```

- [x] **Step 3: Add helper functions**

Add helpers near `selectTask`:

```ts
function applyTaskPrefill(task: TaskPackage, recordId: string) {
  selectedTaskId = task.manifest.id;
  const nextInput = defaultInputValues(task);
  if (recordId) {
    for (const input of task.manifest.inputs) {
      if (input.name === 'record_id') {
        nextInput[input.name] = recordId;
      }
    }
  }
  inputValues = nextInput;
  taskContext = emptyTaskContext();
  pathContextText = '';
  recordContextText = recordId;
  contextOpen = Boolean(recordId);
  clearTaskResult();
}
```

- [x] **Step 4: Preserve query params when switching tabs**

Update `setTab` so changing between task/history/results does not discard `task` and `record` query parameters:

```ts
function setTab(tab: TaskTab) {
  selectedTab = tab;
  const next = new URL(page.url);
  if (tab === 'tasks') {
    next.searchParams.delete('tab');
  } else {
    next.searchParams.set('tab', tab);
  }
  const target = (next.search ? `/tasks${next.search}` : '/tasks') as '/tasks' | `/tasks?${string}`;
  window.history.replaceState(window.history.state, '', resolve(target));
}
```

- [x] **Step 5: Verify task tests pass**

Run:

```bash
bun run test:e2e -- tasks.spec.ts
```

Expected: task e2e tests pass.

### Task 5: Final Verification and Commit

**Files:**
- Modify: `docs/superpowers/plans/2026-05-10-record-task-deeplinks.md`

- [x] **Step 1: Run focused verification**

Run:

```bash
bun run check
bun run test:e2e -- records-split.spec.ts tasks.spec.ts
```

Expected: zero Svelte diagnostics and all focused e2e tests pass.

- [x] **Step 2: Run unit tests**

Run:

```bash
bun run test:unit
```

Expected: unit tests pass.

- [x] **Step 3: Mark plan complete**

Change the unchecked boxes in this plan to checked boxes after the commands above pass.

- [x] **Step 4: Commit**

Run:

```bash
git add docs/superpowers/plans/2026-05-10-record-task-deeplinks.md crates/dadrock/web/src/lib/components/records/RecordDetailPanel.svelte crates/dadrock/web/src/routes/tasks/+page.svelte crates/dadrock/web/tests/e2e/records-split.spec.ts crates/dadrock/web/tests/e2e/tasks.spec.ts
git commit -m "Add record task deeplinks"
```

Expected: one feature commit on branch `task-record-deeplinks`.

---

## Self-Review

- Spec coverage: Covers record-surface task links, `/tasks` as the only runner, query-based task and record handoff, and no inline runner.
- Placeholder scan: No TBD/TODO placeholders remain.
- Type consistency: Uses existing `TaskPackage`, `TaskInputValue`, `TaskContext`, `RecordSummary`, and route helper patterns.
