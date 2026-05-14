# Svelte Web UI — Phase 0 (Foundation) Implementation Plan

> **For agentic executors:** This plan implements **Phase 0 only** (Foundation) of the three-phase Svelte web UI rebuild specified in `docs/superpowers/specs/2026-05-09-svelte-web-ui-design.md`. Phase 1 (porting today's surfaces with polish) and Phase 2 (spec-07 vision) are separate plans, written after Phase 0 ships. Read the spec before starting; this plan presumes you have it open.
>
> Steps use checkbox (`- [ ]`) syntax. Execute task-by-task, not step-by-step out of order. Commit at the end of each task using the message shown in the task's final step.

**Goal:** Stand up the Svelte + SvelteKit + Tailwind v4 + Bun toolchain inside the existing `dadrock` Rust binary, embed the built SPA at `/v2/*` alongside the current `app.js` UI (which keeps working at `/`), and prove the foundation with one real end-to-end round-trip (records list via `/api/project`).

**Architecture:** A new `crates/dadrock/web/` directory hosts the Svelte source. SvelteKit's `adapter-static` builds an SPA into `web/build/`. `crates/dadrock/build.rs` invokes `bun run build` when sources change. The built artifacts are embedded into the Rust binary via `rust-embed` and served at `/v2/*`. The current `app.js` UI is **untouched** in Phase 0 and remains the default at `/`.

**Tech Stack:** Svelte 5, SvelteKit (`@sveltejs/adapter-static`, SPA fallback mode), TypeScript (strict), Tailwind v4, Bits UI, shadcn-svelte, Lucide icons, TanStack Query (`@tanstack/svelte-query`), Zod, Inter Variable, Source Serif 4 Variable, JetBrains Mono Variable, Vitest, Playwright, Bun (runtime + package manager).

**Out of scope for Phase 0:**
- Editing records, validation UI, tag tree, candidate review, or any feature beyond reading the snapshot.
- Polish quality of any screen (Phase 1 owns polish).
- Real backend for spec-07 endpoints (Phase 2 introduces mocks).
- Deletion of `app.js` and the old static-asset routes (Phase 1 owns cutover).

---

## Pre-flight

Before starting tasks, confirm the following are installed:

- [ ] **Confirm prerequisites**

```bash
bun --version            # require 1.1+ (Bun runtime + package manager)
node --version           # require 20+ (Vite still uses Node-compatible JS APIs; Bun ships its own equivalent but some plugins prefer Node)
cargo --version          # require 1.75+
rustc --version
```

If `bun` is missing: install via `curl -fsSL https://bun.sh/install | bash` (macOS/Linux) and add to PATH.

- [ ] **Read the design spec**

```
docs/superpowers/specs/2026-05-09-svelte-web-ui-design.md
```

Key sections to internalize: "Project layout", "Build & shipping", "State model", "Component & design system", "Phase milestones & cutover (Phase 0)".

- [ ] **Verify current binary builds and runs**

```bash
cargo build -p dadrock
cargo run -p dadrock -- preview
# In a browser: http://localhost:<port shown> — confirm the current vanilla-JS UI loads.
# Note the port; you'll proxy to it in Task 4.
# Stop with Ctrl-C.
```

---

## Task 1 — Scaffold the SvelteKit project

**Files:**
- Create: `crates/dadrock/web/package.json`
- Create: `crates/dadrock/web/.gitignore`
- Create: `crates/dadrock/web/svelte.config.js`
- Create: `crates/dadrock/web/vite.config.ts`
- Create: `crates/dadrock/web/src/app.html`
- Create: `crates/dadrock/web/src/app.d.ts`
- Create: `crates/dadrock/web/static/.gitkeep`

- [ ] **Step 1: Create the directory and `package.json`**

```bash
mkdir -p crates/dadrock/web/src/{lib/{api,components/{ui,shell,records,candidates,tags,validation},stores,markdown,utils},routes,tests/{unit,e2e}}
mkdir -p crates/dadrock/web/static
```

Write `crates/dadrock/web/package.json`:

```json
{
  "name": "dadrock-web",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-kit sync && svelte-check --tsconfig ./tsconfig.json",
    "lint": "eslint .",
    "test:unit": "vitest run",
    "test:unit:watch": "vitest",
    "test:e2e": "playwright test"
  },
  "devDependencies": {
    "@sveltejs/adapter-static": "^3.0.6",
    "@sveltejs/kit": "^2.7.0",
    "@sveltejs/vite-plugin-svelte": "^4.0.0",
    "svelte": "^5.0.0",
    "svelte-check": "^4.0.0",
    "typescript": "^5.5.0",
    "vite": "^5.4.0"
  }
}
```

- [ ] **Step 2: Install dependencies**

```bash
cd crates/dadrock/web
bun install
```

Expected: `bun.lock` is created. `node_modules/` populated. No errors.

- [ ] **Step 3: Create `.gitignore`**

`crates/dadrock/web/.gitignore`:

```
node_modules/
.svelte-kit/
build/
dist/
.env
.env.*
!.env.example
.vite/
playwright-report/
test-results/
```

- [ ] **Step 4: Configure SvelteKit (`svelte.config.js`)**

`crates/dadrock/web/svelte.config.js`:

```js
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

export default {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',
      precompress: true,
      strict: true,
    }),
    paths: {
      base: '/v2',
      relative: false,
    },
    prerender: { entries: [] },
  },
};
```

The `paths.base = '/v2'` is critical: it makes the SPA mountable at `/v2/*` so it coexists with the current UI at `/`. Phase 1 will remove this.

- [ ] **Step 5: Configure Vite (`vite.config.ts`)**

`crates/dadrock/web/vite.config.ts`:

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': {
        target: process.env.DADR_PREVIEW_URL ?? 'http://localhost:8765',
        changeOrigin: true,
      },
    },
  },
  build: {
    sourcemap: true,
  },
});
```

The default proxy target `http://localhost:8765` is a placeholder; the engineer running `bun run dev` will set `DADR_PREVIEW_URL` to whatever port `dadrock preview` reports on startup.

- [ ] **Step 6: Configure TypeScript (`tsconfig.json`)**

`crates/dadrock/web/tsconfig.json`:

```json
{
  "extends": "./.svelte-kit/tsconfig.json",
  "compilerOptions": {
    "allowJs": true,
    "checkJs": true,
    "esModuleInterop": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "skipLibCheck": true,
    "sourceMap": true,
    "strict": true,
    "moduleResolution": "bundler",
    "noUncheckedIndexedAccess": true,
    "exactOptionalPropertyTypes": true
  }
}
```

- [ ] **Step 7: Create `app.html`**

`crates/dadrock/web/src/app.html`:

```html
<!doctype html>
<html lang="en" data-theme="system">
  <head>
    <meta charset="utf-8" />
    <link rel="icon" href="%sveltekit.assets%/favicon.svg" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>dadrock</title>
    <script>
      // No-FOUC theme application — runs before any render.
      (function () {
        const stored = localStorage.getItem('dadr.theme');
        const theme = stored === 'light' || stored === 'dark' ? stored : 'system';
        document.documentElement.setAttribute('data-theme', theme);
      })();
    </script>
    %sveltekit.head%
  </head>
  <body data-sveltekit-preload-data="hover">
    <div style="display: contents">%sveltekit.body%</div>
  </body>
</html>
```

- [ ] **Step 8: Create `app.d.ts`**

`crates/dadrock/web/src/app.d.ts`:

```ts
declare global {
  namespace App {
    // interface Error {}
    // interface Locals {}
    // interface PageData {}
    // interface PageState {}
    // interface Platform {}
  }
}

export {};
```

- [ ] **Step 9: Verify the scaffold builds**

```bash
cd crates/dadrock/web
bun run build
```

Expected output: `vite build` runs, emits `build/` containing `index.html`, `_app/...`, etc. No TypeScript errors. Build succeeds.

If you see "no routes defined": create a placeholder `src/routes/+page.svelte` with `<h1>placeholder</h1>` and rerun. We'll properly create routes in later tasks; for this verification a placeholder is fine. Delete the placeholder before committing this task.

- [ ] **Step 10: Commit**

```bash
git add crates/dadrock/web/package.json crates/dadrock/web/bun.lock crates/dadrock/web/.gitignore crates/dadrock/web/svelte.config.js crates/dadrock/web/vite.config.ts crates/dadrock/web/tsconfig.json crates/dadrock/web/src/app.html crates/dadrock/web/src/app.d.ts crates/dadrock/web/static/.gitkeep
git commit -m "scaffold svelte web ui (phase 0 task 1)"
```

---

## Task 2 — Tailwind v4 + design tokens

**Files:**
- Create: `crates/dadrock/web/src/app.css`
- Modify: `crates/dadrock/web/package.json` (add Tailwind dependency)
- Modify: `crates/dadrock/web/vite.config.ts` (Tailwind plugin)

- [ ] **Step 1: Install Tailwind v4 and the Vite plugin**

```bash
cd crates/dadrock/web
bun add -d tailwindcss@next @tailwindcss/vite@next
```

Tailwind v4 uses a Vite plugin (no PostCSS configuration needed) and the `@theme` CSS-first token system.

- [ ] **Step 2: Wire the Tailwind plugin into Vite**

Modify `crates/dadrock/web/vite.config.ts`:

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [tailwindcss(), sveltekit()],
  server: {
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': {
        target: process.env.DADR_PREVIEW_URL ?? 'http://localhost:8765',
        changeOrigin: true,
      },
    },
  },
  build: {
    sourcemap: true,
  },
});
```

- [ ] **Step 3: Create `app.css` with design tokens**

`crates/dadrock/web/src/app.css`:

```css
@import "tailwindcss";

@custom-variant dark (&:where([data-theme="dark"], [data-theme="system"]:where([data-prefers-dark]))) ;

@theme {
  /* Surfaces — warm, slightly tinted neutrals (Obsidian-inspired) */
  --color-bg:               oklch(99% 0.005 95);
  --color-surface:          oklch(97% 0.005 95);
  --color-surface-2:        oklch(94% 0.008 95);
  --color-border:           oklch(88% 0.010 95);

  /* Foreground */
  --color-fg:               oklch(20% 0.015 270);
  --color-fg-muted:         oklch(45% 0.015 270);
  --color-fg-subtle:        oklch(60% 0.015 270);

  /* Accent — restrained blue-violet */
  --color-accent:           oklch(55% 0.18 265);
  --color-accent-fg:        oklch(98% 0 0);
  --color-accent-subtle:    oklch(92% 0.04 265);

  /* Semantic */
  --color-success:          oklch(60% 0.15 145);
  --color-warning:          oklch(70% 0.15 75);
  --color-danger:           oklch(58% 0.20 25);

  /* Status badges */
  --color-candidate:        oklch(65% 0.13 95);
  --color-accepted:         oklch(60% 0.15 145);
  --color-rejected:         oklch(58% 0.05 25);
  --color-superseded:       oklch(60% 0.02 270);

  /* Typography */
  --font-sans:              "Inter Variable", ui-sans-serif, system-ui, sans-serif;
  --font-serif:             "Source Serif 4 Variable", Georgia, serif;
  --font-mono:              "JetBrains Mono Variable", ui-monospace, "SF Mono", monospace;

  /* Radii */
  --radius-sm:              4px;
  --radius:                 6px;
  --radius-lg:              10px;
}

