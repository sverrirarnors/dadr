# Obsidian-Like Workbench Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn Dadrock's web UI into one compact, document-centered workbench system, inspired by Obsidian's balance of dense navigation and readable notes, without copying Obsidian or changing Dadrock's data model.

**Architecture:** Build a shell-level workbench grammar first, then migrate pages into shared two/three-pane patterns. Records become the north-star implementation: compact explorer, document pane, inspector/action area. Tasks, validation, tags, candidate flows, and settings then become variations of the same browse-inspect-edit-act model.

**Tech Stack:** Svelte 5, SvelteKit, Tailwind CSS v4 tokens in `src/app.css`, `tailwind-variants`, Vitest, Playwright.

---

## Target Experience

Dadrock should feel like a local knowledge workbench:

- The document or selected object is visually central.
- Lists, filters, status, and actions are compact support surfaces.
- Pages share the same shell, panes, headers, rows, badges, alerts, and action bars.
- Records, Tasks, Validation, Tags, and Settings feel like views of one app, not separate mini-apps.
- Color is quiet and semantic: lifecycle state, severity, and one primary action.
- Cards are rare. Rows, panes, disclosure rows, and document surfaces are the default.

## Workbench Grammar

Create these primitives and use them broadly:

- `WorkbenchPage`: full-height route shell with optional side/center/inspector slots.
- `WorkbenchPane`: bordered pane with consistent background, overflow, and density.
- `PaneHeader`: compact title/subtitle/actions area.
- `PaneToolbar`: compact filters/search/segmented controls.
- `DocumentPane`: readable Markdown/editor surface with more breathing room.
- `InspectorPane`: compact metadata/action/diagnostic side area.
- `MetadataStrip`: row of badges/chips for status, file, tags, run kind, severity.
- `ActionBar`: consistent primary/secondary/destructive action placement.
- `DisclosureRow`: compact expandable row.
- `ListRow`: compact selectable row.
- `Badge`, `Alert`, `EmptyState`, `Field`, `Button`: shared atoms.

## Page Archetypes

- Records: explorer pane + document pane + optional inspector/actions.
- Tasks: workflow pane + result/document pane + optional run/history inspector.
- Validation: issue queue pane + selected issue/detail pane + optional filters.
- Tags: compact explorer pane + records/result pane.
- Candidate/detail/new pages: document/editor pane with metadata/action bars.
- Settings: inspector-style pane, not a separate visual universe.

---

## File Structure

Create:

- `crates/dadrock/web/src/lib/components/workbench/WorkbenchPage.svelte`
- `crates/dadrock/web/src/lib/components/workbench/WorkbenchPane.svelte`
- `crates/dadrock/web/src/lib/components/workbench/PaneHeader.svelte`
- `crates/dadrock/web/src/lib/components/workbench/PaneToolbar.svelte`
- `crates/dadrock/web/src/lib/components/workbench/DocumentPane.svelte`
- `crates/dadrock/web/src/lib/components/workbench/InspectorPane.svelte`
- `crates/dadrock/web/src/lib/components/workbench/MetadataStrip.svelte`
- `crates/dadrock/web/src/lib/components/workbench/ActionBar.svelte`
- `crates/dadrock/web/src/lib/components/workbench/index.ts`
- `crates/dadrock/web/src/lib/components/ui/badge.svelte`
- `crates/dadrock/web/src/lib/components/ui/alert.svelte`
- `crates/dadrock/web/src/lib/components/ui/empty-state.svelte`
- `crates/dadrock/web/src/lib/components/ui/disclosure-row.svelte`
- `crates/dadrock/web/src/lib/components/ui/list-row.svelte`
- `crates/dadrock/web/src/lib/components/ui/field.svelte`
- `crates/dadrock/web/tests/unit/workbench-primitives.test.ts`

Modify:

- `crates/dadrock/web/src/lib/components/ui/index.ts`
- `crates/dadrock/web/src/lib/components/ui/button/button.svelte`
- `crates/dadrock/web/src/app.css`
- `crates/dadrock/web/src/routes/+layout.svelte`
- `crates/dadrock/web/src/lib/components/records/RecordDetailPanel.svelte`
- `crates/dadrock/web/src/routes/records/+page.svelte`
- `crates/dadrock/web/src/routes/tasks/+page.svelte`
- `crates/dadrock/web/src/routes/validation/+page.svelte`
- `crates/dadrock/web/src/routes/tags/+page.svelte`
- `crates/dadrock/web/src/routes/settings/+page.svelte`
- `crates/dadrock/web/src/routes/candidates/[id]/+page.svelte`
- `crates/dadrock/web/src/routes/candidates/new/+page.svelte`

---

### Task 1: Workbench Primitive Foundation

