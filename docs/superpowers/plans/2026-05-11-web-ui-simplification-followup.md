# Web UI Simplification Followup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Finish the second pass of Dadrock web UI simplification by replacing remaining one-off candidate, task, form, button, panel, and diagnostic/result presentation patterns with shared primitives.

**Architecture:** Keep the current dense workbench model. Add small, reusable UI primitives only where repeated route-local markup already exists, then move bulky task subviews into task-owned components. Preserve routes, API calls, and behavior; this is a presentation and structure refactor.

**Tech Stack:** Svelte 5, SvelteKit, TypeScript, Tailwind v4, TanStack Query, Vitest, Playwright, Bun.

---

### Task 1: Shared Form and Panel Primitives

**Files:**
- Create: `crates/dadrock/web/src/lib/components/ui/text-input.svelte`
- Create: `crates/dadrock/web/src/lib/components/ui/select-input.svelte`
- Create: `crates/dadrock/web/src/lib/components/ui/textarea-input.svelte`
- Create: `crates/dadrock/web/src/lib/components/ui/panel-section.svelte`
- Modify: `crates/dadrock/web/src/lib/components/ui/index.ts`
- Modify: `crates/dadrock/web/tests/unit/workbench-primitives.test.ts`

- [ ] **Step 1: Write the failing export test**

Add the new exports to `workbench-primitives.test.ts`:

```ts
import {
  PanelSection,
  SelectInput,
  TextareaInput,
  TextInput,
} from '../../src/lib/components/ui';

expect(PanelSection).toBeTruthy();
expect(TextInput).toBeTruthy();
expect(SelectInput).toBeTruthy();
expect(TextareaInput).toBeTruthy();
```

- [ ] **Step 2: Verify red**

Run: `bun run test:unit -- tests/unit/workbench-primitives.test.ts`

Expected: FAIL because the new exports do not exist.

- [ ] **Step 3: Implement the primitives**

Create compact wrappers that preserve native attributes and expose stable test hooks:

```svelte
<!-- text-input.svelte -->
<script lang="ts">
  import { cn } from '$lib/utils';
  import type { HTMLInputAttributes } from 'svelte/elements';

  type Props = HTMLInputAttributes & { class?: string };
  let { class: className, type = 'text', ...rest }: Props = $props();
</script>

<input
  data-ui-text-input
  {type}
  class={cn('h-9 w-full rounded-md border border-border bg-bg px-3 text-sm text-fg placeholder:text-fg-subtle focus:outline-none focus:ring-2 focus:ring-accent disabled:opacity-60', className)}
  {...rest}
/>
```

Mirror this pattern for `SelectInput`, `TextareaInput`, and `PanelSection`.

- [ ] **Step 4: Verify green**

Run: `bun run test:unit -- tests/unit/workbench-primitives.test.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit: `Add shared form and panel primitives`

### Task 2: Candidate Detail Uses Shared Vocabulary

**Files:**
- Modify: `crates/dadrock/web/src/routes/candidates/[id]/+page.svelte`
- Test: `crates/dadrock/web/tests/e2e/candidate-actions.spec.ts`

- [ ] **Step 1: Write failing e2e assertions**

Assert candidate detail uses shared tab and button primitives:

```ts
await expect(page.locator('[data-ui-segmented-tabs]')).toBeVisible();
await expect(page.locator('[data-ui-button]').getByText('Accept')).toBeVisible();
await expect(page.locator('[data-ui-button]').getByText('Reject')).toBeVisible();
```

- [ ] **Step 2: Verify red**

Run: `bun run test:e2e -- tests/e2e/candidate-actions.spec.ts`

Expected: FAIL because candidate detail still uses local tab/button markup.

- [ ] **Step 3: Replace local markup**

Import and use `SegmentedTabs` and `Button` from `$lib/components/ui`. Keep the same accept/reject actions and route behavior.

- [ ] **Step 4: Verify green**

Run: `bun run check` and `bun run test:e2e -- tests/e2e/candidate-actions.spec.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit: `Normalize candidate detail controls`

### Task 3: Normalize Buttons and Forms on Candidate Creation

**Files:**
- Modify: `crates/dadrock/web/src/routes/candidates/new/+page.svelte`
- Test: `crates/dadrock/web/tests/e2e/candidate-actions.spec.ts`

- [ ] **Step 1: Write failing e2e assertions**

Assert the new candidate page uses shared form controls and buttons:

```ts
await page.goto('/candidates/new');
await expect(page.locator('[data-ui-text-input]').first()).toBeVisible();
await expect(page.locator('[data-ui-textarea-input]').first()).toBeVisible();
await expect(page.locator('[data-ui-button]').getByText('Create candidate')).toBeVisible();
```

- [ ] **Step 2: Verify red**

Run: `bun run test:e2e -- tests/e2e/candidate-actions.spec.ts -g "new candidate"`

Expected: FAIL because candidate creation still uses local input/button classes.

- [ ] **Step 3: Replace controls**

Use `TextInput`, `TextareaInput`, `PanelSection`, and `Button` while preserving labels, bindings, disabled states, preview behavior, and create action.

- [ ] **Step 4: Verify green**

Run: `bun run check` and `bun run test:e2e -- tests/e2e/candidate-actions.spec.ts -g "new candidate"`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit: `Normalize candidate creation form`

### Task 4: Extract Task Runner and Context Components