/* Dark theme */
:where([data-theme="dark"], [data-theme="system"][data-prefers-dark]) {
  --color-bg:              oklch(18% 0.012 270);
  --color-surface:         oklch(22% 0.012 270);
  --color-surface-2:       oklch(26% 0.014 270);
  --color-border:          oklch(32% 0.014 270);
  --color-fg:              oklch(95% 0.005 270);
  --color-fg-muted:        oklch(72% 0.012 270);
  --color-fg-subtle:       oklch(55% 0.012 270);
  --color-accent-subtle:   oklch(28% 0.06 265);
}

@layer base {
  html, body {
    background-color: var(--color-bg);
    color: var(--color-fg);
    font-family: var(--font-sans);
    font-feature-settings: "cv11", "ss01", "ss03";
    text-rendering: optimizeLegibility;
    -webkit-font-smoothing: antialiased;
  }

  ::selection {
    background-color: var(--color-accent-subtle);
    color: var(--color-fg);
  }
}
```

- [ ] **Step 4: Add a temporary test page to verify Tailwind classes resolve**

Create `crates/dadrock/web/src/routes/+page.svelte`:

```svelte
<script lang="ts">
  import '../app.css';
</script>

<main class="min-h-screen p-8 bg-bg text-fg">
  <h1 class="text-3xl font-sans font-semibold">dadrock — Phase 0 scaffold</h1>
  <p class="mt-2 text-fg-muted">Tailwind v4 tokens active.</p>
  <button class="mt-4 px-3 py-1.5 rounded-md bg-accent text-accent-fg text-sm">Accent button</button>
</main>
```

(This is a temporary placeholder; later tasks replace it with the real layout.)

- [ ] **Step 5: Verify dev server renders the tokens**

In one terminal:

```bash
cargo run -p dadrock -- preview
```

Note the port (e.g., `8765`). In another terminal:

```bash
cd crates/dadrock/web
DADR_PREVIEW_URL=http://localhost:8765 bun run dev
```

Open `http://localhost:5173`. Expected: the page renders with warm-tinted background, blue-violet accent button, Inter UI font (system fallback until Task 3 installs the variable font).

Stop both servers (Ctrl-C).

- [ ] **Step 6: Commit**

```bash
git add crates/dadrock/web/package.json crates/dadrock/web/bun.lock crates/dadrock/web/vite.config.ts crates/dadrock/web/src/app.css crates/dadrock/web/src/routes/+page.svelte
git commit -m "tailwind v4 + design tokens (phase 0 task 2)"
```

---

## Task 3 — Variable fonts

**Files:**
- Modify: `crates/dadrock/web/package.json` (font deps)
- Modify: `crates/dadrock/web/src/app.css` (font imports)

- [ ] **Step 1: Install `@fontsource-variable` packages**

```bash
cd crates/dadrock/web
bun add @fontsource-variable/inter @fontsource-variable/source-serif-4 @fontsource-variable/jetbrains-mono
```

- [ ] **Step 2: Import font CSS at the top of `app.css`**

Modify `crates/dadrock/web/src/app.css`. Add these imports **above** the `@import "tailwindcss";` line:

```css
@import "@fontsource-variable/inter/wght.css";
@import "@fontsource-variable/inter/wght-italic.css";
@import "@fontsource-variable/source-serif-4/wght.css";
@import "@fontsource-variable/source-serif-4/wght-italic.css";
@import "@fontsource-variable/jetbrains-mono/wght.css";

@import "tailwindcss";

/* ... rest of the file unchanged ... */
```

- [ ] **Step 3: Verify fonts load**

```bash
bun run dev
# (Make sure DADR_PREVIEW_URL is set as in Task 2 if you also want /api proxy.)
```

Open `http://localhost:5173`. Open DevTools → Network → filter "Font". Confirm Inter Variable, Source Serif 4 Variable, and JetBrains Mono Variable are loaded. The headline should now visibly use Inter (not the system fallback).

- [ ] **Step 4: Commit**

```bash
git add crates/dadrock/web/package.json crates/dadrock/web/bun.lock crates/dadrock/web/src/app.css
git commit -m "variable fonts (phase 0 task 3)"
```

---

## Task 4 — Theme system (light / dark / system) with no-FOUC

**Files:**
- Create: `crates/dadrock/web/src/lib/stores/theme.svelte.ts`
- Create: `crates/dadrock/web/src/lib/components/shell/ThemeToggle.svelte`
- Create: `crates/dadrock/web/tests/unit/theme.test.ts`
- Modify: `crates/dadrock/web/src/app.html` (extend the no-FOUC script to set `data-prefers-dark`)
- Modify: `crates/dadrock/web/src/routes/+page.svelte` (mount `ThemeToggle` for verification)

- [ ] **Step 1: Extend the no-FOUC script in `app.html`**

The dark variant in `app.css` keys off `data-prefers-dark` (set when the system prefers dark). The script must apply this attribute synchronously before render. Replace the inline script in `app.html`:

```html
<script>
  (function () {
    const stored = localStorage.getItem('dadr.theme');
    const theme = stored === 'light' || stored === 'dark' ? stored : 'system';
    const html = document.documentElement;
    html.setAttribute('data-theme', theme);
    if (theme === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches) {
      html.setAttribute('data-prefers-dark', '');
    }
  })();
</script>
```

- [ ] **Step 2: Write the failing test for the theme store**

`crates/dadrock/web/tests/unit/theme.test.ts`:

```ts
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { theme } from '../../src/lib/stores/theme.svelte';

describe('theme store', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    document.documentElement.removeAttribute('data-prefers-dark');
  });

  it('defaults to system when nothing is stored', () => {
    expect(theme.current).toBe('system');
  });

  it('persists explicit choice and applies data-theme', () => {
    theme.set('dark');
    expect(localStorage.getItem('dadr.theme')).toBe('dark');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
  });

  it('cycles light -> dark -> system -> light', () => {
    theme.set('light');
    theme.cycle();
    expect(theme.current).toBe('dark');
    theme.cycle();
    expect(theme.current).toBe('system');
    theme.cycle();
    expect(theme.current).toBe('light');
  });
});
```

(Vitest is installed in Task 12. To run this test now, you can install it ahead with `bun add -d vitest @vitest/ui jsdom @testing-library/svelte` and add a minimal `vitest.config.ts`. Or skip running it until Task 12 — the implementation below is correct, and Task 12 includes "rerun all unit tests" as a verification step.)

- [ ] **Step 3: Implement the theme store**

`crates/dadrock/web/src/lib/stores/theme.svelte.ts`:

```ts
type ThemeChoice = 'light' | 'dark' | 'system';

const STORAGE_KEY = 'dadr.theme';
const ORDER: ThemeChoice[] = ['light', 'dark', 'system'];

function read(): ThemeChoice {
  if (typeof localStorage === 'undefined') return 'system';
  const stored = localStorage.getItem(STORAGE_KEY);
  return stored === 'light' || stored === 'dark' ? stored : 'system';
}

function apply(choice: ThemeChoice) {
  if (typeof document === 'undefined') return;
  const html = document.documentElement;
  html.setAttribute('data-theme', choice);
  const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
  if (choice === 'system' && prefersDark) {
    html.setAttribute('data-prefers-dark', '');
  } else {
    html.removeAttribute('data-prefers-dark');
  }
}

class ThemeStore {
  current = $state<ThemeChoice>(read());

  set(choice: ThemeChoice) {
    this.current = choice;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(STORAGE_KEY, choice);
    }
    apply(choice);
  }

  cycle() {
    const idx = ORDER.indexOf(this.current);
    this.set(ORDER[(idx + 1) % ORDER.length] ?? 'system');
  }
}

export const theme = new ThemeStore();

if (typeof window !== 'undefined') {
  const mq = window.matchMedia('(prefers-color-scheme: dark)');
  mq.addEventListener('change', () => apply(theme.current));
}
```

- [ ] **Step 4: Create `ThemeToggle.svelte`**

`crates/dadrock/web/src/lib/components/shell/ThemeToggle.svelte`:

```svelte
<script lang="ts">
  import { theme } from '$lib/stores/theme.svelte';
  import { Sun, Moon, Monitor } from 'lucide-svelte';

  const labels: Record<typeof theme.current, string> = {
    light: 'Light theme',
    dark: 'Dark theme',
    system: 'System theme',
  };
</script>

<button
  type="button"
  class="inline-flex items-center justify-center h-8 w-8 rounded-md hover:bg-surface-2 text-fg-muted hover:text-fg transition-colors"
  aria-label={labels[theme.current]}
  title={labels[theme.current]}
  onclick={() => theme.cycle()}
>
  {#if theme.current === 'light'}
    <Sun class="h-4 w-4" />
  {:else if theme.current === 'dark'}
    <Moon class="h-4 w-4" />
  {:else}
    <Monitor class="h-4 w-4" />
  {/if}
</button>
```

(`lucide-svelte` is installed in Task 5; this file will not type-check until then. Continue and resolve in Task 5.)

- [ ] **Step 5: Mount the toggle on the placeholder page**

Modify `crates/dadrock/web/src/routes/+page.svelte`:

```svelte
<script lang="ts">
  import '../app.css';
  import ThemeToggle from '$lib/components/shell/ThemeToggle.svelte';
</script>

<main class="min-h-screen p-8 bg-bg text-fg">
  <header class="flex items-center justify-between">
    <h1 class="text-3xl font-sans font-semibold">dadrock — Phase 0 scaffold</h1>
    <ThemeToggle />
  </header>
  <p class="mt-2 text-fg-muted">Tailwind v4 tokens active. Click the toggle to cycle theme.</p>
  <button class="mt-4 px-3 py-1.5 rounded-md bg-accent text-accent-fg text-sm">Accent button</button>
</main>
```

- [ ] **Step 6: Commit (verification of toggle behavior happens after Task 5 installs Lucide)**