**Files:**
- Create: `crates/dadrock/web/src/lib/components/workbench/*.svelte`
- Create: `crates/dadrock/web/src/lib/components/workbench/index.ts`
- Create: `crates/dadrock/web/src/lib/components/ui/badge.svelte`
- Create: `crates/dadrock/web/src/lib/components/ui/alert.svelte`
- Create: `crates/dadrock/web/src/lib/components/ui/empty-state.svelte`
- Create: `crates/dadrock/web/src/lib/components/ui/disclosure-row.svelte`
- Create: `crates/dadrock/web/src/lib/components/ui/list-row.svelte`
- Create: `crates/dadrock/web/src/lib/components/ui/field.svelte`
- Modify: `crates/dadrock/web/src/lib/components/ui/index.ts`
- Modify: `crates/dadrock/web/src/lib/components/ui/button/button.svelte`
- Test: `crates/dadrock/web/tests/unit/workbench-primitives.test.ts`

- [ ] **Step 1: Write the failing export test**

Create `crates/dadrock/web/tests/unit/workbench-primitives.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import { Alert, Badge, Button, DisclosureRow, EmptyState, Field, ListRow } from '../../src/lib/components/ui';
import {
  ActionBar,
  DocumentPane,
  InspectorPane,
  MetadataStrip,
  PaneHeader,
  PaneToolbar,
  WorkbenchPage,
  WorkbenchPane,
} from '../../src/lib/components/workbench';

describe('workbench primitive exports', () => {
  it('exports the UI atoms and workbench layout primitives', () => {
    expect(Button).toBeTruthy();
    expect(Badge).toBeTruthy();
    expect(Alert).toBeTruthy();
    expect(EmptyState).toBeTruthy();
    expect(DisclosureRow).toBeTruthy();
    expect(ListRow).toBeTruthy();
    expect(Field).toBeTruthy();
    expect(WorkbenchPage).toBeTruthy();
    expect(WorkbenchPane).toBeTruthy();
    expect(PaneHeader).toBeTruthy();
    expect(PaneToolbar).toBeTruthy();
    expect(DocumentPane).toBeTruthy();
    expect(InspectorPane).toBeTruthy();
    expect(MetadataStrip).toBeTruthy();
    expect(ActionBar).toBeTruthy();
  });
});
```

- [ ] **Step 2: Run the test and verify it fails**

```bash
cd crates/dadrock/web
bun run test:unit -- tests/unit/workbench-primitives.test.ts
```

Expected: FAIL because the workbench primitives do not exist.

- [ ] **Step 3: Add compact UI atoms**

Create these files with the indicated public props:

`badge.svelte`

```svelte
<script lang="ts" module>
  import { tv, type VariantProps } from 'tailwind-variants';
  export const badgeVariants = tv({
    base: 'inline-flex max-w-full items-center gap-1 rounded px-1.5 py-0.5 text-[11px] font-medium leading-4',
    variants: {
      variant: {
        neutral: 'bg-surface-2 text-fg-muted',
        accent: 'bg-accent-subtle text-accent',
        success: 'bg-success/10 text-success',
        warning: 'bg-warning/10 text-warning',
        danger: 'bg-danger/10 text-danger',
        candidate: 'bg-candidate/20 text-candidate',
        accepted: 'bg-accepted/20 text-accepted',
        rejected: 'bg-rejected/20 text-rejected',
        superseded: 'bg-superseded/20 text-superseded',
      },
      mono: { true: 'font-mono', false: '' },
    },
    defaultVariants: { variant: 'neutral', mono: false },
  });
  export type BadgeVariants = VariantProps<typeof badgeVariants>;
</script>

<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  type Props = HTMLAttributes<HTMLSpanElement> & BadgeVariants & { class?: string; children?: Snippet };
  let { class: className, variant, mono, children, ...rest }: Props = $props();
</script>

<span data-ui-badge class={cn(badgeVariants({ variant, mono }), className)} {...rest}>
  {@render children?.()}
</span>
```

`alert.svelte`

```svelte
<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  type Variant = 'neutral' | 'info' | 'success' | 'warning' | 'danger';
  const styles: Record<Variant, string> = {
    neutral: 'border-border bg-bg text-fg-muted',
    info: 'border-accent/40 bg-accent-subtle text-accent',
    success: 'border-success/40 bg-success/10 text-success',
    warning: 'border-warning/40 bg-warning/10 text-warning',
    danger: 'border-danger/40 bg-danger/5 text-danger',
  };
  type Props = HTMLAttributes<HTMLDivElement> & { variant?: Variant; title?: string; class?: string; children?: Snippet };
  let { variant = 'neutral', title = '', class: className, children, ...rest }: Props = $props();
</script>

<div data-ui-alert class={cn('rounded-md border px-3 py-2 text-sm', styles[variant], className)} {...rest}>
  {#if title}<div class="font-medium">{title}</div>{/if}
  {#if children}<div class={title ? 'mt-1 text-fg-muted' : ''}>{@render children()}</div>{/if}
</div>
```

`empty-state.svelte`