**Files:**
- Create: `crates/dadrock/web/src/lib/components/tasks/TaskRunnerPane.svelte`
- Create: `crates/dadrock/web/src/lib/components/tasks/TaskContextEditor.svelte`
- Modify: `crates/dadrock/web/src/routes/tasks/+page.svelte`
- Test: `crates/dadrock/web/tests/e2e/tasks.spec.ts`

- [ ] **Step 1: Write failing e2e assertions**

Add stable component markers:

```ts
await expect(page.locator('[data-task-runner-pane]')).toBeVisible();
await expect(page.locator('[data-task-context-editor]')).toBeVisible();
```

- [ ] **Step 2: Verify red**

Run: `bun run test:e2e -- tests/e2e/tasks.spec.ts -g "presents context"`

Expected: FAIL because these components do not exist.

- [ ] **Step 3: Extract components**

Move the task runner shell, task selector, input rendering, additional context editor, and record picker markup out of the route. Keep state and mutations in the route; pass values and callbacks as props.

- [ ] **Step 4: Verify green**

Run: `bun run check` and `bun run test:e2e -- tests/e2e/tasks.spec.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit: `Extract task runner components`

### Task 5: Extract Task Results and Follow-up Composer

**Files:**
- Create: `crates/dadrock/web/src/lib/components/tasks/TaskResultsPanel.svelte`
- Create: `crates/dadrock/web/src/lib/components/tasks/TaskFollowUpComposer.svelte`
- Modify: `crates/dadrock/web/src/routes/tasks/+page.svelte`
- Test: `crates/dadrock/web/tests/e2e/tasks.spec.ts`

- [ ] **Step 1: Write failing e2e assertions**

Add result/follow-up markers in existing task result tests:

```ts
await expect(page.locator('[data-task-results-panel]')).toBeVisible();
await expect(page.locator('[data-task-follow-up-composer]')).toBeVisible();
```

- [ ] **Step 2: Verify red**

Run: `bun run test:e2e -- tests/e2e/tasks.spec.ts -g "fix follow-up|folded stacked cards"`

Expected: FAIL because result and follow-up presentation is still route-local.

- [ ] **Step 3: Extract components**

Move result overview, folded result card rendering, candidate/revision actions, and follow-up note UI into task-owned presentational components. Keep mutations in the route.

- [ ] **Step 4: Verify green**

Run: `bun run check` and `bun run test:e2e -- tests/e2e/tasks.spec.ts`

Expected: PASS.

- [ ] **Step 5: Commit**

Commit: `Extract task result components`

### Task 6: Shared Diagnostic and Result Rows

**Files:**
- Create: `crates/dadrock/web/src/lib/components/ui/stacked-card.svelte`
- Modify: `crates/dadrock/web/src/lib/components/ui/index.ts`
- Modify: `crates/dadrock/web/src/lib/components/validation/DiagnosticPanel.svelte`
- Modify: task result components from Task 5
- Test: `crates/dadrock/web/tests/e2e/tasks.spec.ts`
- Test: `crates/dadrock/web/tests/e2e/validation.spec.ts`
- Test: `crates/dadrock/web/tests/unit/workbench-primitives.test.ts`

- [ ] **Step 1: Write failing tests**

Assert `StackedCard` exports and that task/validation result rows have `data-ui-stacked-card`.

- [ ] **Step 2: Verify red**

Run:

```bash
bun run test:unit -- tests/unit/workbench-primitives.test.ts
bun run test:e2e -- tests/e2e/tasks.spec.ts -g "folded stacked cards"
bun run test:e2e -- tests/e2e/validation.spec.ts
```

Expected: FAIL because the shared stacked-card primitive does not exist.

- [ ] **Step 3: Implement and adopt**

Create `StackedCard` as a thin wrapper over `details` or `article` depending on an `openable` prop. Replace local result/diagnostic card borders where behavior matches.

- [ ] **Step 4: Verify green**

Run: `bun run check`, `bun run test:unit`, and focused e2e for tasks and validation.

Expected: PASS.

- [ ] **Step 5: Commit**

Commit: `Share stacked result cards`

### Task 7: Final Vocabulary Guard and Verification

**Files:**
- Modify: `crates/dadrock/web/docs/web-ui-vocabulary.md`
- Modify: `crates/dadrock/web/tests/unit/ui-vocabulary.test.ts`

- [ ] **Step 1: Update guard tests**

Require candidate pages and task components to use the shared vocabulary:

```ts
['../../src/routes/candidates/[id]/+page.svelte', ['SegmentedTabs', 'Button']],
['../../src/routes/candidates/new/+page.svelte', ['TextInput', 'TextareaInput', 'PanelSection']],
['../../src/lib/components/tasks/TaskRunnerPane.svelte', ['PanelSection', 'TextInput', 'SelectInput']],
['../../src/lib/components/tasks/TaskResultsPanel.svelte', ['StackedCard']],
```

- [ ] **Step 2: Update the guide**

Document `PanelSection`, `TextInput`, `SelectInput`, `TextareaInput`, and `StackedCard`.

- [ ] **Step 3: Verify**

Run:

```bash
bun run check
bun run test:unit
bun run test:e2e -- tests/e2e/tasks.spec.ts tests/e2e/candidate-actions.spec.ts tests/e2e/validation.spec.ts tests/e2e/settings.spec.ts tests/e2e/record-editing.spec.ts
```

Expected: all pass.

- [ ] **Step 4: Commit**

Commit: `Tighten web UI vocabulary guards`