```bash
git add crates/dadrock/web/src/app.html crates/dadrock/web/src/lib/stores/theme.svelte.ts crates/dadrock/web/src/lib/components/shell/ThemeToggle.svelte crates/dadrock/web/src/routes/+page.svelte crates/dadrock/web/tests/unit/theme.test.ts
git commit -m "theme system with no-FOUC (phase 0 task 4)"
```

---

## Task 5 — Lucide icons

**Files:**
- Modify: `crates/dadrock/web/package.json` (lucide dep)

- [ ] **Step 1: Install Lucide for Svelte**

```bash
cd crates/dadrock/web
bun add lucide-svelte
```

- [ ] **Step 2: Verify the toggle renders icons**

Restart `bun run dev` and reload `http://localhost:5173`. Click the theme toggle: icons should change Sun → Moon → Monitor → Sun. Document background and text colors should change between light and dark themes.

- [ ] **Step 3: Commit**

```bash
git add crates/dadrock/web/package.json crates/dadrock/web/bun.lock
git commit -m "lucide icons (phase 0 task 5)"
```

---

## Task 6 — Bits UI + shadcn-svelte initial recipes

**Files:**
- Modify: `crates/dadrock/web/package.json`
- Create: `crates/dadrock/web/components.json`
- Create: `crates/dadrock/web/src/lib/utils.ts` (cn helper)
- Create: shadcn-svelte ui recipes under `crates/dadrock/web/src/lib/components/ui/` (Button, Dialog, DropdownMenu, Tabs, Toast, Tooltip, Separator, Badge, ScrollArea, Skeleton, Command — the minimum needed by later tasks)

- [ ] **Step 1: Install Bits UI**

```bash
cd crates/dadrock/web
bun add bits-ui clsx tailwind-merge
bun add -d tailwind-variants
```

- [ ] **Step 2: Create `cn` utility**

`crates/dadrock/web/src/lib/utils.ts`:

```ts
import { clsx, type ClassValue } from 'clsx';
import { twMerge } from 'tailwind-merge';

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

- [ ] **Step 3: Create `components.json` for shadcn-svelte**

`crates/dadrock/web/components.json`:

```json
{
  "$schema": "https://shadcn-svelte.com/schema.json",
  "style": "default",
  "tailwind": {
    "config": "",
    "css": "src/app.css",
    "baseColor": "neutral"
  },
  "aliases": {
    "components": "$lib/components",
    "utils": "$lib/utils",
    "ui": "$lib/components/ui",
    "hooks": "$lib/hooks",
    "lib": "$lib"
  },
  "typescript": true,
  "registry": "https://shadcn-svelte.com/registry"
}
```

- [ ] **Step 4: Add the ui recipes shadcn-svelte CLI would generate**

The shadcn-svelte CLI normally scaffolds these. For determinism in this plan, write them directly. Create the following files exactly as shown.

`crates/dadrock/web/src/lib/components/ui/button/button.svelte`:

```svelte
<script lang="ts" module>
  import { tv, type VariantProps } from 'tailwind-variants';

  export const buttonVariants = tv({
    base: 'inline-flex items-center justify-center gap-2 whitespace-nowrap rounded-md text-sm font-medium transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:pointer-events-none disabled:opacity-50',
    variants: {
      variant: {
        default: 'bg-accent text-accent-fg hover:bg-accent/90',
        outline: 'border border-border bg-bg hover:bg-surface text-fg',
        ghost: 'hover:bg-surface-2 text-fg',
        danger: 'bg-danger text-accent-fg hover:opacity-90',
      },
      size: {
        sm: 'h-8 px-3',
        md: 'h-9 px-4',
        lg: 'h-10 px-5',
        icon: 'h-9 w-9',
      },
    },
    defaultVariants: { variant: 'default', size: 'md' },
  });

  export type ButtonVariants = VariantProps<typeof buttonVariants>;
</script>

<script lang="ts">
  import { cn } from '$lib/utils';
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';

  type Props = HTMLButtonAttributes & ButtonVariants & {
    class?: string;
    children?: Snippet;
  };
  let { class: className, variant, size, children, ...rest }: Props = $props();
</script>

<button class={cn(buttonVariants({ variant, size }), className)} {...rest}>
  {@render children?.()}
</button>
```

`crates/dadrock/web/src/lib/components/ui/button/index.ts`:

```ts
import Button from './button.svelte';
export { Button };
export type { ButtonVariants } from './button.svelte';
```

- [ ] **Step 5: Add a placeholder index for the `ui/` barrel**

`crates/dadrock/web/src/lib/components/ui/index.ts`:

```ts
export * from './button';
```

(Other ui recipes — Dialog, DropdownMenu, Tabs, Toast, Tooltip, Command, etc. — are scaffolded on demand in Tasks 7–14 as each is first needed. shadcn-svelte's pattern is "copy what you use," not "copy the whole library.")

- [ ] **Step 6: Verify the Button recipe**

Modify `crates/dadrock/web/src/routes/+page.svelte`:

```svelte
<script lang="ts">
  import '../app.css';
  import ThemeToggle from '$lib/components/shell/ThemeToggle.svelte';
  import { Button } from '$lib/components/ui/button';
</script>

<main class="min-h-screen p-8 bg-bg text-fg">
  <header class="flex items-center justify-between">
    <h1 class="text-3xl font-sans font-semibold">dadrock — Phase 0 scaffold</h1>
    <ThemeToggle />
  </header>
  <p class="mt-2 text-fg-muted">Foundation in place.</p>
  <div class="mt-4 flex gap-2">
    <Button>Primary</Button>
    <Button variant="outline">Outline</Button>
    <Button variant="ghost">Ghost</Button>
    <Button variant="danger">Danger</Button>
  </div>
</main>
```

Run `bun run dev` and confirm all four button variants render correctly in both light and dark themes.

- [ ] **Step 7: Commit**

```bash
git add crates/dadrock/web/package.json crates/dadrock/web/bun.lock crates/dadrock/web/components.json crates/dadrock/web/src/lib/utils.ts crates/dadrock/web/src/lib/components/ui/button crates/dadrock/web/src/lib/components/ui/index.ts crates/dadrock/web/src/routes/+page.svelte
git commit -m "bits ui + shadcn-svelte button recipe (phase 0 task 6)"
```

---

## Task 7 — TanStack Query + provider

**Files:**
- Modify: `crates/dadrock/web/package.json`
- Create: `crates/dadrock/web/src/lib/query.ts`
- Create: `crates/dadrock/web/src/routes/+layout.ts`
- Modify: `crates/dadrock/web/src/routes/+layout.svelte` (will be created in Task 14; for Phase 0 verification we briefly wire it here)

- [ ] **Step 1: Install TanStack Query**

```bash
cd crates/dadrock/web
bun add @tanstack/svelte-query
bun add -d @tanstack/svelte-query-devtools
```

- [ ] **Step 2: Create the query client**

`crates/dadrock/web/src/lib/query.ts`:

```ts
import { QueryClient } from '@tanstack/svelte-query';

export function createQueryClient(): QueryClient {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 30_000,
        retry: 1,
        refetchOnWindowFocus: false,
      },
      mutations: {
        retry: 0,
      },
    },
  });
}
```

- [ ] **Step 3: Hook the client into the root layout via `+layout.ts`**

`crates/dadrock/web/src/routes/+layout.ts`:

```ts
import { browser } from '$app/environment';
import { createQueryClient } from '$lib/query';

export const ssr = false;
export const prerender = false;

export async function load() {
  if (!browser) {
    return { queryClient: null };
  }
  return { queryClient: createQueryClient() };
}
```

(`ssr = false` and `prerender = false` enforce SPA mode.)

- [ ] **Step 4: Verify the install**

```bash
bun run check
```

Expected: no TS errors. If you see "Cannot find module '@tanstack/svelte-query'", rerun `bun install`.

- [ ] **Step 5: Commit**

```bash
git add crates/dadrock/web/package.json crates/dadrock/web/bun.lock crates/dadrock/web/src/lib/query.ts crates/dadrock/web/src/routes/+layout.ts
git commit -m "tanstack query setup (phase 0 task 7)"
```

---

## Task 8 — Zod + first schema (snapshot)

**Files:**
- Modify: `crates/dadrock/web/package.json`
- Create: `crates/dadrock/web/src/lib/api/schemas.ts`
- Create: `crates/dadrock/web/tests/unit/schemas.test.ts`

- [ ] **Step 1: Install Zod**

```bash
cd crates/dadrock/web
bun add zod
```

- [ ] **Step 2: Inspect the real `/api/project` response shape**

Run the dadrock server and inspect a real snapshot to ensure schema accuracy:

```bash
cargo run -p dadrock -- preview &
PORT=$(lsof -nP -iTCP -sTCP:LISTEN | grep dadrock | head -1 | awk '{print $9}' | sed 's/.*://')
curl -s http://localhost:$PORT/api/project > /tmp/dadr-snapshot.json
kill %1
cat /tmp/dadr-snapshot.json | head -80
```

Use the real shape to inform the schema below — the example below covers the fields used in Task 14, but extend it if the real response includes additional required keys.

- [ ] **Step 3: Write a failing test for the schema**

`crates/dadrock/web/tests/unit/schemas.test.ts`:

```ts
import { describe, it, expect } from 'vitest';
import { snapshotSchema, recordSummarySchema } from '../../src/lib/api/schemas';

describe('snapshotSchema', () => {
  it('parses a minimal valid snapshot', () => {
    const input = {
      project: { root: '/tmp/proj', decisions_dir: '/tmp/proj/decisions', initialized: true },
      records: [],
      tags: [],
      counts: { candidate: 0, accepted: 0, rejected: 0, superseded: 0 },
    };
    const parsed = snapshotSchema.parse(input);
    expect(parsed.records).toEqual([]);
  });

  it('parses a snapshot with records', () => {
    const input = {
      project: { root: '/tmp/proj', decisions_dir: '/tmp/proj/decisions', initialized: true },
      records: [
        {
          id: '0193fb1f-a3a0-7000-8000-000000000000',
          title: 'Use Polars over pandas',
          status: 'accepted',
          path: 'decisions/0001-use-polars.md',
          tags: ['#dadr/data/processing'],
          successors: [],
        },
      ],
      tags: ['#dadr/data/processing'],
      counts: { candidate: 0, accepted: 1, rejected: 0, superseded: 0 },
    };
    const parsed = snapshotSchema.parse(input);
    expect(parsed.records[0].title).toBe('Use Polars over pandas');
  });

  it('rejects an invalid status', () => {
    const input = {
      project: { root: '/tmp/proj', decisions_dir: '/tmp/proj/decisions', initialized: true },
      records: [
        {
          id: '0193fb1f-a3a0-7000-8000-000000000000',
          title: 'x',
          status: 'invalid',
          path: 'x.md',
          tags: [],
          successors: [],
        },
      ],
      tags: [],
      counts: { candidate: 0, accepted: 0, rejected: 0, superseded: 0 },
    };
    expect(() => snapshotSchema.parse(input)).toThrow();
  });
});