```svelte
<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  type Props = HTMLAttributes<HTMLDivElement> & { class?: string; children?: Snippet };
  let { class: className, children, ...rest }: Props = $props();
</script>

<div data-ui-empty-state class={cn('rounded-md border border-dashed border-border px-4 py-6 text-sm text-fg-subtle', className)} {...rest}>
  {@render children?.()}
</div>
```

`disclosure-row.svelte`

```svelte
<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  type Props = HTMLAttributes<HTMLDetailsElement> & {
    class?: string;
    summaryClass?: string;
    bodyClass?: string;
    summary: Snippet;
    children?: Snippet;
  };
  let { class: className, summaryClass = '', bodyClass = '', summary, children, ...rest }: Props = $props();
</script>

<details data-ui-disclosure-row class={cn('rounded-md border border-border bg-bg', className)} {...rest}>
  <summary class={cn('cursor-pointer px-3 py-2 text-sm text-fg', summaryClass)}>{@render summary()}</summary>
  {#if children}<div class={cn('border-t border-border px-3 py-2', bodyClass)}>{@render children()}</div>{/if}
</details>
```

`list-row.svelte`

```svelte
<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  type Props = HTMLAttributes<HTMLDivElement> & { class?: string; selected?: boolean; children?: Snippet };
  let { class: className, selected = false, children, ...rest }: Props = $props();
</script>

<div
  data-ui-list-row
  class={cn(
    'grid gap-2 border-b border-border px-3 py-2 text-sm last:border-b-0 md:grid-cols-[1fr_auto]',
    selected ? 'bg-accent/5' : '',
    className,
  )}
  {...rest}
>
  {@render children?.()}
</div>
```

`field.svelte`

```svelte
<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  type Props = HTMLAttributes<HTMLLabelElement> & {
    label: string;
    help?: string;
    required?: boolean;
    class?: string;
    children?: Snippet;
  };
  let { label, help = '', required = false, class: className, children, ...rest }: Props = $props();
</script>

<label data-ui-field class={cn('block text-sm', className)} {...rest}>
  <span class="mb-1 flex items-center gap-2 font-medium text-fg">
    <span>{label}</span>
    {#if required}<span class="text-[11px] font-normal text-fg-subtle">required</span>{/if}
  </span>
  {@render children?.()}
  {#if help}<span class="mt-1 block text-xs text-fg-subtle">{help}</span>{/if}
</label>
```

- [ ] **Step 4: Add workbench layout primitives**

Create `WorkbenchPage.svelte`:

```svelte
<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  type Props = HTMLAttributes<HTMLDivElement> & {
    layout?: 'single' | 'two-pane' | 'three-pane';
    class?: string;
    children?: Snippet;
  };
  let { layout = 'single', class: className, children, ...rest }: Props = $props();
  const layoutClass = {
    single: 'grid-cols-1',
    'two-pane': 'grid-cols-1 lg:grid-cols-[20rem_minmax(0,1fr)]',
    'three-pane': 'grid-cols-1 lg:grid-cols-[18rem_minmax(0,1fr)_20rem]',
  };
</script>

<div data-workbench-page class={cn('grid h-full min-h-0 gap-2 overflow-hidden p-2', layoutClass[layout], className)} {...rest}>
  {@render children?.()}
</div>
```

Create `WorkbenchPane.svelte`:

```svelte
<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  type Props = HTMLAttributes<HTMLElement> & { class?: string; children?: Snippet };
  let { class: className, children, ...rest }: Props = $props();
</script>

<section data-workbench-pane class={cn('flex min-h-0 flex-col overflow-hidden rounded-md border border-border bg-surface', className)} {...rest}>
  {@render children?.()}
</section>
```

Create `PaneHeader.svelte`:

```svelte
<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  type Props = HTMLAttributes<HTMLDivElement> & { title: string; subtitle?: string; class?: string; actions?: Snippet };
  let { title, subtitle = '', class: className, actions, ...rest }: Props = $props();
</script>

<div data-pane-header class={cn('flex flex-wrap items-start justify-between gap-3 border-b border-border px-3 py-2', className)} {...rest}>
  <div class="min-w-0">
    <h2 class="truncate text-sm font-semibold text-fg">{title}</h2>
    {#if subtitle}<p class="mt-0.5 truncate text-xs text-fg-muted">{subtitle}</p>{/if}
  </div>
  {#if actions}<div class="flex shrink-0 flex-wrap items-center gap-2">{@render actions()}</div>{/if}
</div>
```

Create `PaneToolbar.svelte`, `DocumentPane.svelte`, `InspectorPane.svelte`, `MetadataStrip.svelte`, and `ActionBar.svelte` using the same pattern:

```svelte
<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLAttributes } from 'svelte/elements';
  type Props = HTMLAttributes<HTMLDivElement> & { class?: string; children?: Snippet };
  let { class: className, children, ...rest }: Props = $props();
</script>
```

Use these root elements:

```svelte
<!-- PaneToolbar.svelte -->
<div data-pane-toolbar class={cn('flex flex-wrap items-center gap-2 border-b border-border px-3 py-2', className)} {...rest}>{@render children?.()}</div>

<!-- DocumentPane.svelte -->
<div data-document-pane class={cn('min-h-0 flex-1 overflow-auto bg-bg p-6', className)} {...rest}>{@render children?.()}</div>

<!-- InspectorPane.svelte -->
<aside data-inspector-pane class={cn('min-h-0 overflow-auto border-l border-border bg-surface p-3 text-sm', className)} {...rest}>{@render children?.()}</aside>

<!-- MetadataStrip.svelte -->
<div data-metadata-strip class={cn('flex flex-wrap items-center gap-1.5 text-xs text-fg-muted', className)} {...rest}>{@render children?.()}</div>

<!-- ActionBar.svelte -->
<div data-action-bar class={cn('flex flex-wrap items-center justify-end gap-2 border-t border-border px-3 py-2', className)} {...rest}>{@render children?.()}</div>
```

- [ ] **Step 5: Export primitives**

Create `crates/dadrock/web/src/lib/components/workbench/index.ts`:

```ts
export { default as ActionBar } from './ActionBar.svelte';
export { default as DocumentPane } from './DocumentPane.svelte';
export { default as InspectorPane } from './InspectorPane.svelte';
export { default as MetadataStrip } from './MetadataStrip.svelte';
export { default as PaneHeader } from './PaneHeader.svelte';
export { default as PaneToolbar } from './PaneToolbar.svelte';
export { default as WorkbenchPage } from './WorkbenchPage.svelte';
export { default as WorkbenchPane } from './WorkbenchPane.svelte';
```

Modify `crates/dadrock/web/src/lib/components/ui/index.ts`:

```ts
export { default as Alert } from './alert.svelte';
export { default as Badge } from './badge.svelte';
export { default as Button } from './button/button.svelte';
export { default as DisclosureRow } from './disclosure-row.svelte';
export { default as EmptyState } from './empty-state.svelte';
export { default as Field } from './field.svelte';
export { default as ListRow } from './list-row.svelte';
export type { ButtonVariants } from './button/button.svelte';
```

- [ ] **Step 6: Make `Button` compact by default**

Modify `crates/dadrock/web/src/lib/components/ui/button/button.svelte`:

```ts
size: {
  sm: 'h-8 px-2.5',
  md: 'h-8 px-3',
  lg: 'h-9 px-4',
  icon: 'h-8 w-8',
},
defaultVariants: { variant: 'default', size: 'md' },
```

- [ ] **Step 7: Verify foundation**

```bash
cd crates/dadrock/web
bun run test:unit -- tests/unit/workbench-primitives.test.ts
bun run check
```

Expected: both pass.

- [ ] **Step 8: Commit**

```bash
git add crates/dadrock/web/src/lib/components crates/dadrock/web/tests/unit/workbench-primitives.test.ts
git commit -m "Add workbench UI primitives"
```

---

### Task 2: Records As North-Star Workbench

**Files:**
- Modify: `crates/dadrock/web/src/routes/records/+page.svelte`
- Modify: `crates/dadrock/web/src/lib/components/records/RecordDetailPanel.svelte`
- Test: `crates/dadrock/web/tests/e2e/records-split.spec.ts`
- Test: `crates/dadrock/web/tests/e2e/record-editing.spec.ts`

- [ ] **Step 1: Add failing workbench assertions**

In `records-split.spec.ts`, add to `records route reads selected records inline in a split view`:

```ts
await expect(page.locator('[data-workbench-page]')).toBeVisible();
await expect(page.locator('[data-workbench-pane]').first()).toBeVisible();
await expect(page.locator('[data-document-pane]')).toBeVisible();
await expect(page.locator('[data-ui-list-row]').first()).toBeVisible();
```

- [ ] **Step 2: Run targeted records test and verify failure**

```bash
cd crates/dadrock/web
bun run test:e2e -- records-split.spec.ts -g "reads selected records inline"
```

Expected: FAIL because Records does not yet use workbench primitives.

- [ ] **Step 3: Convert Records route to workbench panes**

Import in `records/+page.svelte`:

```svelte
import { Badge, Button, EmptyState, Field, ListRow } from '$lib/components/ui';
import { DocumentPane, MetadataStrip, PaneHeader, PaneToolbar, WorkbenchPage, WorkbenchPane } from '$lib/components/workbench';
```

Replace the route root:

```svelte
<div class="flex h-full min-h-0 flex-col overflow-hidden p-2">
```

with:

```svelte
<WorkbenchPage layout="two-pane">
```

Make the record list live in:

```svelte
<WorkbenchPane aria-label="Record explorer">
  <PaneHeader title="Records" subtitle={`${filteredRecords.length} matching`} />
  <PaneToolbar>
    <!-- existing search/status/scope controls moved here -->
  </PaneToolbar>
  <div class="min-h-0 flex-1 overflow-auto">
    <!-- list rows -->
  </div>
</WorkbenchPane>
```

