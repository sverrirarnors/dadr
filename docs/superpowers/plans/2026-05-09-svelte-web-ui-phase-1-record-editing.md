# Svelte Web UI Phase 1 Record Editing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add raw Markdown editing and saving to the Svelte record detail route.

**Architecture:** Extend the existing `DadrApi` boundary with `saveRecordSource(id, content)`, validating the Rust `ApiMutationResponse` shape with Zod. Use TanStack Query mutation state in `/records/[id]`, invalidate the snapshot and record-source queries on save, and keep edit state local to the route with Svelte runes.

**Tech Stack:** Svelte 5, SvelteKit, TypeScript, TanStack Query, Zod, Vitest, Playwright.

---

## Files

- Modify: `crates/dadrock/web/src/lib/api/schemas.ts`
- Modify: `crates/dadrock/web/src/lib/api/types.ts`
- Modify: `crates/dadrock/web/src/lib/api/real.ts`
- Modify: `crates/dadrock/web/src/lib/api/mock.ts`
- Modify: `crates/dadrock/web/tests/unit/api-real.test.ts`
- Modify: `crates/dadrock/web/src/routes/records/[id]/+page.svelte`

## Task 1: Save API Boundary

- [ ] Add `mutationFocusSchema`, `mutationResponseSchema`, and `MutationResponse` to `schemas.ts`.
- [ ] Extend `DadrApi` with `saveRecordSource(id: string, content: string): Promise<MutationResponse>`.
- [ ] Add failing tests in `api-real.test.ts` for successful save parsing and invalid response rejection.
- [ ] Implement `RealApi.saveRecordSource`.
- [ ] Implement `MockApi.saveRecordSource`.
- [ ] Run `bun run test:unit -- tests/unit/api-real.test.ts`.

## Task 2: Detail Editing UI

- [ ] Add local editor state to `/records/[id]`.
- [ ] Sync editor state from loaded source unless the buffer is dirty.
- [ ] Add a textarea, Save button, Reset button, dirty indicator, and mutation error display.
- [ ] Wire `Cmd+S` / `Ctrl+S` to save when dirty.
- [ ] On successful save, update local editor content, clear dirty state, and invalidate `snapshot` plus `record-source`.

## Task 3: Verification

- [ ] Run `bun run test:unit`.
- [ ] Run `bun run lint`.
- [ ] Run `bun run check`.
- [ ] Run `bun run build`.
- [ ] Run `cargo build -p dadrock`.
- [ ] Run `cargo test -p dadrock`.
- [ ] Run `bun run test:e2e`.

## Self-Review

- This slice only edits raw Markdown; rendered Markdown and structured front matter editing remain future slices.
- Save behavior follows the existing Rust endpoint and legacy UI semantics.
- Query invalidation keeps the list and detail page coherent after save.
