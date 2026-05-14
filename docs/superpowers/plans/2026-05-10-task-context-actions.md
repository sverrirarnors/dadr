# Task Context Actions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Add structured run context and practical typed result actions to the Dadrock task runner.

**Architecture:** Extend task run requests with a `TaskContext` envelope that is persisted with run input and passed to agent prompts. Add a Svelte context tray on the task runner and a small result action layer that supports navigation, candidate writing, local dismissal, and follow-up reruns with user notes.

**Tech Stack:** Rust (`dadrock-core`, `dadrock` web server), SvelteKit/TypeScript frontend, Zod schemas, Vitest, Playwright.

---

### Task 1: Persist And Prompt Structured Task Context

**Files:**
- Modify: `crates/dadrock-core/src/tasks/runner.rs`
- Modify: `crates/dadrock-core/src/agent/types.rs`
- Modify: `crates/dadrock-core/src/agent/prompts.rs`
- Test: `crates/dadrock-core/tests/task_system.rs`

- [x] **Step 1: Write failing Rust tests**

Add tests that construct a `TaskRunCommandRequest` with context and verify the serialized `AgentTaskRequest` includes `context.instructions`, `context.paths`, `context.records`, and `context.text`.

- [x] **Step 2: Verify red**

Run: `cargo test -p dadrock-core --test task_system task_context`

Expected: compile failure or assertion failure because task context types do not exist.

- [x] **Step 3: Implement minimal Rust context types**

Add `TaskContext` to `tasks::runner`, add it to `TaskRunCommandRequest`, default it to empty, and copy it into `AgentTaskRequest`.

- [x] **Step 4: Verify green**

Run: `cargo test -p dadrock-core --test task_system task_context`

Expected: new task context tests pass.

### Task 2: Accept Context Through CLI And Web API

**Files:**
- Modify: `crates/dadrock/src/main.rs`
- Modify: `crates/dadrock/src/web.rs`
- Modify: `crates/dadrock/web/src/lib/api/types.ts`
- Modify: `crates/dadrock/web/src/lib/api/schemas.ts`
- Modify: `crates/dadrock/web/src/lib/api/real.ts`
- Modify: `crates/dadrock/web/src/lib/api/mock.ts`
- Test: `crates/dadrock/web/tests/unit/api-real.test.ts`

- [x] **Step 1: Write failing API tests**

Add a Vitest assertion that `RealApi.runTask` posts `{ task_id, input, context }`.

- [x] **Step 2: Verify red**

Run: `bun run test:unit -- api-real`

Expected: posted JSON lacks `context`.

- [x] **Step 3: Implement API types and request plumbing**

Add `TaskContext` to TypeScript types, include it in `RunTaskRequest`, and deserialize it in the Rust web `RunTaskRequest`.

- [x] **Step 4: Verify green**

Run: `bun run test:unit -- api-real`

Expected: API unit tests pass.

### Task 3: Add The Context Tray UI

**Files:**
- Modify: `crates/dadrock/web/src/routes/tasks/+page.svelte`
- Test: `crates/dadrock/web/tests/e2e/tasks.spec.ts`

- [x] **Step 1: Write failing Playwright test**

Add a test that fills `Instructions`, `Paths`, `Records`, and `Pasted context`, runs a task, and verifies the mock result reflects the instruction text.

- [x] **Step 2: Verify red**

Run: `bun run test:e2e -- tasks.spec.ts`

Expected: labels are missing.

- [x] **Step 3: Implement UI state and form controls**

Add `taskContext` state, include it in `api.runTask`, reset it on task selection, and render a collapsible `Additional context` tray below task-specific fields.

- [x] **Step 4: Verify green**

Run: `bun run test:e2e -- tasks.spec.ts`

Expected: task route tests pass.

### Task 4: Add Typed Result Actions And Follow-Up Reruns

**Files:**
- Modify: `crates/dadrock/web/src/routes/tasks/+page.svelte`
- Modify: `crates/dadrock/web/src/lib/api/mock.ts`
- Test: `crates/dadrock/web/tests/e2e/tasks.spec.ts`

- [x] **Step 1: Write failing Playwright tests**

Add tests for `Fix with note` on a diagnosis issue and `Revise with note` on a candidate batch item. Each test should verify a follow-up task run is created and the result includes the note.

- [x] **Step 2: Verify red**

Run: `bun run test:e2e -- tasks.spec.ts`

Expected: action controls do not exist.

- [x] **Step 3: Implement low-risk action registry**

Add frontend helpers that map result kinds to available actions. Implement action handlers for navigation, write candidate, dismiss local result items, and rerun/follow-up with context.

- [x] **Step 4: Verify green**

Run: `bun run test:e2e -- tasks.spec.ts`

Expected: task route action tests pass.

### Task 5: Final Verification

**Files:**
- Verify all changed Rust and web code.

- [x] **Step 1: Format**

Run: `cargo fmt`

- [x] **Step 2: Rust tests**

Run: `cargo test -p dadrock-core`

- [x] **Step 3: Web unit tests**

Run: `bun run test:unit`

- [x] **Step 4: Web e2e task tests**

Run: `bun run test:e2e -- tasks.spec.ts`

- [x] **Step 5: Commit**

Commit the implementation with message `Add task context and result actions`.
