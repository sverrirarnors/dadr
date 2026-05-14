# dadrock web (Svelte)

The Svelte web UI for dadrock. It is embedded into the dadrock binary at build time and served as the primary web interface.

## Stack

Svelte 5, SvelteKit static SPA mode, TypeScript, Tailwind v4, Bits UI, shadcn-svelte recipes, Lucide, TanStack Query, Zod, Vitest, Playwright, and Bun.

## Prerequisites

- Bun 1.1+
- Rust 1.75+

## Dev Workflow

Run the Rust API server:

```bash
cargo run -p dadrock -- preview
```

Run the Vite dev server, proxying `/api/*` to the Rust server:

```bash
cd crates/dadrock/web
DADR_PREVIEW_URL=http://localhost:8765 bun run dev
```

Open `http://localhost:5173`. Edits hot-reload.

## Production Build

The Rust `build.rs` invokes `bun install --frozen-lockfile` when needed and `bun run build` when `cargo build -p dadrock` runs. The output in `crates/dadrock/web/build/` is embedded into the binary with `rust-embed`.

To skip the JS build during fast Rust iteration:

```bash
CARGO_FEATURE_NO_JS_BUILD=1 cargo build -p dadrock
```

## Scripts

| Command | What it does |
|---|---|
| `bun run dev` | Vite dev server with `/api/*` proxy |
| `bun run build` | Production build to `build/` |
| `bun run preview` | Serve the production build locally |
| `bun run check` | `svelte-check` |
| `bun run lint` | ESLint |
| `bun run test:unit` | Vitest |
| `bun run test:e2e` | Playwright |

## Conventions

See `docs/superpowers/specs/2026-05-09-svelte-web-ui-design.md` for the full design spec, including component layers, state model, API boundary, design tokens, and phase milestones.
