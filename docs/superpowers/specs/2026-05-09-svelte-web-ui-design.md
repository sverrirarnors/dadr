# Svelte Web UI — Design

**Status:** approved (brainstorming) — pending implementation plan
**Date:** 2026-05-09
**Owner:** Vincent Arel-Bundock

## Goal

Replace the existing 1.3k-line vanilla `app.js` web UI with a polished, Svelte-based primary interface for dadrock. The web UI is no longer a "preview" — it is the primary surface users interact with. Polish quality target: Linear / Obsidian-class. Visual tone: Obsidian-style knowledge tool (panels, comfortable density, tinted dark mode, markdown-native, decisions-as-durable-knowledge framing).

This design covers the full surface area of `specs/07-agent-and-app-architecture.md` — today's screens (records, tags, validation) plus the spec-07 vision (Inbox, candidate review with diff and source snippets, supersession chains, link health queue, audit trail, SSE for live updates). Backend gaps (spec-07 endpoints not yet implemented in Rust) are mocked behind a typed API boundary; replacing mocks with real endpoints is mechanical, not a rewrite.

## Non-goals

- Robustness queue, faithfulness checks UI, report generation UI, Tauri desktop wrap — each is a separate design.
- Server-side rendering. The dadrock binary is the server; SvelteKit runs as a static SPA.
- Backwards compatibility with the current `app.js`. Phase 1 cutover deletes it.
- Visual regression testing (Chromatic) in v1 — revisit if it bites.
- Internationalization in v1.

## Stack

| Layer | Choice |
|---|---|
| UI framework | Svelte 5 with runes |
| Meta-framework | SvelteKit + `@sveltejs/adapter-static` (SPA fallback mode) |
| Language | TypeScript (strict) |
| Styling | Tailwind v4 (OKLCH palette, `@theme` tokens, container queries) |
| Headless components | Bits UI (Radix-equivalent for Svelte 5) |
| Component recipes | shadcn-svelte (copy-paste, source-owned) |
| Icons | Lucide |
| Server state | TanStack Query (`@tanstack/svelte-query`) |
| API validation | Zod |
| Markdown | `marked` + Shiki (syntax highlighting) |
| Code editor | CodeMirror 6 |
| Build tool | Vite (via SvelteKit) |
| Runtime + package manager | Bun (`bun install`, `bun run build`, `bun.lock` text lockfile) |
| Testing | Vitest (unit + component), Playwright (E2E) |

The package manager and JS runtime is **Bun**, not pnpm. Vite still drives the actual build under SvelteKit.

## Project layout

```
crates/dadrock/
├── Cargo.toml
├── build.rs                     ← invokes `bun install --frozen-lockfile && bun run build`
├── src/
│   ├── main.rs
│   ├── web.rs                   ← shrinks: routes + rust-embed asset handler
│   └── ...
└── web/                         ← all Svelte source lives here
    ├── package.json
    ├── bun.lock
    ├── svelte.config.js         ← adapter-static, fallback: 'index.html'
    ├── vite.config.ts
    ├── tsconfig.json
    ├── tailwind.config.ts
    ├── components.json          ← shadcn-svelte config
    ├── src/
    │   ├── app.html             ← inline theme-application script (no FOUC)
    │   ├── app.css              ← Tailwind directives + design tokens
    │   ├── lib/
    │   │   ├── api/             ← DadrApi interface, RealApi, MockApi, Zod schemas
    │   │   │   ├── index.ts     ← chooses real vs mock based on env
    │   │   │   ├── types.ts     ← DadrApi interface
    │   │   │   ├── real.ts
    │   │   │   ├── mock.ts
    │   │   │   ├── schemas.ts   ← Zod schemas (single source of truth)
    │   │   │   └── mock/fixtures/  ← JSON fixtures for mocked endpoints
    │   │   ├── components/
    │   │   │   ├── ui/          ← shadcn-svelte recipes
    │   │   │   ├── records/
    │   │   │   ├── candidates/
    │   │   │   ├── validation/
    │   │   │   ├── tags/
    │   │   │   └── shell/       ← Sidebar, Header, CommandPalette, ToastRegion
    │   │   ├── stores/          ← .svelte.ts files: theme, palette, toasts, agent status
    │   │   ├── markdown/        ← marked + Shiki + wikilink/ID auto-link
    │   │   └── utils/
    │   └── routes/
    │       ├── +layout.svelte   ← app shell
    │       ├── +layout.ts       ← snapshot prefetch, query client setup
    │       ├── +page.svelte     ← / → redirects to /inbox (with /records fallback)
    │       ├── +error.svelte    ← 404 / unknown route
    │       ├── inbox/+page.svelte
    │       ├── records/
    │       │   ├── +layout.svelte           ← split-pane layout
    │       │   ├── +page.svelte             ← list view
    │       │   └── [id]/+page.svelte        ← detail view
    │       ├── candidates/[id]/+page.svelte
    │       ├── tags/+page.svelte
    │       ├── validation/+page.svelte
    │       ├── links/+page.svelte
    │       ├── audit/+page.svelte
    │       └── settings/+page.svelte
    ├── static/                  ← favicon, fonts (Inter, Source Serif 4, JetBrains Mono)
    └── tests/
        ├── unit/                ← Vitest
        └── e2e/                 ← Playwright
```

