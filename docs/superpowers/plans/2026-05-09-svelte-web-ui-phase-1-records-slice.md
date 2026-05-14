# Svelte Web UI Phase 1 Records Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the Svelte UI useful for browsing records by adding records search, status filtering, and a real record-source detail page.

**Architecture:** Keep server state in TanStack Query through the existing `DadrApi` boundary. Put pure records derivation logic in a small tested helper module, then compose the records route and detail route from that helper and API boundary. Do not port editing, validation, tags, or candidate creation in this slice.

**Tech Stack:** Svelte 5, SvelteKit static SPA, TypeScript, TanStack Query, Zod, Vitest, Playwright.

---

## Files

- Create: `crates/dadrock/web/src/lib/records/filters.ts`
- Test: `crates/dadrock/web/tests/unit/records-filters.test.ts`
- Modify: `crates/dadrock/web/src/lib/api/schemas.ts`
- Modify: `crates/dadrock/web/src/lib/api/types.ts`
- Modify: `crates/dadrock/web/src/lib/api/real.ts`
- Modify: `crates/dadrock/web/src/lib/api/mock.ts`
- Test: `crates/dadrock/web/tests/unit/api-real.test.ts`
- Modify: `crates/dadrock/web/src/routes/records/+page.svelte`
- Modify: `crates/dadrock/web/src/routes/records/[id]/+page.svelte`

## Task 1: Records Filtering Helper

- [ ] Write `tests/unit/records-filters.test.ts` with tests for status filtering, query matching by title/path/tag/id prefix, and status count labels.
- [ ] Run `bun run test:unit -- tests/unit/records-filters.test.ts` and verify the module is missing.
- [ ] Create `src/lib/records/filters.ts` exporting `filterRecords`, `statusOptions`, and `recordSearchText`.
- [ ] Run the focused test and verify it passes.

## Task 2: Record Source API Boundary

- [ ] Extend `schemas.ts` with `recordSourceSchema` and `RecordSource`.
- [ ] Extend `DadrApi` with `getRecordSource(id: string)`.
- [ ] Add focused tests in `api-real.test.ts` proving `RealApi.getRecordSource` calls `/api/records/:id/source`, parses valid JSON, and rejects invalid JSON.
- [ ] Run the API test and verify it fails before implementation.
- [ ] Implement `RealApi.getRecordSource` and `MockApi.getRecordSource`.
- [ ] Run the API test and verify it passes.

## Task 3: Records Route

- [ ] Replace the bare records page with search and status controls backed by URL parameters `q` and `status`.
- [ ] Use `filterRecords` for all visible records.
- [ ] Render counts, empty states, tags, path, and selected status badge.
- [ ] Keep records links routed through SvelteKit `resolve('/records/[id]', { id })`.

## Task 4: Record Detail Route

- [ ] Fetch source with `createQuery(() => ({ queryKey: ['record-source', id], queryFn: () => api.getRecordSource(id) }))`.
- [ ] Render loading, error, and loaded states.
- [ ] Show path, id, and raw Markdown in a monospaced preformatted panel.
- [ ] Leave rendered/edit tabs for the next Phase 1 slice.

## Task 5: Verification

- [ ] Run `bun run test:unit`.
- [ ] Run `bun run lint`.
- [ ] Run `bun run check`.
- [ ] Run `bun run build`.
- [ ] Run `cargo build -p dadrock`.
- [ ] Run `cargo test -p dadrock`.
- [ ] Run `bun run test:e2e`.

## Self-Review

- Scope is intentionally smaller than the full Phase 1 cutover: it ports records browsing and detail source reading only.
- The slice keeps server data inside TanStack Query and only pure derivations in tested helpers.
- No placeholders are introduced in shipped UI beyond explicit copy saying rendered/edit tabs are future slices.
