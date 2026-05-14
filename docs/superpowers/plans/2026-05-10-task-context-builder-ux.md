# Task Context Builder UX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make record-routed task runs feel contextual and make task context easier to build without pasting record UUIDs.

**Architecture:** Keep `/tasks` as the only task runner and preserve the existing task request contract. The tasks page reads project snapshot records client-side, uses the existing record filtering helpers for search, and syncs selected record chips back to the current `recordContextText` field. Record return links are computed from the `record` URL parameter, so no backend change is required.

**Tech Stack:** Svelte 5, SvelteKit, TanStack Svelte Query, Playwright e2e tests, Bun tooling.

---

### Task 1: Add Failing UX Tests

**Files:**
- Modify: `crates/dadrock/web/tests/e2e/tasks.spec.ts`

- [x] **Step 1: Add tests**

Add tests for:
- contextual source banner and back links from `/tasks?task=...&record=...`
- context summary near the runner actions
- searchable record picker that writes selected records into the existing `Related records` field
- task-aware default record picker filter for `edit-record`

- [x] **Step 2: Verify red**

Run:

```bash
bun run test:e2e -- tasks.spec.ts
```

Expected: new tests fail because the current task page has no source banner, no context summary chip, and no record picker.

### Task 2: Add Snapshot-Backed Context State

**Files:**
- Modify: `crates/dadrock/web/src/routes/tasks/+page.svelte`

- [x] **Step 1: Import record types and filter helpers**

Import `RecordSummary`, `filterRecords`, `statusOptions`, `searchScopeOptions`, `RecordStatusFilter`, and `RecordSearchScope`.

- [x] **Step 2: Add snapshot query and picker state**

Add `snapshot` query plus `recordSearchQuery`, `recordStatusFilter`, and `recordSearchScope` state.

- [x] **Step 3: Add derived context values**

Add derived values for selected record IDs, selected record summaries, source record ID, source record summary, record picker results, and context summary text.

### Task 3: Implement Context Helpers

**Files:**
- Modify: `crates/dadrock/web/src/routes/tasks/+page.svelte`

- [x] **Step 1: Add record context mutation helpers**

Add helpers to add/remove record IDs from `recordContextText`, format record labels, compute record status, and generate record links.

- [x] **Step 2: Add task-aware defaults**

When selecting or prefilling a task, default the picker status filter to candidates for `edit-record`, accepted for `supersede-record`, and all otherwise.

- [x] **Step 3: Preserve existing raw context behavior**

Keep the `Related records` textarea and continue building task context through `taskRunContext()` from `recordContextText`.

### Task 4: Render Contextual UX

**Files:**
- Modify: `crates/dadrock/web/src/routes/tasks/+page.svelte`

- [x] **Step 1: Add source banner**

Render a compact banner when the URL has `record=<id>`, showing the record title/status/path when snapshot data is available and a `Back to record` link.

- [x] **Step 2: Add context summary control**

Render a `Context: ...` button near `Run task` that toggles the context tray.

- [x] **Step 3: Add record chips and picker**

Inside the context tray, render selected records as removable chips/cards and add a search/filter picker with Add buttons.

- [x] **Step 4: Add result return links**

When a task was launched with a record parameter, show `Back to record` and `Run another task for this record` actions in the results header.

### Task 5: Verify and Commit

**Files:**
- Modify: `docs/superpowers/plans/2026-05-10-task-context-builder-ux.md`

- [x] **Step 1: Run focused verification**

Run:

```bash
bun run check
bun run test:e2e -- tasks.spec.ts records-split.spec.ts
```

Expected: Svelte diagnostics are clean and focused e2e tests pass.

- [x] **Step 2: Run unit tests**

Run:

```bash
bun run test:unit
```

Expected: unit tests pass.

- [x] **Step 3: Mark plan complete**

Check off all plan steps after verification passes.

- [x] **Step 4: Commit**

Run:

```bash
git add docs/superpowers/plans/2026-05-10-task-context-builder-ux.md crates/dadrock/web/src/routes/tasks/+page.svelte crates/dadrock/web/tests/e2e/tasks.spec.ts
git commit -m "Improve task context builder UX"
```

Expected: one feature commit on `task-context-builder`.

---

## Self-Review

- Spec coverage: Covers the combined design from the previous two UX discussions while keeping `/tasks` as the only runner.
- Placeholder scan: No TBD/TODO placeholders.
- Type consistency: Uses existing `RecordSummary`, task context fields, and record filter helpers.