Make the detail pane live in:

```svelte
<WorkbenchPane aria-label="Record detail">
  <DocumentPane class="p-4">
    <!-- existing detail component -->
  </DocumentPane>
</WorkbenchPane>
```

- [ ] **Step 4: Convert record list items to `ListRow`**

For each record row wrapper, preserve click behavior and selected state, but render:

```svelte
<ListRow data-ui-list-row selected={record.id === selectedRecordId} class="cursor-pointer hover:bg-surface-2" onclick={() => selectRecord(record.id)}>
  <div class="min-w-0">
    <div class="flex items-center gap-2">
      <span class="truncate font-medium text-fg">{record.title}</span>
      <Badge variant={record.status === 'candidate' ? 'candidate' : record.status === 'accepted' && record.superseded ? 'superseded' : record.status === 'accepted' ? 'accepted' : 'rejected'}>
        {displayRecordStatus(record)}
      </Badge>
    </div>
    <div class="mt-1 truncate font-mono text-xs text-fg-subtle">{record.path}</div>
  </div>
  <div class="text-xs text-fg-subtle">{shortDate(record.timestamp_ms)}</div>
</ListRow>
```

- [ ] **Step 5: Convert `RecordDetailPanel` header and actions**

Import:

```svelte
import { Alert, Badge, Button, EmptyState } from '$lib/components/ui';
import { ActionBar, DocumentPane, MetadataStrip, PaneHeader } from '$lib/components/workbench';
```

Replace lifecycle badges and tag chips with `Badge`.

Replace edit/view action cluster with `ActionBar`:

```svelte
<ActionBar class="mt-4 border-t-0 px-0 py-0">
  <Button variant="outline" size="sm" onclick={startEdit}>Edit</Button>
  <Button variant="outline" size="sm" href={resolve(`/tasks?task=edit-record&record=${record.id}`)}>Revise</Button>
</ActionBar>
```

Keep rendered Markdown padding more spacious:

```svelte
<article data-record-rendered-pane class="record-markdown mt-4 rounded-md border border-border bg-bg p-6">
```

- [ ] **Step 6: Run records verification**

```bash
cd crates/dadrock/web
bun run test:e2e -- records-split.spec.ts record-editing.spec.ts
bun run check
```

Expected: both pass.

- [ ] **Step 7: Commit**

```bash
git add crates/dadrock/web/src/routes/records/+page.svelte crates/dadrock/web/src/lib/components/records/RecordDetailPanel.svelte crates/dadrock/web/tests/e2e/records-split.spec.ts
git commit -m "Make records the workbench reference view"
```

---

### Task 3: Tasks As Two-Pane Workflow Workbench

**Files:**
- Modify: `crates/dadrock/web/src/routes/tasks/+page.svelte`
- Test: `crates/dadrock/web/tests/e2e/tasks.spec.ts`

- [ ] **Step 1: Add failing Tasks workbench assertion**

Add a test to `tasks.spec.ts`:

```ts
test('tasks route uses the workbench workflow layout', async ({ page }) => {
  await page.goto('/tasks');
  await expect(page.locator('[data-workbench-page]')).toBeVisible();
  await expect(page.locator('[data-workbench-pane]').first()).toBeVisible();
  await expect(page.getByRole('region', { name: 'Task workflow' })).toBeVisible();
  await expect(page.getByRole('region', { name: 'Task result workbench' })).toBeVisible();
});
```

- [ ] **Step 2: Run the test and verify failure**

```bash
cd crates/dadrock/web
bun run test:e2e -- tasks.spec.ts -g "workbench workflow layout"
```

Expected: FAIL because Tasks still uses page-local sections.

- [ ] **Step 3: Convert Tasks route root to two panes**

Import:

```svelte
import { Alert, Badge, Button, DisclosureRow, EmptyState, Field, ListRow } from '$lib/components/ui';
import { ActionBar, DocumentPane, MetadataStrip, PaneHeader, PaneToolbar, WorkbenchPage, WorkbenchPane } from '$lib/components/workbench';
```

Restructure route body:

```svelte
<WorkbenchPage layout="two-pane">
  <WorkbenchPane aria-label="Task workflow">
    <PaneHeader title="Tasks" subtitle={`${tasks.length} available`} />
    <PaneToolbar>
      <!-- intent controls, search, selected task select -->
    </PaneToolbar>
    <div class="min-h-0 flex-1 overflow-auto p-3">
      <!-- task inputs, context builder, run controls -->
    </div>
  </WorkbenchPane>

  <WorkbenchPane aria-label="Task result workbench">
    <PaneHeader title="Results" subtitle={taskResultRunId || 'No run selected'} />
    <div class="min-h-0 flex-1 overflow-auto p-3">
      <!-- status, results, history depending on selected tab -->
    </div>
  </WorkbenchPane>
</WorkbenchPage>
```