## Build & shipping

- `crates/dadrock/build.rs` invokes `bun install --frozen-lockfile && bun run build` in `web/` when any source under `web/src/` is newer than `web/build/`. Outputs `cargo:rerun-if-changed=web/src` and `cargo:rerun-if-changed=web/package.json`. Skips in `cargo check`.
- `web.rs` uses `rust_embed::RustEmbed` against `web/build/`. The three current `include_str!` constants are removed. A single asset handler maps requests to embedded files, sets correct MIME types, and serves precompressed Brotli/gzip when available.
- All previously unmatched routes fall through to SvelteKit's SPA fallback (`index.html`) so client-side routing works.
- Dev workflow: `cd crates/dadrock/web && bun run dev` runs Vite at `localhost:5173` with `/api/*` proxied to a running `dadrock preview` (configurable via `vite.config.ts`). No `cargo build` needed during frontend iteration.
- `build.rs` produces a clear error if `bun` is not on `PATH`, pointing at setup docs. JS toolchain is now a hard prerequisite for `cargo build` of the `dadrock` crate.

## Routing & navigation

### Route tree

| Route | Screen |
|---|---|
| `/` | Redirects to `/inbox` (fallback `/records` if no candidates) |
| `/inbox` | Candidates by confidence, age, scope |
| `/records` | Records list (filter + search + status) |
| `/records/[id]` | Record detail (rendered + raw + edit, supersession chain) |
| `/candidates/[id]` | Candidate review (rendered + raw + front matter + source + diff) |
| `/tags` | Tag tree |
| `/validation` | Project / records / links / tags diagnostics |
| `/links` | Link health queue |
| `/audit` | Audit trail |
| `/settings` | Project + agent settings, templates, theme |

### Layout shell