describe('recordSummarySchema', () => {
  it('requires UUIDv7 format for id', () => {
    expect(() =>
      recordSummarySchema.parse({
        id: 'not-a-uuid',
        title: 't',
        status: 'accepted',
        path: 'p.md',
        tags: [],
        successors: [],
      }),
    ).toThrow();
  });
});
```

- [ ] **Step 4: Implement the schema**

`crates/dadrock/web/src/lib/api/schemas.ts`:

```ts
import { z } from 'zod';

export const recordStatusSchema = z.enum(['candidate', 'accepted', 'rejected', 'superseded']);
export type RecordStatus = z.infer<typeof recordStatusSchema>;

const uuidv7 = z.string().regex(
  /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i,
  'expected UUIDv7',
);

export const recordSummarySchema = z.object({
  id: uuidv7,
  title: z.string(),
  status: recordStatusSchema,
  path: z.string(),
  tags: z.array(z.string()),
  successors: z.array(uuidv7),
});
export type RecordSummary = z.infer<typeof recordSummarySchema>;

export const projectInfoSchema = z.object({
  root: z.string(),
  decisions_dir: z.string(),
  initialized: z.boolean(),
});

export const countsSchema = z.object({
  candidate: z.number().int().nonnegative(),
  accepted: z.number().int().nonnegative(),
  rejected: z.number().int().nonnegative(),
  superseded: z.number().int().nonnegative(),
});

export const snapshotSchema = z.object({
  project: projectInfoSchema,
  records: z.array(recordSummarySchema),
  tags: z.array(z.string()),
  counts: countsSchema,
});
export type Snapshot = z.infer<typeof snapshotSchema>;

export const diagnosticSeveritySchema = z.enum(['error', 'warning', 'info']);
export const diagnosticSchema = z.object({
  severity: diagnosticSeveritySchema,
  message: z.string(),
  path: z.string().optional(),
  line: z.number().int().optional(),
});
export type Diagnostic = z.infer<typeof diagnosticSchema>;

export function commandEnvelopeSchema<T extends z.ZodTypeAny>(data: T) {
  return z.object({
    command: z.string(),
    ok: z.boolean(),
    data,
    diagnostics: z.array(diagnosticSchema),
    changed_paths: z.array(z.string()),
  });
}
```

If the real snapshot inspected in Step 2 has additional required fields, extend the schemas accordingly. Then rerun the test step (Task 12 includes `bun run test:unit`).

- [ ] **Step 5: Commit**

```bash
git add crates/dadrock/web/package.json crates/dadrock/web/bun.lock crates/dadrock/web/src/lib/api/schemas.ts crates/dadrock/web/tests/unit/schemas.test.ts
git commit -m "zod + snapshot schema (phase 0 task 8)"
```

---

## Task 9 — `DadrApi` interface, `RealApi`, `MockApi`, switching

**Files:**
- Create: `crates/dadrock/web/src/lib/api/types.ts`
- Create: `crates/dadrock/web/src/lib/api/real.ts`
- Create: `crates/dadrock/web/src/lib/api/mock.ts`
- Create: `crates/dadrock/web/src/lib/api/index.ts`
- Create: `crates/dadrock/web/tests/unit/api-real.test.ts`

- [ ] **Step 1: Define the `DadrApi` interface**

`crates/dadrock/web/src/lib/api/types.ts`:

```ts
import type { Snapshot } from './schemas';

export interface DadrApi {
  getSnapshot(): Promise<Snapshot>;
}
```

(Phase 0 only requires `getSnapshot`. Phase 1 and Phase 2 extend the interface as new endpoints come online.)

- [ ] **Step 2: Write a failing test for `RealApi.getSnapshot`**

`crates/dadrock/web/tests/unit/api-real.test.ts`:

```ts
import { describe, it, expect, vi } from 'vitest';
import { RealApi } from '../../src/lib/api/real';

describe('RealApi.getSnapshot', () => {
  it('parses a valid response with the snapshot schema', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(
        JSON.stringify({
          project: { root: '/tmp/proj', decisions_dir: '/tmp/proj/decisions', initialized: true },
          records: [],
          tags: [],
          counts: { candidate: 0, accepted: 0, rejected: 0, superseded: 0 },
        }),
        { headers: { 'content-type': 'application/json' } },
      ),
    );
    const api = new RealApi('http://example.test', fakeFetch as any);
    const snapshot = await api.getSnapshot();
    expect(snapshot.records).toEqual([]);
    expect(fakeFetch).toHaveBeenCalledWith('http://example.test/api/project', expect.any(Object));
  });

  it('throws when the response shape is invalid', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(
      new Response(JSON.stringify({ wrong: 'shape' }), {
        headers: { 'content-type': 'application/json' },
      }),
    );
    const api = new RealApi('http://example.test', fakeFetch as any);
    await expect(api.getSnapshot()).rejects.toThrow();
  });

  it('throws on non-2xx status', async () => {
    const fakeFetch = vi.fn().mockResolvedValue(new Response('boom', { status: 500 }));
    const api = new RealApi('http://example.test', fakeFetch as any);
    await expect(api.getSnapshot()).rejects.toThrow();
  });
});
```

- [ ] **Step 3: Implement `RealApi`**

`crates/dadrock/web/src/lib/api/real.ts`:

```ts
import { snapshotSchema, type Snapshot } from './schemas';
import type { DadrApi } from './types';

type Fetcher = typeof fetch;

export class RealApi implements DadrApi {
  constructor(private readonly baseUrl: string = '', private readonly fetcher: Fetcher = fetch) {}

  async getSnapshot(): Promise<Snapshot> {
    const res = await this.fetcher(`${this.baseUrl}/api/project`, {
      method: 'GET',
      headers: { 'accept': 'application/json' },
    });
    if (!res.ok) {
      throw new Error(`GET /api/project failed with ${res.status}`);
    }
    const json = await res.json();
    return snapshotSchema.parse(json);
  }
}
```

- [ ] **Step 4: Implement `MockApi`**

`crates/dadrock/web/src/lib/api/mock.ts`:

```ts
import type { DadrApi } from './types';
import type { Snapshot } from './schemas';

const fixture: Snapshot = {
  project: { root: '/tmp/dadr-mock', decisions_dir: '/tmp/dadr-mock/decisions', initialized: true },
  records: [
    {
      id: '0193fb1f-a3a0-7000-8000-000000000001',
      title: 'Use Polars over pandas',
      status: 'accepted',
      path: 'decisions/0001-use-polars.md',
      tags: ['#dadr/data/processing'],
      successors: [],
    },
    {
      id: '0193fb1f-a3a0-7000-8000-000000000002',
      title: 'Adopt pre-registration for studies',
      status: 'candidate',
      path: 'decisions/candidates/0002-preregister.md',
      tags: ['#dadr/methodology'],
      successors: [],
    },
  ],
  tags: ['#dadr/data/processing', '#dadr/methodology'],
  counts: { candidate: 1, accepted: 1, rejected: 0, superseded: 0 },
};

export class MockApi implements DadrApi {
  async getSnapshot(): Promise<Snapshot> {
    return JSON.parse(JSON.stringify(fixture));
  }
}
```

- [ ] **Step 5: Implement the API selector**

`crates/dadrock/web/src/lib/api/index.ts`:

```ts
import { MockApi } from './mock';
import { RealApi } from './real';
import type { DadrApi } from './types';

export type { DadrApi } from './types';
export type * from './schemas';

export function createApi(): DadrApi {
  const useMock = import.meta.env.VITE_DADR_MOCK === '1';
  if (useMock) {
    return new MockApi();
  }
  return new RealApi();
}
```

- [ ] **Step 6: Commit**

```bash
git add crates/dadrock/web/src/lib/api/types.ts crates/dadrock/web/src/lib/api/real.ts crates/dadrock/web/src/lib/api/mock.ts crates/dadrock/web/src/lib/api/index.ts crates/dadrock/web/tests/unit/api-real.test.ts
git commit -m "DadrApi boundary with real + mock (phase 0 task 9)"
```

---

## Task 10 — Sidebar collapse store + Toast queue store

**Files:**
- Create: `crates/dadrock/web/src/lib/stores/sidebar.svelte.ts`
- Create: `crates/dadrock/web/src/lib/stores/toast.svelte.ts`
- Create: `crates/dadrock/web/tests/unit/stores.test.ts`

- [ ] **Step 1: Write failing tests**

`crates/dadrock/web/tests/unit/stores.test.ts`:

```ts
import { describe, it, expect, beforeEach } from 'vitest';
import { sidebar } from '../../src/lib/stores/sidebar.svelte';
import { toasts } from '../../src/lib/stores/toast.svelte';

describe('sidebar store', () => {
  beforeEach(() => localStorage.clear());

  it('defaults to expanded', () => {
    expect(sidebar.collapsed).toBe(false);
  });

  it('toggles and persists', () => {
    sidebar.toggle();
    expect(sidebar.collapsed).toBe(true);
    expect(localStorage.getItem('dadr.sidebar')).toBe('collapsed');
  });
});

describe('toasts store', () => {
  beforeEach(() => {
    toasts.list.length = 0;
  });

  it('adds and removes toasts', () => {
    const id = toasts.push({ severity: 'info', message: 'hello' });
    expect(toasts.list.length).toBe(1);
    toasts.dismiss(id);
    expect(toasts.list.length).toBe(0);
  });

  it('assigns unique ids', () => {
    const a = toasts.push({ severity: 'info', message: 'a' });
    const b = toasts.push({ severity: 'info', message: 'b' });
    expect(a).not.toBe(b);
  });
});
```

- [ ] **Step 2: Implement the sidebar store**

`crates/dadrock/web/src/lib/stores/sidebar.svelte.ts`:

```ts
const KEY = 'dadr.sidebar';

function read(): boolean {
  if (typeof localStorage === 'undefined') return false;
  return localStorage.getItem(KEY) === 'collapsed';
}

class SidebarStore {
  collapsed = $state<boolean>(read());

  toggle() {
    this.collapsed = !this.collapsed;
    if (typeof localStorage !== 'undefined') {
      localStorage.setItem(KEY, this.collapsed ? 'collapsed' : 'expanded');
    }
  }
}

export const sidebar = new SidebarStore();
```

- [ ] **Step 3: Implement the toasts store**

`crates/dadrock/web/src/lib/stores/toast.svelte.ts`:

```ts
export type ToastSeverity = 'info' | 'success' | 'warning' | 'error';