Keep the existing tabs initially inside the result pane, not the global top of the page. This reduces route-level chrome without deleting user workflows.

- [ ] **Step 4: Convert task history to compact rows**

Render each run with `ListRow`:

```svelte
<ListRow data-ui-list-row selected={run.id === selectedRunId} class="cursor-pointer hover:bg-surface-2" onclick={() => loadTaskResult(run.id)}>
  <div class="min-w-0">
    <div class="truncate font-medium text-fg">{run.task_id}</div>
    <div class="mt-1 truncate font-mono text-xs text-fg-subtle">{run.id}</div>
  </div>
  <MetadataStrip>
    <Badge>{run.status}</Badge>
    <span>{shortDate(run.created_at)}</span>
  </MetadataStrip>
</ListRow>
```

- [ ] **Step 5: Convert result cards to `DisclosureRow`**

Replace each `details data-result-card` with `DisclosureRow data-result-card`.

Use `Badge` for severity, kind, status, and confidence. Use `Button` for all result actions.

- [ ] **Step 6: Run Tasks verification**

```bash
cd crates/dadrock/web
bun run test:e2e -- tasks.spec.ts
bun run check
```

Expected: both pass.

- [ ] **Step 7: Commit**

```bash
git add crates/dadrock/web/src/routes/tasks/+page.svelte crates/dadrock/web/tests/e2e/tasks.spec.ts
git commit -m "Make tasks a two-pane workbench workflow"
```

---

### Task 4: Validation As Issue Queue

**Files:**
- Modify: `crates/dadrock/web/src/routes/validation/+page.svelte`
- Modify: `crates/dadrock/web/src/lib/components/validation/DiagnosticPanel.svelte`
- Test: `crates/dadrock/web/tests/e2e/validation.spec.ts`

- [ ] **Step 1: Add failing issue-queue assertions**

In `validation.spec.ts`, add:

```ts
await expect(page.getByRole('region', { name: 'Validation issue queue' })).toBeVisible();
await expect(page.getByRole('region', { name: 'Validation detail' })).toBeVisible();
await expect(page.locator('[data-ui-list-row]').first()).toBeVisible();
```

- [ ] **Step 2: Run validation tests and verify failure**

```bash
cd crates/dadrock/web
bun run test:e2e -- validation.spec.ts
```

Expected: FAIL because Validation still uses dashboard-style panels.

- [ ] **Step 3: Convert Validation route to two-pane issue queue**

Use:

```svelte
<WorkbenchPage layout="two-pane">
  <WorkbenchPane aria-label="Validation issue queue">
    <PaneHeader title="Validation" subtitle={formatDiagnosticCounts(totalCounts)} />
    <PaneToolbar>
      <!-- existing tab/category/filter controls -->
    </PaneToolbar>
    <div class="min-h-0 flex-1 overflow-auto">
      <!-- grouped compact diagnostic rows -->
    </div>
  </WorkbenchPane>
  <WorkbenchPane aria-label="Validation detail">
    <DocumentPane class="p-4">
      <!-- selected group/detail, link validation, audit detail -->
    </DocumentPane>
  </WorkbenchPane>
</WorkbenchPage>
```

Preserve all existing validation actions and route behavior.

- [ ] **Step 4: Convert `DiagnosticPanel` rows**

Use `ListRow`, `Badge`, `Alert`, and `EmptyState`. Render severity badges consistently:

```svelte
<Badge variant={diagnostic.severity === 'error' ? 'danger' : diagnostic.severity === 'warning' ? 'warning' : 'accent'}>
  {diagnostic.severity}
</Badge>
```

- [ ] **Step 5: Run validation verification**

```bash
cd crates/dadrock/web
bun run test:e2e -- validation.spec.ts
bun run check
```

Expected: both pass.

- [ ] **Step 6: Commit**

```bash
git add crates/dadrock/web/src/routes/validation/+page.svelte crates/dadrock/web/src/lib/components/validation/DiagnosticPanel.svelte crates/dadrock/web/tests/e2e/validation.spec.ts
git commit -m "Make validation an issue queue"
```

---

### Task 5: Tags As Explorer View

**Files:**
- Modify: `crates/dadrock/web/src/routes/tags/+page.svelte`
- Test: `crates/dadrock/web/tests/e2e/tags.spec.ts`

- [ ] **Step 1: Add failing explorer assertions**

In `tags.spec.ts`, add:

```ts
await expect(page.locator('[data-workbench-page]')).toBeVisible();
await expect(page.getByRole('region', { name: 'Tag explorer' })).toBeVisible();
await expect(page.locator('[data-ui-list-row]').first()).toBeVisible();
```

- [ ] **Step 2: Run tags tests and verify failure**

```bash
cd crates/dadrock/web
bun run test:e2e -- tags.spec.ts
```