- Persistent root layout: header (breadcrumbs + search/⌘K trigger + theme + agent status pill), collapsible sidebar with primary navigation and count pills (`Inbox 12`, `Validation 3 errors`), workspace area (route slot), command palette mount, toast region.
- Sidebar collapse state persists in `localStorage`. Bound to `⌘\`.
- Nested layouts: `routes/records/+layout.svelte` hosts a resizable split-pane (Bits UI `Resizable`); `/records` and `/records/[id]` share the list state.

### Command palette

cmdk-style via Bits UI `Command`. Bound to `⌘K` / `Ctrl+K`. Three sections, fuzzy-searched together:

1. **Navigation** — go to Inbox, Records, Tags, Validation, Links, Audit, Settings
2. **Records** — search by title / id / path / tag (results from snapshot, no API call)
3. **Actions** — New candidate (manual), Run validation, Toggle theme, Reload snapshot, Open settings

Recent items (stored in `localStorage`) bubble to top.

### URL is source of truth

Every meaningful navigational state lives in the URL: selected record (`/records/[id]`), search query (`?q=...`), status filter (`?status=candidate`), candidate review tab (`?tab=diff`). TanStack Query keys derive from URL parameters. Browser back/forward and bookmarking work for free.

## State model

Three layers with crisp boundaries; no overlap.

### Server state — TanStack Query

Every API endpoint becomes a query key. The fetcher does `schema.parse(response)` against a Zod schema before the data reaches components. Backend drift becomes a typed error in dev, not a render-time crash.

Examples:

```
['snapshot']             → GET  /api/project
['record', id]           → GET  /api/records/:id (source)
['candidates']           → GET  /api/candidates  (mocked initially)
['candidate', id]        → GET  /api/candidates/:id  (mocked)
['validation', kind]     → POST /api/validate/:kind
['audit']                → GET  /api/audit  (mocked)
```

### Mutations — `useMutation` + `CommandEnvelope`

The Rust `CommandEnvelope { command, ok, data, diagnostics, changed_paths }` shape is the contract. A typical mutation:

1. Optimistic update where safe (e.g., candidate row marked `accepting`).
2. `POST` with Zod-validated request, response parsed into `CommandEnvelope<T>`.
3. `diagnostics` surface via `DiagnosticPanel` and toasts (severity-mapped).
4. `changed_paths` drives surgical query-key invalidation via a `invalidateForChangedPaths` mapper. No blanket `invalidateAll`.
5. Optional navigation to a successor resource if the envelope's `data` contains a new ID.

### Local + global UI state

- **Local UI state** (open dialog, edit dirty flag, hover): Svelte 5 runes (`$state`, `$derived`) inside the component. No store needed.
- **Global UI state** (theme, command palette open/closed, toast queue, agent status, sidebar collapsed): `.svelte.ts` modules exposing `$state`-backed singletons. Not Redux. Not a heavy store library. A handful of reactive globals.

**Hard rule:** server data never lives in stores. It lives only in the query cache. Avoids the dual-source-of-truth bug.

### SSE for live updates

- Single `EventSource('/api/events')` opened in `+layout.ts`.
- Spec-07 event names (`candidate.drafted`, `record.superseded`, `links.broken`, etc.) map to query-key invalidations.
- Reconnect with exponential backoff, capped at 30s.
- Backend doesn't exist yet → frontend uses `MockEventSource` emitting canned events on a timer in dev. When the Rust SSE endpoint lands, the swap is a single line in `lib/api/index.ts`.

## Component & design system

### Visual tone

Obsidian-style knowledge tool. Panels and split-views, comfortable density, tinted dark mode (slight purple-blue undertone in greys, never pure black), markdown-native aesthetic. The accent color is restrained on purpose — identity comes from typography and surface treatment, not loud color. Saturated color appears only in status badges and semantic states.

### Design tokens

Defined in `app.css` via Tailwind v4 `@theme`. Light + dark are first-class; both production-quality from day one. Token categories:

- **Surfaces**: `--color-bg`, `--color-surface`, `--color-surface-2`, `--color-border` (warm tinted neutrals)
- **Foreground**: `--color-fg`, `--color-fg-muted`, `--color-fg-subtle`
- **Accent**: `--color-accent` (restrained blue-violet, oklch ~55% 0.18 265), `--color-accent-fg`, `--color-accent-subtle`
- **Semantic**: `--color-success`, `--color-warning`, `--color-danger`
- **Status badges**: `--color-candidate` (warm yellow), `--color-accepted` (green), `--color-rejected` (desaturated red), `--color-superseded` (grey)
- **Type**: `--font-sans` (Inter Variable), `--font-serif` (Source Serif 4 Variable), `--font-mono` (JetBrains Mono Variable)
- **Rhythm**: 4px base grid, modular type scale (1.2 ratio), `--radius-sm`, `--radius`, `--radius-lg`

Dark mode overrides surface and foreground tokens; semantic and accent tokens carry over unchanged.

### Typography

- **UI**: Inter Variable, tight tracking on headers (`-0.02em`).
- **Record body** (rendered markdown): **Source Serif 4 Variable**. Decision records are documents — a serif body in the rendered pane signals "this is the artifact, not chrome."
- **Code, IDs, file paths**: JetBrains Mono Variable.

Font files served from `web/static/` so there is no FOUT against external font CDNs.

### Component layers

Three tiers, no exceptions:

1. **`lib/components/ui/`** — shadcn-svelte recipes (button, dialog, popover, dropdown, tabs, command, toast, toggle, separator, scroll-area, tooltip, skeleton, badge). Tailwind classes only; no scoped styles.
2. **`lib/components/{records,candidates,tags,validation,...}/`** — domain components composed from `ui/` primitives. Examples: `RecordRow`, `RecordDetailPanes`, `SupersessionChain`, `DiagnosticPanel`, `CandidateDiff`, `TagTree`.
3. **`lib/components/shell/`** — app chrome: `Sidebar`, `Header`, `CommandPalette`, `ToastRegion`, `ThemeToggle`, `AgentStatusPill`.

**Hard rule:** domain components never style themselves with raw colors — only token references (`bg-surface`, `text-fg-muted`). Theme switching must require zero domain-component changes.

### Iconography

Lucide everywhere. Consistent 1.5px stroke. A small custom set for status (candidate / accepted / superseded / rejected) so they read at a glance even without color.

### Motion

Three rules; everything else is "no motion":

1. Spring-based, never duration-based, for interactive transforms (drawer open, sidebar collapse, dialog enter). Svelte 5's built-in `spring()` or Motion One.
2. Tween, 120ms, ease-out, for state changes that aren't gestural (theme switch crossfade, list reorder, badge count change).
3. Respect `prefers-reduced-motion` — collapse all motion to opacity changes.

No fancy page transitions. A record opening should feel instant.

### Markdown rendering

`marked` for parsing → custom renderer that:

- Replaces code fences with Shiki-highlighted output (precomputed at build for static content; runtime fallback for edited content).
- Maps wikilinks (`[[...]]`) to internal route links with hover preview.
- Renders front matter as a structured panel above the body, not as a code block.
- Auto-links record IDs (UUIDv7) in prose to the corresponding record's route.

Body in `--font-serif` for the rendered view, `--font-mono` for the raw view. Toggle via tabs, instant.

### Theme switching

Three states: `light`, `dark`, `system` (default). Persisted in `localStorage`, applied via `data-theme="..."` on `<html>`. Tailwind v4's `@variant dark` is wired to handle both explicit dark and system-dark.

No flash on first paint: theme is applied via inline `<script>` in `app.html` before any render.

## API mocking, error handling, testing

### MockApi boundary

A single `DadrApi` interface in `lib/api/types.ts` covers everything the UI calls. Two implementations:

- `RealApi` — fetch + Zod validation, used in production.
- `MockApi` — fixture-backed, used in dev when `VITE_DADR_MOCK=1`.

Switching is a single `if` in `lib/api/index.ts`. TanStack Query never knows which implementation is active. Component code never sees `fetch`.

### Mocked vs real (Phase 0/1/2 starting position)

| Endpoint | Today's status | Plan |
|---|---|---|
| `/api/project` (snapshot) | Real | Use as-is |
| `/api/records/*` | Real | Use as-is |
| `/api/validate/*` | Real | Use as-is |
| `/api/templates/candidate` | Real | Use as-is |
| `/api/agent/draft` | Real | Use as-is |
| Inbox (candidates with confidence/age/scope) | Missing | Mock from snapshot's `candidates` + synthesized fields |
| Candidate diff + source snippets | Missing | Mock with seeded fixtures |
| Audit trail | Missing | Mock |
| `/api/events` (SSE) | Missing | `MockEventSource` emits canned events on a timer |

Mocks live in `lib/api/mock/fixtures/` as JSON files plus a small generator. Rule: **a mock must be deletable in one commit** when the real endpoint lands. No mock-specific code paths leak into components.

### Mock-to-real swap pattern

When Rust ships a new endpoint:

1. Add Zod schema + `RealApi` method.
2. Delete the corresponding `MockApi` method and its fixture.
3. Run E2E. If green, ship.

Zero component changes — that's the point of the boundary.

### Error handling

Four error classes, four handlers:

| Class | Source | Surface |
|---|---|---|
| Network failure | `fetch` rejects, timeout | Toast with retry. Mutation marks failed; optimistic update rolls back. |
| Zod schema mismatch | API response shape doesn't match | Dev: red full-screen overlay with the Zod issue. Prod: toast "Backend mismatch — try refresh" + console log. |
| `CommandEnvelope` diagnostics | `envelope.diagnostics[]` from a successful response | `error` → red inline panel + toast. `warning` → toast. `info` → console only. |
| 404 / unknown route | URL doesn't match a route | Dedicated `+error.svelte` view, breadcrumb back to last known good route. |

Global error boundary at `+layout.svelte` catches unhandled exceptions and renders a recoverable error card with "Reload" and "Copy details." Never a white screen.

`DiagnosticPanel` is a single shared component used wherever diagnostics appear: validation results, mutation responses, agent draft envelopes. Consistent treatment everywhere.

### Testing

Three tiers, each with a clear responsibility.

**Unit (Vitest) — `tests/unit/`.** Pure logic and schema:
- Zod schemas (fixtures pass; malformed input fails predictably).
- Snapshot derivations (Inbox count, filtered records, tag tree construction).
- Markdown renderer transformations (wikilink → route, ID auto-link).
- `invalidateForChangedPaths` mapping.

**Component (Vitest + `@testing-library/svelte`) — `tests/unit/components/`.** Domain components in isolation. Mock the API at the `DadrApi` boundary; component tests never touch network.

**E2E (Playwright) — `tests/e2e/`.** Critical flows only. Run against a real `dadrock preview` server pointed at a fixture project, plus mock API for spec-07 screens. Flows:

1. Open project → see records list → select record → see detail.
2. Edit record → save → diagnostics surface → record updates in list.
3. Run validation → see diagnostics.
4. Open command palette → fuzzy-search a record → navigate to it.
5. Accept candidate (mocked) → record appears in `/records`, candidate disappears from `/inbox`.
6. Theme switch persists across reload.

### What we don't test

- shadcn-svelte recipes (already tested upstream).
- Tailwind class output.
- Pure presentational composition (covered transitively by E2E).
- Visual regression (no Chromatic in v1).

### CI

```
bun install --frozen-lockfile
bun run lint           # eslint + svelte-check + tsc --noEmit
bun run test:unit      # Vitest
bun run build          # SvelteKit static build (proves it ships)
bun run test:e2e       # Playwright against built artifacts + dadrock server
cargo test -p dadrock  # Rust side; build.rs runs JS pipeline transitively
```

The `cargo test` step exercises `build.rs`, so JS tests gate the Rust artifact too. "Frontend builds standalone but `cargo build` fails" is caught before merge.

## Phase milestones & cutover

Three phases, each with a shippable binary. Current `app.js` keeps working until Phase 1 cutover; nothing is broken in between.

### Phase 0 — Foundation

A binary that serves the existing UI at `/` *and* a new (mostly empty) Svelte app at `/v2/`. Design system is functional; routes are stubbed.

**Done when:**
- `crates/dadrock/web/` scaffolded with full stack (SvelteKit + adapter-static + TS + Tailwind v4 + Bits UI + shadcn-svelte).
- `bun install` and `bun run build` work; `web/build/` is produced.
- `build.rs` invokes the build via Bun when sources change.
- `web.rs` serves `web/build/` via `rust-embed` at `/v2/*`; existing `/` and `/assets/*` routes unchanged.
- Dev workflow documented: `bun run dev` proxies `/api/*` to a running `dadrock preview`.
- App shell complete: header, sidebar, command palette skeleton, toast region, theme switcher, error boundary.
- All 10 routes from the route tree exist as `+page.svelte` files (mostly placeholder).
- CI runs lint + unit + build + Rust tests.
- One round-trip is real: `/v2/records` lists records via `/api/project` — proves the API client + TanStack Query + Zod stack works end-to-end.

**Out of scope:** any feature beyond reading the snapshot.

### Phase 1 — Port today's surfaces (the polish bar lives here)

A binary where the Svelte UI replaces `app.js` entirely. Old static assets are deleted in this phase.

**Done when:**
- **Records** (`/records`, `/records/[id]`): list + filter + search + status filter + detail (rendered + raw + edit) + save + supersession chain visualization.
- **Tags** (`/tags`): tag tree, click-through to filtered records.
- **Validation** (`/validation`): all four kinds, `DiagnosticPanel` rendering, run-on-demand.
- **Init** flow when `dadr.toml` is missing.
- **Candidate creation** (manual + agent-assisted via existing `/api/agent/draft`).
- **Command palette** functional: navigation + record search + actions.
- Theme persistence; keyboard shortcuts (⌘K, ⌘\, j/k for record nav, ⌘S for save); `prefers-reduced-motion` honored.
- E2E flows pass against the new UI.
- **Cutover commit:** delete `crates/dadrock/src/web/static/{index.html,app.css,app.js}`, the three `include_str!` constants, and the `/assets/*` and `/index.html` routes from `web.rs`. Route `/` and unknown paths to the SvelteKit SPA fallback. Drop the `/v2/` prefix.
- **Polish acceptance:** every screen reviewed against the visual tone at light + dark with someone who didn't build it. Screenshots committed to `docs/superpowers/specs/screenshots/phase-1/`.

Phase 1 cutover is the moment dadrock becomes a Svelte product. Don't ship Phase 1 until the screens would belong on the docs site.

### Phase 2 — Spec-07 vision (frontend ahead of backend, mocked behind `DadrApi`)

A binary with the full spec-07 review workflow visible end-to-end. Some flows mock-backed; replacing mocks with real endpoints is incremental work that lands as Rust catches up.

**Done when:**
- **Inbox** (`/inbox`): candidates by confidence + age + scope, filterable, default landing route. Mock-backed.
- **Candidate review** (`/candidates/[id]`): tabs for rendered / raw / front matter / source snippets / diff. Accept / reject / split actions. Source snippets and diff mock-backed.
- **Supersession chain** detail view extends record detail with full chain + freshness.
- **Link health queue** (`/links`) on top of existing `/api/validate/links`.
- **Audit trail** (`/audit`): timeline view, mock-backed until Rust emits events.
- **SSE wired** with `MockEventSource`; query invalidation logic in place. Real `EventSource` swap when Rust SSE endpoint lands (single line change).
- Each Phase 2 screen ships with the same polish bar as Phase 1.

**Out of scope (Phase 3+):** robustness queue, faithfulness checks UI, reports/build/preview UI, Tauri desktop wrap.

## Risks & mitigations

| Risk | Mitigation |
|---|---|
| Phase 1 stalls; we live in `/v2/` purgatory | Hard rule: Phase 1 isn't done until cutover commit lands. No "we'll cut over later." |
| Mock data drifts from real shape; swap breaks | Zod schemas are the single source of truth — same schema validates real and mock. Mocks parse against the schema in unit tests. |
| `build.rs` slows `cargo build` for everyone | Cache: only rebuild if `web/src/**` or `web/package.json` changed. Skip in `cargo check`. |
| JS toolchain breaks for Rust contributors | `build.rs` produces a clear error message pointing to setup docs if `bun` is not on PATH. |
| Polish bar slips to "good enough" | Phase 1 cutover gate: side-by-side screenshot review at light + dark before merge. |
| TanStack Query learning curve | Pair-program through the first three queries and the first three mutations; document the patterns in `web/README.md`. |
| Bun-specific build incompatibility with a Svelte plugin | Fallback to pnpm is a one-line change in `build.rs` and `web/package.json` — keep the option open for ~6 months post-cutover. |

## Open questions (intentionally deferred)

- **Default landing route** (`/inbox` once it exists, `/records` until then) — answer naturally during Phase 2.
- **Hover preview** for wikilinks — visual design TBD; not blocking.
- **Settings screen scope** — covered by Phase 1's "Init flow" plus a thin settings page; full settings UX is its own design exercise.
- **Tauri desktop wrap** — out of scope. The static SPA build is Tauri-compatible if/when we want it.

## Implementation plan

This design hands off to the `writing-plans` skill, which will produce a detailed phase-by-phase implementation plan with checkpoints. The plan should preserve the three-phase structure here and produce concrete tasks per phase.