export interface Toast {
  id: number;
  severity: ToastSeverity;
  message: string;
  detail?: string;
}

let nextId = 1;

class ToastStore {
  list = $state<Toast[]>([]);

  push(toast: Omit<Toast, 'id'>): number {
    const id = nextId++;
    this.list.push({ id, ...toast });
    return id;
  }

  dismiss(id: number) {
    const idx = this.list.findIndex((t) => t.id === id);
    if (idx !== -1) this.list.splice(idx, 1);
  }
}

export const toasts = new ToastStore();
```

- [ ] **Step 4: Commit**

```bash
git add crates/dadrock/web/src/lib/stores/sidebar.svelte.ts crates/dadrock/web/src/lib/stores/toast.svelte.ts crates/dadrock/web/tests/unit/stores.test.ts
git commit -m "sidebar + toast stores (phase 0 task 10)"
```

---

## Task 11 — Shell components: Header, Sidebar, ToastRegion, CommandPalette skeleton

**Files:**
- Create: `crates/dadrock/web/src/lib/components/shell/Header.svelte`
- Create: `crates/dadrock/web/src/lib/components/shell/Sidebar.svelte`
- Create: `crates/dadrock/web/src/lib/components/shell/ToastRegion.svelte`
- Create: `crates/dadrock/web/src/lib/components/shell/CommandPalette.svelte`
- Create: `crates/dadrock/web/src/lib/stores/palette.svelte.ts`

- [ ] **Step 1: Create the command-palette open/close store**

`crates/dadrock/web/src/lib/stores/palette.svelte.ts`:

```ts
class PaletteStore {
  open = $state(false);

  toggle() { this.open = !this.open; }
  show() { this.open = true; }
  hide() { this.open = false; }
}

export const palette = new PaletteStore();
```

- [ ] **Step 2: Create `Header.svelte`**

`crates/dadrock/web/src/lib/components/shell/Header.svelte`:

```svelte
<script lang="ts">
  import { Search, Sidebar as SidebarIcon } from 'lucide-svelte';
  import ThemeToggle from './ThemeToggle.svelte';
  import { sidebar } from '$lib/stores/sidebar.svelte';
  import { palette } from '$lib/stores/palette.svelte';
</script>

<header class="h-12 flex items-center px-3 gap-3 border-b border-border bg-bg">
  <button
    type="button"
    class="inline-flex items-center justify-center h-8 w-8 rounded-md hover:bg-surface-2 text-fg-muted hover:text-fg transition-colors"
    aria-label="Toggle sidebar"
    onclick={() => sidebar.toggle()}
  >
    <SidebarIcon class="h-4 w-4" />
  </button>

  <div class="text-sm font-medium text-fg">dadrock</div>

  <button
    type="button"
    class="ml-4 flex items-center gap-2 h-8 px-3 rounded-md bg-surface hover:bg-surface-2 border border-border text-sm text-fg-muted hover:text-fg transition-colors min-w-64"
    aria-label="Open command palette"
    onclick={() => palette.show()}
  >
    <Search class="h-3.5 w-3.5" />
    <span>Search…</span>
    <kbd class="ml-auto text-[10px] font-mono px-1.5 py-0.5 rounded bg-surface-2 text-fg-subtle">⌘K</kbd>
  </button>

  <div class="flex-1"></div>

  <ThemeToggle />
</header>
```

- [ ] **Step 3: Create `Sidebar.svelte`**

`crates/dadrock/web/src/lib/components/shell/Sidebar.svelte`:

```svelte
<script lang="ts">
  import { page } from '$app/state';
  import { sidebar } from '$lib/stores/sidebar.svelte';
  import { Inbox, FileText, Tag, ShieldCheck, Link as LinkIcon, History, Settings } from 'lucide-svelte';

  const items = [
    { href: '/inbox', label: 'Inbox', icon: Inbox },
    { href: '/records', label: 'Records', icon: FileText },
    { href: '/tags', label: 'Tags', icon: Tag },
    { href: '/validation', label: 'Validation', icon: ShieldCheck },
    { href: '/links', label: 'Links', icon: LinkIcon },
    { href: '/audit', label: 'Audit', icon: History },
  ] as const;

  function isActive(href: string) {
    return page.url.pathname === href || page.url.pathname.startsWith(href + '/');
  }
</script>

<aside
  class="border-r border-border bg-surface transition-[width] duration-150 overflow-hidden"
  class:w-56={!sidebar.collapsed}
  class:w-12={sidebar.collapsed}
>
  <nav class="flex flex-col gap-0.5 p-2">
    {#each items as item}
      <a
        href={item.href}
        class="flex items-center gap-2 h-8 px-2 rounded-md text-sm transition-colors text-fg-muted hover:bg-surface-2 hover:text-fg"
        class:bg-surface-2={isActive(item.href)}
        class:!text-fg={isActive(item.href)}
      >
        <item.icon class="h-4 w-4 shrink-0" />
        {#if !sidebar.collapsed}
          <span>{item.label}</span>
        {/if}
      </a>
    {/each}
  </nav>

  <div class="mt-auto p-2 border-t border-border">
    <a
      href="/settings"
      class="flex items-center gap-2 h-8 px-2 rounded-md text-sm text-fg-muted hover:bg-surface-2 hover:text-fg transition-colors"
    >
      <Settings class="h-4 w-4 shrink-0" />
      {#if !sidebar.collapsed}
        <span>Settings</span>
      {/if}
    </a>
  </div>
</aside>
```

- [ ] **Step 4: Create `ToastRegion.svelte`**

`crates/dadrock/web/src/lib/components/shell/ToastRegion.svelte`:

```svelte
<script lang="ts">
  import { toasts, type ToastSeverity } from '$lib/stores/toast.svelte';
  import { CheckCircle2, Info, AlertTriangle, XCircle, X } from 'lucide-svelte';

  const ICONS: Record<ToastSeverity, typeof Info> = {
    info: Info,
    success: CheckCircle2,
    warning: AlertTriangle,
    error: XCircle,
  };

  const RING: Record<ToastSeverity, string> = {
    info: 'border-border',
    success: 'border-success/40',
    warning: 'border-warning/50',
    error: 'border-danger/50',
  };
</script>

<div class="fixed bottom-4 right-4 flex flex-col gap-2 z-50 pointer-events-none">
  {#each toasts.list as toast (toast.id)}
    {@const Icon = ICONS[toast.severity]}
    <div
      role="status"
      class="pointer-events-auto flex items-start gap-2 px-3 py-2 rounded-lg bg-bg border {RING[toast.severity]} shadow-lg max-w-sm"
    >
      <Icon class="h-4 w-4 mt-0.5 shrink-0 text-fg-muted" />
      <div class="flex-1 text-sm text-fg">
        <div>{toast.message}</div>
        {#if toast.detail}
          <div class="text-xs text-fg-muted mt-0.5">{toast.detail}</div>
        {/if}
      </div>
      <button
        type="button"
        class="text-fg-subtle hover:text-fg p-1 -m-1"
        aria-label="Dismiss"
        onclick={() => toasts.dismiss(toast.id)}
      >
        <X class="h-3.5 w-3.5" />
      </button>
    </div>
  {/each}
</div>
```

- [ ] **Step 5: Create `CommandPalette.svelte` (skeleton)**

`crates/dadrock/web/src/lib/components/shell/CommandPalette.svelte`:

```svelte
<script lang="ts">
  import { palette } from '$lib/stores/palette.svelte';
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';

  const navItems = [
    { href: '/inbox', label: 'Go to Inbox' },
    { href: '/records', label: 'Go to Records' },
    { href: '/tags', label: 'Go to Tags' },
    { href: '/validation', label: 'Go to Validation' },
    { href: '/links', label: 'Go to Links' },
    { href: '/audit', label: 'Go to Audit' },
    { href: '/settings', label: 'Go to Settings' },
  ];

  let query = $state('');
  let inputEl: HTMLInputElement | undefined;

  $effect(() => {
    if (palette.open && inputEl) {
      queueMicrotask(() => inputEl?.focus());
    }
    if (!palette.open) {
      query = '';
    }
  });

  function handleKey(event: KeyboardEvent) {
    if ((event.metaKey || event.ctrlKey) && event.key === 'k') {
      event.preventDefault();
      palette.toggle();
    }
    if (event.key === 'Escape' && palette.open) {
      palette.hide();
    }
  }

  onMount(() => {
    window.addEventListener('keydown', handleKey);
    return () => window.removeEventListener('keydown', handleKey);
  });

  const filtered = $derived(
    navItems.filter((item) => item.label.toLowerCase().includes(query.toLowerCase())),
  );

  function pick(href: string) {
    palette.hide();
    goto(href);
  }
</script>

{#if palette.open}
  <div
    class="fixed inset-0 z-50 bg-fg/30 backdrop-blur-sm flex items-start justify-center pt-32"
    role="presentation"
    onclick={() => palette.hide()}
  >
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      class="w-full max-w-md bg-bg rounded-xl border border-border shadow-2xl overflow-hidden"
      onclick={(e) => e.stopPropagation()}
    >
      <input
        bind:this={inputEl}
        bind:value={query}
        type="text"
        placeholder="Search commands…"
        class="w-full h-12 px-4 bg-bg border-b border-border text-sm text-fg placeholder:text-fg-subtle focus:outline-none"
      />
      <ul class="max-h-72 overflow-y-auto py-1">
        {#each filtered as item (item.href)}
          <li>
            <button
              type="button"
              class="w-full text-left px-4 py-2 text-sm text-fg hover:bg-surface-2 transition-colors"
              onclick={() => pick(item.href)}
            >{item.label}</button>
          </li>
        {/each}
        {#if filtered.length === 0}
          <li class="px-4 py-3 text-sm text-fg-subtle">No matches.</li>
        {/if}
      </ul>
    </div>
  </div>
{/if}
```

(Phase 1 replaces this skeleton with a Bits UI `Command` primitive, fuzzy search, three sections, and recent-items support. For Phase 0 we just need ⌘K to open something.)

- [ ] **Step 6: Commit**

```bash
git add crates/dadrock/web/src/lib/stores/palette.svelte.ts crates/dadrock/web/src/lib/components/shell/Header.svelte crates/dadrock/web/src/lib/components/shell/Sidebar.svelte crates/dadrock/web/src/lib/components/shell/ToastRegion.svelte crates/dadrock/web/src/lib/components/shell/CommandPalette.svelte
git commit -m "shell components: header sidebar toasts palette skeleton (phase 0 task 11)"
```

---

## Task 12 — Vitest setup + run all unit tests

**Files:**
- Modify: `crates/dadrock/web/package.json`
- Create: `crates/dadrock/web/vitest.config.ts`
- Create: `crates/dadrock/web/tests/setup.ts`

- [ ] **Step 1: Install Vitest and Testing Library**

```bash
cd crates/dadrock/web
bun add -d vitest @vitest/ui jsdom @testing-library/svelte @testing-library/jest-dom
```

- [ ] **Step 2: Configure Vitest**

`crates/dadrock/web/vitest.config.ts`:

```ts
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  plugins: [sveltekit()],
  test: {
    environment: 'jsdom',
    setupFiles: ['./tests/setup.ts'],
    include: ['tests/unit/**/*.test.ts'],
  },
});
```

- [ ] **Step 3: Configure the test environment**

`crates/dadrock/web/tests/setup.ts`:

```ts
import '@testing-library/jest-dom/vitest';

if (typeof window !== 'undefined' && typeof window.matchMedia !== 'function') {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: () => {},
      removeEventListener: () => {},
      addListener: () => {},
      removeListener: () => {},
      dispatchEvent: () => false,
    }),
  });
}
```

- [ ] **Step 4: Run all unit tests**

```bash
bun run test:unit
```

Expected: all tests from Tasks 4, 8, 9, 10 pass. If any fail because of missing imports or type errors introduced by their cross-task references, fix inline before committing.

- [ ] **Step 5: Commit**

```bash
git add crates/dadrock/web/package.json crates/dadrock/web/bun.lock crates/dadrock/web/vitest.config.ts crates/dadrock/web/tests/setup.ts
git commit -m "vitest setup + unit tests passing (phase 0 task 12)"
```

---

## Task 13 — ESLint + svelte-check

**Files:**
- Modify: `crates/dadrock/web/package.json`
- Create: `crates/dadrock/web/eslint.config.js`

- [ ] **Step 1: Install ESLint and Svelte plugin**

```bash
cd crates/dadrock/web
bun add -d eslint @eslint/js eslint-plugin-svelte typescript-eslint globals
```

- [ ] **Step 2: Create flat ESLint config**

`crates/dadrock/web/eslint.config.js`:

```js
import js from '@eslint/js';
import ts from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import globals from 'globals';

export default [
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs['flat/recommended'],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
      },
    },
  },
  {
    ignores: ['build/', '.svelte-kit/', 'dist/', 'node_modules/'],
  },
];
```

- [ ] **Step 3: Run lint and svelte-check**

```bash
bun run lint
bun run check
```

Expected: both succeed with no errors. Warnings are acceptable. If there are real errors (not warnings), fix inline.

- [ ] **Step 4: Commit**

```bash
git add crates/dadrock/web/package.json crates/dadrock/web/bun.lock crates/dadrock/web/eslint.config.js
git commit -m "eslint + svelte-check (phase 0 task 13)"
```

---

## Task 14 — Root layout, error route, and 10 placeholder routes

**Files:**
- Create: `crates/dadrock/web/src/routes/+layout.svelte`
- Create: `crates/dadrock/web/src/routes/+error.svelte`
- Modify: `crates/dadrock/web/src/routes/+page.svelte` (replace placeholder with redirect to `/inbox`)
- Create: `crates/dadrock/web/src/routes/inbox/+page.svelte`
- Create: `crates/dadrock/web/src/routes/records/+page.svelte`
- Create: `crates/dadrock/web/src/routes/records/[id]/+page.svelte`
- Create: `crates/dadrock/web/src/routes/candidates/[id]/+page.svelte`
- Create: `crates/dadrock/web/src/routes/tags/+page.svelte`
- Create: `crates/dadrock/web/src/routes/validation/+page.svelte`
- Create: `crates/dadrock/web/src/routes/links/+page.svelte`
- Create: `crates/dadrock/web/src/routes/audit/+page.svelte`
- Create: `crates/dadrock/web/src/routes/settings/+page.svelte`

- [ ] **Step 1: Create the root `+layout.svelte`**

`crates/dadrock/web/src/routes/+layout.svelte`:

```svelte
<script lang="ts">
  import '../app.css';
  import { QueryClientProvider } from '@tanstack/svelte-query';
  import Header from '$lib/components/shell/Header.svelte';
  import Sidebar from '$lib/components/shell/Sidebar.svelte';
  import ToastRegion from '$lib/components/shell/ToastRegion.svelte';
  import CommandPalette from '$lib/components/shell/CommandPalette.svelte';
  import type { Snippet } from 'svelte';

  let { data, children }: { data: { queryClient: import('@tanstack/svelte-query').QueryClient | null }; children: Snippet } = $props();