Expected: FAIL because Tags does not yet use workbench primitives.

- [ ] **Step 3: Convert Tags route**

Use a single-pane workbench:

```svelte
<WorkbenchPage layout="single">
  <WorkbenchPane aria-label="Tag explorer">
    <PaneHeader title="Tags" subtitle={`${tagRows.length} tags`} />
    <div class="min-h-0 flex-1 overflow-auto">
      <!-- ListRow tag rows -->
    </div>
  </WorkbenchPane>
</WorkbenchPage>
```

Render tag rows with `ListRow` and child counts with `Badge`.

- [ ] **Step 4: Run tags verification**

```bash
cd crates/dadrock/web
bun run test:e2e -- tags.spec.ts
bun run check
```

Expected: both pass.

- [ ] **Step 5: Commit**

```bash
git add crates/dadrock/web/src/routes/tags/+page.svelte crates/dadrock/web/tests/e2e/tags.spec.ts
git commit -m "Make tags a compact explorer"
```

---

### Task 6: Candidate And Settings Views Join The Workbench

**Files:**
- Modify: `crates/dadrock/web/src/routes/candidates/[id]/+page.svelte`
- Modify: `crates/dadrock/web/src/routes/candidates/new/+page.svelte`
- Modify: `crates/dadrock/web/src/routes/settings/+page.svelte`
- Test: `crates/dadrock/web/tests/e2e/candidate-review.spec.ts`
- Test: `crates/dadrock/web/tests/e2e/candidate-creation.spec.ts`
- Test: `crates/dadrock/web/tests/e2e/settings.spec.ts`

- [ ] **Step 1: Add failing workbench assertions**

In `candidate-review.spec.ts`, assert:

```ts
await expect(page.locator('[data-workbench-page]')).toBeVisible();
await expect(page.locator('[data-document-pane]')).toBeVisible();
```

In `settings.spec.ts`, assert:

```ts
await expect(page.locator('[data-workbench-page]')).toBeVisible();
await expect(page.getByRole('region', { name: 'Settings inspector' })).toBeVisible();
```

- [ ] **Step 2: Run targeted tests and verify failure**

```bash
cd crates/dadrock/web
bun run test:e2e -- candidate-review.spec.ts candidate-creation.spec.ts settings.spec.ts
```

Expected: FAIL because these views do not yet use workbench primitives.

- [ ] **Step 3: Convert candidate detail and new candidate pages**

Use:

```svelte
<WorkbenchPage layout="single">
  <WorkbenchPane>
    <PaneHeader title="Candidate" subtitle={recordIdOrStatus} />
    <DocumentPane>
      <!-- existing candidate content/editor -->
    </DocumentPane>
  </WorkbenchPane>
</WorkbenchPage>
```

Use `ActionBar` for accept/reject/write/create actions and `Badge` for lifecycle/status metadata.

- [ ] **Step 4: Convert Settings to inspector page**

Use:

```svelte
<WorkbenchPage layout="single">
  <WorkbenchPane aria-label="Settings inspector">
    <PaneHeader title="Settings" subtitle="Project and configuration" />
    <div class="min-h-0 flex-1 overflow-auto p-3">
      <!-- existing settings groups as compact ListRow or Field blocks -->
    </div>
  </WorkbenchPane>
</WorkbenchPage>
```

- [ ] **Step 5: Run verification**

```bash
cd crates/dadrock/web
bun run test:e2e -- candidate-review.spec.ts candidate-creation.spec.ts settings.spec.ts
bun run check
```

Expected: both pass.

- [ ] **Step 6: Commit**

```bash
git add crates/dadrock/web/src/routes/candidates crates/dadrock/web/src/routes/settings/+page.svelte crates/dadrock/web/tests/e2e/candidate-review.spec.ts crates/dadrock/web/tests/e2e/candidate-creation.spec.ts crates/dadrock/web/tests/e2e/settings.spec.ts
git commit -m "Bring candidate and settings views into workbench"
```

---

### Task 7: Layout Shell And Global Rhythm

**Files:**
- Modify: `crates/dadrock/web/src/routes/+layout.svelte`
- Modify: `crates/dadrock/web/src/lib/components/shell/Header.svelte`
- Modify: `crates/dadrock/web/src/app.css`
- Test: `crates/dadrock/web/tests/e2e/smoke.spec.ts`

- [ ] **Step 1: Add failing shell rhythm assertions**

In `smoke.spec.ts`, add:

```ts
await expect(page.locator('[data-app-shell]')).toBeVisible();
await expect(page.locator('[data-app-header]')).toBeVisible();
```

- [ ] **Step 2: Run smoke tests and verify failure**

```bash
cd crates/dadrock/web
bun run test:e2e -- smoke.spec.ts
```

Expected: FAIL because shell data attributes do not exist.

- [ ] **Step 3: Add shell attributes and compact rhythm**

In `+layout.svelte`, set:

```svelte
<div data-app-shell class="flex h-screen flex-col bg-bg text-fg">
```

In `Header.svelte`, set:

```svelte
<header data-app-header class="flex h-11 items-center gap-3 border-b border-border bg-bg px-2">
```

Keep the existing navigation and command palette behavior. Do not add a large sidebar in this task; workbench panes now provide the view-specific explorer structure.

- [ ] **Step 4: Tighten tokens in `app.css`**

Set radii to the compact system:

```css
--radius-sm: 3px;
--radius: 5px;
--radius-lg: 8px;
```

Keep existing colors, fonts, and theme names stable.

- [ ] **Step 5: Run smoke verification**

```bash
cd crates/dadrock/web
bun run test:e2e -- smoke.spec.ts
bun run check
```

Expected: both pass.

- [ ] **Step 6: Commit**

```bash
git add crates/dadrock/web/src/routes/+layout.svelte crates/dadrock/web/src/lib/components/shell/Header.svelte crates/dadrock/web/src/app.css crates/dadrock/web/tests/e2e/smoke.spec.ts
git commit -m "Tighten workbench shell rhythm"
```

---

### Task 8: Visual Vocabulary Enforcement Pass

**Files:**
- Modify route/component files under `crates/dadrock/web/src/routes` and `crates/dadrock/web/src/lib/components` only when replacing local visual recipes with primitives.
- Test: all web tests.

- [ ] **Step 1: Search for remaining local recipes**

```bash
rg -n "rounded-md border border-border bg-(surface|bg)|rounded bg-surface-2 px-1\\.5 py-0\\.5|inline-flex h-8 items-center" crates/dadrock/web/src/routes crates/dadrock/web/src/lib/components
```

Expected: remaining matches should be either inside primitives or rare document-specific surfaces.

- [ ] **Step 2: Replace remaining badge recipes**

Replace visual-only metadata spans with `Badge`. Keep anchors as anchors when they navigate.

Example:

```svelte
<Badge mono>{record.id_prefix}</Badge>
```

- [ ] **Step 3: Replace remaining quiet action recipes**

Replace visual-only secondary buttons with:

```svelte
<Button variant="outline" size="sm" ...>
```

Use `variant="default"` only for the primary action in a pane. Use `variant="danger"` only for destructive actions.

- [ ] **Step 4: Replace remaining empty and alert blocks**

Use:

```svelte
<EmptyState>No records match the current filters.</EmptyState>
<Alert variant="danger" title="Failed to load snapshot">{snapshot.error.message}</Alert>
```

- [ ] **Step 5: Run full web verification**

```bash
cd crates/dadrock/web
bun run check
bun run test:unit
bun run test:e2e
```

Expected:

- `svelte-check found 0 errors and 0 warnings`
- all Vitest tests pass
- all Playwright tests pass

- [ ] **Step 6: Commit**

```bash
git add crates/dadrock/web/src crates/dadrock/web/tests
git commit -m "Enforce workbench visual vocabulary"
```

---

### Task 9: Final Review And Merge Verification

**Files:**
- No planned source changes.

- [ ] **Step 1: Run final checks**

```bash
cd crates/dadrock/web
bun run check
bun run test:unit
bun run test:e2e
```

Expected: all pass.

- [ ] **Step 2: Run diff hygiene**

```bash
git diff --check
git status --short
```

Expected: no whitespace errors. Only intended web files should be modified.

- [ ] **Step 3: Review scope**

```bash
git diff --stat main...HEAD
git diff -- crates/dadrock/web/src/lib/components/workbench crates/dadrock/web/src/lib/components/ui | sed -n '1,260p'
```

Expected: changes are concentrated in web UI primitives, route conversions, and e2e tests. There should be no Rust, task schema, or API changes.

- [ ] **Step 4: Commit any review cleanup**

If review changes are made, run:

```bash
cd crates/dadrock/web
bun run check
bun run test:unit
bun run test:e2e
```

Then commit:

```bash
git add crates/dadrock/web
git commit -m "Polish workbench visual redesign"
```

If no cleanup is needed, do not create an empty commit.

---

## Self-Review

- Scope coverage: This plan includes a shell-level redesign, records as north-star, tasks as workflow workbench, validation as issue queue, tags as explorer, candidate/settings conversion, global rhythm, and enforcement pass.
- Placeholder scan: The plan avoids placeholder markers and avoids unspecified implementation work.
- Type consistency: Primitive names are stable across tasks: `WorkbenchPage`, `WorkbenchPane`, `PaneHeader`, `PaneToolbar`, `DocumentPane`, `InspectorPane`, `MetadataStrip`, `ActionBar`, `Badge`, `Alert`, `EmptyState`, `DisclosureRow`, `ListRow`, `Field`, and `Button`.
- Risk control: Each task has a focused e2e gate and commit boundary. The plan does not change Rust APIs, task schemas, command envelopes, or record lifecycle behavior.