</script>

{#if data.queryClient}
  <QueryClientProvider client={data.queryClient}>
    <div class="h-screen flex flex-col bg-bg text-fg">
      <Header />
      <div class="flex-1 flex overflow-hidden">
        <Sidebar />
        <main class="flex-1 overflow-auto">
          {@render children()}
        </main>
      </div>
    </div>
    <ToastRegion />
    <CommandPalette />
  </QueryClientProvider>
{/if}
```

- [ ] **Step 2: Create `+error.svelte`**

`crates/dadrock/web/src/routes/+error.svelte`:

```svelte
<script lang="ts">
  import { page } from '$app/state';
</script>

<div class="min-h-full flex items-center justify-center p-12">
  <div class="text-center max-w-md">
    <div class="text-5xl font-mono text-fg-subtle">{page.status}</div>
    <h1 class="mt-4 text-xl font-semibold">{page.error?.message ?? 'Something went wrong.'}</h1>
    <p class="mt-2 text-sm text-fg-muted">The page you requested could not be loaded.</p>
    <a href="/records" class="mt-6 inline-flex items-center px-3 h-9 rounded-md bg-accent text-accent-fg text-sm">Back to records</a>
  </div>
</div>
```

- [ ] **Step 3: Replace the root `+page.svelte` with a redirect**

`crates/dadrock/web/src/routes/+page.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { goto } from '$app/navigation';

  onMount(() => {
    goto('/inbox', { replaceState: true });
  });
</script>

<div class="min-h-full flex items-center justify-center text-fg-muted text-sm">
  Loading…
</div>
```

(Phase 2 changes the default landing route based on inbox content; for Phase 0 we hard-redirect to `/inbox`.)

- [ ] **Step 4: Create the placeholder pages**

Each placeholder uses the same shape — title, "coming in Phase N" line. Write each file exactly as below.

`crates/dadrock/web/src/routes/inbox/+page.svelte`:

```svelte
<div class="p-8">
  <h1 class="text-2xl font-semibold">Inbox</h1>
  <p class="mt-2 text-sm text-fg-muted">Candidates by confidence, age, and scope. Coming in Phase 2.</p>
</div>
```

`crates/dadrock/web/src/routes/records/+page.svelte`:

```svelte
<div class="p-8">
  <h1 class="text-2xl font-semibold">Records</h1>
  <p class="mt-2 text-sm text-fg-muted">Records list. Phase 0 fetches from the real API; Phase 1 ships the polished UI.</p>
</div>
```

(Task 15 replaces this placeholder with the real round-trip implementation.)

`crates/dadrock/web/src/routes/records/[id]/+page.svelte`:

```svelte
<script lang="ts">
  import { page } from '$app/state';
</script>
<div class="p-8">
  <h1 class="text-2xl font-semibold">Record {page.params.id}</h1>
  <p class="mt-2 text-sm text-fg-muted">Detail view coming in Phase 1.</p>
</div>
```

`crates/dadrock/web/src/routes/candidates/[id]/+page.svelte`:

```svelte
<script lang="ts">
  import { page } from '$app/state';
</script>
<div class="p-8">
  <h1 class="text-2xl font-semibold">Candidate {page.params.id}</h1>
  <p class="mt-2 text-sm text-fg-muted">Review screen coming in Phase 2.</p>
</div>
```

`crates/dadrock/web/src/routes/tags/+page.svelte`:

```svelte
<div class="p-8">
  <h1 class="text-2xl font-semibold">Tags</h1>
  <p class="mt-2 text-sm text-fg-muted">Tag tree. Coming in Phase 1.</p>
</div>
```

`crates/dadrock/web/src/routes/validation/+page.svelte`:

```svelte
<div class="p-8">
  <h1 class="text-2xl font-semibold">Validation</h1>
  <p class="mt-2 text-sm text-fg-muted">Project, records, links, and tag diagnostics. Coming in Phase 1.</p>
</div>
```

`crates/dadrock/web/src/routes/links/+page.svelte`:

```svelte
<div class="p-8">
  <h1 class="text-2xl font-semibold">Links</h1>
  <p class="mt-2 text-sm text-fg-muted">Link health queue. Coming in Phase 2.</p>
</div>
```

`crates/dadrock/web/src/routes/audit/+page.svelte`:

```svelte
<div class="p-8">
  <h1 class="text-2xl font-semibold">Audit trail</h1>
  <p class="mt-2 text-sm text-fg-muted">Coming in Phase 2.</p>
</div>
```

`crates/dadrock/web/src/routes/settings/+page.svelte`:

```svelte
<div class="p-8">
  <h1 class="text-2xl font-semibold">Settings</h1>
  <p class="mt-2 text-sm text-fg-muted">Coming in Phase 1.</p>
</div>
```

- [ ] **Step 5: Verify dev server**

```bash
cargo run -p dadrock -- preview &
DADR_PREVIEW_URL=http://localhost:<port> bun run dev
```

Open `http://localhost:5173` (no `/v2/` prefix in dev because the SvelteKit dev server serves the SPA root). Confirm:

- The root URL redirects to `/inbox`.
- All sidebar links navigate; placeholder content renders.
- ⌘K opens the palette; selecting a navigation item routes to it.
- Theme toggle cycles light/dark/system; sidebar collapse persists across reload.
- An unknown URL like `/nope` shows the error route.

Stop both servers.

- [ ] **Step 6: Commit**

```bash
git add crates/dadrock/web/src/routes
git commit -m "root layout + error route + 10 route placeholders (phase 0 task 14)"
```

---

## Task 15 — Real round-trip: `/records` lists records via `/api/project`

**Files:**
- Modify: `crates/dadrock/web/src/routes/records/+page.svelte`

- [ ] **Step 1: Replace the placeholder with a real query**

`crates/dadrock/web/src/routes/records/+page.svelte`:

```svelte
<script lang="ts">
  import { createQuery } from '@tanstack/svelte-query';
  import { createApi } from '$lib/api';
  import type { RecordSummary } from '$lib/api';

  const api = createApi();

  const snapshot = createQuery({
    queryKey: ['snapshot'],
    queryFn: () => api.getSnapshot(),
  });

  const STATUS_BADGE: Record<RecordSummary['status'], string> = {
    candidate: 'bg-candidate/20 text-candidate',
    accepted: 'bg-accepted/20 text-accepted',
    rejected: 'bg-rejected/20 text-rejected',
    superseded: 'bg-superseded/20 text-superseded',
  };
</script>

<div class="p-8 max-w-3xl">
  <h1 class="text-2xl font-semibold">Records</h1>
  <p class="mt-1 text-sm text-fg-muted">Phase 0: bare list, real API round-trip.</p>

  {#if $snapshot.isPending}
    <p class="mt-6 text-sm text-fg-muted">Loading…</p>
  {:else if $snapshot.isError}
    <div class="mt-6 p-4 rounded-md border border-danger/40 bg-danger/5 text-sm">
      <div class="font-medium text-danger">Failed to load snapshot</div>
      <div class="mt-1 text-fg-muted">{$snapshot.error.message}</div>
    </div>
  {:else}
    <ul class="mt-6 divide-y divide-border border border-border rounded-md overflow-hidden">
      {#each $snapshot.data.records as record (record.id)}
        <li class="flex items-center gap-3 px-3 py-2 bg-bg hover:bg-surface transition-colors">
          <span class="text-[10px] font-mono uppercase px-1.5 py-0.5 rounded {STATUS_BADGE[record.status]}">
            {record.status.slice(0, 3)}
          </span>
          <a class="text-sm text-fg hover:underline" href="/records/{record.id}">{record.title}</a>
        </li>
      {:else}
        <li class="px-3 py-4 text-sm text-fg-subtle">No records yet.</li>
      {/each}
    </ul>
    <p class="mt-4 text-xs text-fg-subtle">
      {$snapshot.data.counts.accepted} accepted · {$snapshot.data.counts.candidate} candidates ·
      {$snapshot.data.counts.rejected} rejected · {$snapshot.data.counts.superseded} superseded
    </p>
  {/if}
</div>
```

- [ ] **Step 2: Verify the round-trip**

In one terminal:

```bash
cargo run -p dadrock -- preview
# Note the port; e.g., 8765.
```

In another:

```bash
cd crates/dadrock/web
DADR_PREVIEW_URL=http://localhost:8765 bun run dev
```

Open `http://localhost:5173/records`. Expected: records list populates from the real Rust server. Refreshing should refetch silently. With DevTools Network tab open, confirm `/api/project` returned 200 with the snapshot JSON.

If the schema rejected the response, extend `snapshotSchema` in `lib/api/schemas.ts` to match the real shape, then rerun `bun run test:unit`.

- [ ] **Step 3: Commit**

```bash
git add crates/dadrock/web/src/routes/records/+page.svelte
git commit -m "records page real round-trip via /api/project (phase 0 task 15)"
```

---

## Task 16 — Playwright E2E setup + smoke test

**Files:**
- Modify: `crates/dadrock/web/package.json`
- Create: `crates/dadrock/web/playwright.config.ts`
- Create: `crates/dadrock/web/tests/e2e/smoke.spec.ts`

- [ ] **Step 1: Install Playwright**

```bash
cd crates/dadrock/web
bun add -d @playwright/test
bunx playwright install chromium
```

- [ ] **Step 2: Configure Playwright**

`crates/dadrock/web/playwright.config.ts`:

```ts
import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests/e2e',
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: 'list',
  use: {
    baseURL: process.env.E2E_BASE_URL ?? 'http://localhost:5173',
    trace: 'retain-on-failure',
  },
  webServer: process.env.E2E_BASE_URL
    ? undefined
    : {
        command: 'bun run dev',
        url: 'http://localhost:5173',
        timeout: 60_000,
        reuseExistingServer: !process.env.CI,
      },
});
```

- [ ] **Step 3: Write the smoke test**

`crates/dadrock/web/tests/e2e/smoke.spec.ts`:

```ts
import { test, expect } from '@playwright/test';

test('default route redirects to inbox and chrome renders', async ({ page }) => {
  await page.goto('/');
  await expect(page).toHaveURL(/\/inbox$/);
  await expect(page.getByRole('heading', { name: 'Inbox' })).toBeVisible();
  await expect(page.getByLabel('Toggle sidebar')).toBeVisible();
  await expect(page.getByLabel(/theme/i)).toBeVisible();
});

test('command palette opens with Cmd+K and navigates', async ({ page, browserName }) => {
  await page.goto('/inbox');
  const modifier = browserName === 'webkit' ? 'Meta' : 'Control';
  await page.keyboard.press(`${modifier}+K`);
  await expect(page.getByRole('dialog', { name: 'Command palette' })).toBeVisible();
  await page.getByPlaceholder('Search commands…').fill('records');
  await page.getByRole('button', { name: 'Go to Records' }).click();
  await expect(page).toHaveURL(/\/records$/);
});

test('theme toggle cycles and persists', async ({ page }) => {
  await page.goto('/inbox');
  const html = page.locator('html');
  // Default state — system.
  await expect(html).toHaveAttribute('data-theme', 'system');
  await page.getByLabel(/theme/i).click(); // → light
  await expect(html).toHaveAttribute('data-theme', 'light');
  await page.reload();
  await expect(html).toHaveAttribute('data-theme', 'light');
});
```

- [ ] **Step 4: Run E2E**

The Vite dev server must be able to reach the dadrock API. Either start `dadrock preview` first (and set `DADR_PREVIEW_URL`), or accept that the records-page test isn't in this smoke set (these three tests only touch shell/navigation, not the real API).

```bash
bun run test:e2e
```

Expected: 3 passed. If a test fails because of a selector mismatch, fix the selector to match the real DOM rather than the test (the components are the source of truth).

- [ ] **Step 5: Commit**

```bash
git add crates/dadrock/web/package.json crates/dadrock/web/bun.lock crates/dadrock/web/playwright.config.ts crates/dadrock/web/tests/e2e/smoke.spec.ts
git commit -m "playwright smoke tests (phase 0 task 16)"
```

---

## Task 17 — `build.rs` + `rust-embed` integration

**Files:**
- Modify: `crates/dadrock/Cargo.toml` (add `rust-embed` dep, build deps, build script)
- Create: `crates/dadrock/build.rs`

- [ ] **Step 1: Add `rust-embed` to runtime dependencies**

Modify `crates/dadrock/Cargo.toml`. Add to `[dependencies]`:

```toml
rust-embed = { version = "8", features = ["compression"] }
mime_guess = "2"
```

And add a build-dependency section and the build-script declaration:

```toml
[build-dependencies]
walkdir = "2"

[package]
name = "dadrock"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
build = "build.rs"
```

(Add `build = "build.rs"` to the existing `[package]` section — do not duplicate the section header.)

- [ ] **Step 2: Write `build.rs`**

`crates/dadrock/build.rs`:

```rust
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let web_dir = manifest_dir.join("web");

    if !web_dir.exists() {
        // No web/ directory yet (e.g., very old branch). Skip the build.
        println!("cargo:warning=crates/dadrock/web does not exist; skipping JS build");
        return;
    }

    // Tell Cargo to rerun if any source file under web/src/ changes,
    // or if package.json / lockfile / config change.
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/bun.lock");
    println!("cargo:rerun-if-changed=web/svelte.config.js");
    println!("cargo:rerun-if-changed=web/vite.config.ts");
    println!("cargo:rerun-if-changed=web/tsconfig.json");

    for entry in WalkDir::new(web_dir.join("src"))
        .into_iter()
        .filter_map(Result::ok)
    {
        if entry.file_type().is_file() {
            println!("cargo:rerun-if-changed={}", entry.path().display());
        }
    }

    // Skip on `cargo check` to keep dev iteration fast.
    if env::var("CARGO_FEATURE_NO_JS_BUILD").is_ok() {
        println!("cargo:warning=skipping JS build (CARGO_FEATURE_NO_JS_BUILD set)");
        return;
    }

    // Verify Bun is available.
    let bun = which("bun");
    if bun.is_none() {
        panic!(
            "bun not found on PATH. Install Bun (https://bun.sh) and retry. \
             Or set CARGO_FEATURE_NO_JS_BUILD=1 to skip the JS build."
        );
    }

    // Install if node_modules missing.
    if !web_dir.join("node_modules").exists() {
        run(&web_dir, "bun", &["install", "--frozen-lockfile"]);
    }

    // Build.
    run(&web_dir, "bun", &["run", "build"]);
}

fn run(cwd: &Path, program: &str, args: &[&str]) {
    let status = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap_or_else(|err| panic!("failed to spawn `{program} {}`: {err}", args.join(" ")));
    if !status.success() {
        panic!("`{program} {}` failed with {status}", args.join(" "));
    }
}

fn which(program: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let candidate = dir.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}
```

- [ ] **Step 3: Verify `cargo build` runs the JS build transitively**

```bash
cargo clean -p dadrock
cargo build -p dadrock
```

Expected: build output includes Bun running install (if needed) and Vite build, then Rust compile. `crates/dadrock/web/build/index.html` exists afterward.

- [ ] **Step 4: Commit**

```bash
git add crates/dadrock/Cargo.toml crates/dadrock/build.rs Cargo.lock
git commit -m "build.rs invokes bun build (phase 0 task 17)"
```

---

## Task 18 — Embed `web/build/` and serve at `/v2/*`

**Files:**
- Modify: `crates/dadrock/src/web.rs`

- [ ] **Step 1: Add the `RustEmbed` struct and `/v2/*` route**

Modify `crates/dadrock/src/web.rs`. Add near the top of the file, just after the existing `use` block (around line 33):

```rust
use rust_embed::RustEmbed;
```

After the `APP_JS` constant declaration (around line 37), add:

```rust
#[derive(RustEmbed)]
#[folder = "web/build/"]
#[prefix = ""]
struct WebV2;

fn web_v2_response(path: &str) -> Option<HttpResponse> {
    let trimmed = path.trim_start_matches('/');
    let asset_path = if trimmed.is_empty() || !trimmed.contains('.') {
        // SPA fallback: anything that doesn't look like a file → index.html.
        "index.html"
    } else {
        trimmed
    };
    let file = WebV2::get(asset_path)?;
    let mime = mime_guess::from_path(asset_path).first_or_octet_stream();
    let mime_str = format!("{mime}; charset=utf-8");
    Some(static_response(200, &mime_str, file.data.as_ref()))
}
```

- [ ] **Step 2: Wire the route in `route_request`**

Locate the `route_request` function (around line 100) and add a new arm for `/v2/*` **above** the existing 404 fallback. The match starts:

```rust
match (request.method.as_str(), path) {
    ("GET", "/") | ("GET", "/index.html") => Ok(static_response(...)),
    // ... existing arms ...
```

Add this arm just before the catch-all `("GET", record_path) => { ... }` arm (so dynamic record-path matching does not swallow `/v2/*`):

```rust
        ("GET", path) if path.starts_with("/v2") => {
            let suffix = &path["/v2".len()..];
            web_v2_response(suffix)
                .map(Ok)
                .unwrap_or_else(|| json_response(404, &ApiError::not_found()))
        }
```

The complete top of the match should now read (only the new arm added; no existing arms changed):

```rust
match (request.method.as_str(), path) {
    ("GET", "/") | ("GET", "/index.html") => Ok(static_response(
        200,
        "text/html; charset=utf-8",
        INDEX_HTML.as_bytes(),
    )),
    ("GET", "/assets/app.css") => Ok(static_response(
        200,
        "text/css; charset=utf-8",
        APP_CSS.as_bytes(),
    )),
    ("GET", "/assets/app.js") => Ok(static_response(
        200,
        "text/javascript; charset=utf-8",
        APP_JS.as_bytes(),
    )),
    ("GET", path) if path.starts_with("/v2") => {
        let suffix = &path["/v2".len()..];
        web_v2_response(suffix)
            .map(Ok)
            .unwrap_or_else(|| json_response(404, &ApiError::not_found()))
    }
    ("GET", "/api/project") => json_response(200, &build_snapshot(cwd)?),
    // ... rest unchanged ...
```

- [ ] **Step 3: Verify the embedded SPA serves at `/v2/`**

```bash
cargo build -p dadrock
cargo run -p dadrock -- preview
# Note the port; e.g., 8765.
```

In a browser:

- `http://localhost:8765/` — current vanilla JS UI loads (unchanged).
- `http://localhost:8765/v2/` — the SvelteKit SPA loads, redirects to `/v2/inbox`, renders the inbox placeholder with header + sidebar + theme toggle.
- `http://localhost:8765/v2/records` — records list populates from the **same** Rust process's `/api/project`.
- `http://localhost:8765/v2/some-deep/non/existent/path` — SvelteKit's SPA fallback shows the `+error.svelte` page.
- `http://localhost:8765/v2/_app/immutable/<some-asset>.js` — returns the asset (200, correct MIME).

Stop the server.

- [ ] **Step 4: Commit**

```bash
git add crates/dadrock/src/web.rs
git commit -m "embed svelte build at /v2/* (phase 0 task 18)"
```

---

## Task 19 — Web README (dev workflow doc)

**Files:**
- Create: `crates/dadrock/web/README.md`

- [ ] **Step 1: Write the README**

`crates/dadrock/web/README.md`:

```markdown
# dadrock web (Svelte)

The polished Svelte web UI for dadrock. Embedded into the dadrock binary at build time and currently served at `/v2/*` alongside the legacy vanilla-JS UI at `/`. Phase 1 will cut over and remove the legacy UI.

## Stack

Svelte 5 · SvelteKit (`adapter-static`, SPA mode) · TypeScript (strict) · Tailwind v4 · Bits UI · shadcn-svelte recipes · Lucide · TanStack Query · Zod · Vitest · Playwright · Bun.

## Prerequisites

- Bun 1.1+ (`curl -fsSL https://bun.sh/install | bash`)
- Rust 1.75+ (for the parent dadrock crate)

## Dev workflow

In one terminal, run the Rust API server:

```bash
cargo run -p dadrock -- preview
# Note the port shown (e.g., 8765).
```

In another terminal, run the Vite dev server proxying `/api/*` to the Rust server:

```bash
cd crates/dadrock/web
DADR_PREVIEW_URL=http://localhost:8765 bun run dev
```

Open http://localhost:5173. Edits hot-reload. The browser does not need to know about `/v2/`; the dev server serves the SPA at the root.

## Production build

The Rust `build.rs` invokes `bun install --frozen-lockfile && bun run build` automatically when `cargo build -p dadrock` runs. The output (`crates/dadrock/web/build/`) is embedded into the binary via `rust-embed`.

To skip the JS build during fast Rust iteration:

```bash
CARGO_FEATURE_NO_JS_BUILD=1 cargo build -p dadrock
```

(The binary will still serve the embedded artifacts from a previous full build; you'll only see stale frontend.)

## Scripts

| Command | What it does |
|---|---|
| `bun run dev` | Vite dev server, hot reload, proxies `/api/*` |
| `bun run build` | Production build to `build/` |
| `bun run preview` | Serve the production build locally |
| `bun run check` | `svelte-check` + `tsc --noEmit` |
| `bun run lint` | ESLint |
| `bun run test:unit` | Vitest |
| `bun run test:e2e` | Playwright |

## Project conventions

See `docs/superpowers/specs/2026-05-09-svelte-web-ui-design.md` for the full design spec, including:

- Component layers (`ui/` recipes → domain components → shell)
- State model (TanStack Query for server state; runes for local; `.svelte.ts` modules for global UI)
- API mocking boundary (`DadrApi` interface, `RealApi` vs `MockApi`)
- Visual tone and design tokens
- Phase milestones
```

- [ ] **Step 2: Commit**

```bash
git add crates/dadrock/web/README.md
git commit -m "web README (phase 0 task 19)"
```

---

## Task 20 — Top-level Makefile target + final verification

**Files:**
- Modify: `Makefile`

- [ ] **Step 1: Inspect the existing Makefile**

```bash
cat Makefile
```

- [ ] **Step 2: Add web-specific targets**

Append to the existing `Makefile`:

```makefile
.PHONY: web-dev web-build web-test web-lint

web-dev:
	cd crates/dadrock/web && bun run dev

web-build:
	cd crates/dadrock/web && bun run build

web-test:
	cd crates/dadrock/web && bun run test:unit

web-lint:
	cd crates/dadrock/web && bun run lint && bun run check
```

- [ ] **Step 3: Final end-to-end verification**

```bash
# Clean rebuild, exercises build.rs.
cargo clean -p dadrock
cargo build -p dadrock

# Run all unit tests (Rust + JS).
cargo test -p dadrock
make web-test
make web-lint

# Smoke-launch and check both UIs.
cargo run -p dadrock -- preview &
SERVER_PID=$!
sleep 2
PORT=$(lsof -nP -p $SERVER_PID -iTCP -sTCP:LISTEN | tail -1 | awk '{print $9}' | sed 's/.*://')
curl -sf "http://localhost:$PORT/" | head -5
curl -sf "http://localhost:$PORT/v2/" | head -5
curl -sf "http://localhost:$PORT/api/project" | head -5
kill $SERVER_PID
```

Expected:

- `cargo build -p dadrock`: succeeds; runs Bun + Vite under the hood.
- `cargo test -p dadrock`: existing Rust tests pass.
- `make web-test`: all unit tests pass.
- `make web-lint`: lint + svelte-check pass.
- `curl /` returns the legacy `index.html`.
- `curl /v2/` returns the SvelteKit `index.html` (look for `<div style="display: contents">`).
- `curl /api/project` returns a JSON snapshot.

- [ ] **Step 4: Commit**

```bash
git add Makefile
git commit -m "makefile web targets + phase 0 final (phase 0 task 20)"
```

---

## Phase 0 done

At this point:

- The Svelte/SvelteKit/TypeScript/Tailwind/Bits-UI/shadcn-svelte/TanStack-Query/Zod stack is fully scaffolded inside `crates/dadrock/web/`.
- `cargo build -p dadrock` builds the JS toolchain transitively via `build.rs` and embeds the artifacts.
- The new SPA is reachable at `/v2/*`; the existing UI at `/` is untouched.
- One real round-trip works: `/v2/records` lists records via `/api/project` with TanStack Query + Zod validation.
- The shell (Header, Sidebar, ToastRegion, CommandPalette skeleton) is in place; theme switching works light/dark/system with no FOUC; sidebar collapse persists.
- Vitest + Playwright + ESLint + svelte-check are configured; CI has the commands wired through `make`.

**Next:** Phase 1 (port today's surfaces with polish, then cut over and delete the legacy UI). Write the Phase 1 plan with these Phase 0 artifacts in hand — concrete component conventions, real Tailwind tokens lived with, and lessons from `build.rs` integration.

---

## Self-review

**Spec coverage check:**

- ✅ Stack (Svelte 5, SvelteKit, TS, Tailwind v4, Bits UI, shadcn-svelte, Lucide, TanStack Query, Zod, Bun) — Tasks 1–9
- ✅ Project layout (`crates/dadrock/web/...`) — Task 1
- ✅ `build.rs` invokes Bun + embeds via `rust-embed` — Task 17
- ✅ `web.rs` serves embedded assets at `/v2/*` with SPA fallback; existing routes untouched — Task 18
- ✅ App shell (Header, Sidebar, CommandPalette, ToastRegion, ThemeToggle, error boundary) — Tasks 4, 11, 14
- ✅ Theme system (light/dark/system) with no-FOUC — Task 4
- ✅ All 10 routes as `+page.svelte` placeholders — Task 14
- ✅ DadrApi interface + RealApi + MockApi + selector — Task 9
- ✅ Zod schemas for first endpoint (snapshot) — Task 8
- ✅ TanStack Query setup with QueryClientProvider — Tasks 7, 14
- ✅ One real round-trip end-to-end — Task 15
- ✅ CI / lint / test scripts — Tasks 12, 13, 16, 20
- ✅ Dev workflow doc — Task 19

**Out-of-scope items deferred (matches spec):**

- Editing, validation UI, tag tree, candidate review, polish quality — Phase 1
- SSE, mocked spec-07 endpoints (audit, candidates with confidence, diff/source) — Phase 2
- Cutover (deleting `app.js` and `/assets/*` routes) — Phase 1

**Ambiguity / risk notes for the executor:**

- The exact `/api/project` response shape is verified empirically in Task 8 Step 2. If the schema in `lib/api/schemas.ts` doesn't match the real response, extend it before completing Task 15.
- Bun 1.1's flat lockfile is `bun.lock` (text). Older Bun versions use `bun.lockb` (binary); upgrade if you see the binary file.
- The `web_v2_response` function in Task 18 distinguishes "looks like a file" from "looks like a route" by checking for `.` in the path. Edge cases (file paths without extensions, route names with dots) are unlikely for this codebase but worth knowing.
- Some Svelte 5 + `@tanstack/svelte-query` versions still use `$<store>` reactive prefix; the plan uses the current API. If `@tanstack/svelte-query` updates its idiom (e.g., to a `createQuery` hook returning a rune), update Task 15's `+page.svelte` accordingly.
- Tailwind v4's `@custom-variant dark` syntax is current as of late 2025 / 2026. If a future Tailwind version changes the dark-mode incantation, update `app.css` accordingly.
