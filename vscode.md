# Building a VS Code / Positron extension for dadrock

Notes on porting the scrutin extension pattern to dadrock, including marketplace setup and theme integration for the Svelte web app inside a webview.

## 1. The scrutin template (what we're copying)

`editors/vscode/` in scrutin is a standalone VS Code extension that:

1. **Spawns the Rust binary** as a subprocess (`scrutin-process.ts`) with a randomly chosen free port (`-r web:127.0.0.1:PORT <workspace>`), then waits for the line `scrutin-web: listening on http://...` on stderr to extract the base URL.
2. **Hosts the existing web UI inside a webview** (`webview-panel.ts`): reuses `crates/scrutin-web/frontend/` (vanilla JS + CSS) by bundling it at build time with `build-webview.mjs` (esbuild for `app.js`, regex strip/inject for `index.html`, CSS overlay for VS Code theme tokens).
3. **Subscribes to SSE** (`/api/events`) for status-bar updates and live test events; calls `/api/snapshot` for state.
4. **Wires VS Code APIs**: status bar, test controller, commands `scrutin.start/stop/restart/showPanel/init`, `scrutin.binaryPath` + `scrutin.autoStart` settings, `openFile` message handler from the webview.
5. **Packages** with `vsce package`, copies the release binary into `editors/vscode/bin/` so the VSIX is self-contained per platform.

Makefile glue: `sync-webview` (run before any package), `vscode`, `positron` (same VSIX, different CLI), `rstudio` (R package), `editors = vscode positron rstudio`, plus `vscode-sync-version`, `vscode-package-target`, `vscode-package-universal`, `vscode-publish` for marketplace releases.

## 2. The dadrock-specific complication

Scrutin's webview-embedding works because the frontend is plain `app.js` + `style.css` + `index.html` — easy to esbuild into one file and rewrite tag URIs. **`crates/dadrock/web/` is SvelteKit + Vite + Tailwind**, which means:

- The static build emits a hashed `_app/immutable/...` tree with many dynamically imported chunks. Their import paths are baked at build time relative to `paths.base`.
- Trying to rewrite each chunk URL to `vscode-webview://...` is fragile — every chunk's dynamic `import()` resolves relative to a different origin, and `paths.relative: false` is set today (`svelte.config.js:14`).
- The Rust binary already embeds the built SPA via `rust_embed` (`WebAssets` in `crates/dadrock/src/web.rs`) and serves it.

**Recommended pivot:** don't bundle the Svelte app into the webview at all. Spawn the preview server, then render an `<iframe>` pointed at `http://127.0.0.1:<port>` inside the webview. This is the same strategy Microsoft's own Live Preview extension uses, and it sidesteps the SvelteKit asset-rewriting work entirely.

Tradeoffs: the webview's CSP has to allow `frame-src` to a localhost port, and "reveal in editor" needs `postMessage` plumbing instead of direct webview message handlers — manageable but worth knowing.

## 3. Skeleton

```
editors/
  vscode/
    package.json          # publisher, commands dadrock.start/stop/init/showPanel/restart,
                          # activationEvents: workspaceContains:**/dadr.toml, **/.dadr-dir
    tsconfig.json
    src/
      extension.ts        # ports cleanly from scrutin
      dadrock-process.ts  # spawn + readiness; see notes below
      webview-panel.ts    # iframe shell with CSP allowing frame-src http://127.0.0.1:*
    images/icon.png
    bin/                  # gitignored; populated at package time
```

Key code adaptations from the scrutin TS:

- **`dadrock-process.ts`**: `dadrock preview` today prints `open http://127.0.0.1:PORT` to stdout (see `crates/dadrock/src/web.rs:94`). Either change the regex and read stdout, or — better — teach `dadrock preview` to print a stable, machine-readable ready line on stderr (e.g. `dadrock-preview: listening on http://...`). Also have the extension pick a free port via `net.createServer().listen(0)` (the same trick scrutin uses) and pass it as `--port`.
- **`webview-panel.ts`**: drop the script/style URI substitution; instead, set webview HTML to a small shell with `<iframe src="{baseUrl}" sandbox="allow-scripts allow-same-origin allow-forms">` and CSP `frame-src http://127.0.0.1:*;`. Keep the `openFile` message handler — the Svelte app can `postMessage` to the iframe parent for "reveal in editor" interactions.
- **No SSE / test controller**: dadrock has no equivalent live event stream today, so the `event-bus.ts` and `testing.ts` parts of scrutin drop away. Status bar can just say "running" / "stopped" based on the process state.
- **Commands**: `dadrock.start`, `dadrock.stop`, `dadrock.showPanel`, `dadrock.restart`, `dadrock.init` (calls `dadrock init <workspace>`).
- **Settings**: `dadrock.binaryPath`, `dadrock.autoStart`.

## 4. Makefile additions

Mirror scrutin's structure:

```make
VSCODE_DIR := editors/vscode

vscode-sync-version:
	@node -e "const fs=require('fs'); \
	  const p='$(VSCODE_DIR)/package.json'; const j=JSON.parse(fs.readFileSync(p)); \
	  j.version='$(VERSION)'; fs.writeFileSync(p, JSON.stringify(j, null, 2)+'\n');"

vscode: vscode-sync-version ## Build and install the VS Code extension with bundled dadrock
	cargo build --release -p dadrock --bin dadrock
	rm -f $(VSCODE_DIR)/dadrock-*.vsix
	rm -rf $(VSCODE_DIR)/bin
	mkdir -p $(VSCODE_DIR)/bin
	cp target/release/dadrock $(VSCODE_DIR)/bin/
	chmod +x $(VSCODE_DIR)/bin/dadrock
	cd $(VSCODE_DIR) && npm install --no-audit --no-fund --silent
	cd $(VSCODE_DIR) && npx tsc -p ./ && npx vsce package --no-dependencies
	code --uninstall-extension VincentArel-Bundock.dadrock 2>/dev/null || true
	code --install-extension $(VSCODE_DIR)/dadrock-$(VERSION).vsix --force

UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
  POSITRON_CLI := /Applications/Positron.app/Contents/Resources/app/bin/code
else
  POSITRON_CLI := positron
endif

positron: vscode-sync-version
	cargo build --release -p dadrock --bin dadrock
	rm -f $(VSCODE_DIR)/dadrock-*.vsix
	rm -rf $(VSCODE_DIR)/bin
	mkdir -p $(VSCODE_DIR)/bin
	cp target/release/dadrock $(VSCODE_DIR)/bin/
	chmod +x $(VSCODE_DIR)/bin/dadrock
	cd $(VSCODE_DIR) && npm install --no-audit --no-fund --silent
	cd $(VSCODE_DIR) && npx tsc -p ./ && npx vsce package --no-dependencies
	$(POSITRON_CLI) --uninstall-extension VincentArel-Bundock.dadrock 2>/dev/null || true
	$(POSITRON_CLI) --install-extension $(VSCODE_DIR)/dadrock-$(VERSION).vsix --force

editors: vscode positron
```

You don't need a `sync-webview` step here (no asset rebundling) and there's no RStudio analogue, so `editors` is just `vscode positron`. Note: `cargo build --release -p dadrock` needs `bun` available to embed the Svelte assets — if `bun` is missing, build with `--features no-js-build` and the preview shows a no-assets notice instead.

## 5. Order of work

1. **Tiny upstream tweak first**: make `dadrock preview` print a stable ready-line on stderr (e.g. `dadrock-preview: listening on http://HOST:PORT`) and accept `--port <N>`. Without this, the extension is racy on port discovery.
2. Stamp out `editors/vscode/` by copying scrutin's tree and search-replacing `scrutin` → `dadrock`; delete `event-bus.ts`, `testing.ts`, `build-webview.mjs`, `webview-overrides.css`, `webview/`.
3. Rewrite `webview-panel.ts` to render an iframe + CSP shell.
4. Add the Makefile block above.
5. Once it works locally, mirror scrutin's `vscode-package-target` / `vscode-publish` block for cross-platform VSIXes if you want marketplace distribution.

## 6. VS Code Marketplace setup

If you already publish `scrutin-runner` under `VincentArel-Bundock`, the marketplace side is mostly already wired — a new extension under the same publisher just needs a fresh `name` and the right manifest fields.

### Nothing you need to register

- **Publisher**: `VincentArel-Bundock` already exists. New extensions inherit verification, domain trust, and badges. No new publisher account.
- **PAT**: your existing Azure DevOps `$VSCE_PAT` (Marketplace → Manage scope on *all accessible organizations*) covers any extension you own. Reuse it.
- **Open VSX**: same — your `$OVSX_PAT` works for any namespace you've claimed. If you haven't claimed `VincentArel-Bundock` on Open VSX yet, do that once via the eclipse.org account; future extensions slot in.

### What you do need to set in `editors/vscode/package.json`

```json
{
  "name": "dadrock",
  "displayName": "Dadrock",
  "description": "Data Analysis Decision Records browser and editor.",
  "version": "0.0.1",
  "publisher": "VincentArel-Bundock",
  "license": "MIT",
  "icon": "images/icon.png",
  "homepage": "https://vincentarelbundock.github.io/dadr",
  "repository": { "type": "git", "url": "https://github.com/vincentarelbundock/dadr" },
  "bugs": { "url": "https://github.com/vincentarelbundock/dadr/issues" },
  "galleryBanner": { "color": "#0f172a", "theme": "dark" },
  "categories": ["Data Science", "Notebooks", "Other"],
  "keywords": ["dadr", "decision records", "data analysis", "adr"]
}
```

The first `vsce publish` creates the listing automatically — no "register extension" step in the web UI. `README.md` in `editors/vscode/` becomes the marketplace page body, and `CHANGELOG.md` is the Changelog tab; both should exist before the first publish.

### Things to know that bite people once

- **Once published, you cannot rename or transfer ownership.** Pick the `name` deliberately.
- **Versions are monotonic per extension.** Independent of scrutin's. Start at `0.0.1`; `vsce` refuses to publish a version ≤ the latest live one.
- **Per-platform VSIXes** require unique `--target` values per VSIX; each is published as a variant of the same version. If you only want one universal VSIX (no bundled binary, relies on `$PATH`), skip that.
- **Verified publisher checkmark** is granted at the publisher level (you verified `vincentarelbundock.github.io` via DNS for scrutin) — it carries over automatically.
- Optional but suppresses `vsce package` warnings: `"pricing": "Free"`, `"qna": "marketplace"`.

## 7. Making the Svelte web app look native inside the webview

The webview-embeds-an-iframe topology has a wrinkle: **VS Code's CSS variables and theme class are on the *webview document*, not the iframe**. The Svelte app lives in a separate origin and doesn't see them automatically. So the work splits in two: the extension plumbs theme info into the iframe, and the web app consumes it.

### Handoff (extension side)

The webview parent reads `vscode.window.activeColorTheme.kind`, computes the values it cares about from `getComputedStyle(document.body)` (the `--vscode-*` variables are all there), and calls `iframe.contentWindow.postMessage({ kind: 'vscode-theme', ... }, '*')` on load and on `vscode.window.onDidChangeActiveColorTheme`. Two options:

- Send just a `kind` enum (`dark | light | high-contrast`) and ship hand-tuned light/dark palettes — simpler.
- Send `kind` plus a flat blob of CSS variable values — more native-feeling.

Start with `kind` only; add value-mirroring later if it feels off.

### Web app changes in `crates/dadrock/web/`

**1. Detect webview mode.** Extension loads the iframe with `?vscode=1`. In `src/app.html` or `src/routes/+layout.svelte`, set `<html data-host="vscode">` when the URL param is present. The inline theme-bootstrap script in `app.html:7-15` should bail out when `data-host="vscode"` — VS Code owns the theme; `localStorage.getItem('dadr.theme')` should not run.

**2. Replace palette tokens with a VS Code mapping under `[data-host="vscode"]`.** In `src/app.css`:

```css
:root[data-host="vscode"] {
  /* Surfaces */
  --color-background:         var(--vscode-editor-background);
  --color-surface:            var(--vscode-sideBar-background);
  --color-surface-raised:     var(--vscode-editorWidget-background);
  --color-foreground:         var(--vscode-foreground);
  --color-foreground-muted:   var(--vscode-descriptionForeground);

  /* Borders & focus */
  --color-border:             var(--vscode-panel-border);
  --color-border-strong:      var(--vscode-widget-border);
  --color-focus:              var(--vscode-focusBorder);

  /* Accents & semantic */
  --color-accent:             var(--vscode-textLink-foreground);
  --color-accent-hover:       var(--vscode-textLink-activeForeground);
  --color-error:              var(--vscode-errorForeground);
  --color-warning:            var(--vscode-editorWarning-foreground);
  --color-success:            var(--vscode-testing-iconPassed, var(--vscode-charts-green));

  /* Inputs */
  --input-background:         var(--vscode-input-background);
  --input-foreground:         var(--vscode-input-foreground);
  --input-border:             var(--vscode-input-border, transparent);

  /* Buttons */
  --button-background:        var(--vscode-button-background);
  --button-foreground:        var(--vscode-button-foreground);
  --button-hover:             var(--vscode-button-hoverBackground);

  /* Lists / hover */
  --hover-background:         var(--vscode-list-hoverBackground);
  --selected-background:      var(--vscode-list-activeSelectionBackground);
  --selected-foreground:      var(--vscode-list-activeSelectionForeground);

  /* Code */
  --code-background:          var(--vscode-textCodeBlock-background);

  /* Fonts */
  --font-sans:                var(--vscode-font-family);
  --font-mono:                var(--vscode-editor-font-family);
  font-size:                  var(--vscode-font-size, 13px);
}
```

Since you're on **Tailwind v4**, expose these through `@theme` so utility classes pick them up:

```css
@theme {
  --color-bg: var(--color-background);
  --color-fg: var(--color-foreground);
  --color-border: var(--color-border);
  /* etc. */
}
```

That way `bg-bg`, `text-fg`, `border-border` resolve to VS Code tokens automatically when `data-host="vscode"`, and to your normal palette otherwise. No component code changes.

The VS Code variables only resolve inside the iframe if the parent forwards them. If you take the simpler "kind-only" path, replace each `var(--vscode-...)` with a hand-tuned color in two static blocks: `[data-host="vscode"][data-theme="dark"]` and `[data-host="vscode"][data-theme="light"]`.

**3. Light / dark / HC.** Listen for the parent's postMessage and set `data-theme` on `<html>`:

```ts
// in onMount of +layout.svelte, gated by data-host="vscode"
window.addEventListener('message', (e) => {
  if (e.data?.kind !== 'vscode-theme') return;
  document.documentElement.dataset.theme = e.data.theme; // 'light' | 'dark' | 'hc'
});
window.parent.postMessage({ kind: 'ready' }, '*');
```

Add `color-scheme: light` / `dark` on `:root` for each — fixes native scrollbars, form controls, and the gap-between-elements background flash.

**4. Fonts and density.** VS Code's UI is denser than typical SaaS web:

```css
:root[data-host="vscode"] {
  font-size: 13px;
  line-height: 1.4;
}
:root[data-host="vscode"] body { font-family: var(--font-sans); }
:root[data-host="vscode"] code,
:root[data-host="vscode"] pre { font-family: var(--font-mono); }
```

Reduce paddings on shell layout under `[data-host="vscode"]` — section titles, sidebar gutters, table cell padding all want about 25–30% less than the standalone build.

**5. Scrollbars and selection.**

```css
:root[data-host="vscode"] ::-webkit-scrollbar { width: 10px; height: 10px; }
:root[data-host="vscode"] ::-webkit-scrollbar-thumb {
  background: var(--vscode-scrollbarSlider-background);
}
:root[data-host="vscode"] ::-webkit-scrollbar-thumb:hover {
  background: var(--vscode-scrollbarSlider-hoverBackground);
}
:root[data-host="vscode"] ::selection {
  background: var(--vscode-editor-selectionBackground);
  color: var(--vscode-editor-selectionForeground);
}
:root[data-host="vscode"] :focus-visible {
  outline: 1px solid var(--color-focus);
  outline-offset: -1px;
}
```

**6. Hide chrome that VS Code owns.** In webview mode hide:

- Your own theme toggle (VS Code drives it).
- Any "open in browser" / "settings" buttons that point to web-only flows.
- Top-of-page logo/title bar if VS Code's tab title already shows "Dadrock".
- GitHub/marketing footer.

One rule plus class tagging: `[data-host="vscode"] .web-only { display: none; }`.

**7. Bits-UI portals.** `bits-ui` portals popovers/dialogs to `document.body` by default. The `data-host` attribute is on `<html>`, so portal'd nodes inherit correctly — but **only** if your theme rules cascade from `:root`/`html`, not from `body`. Double-check that the rules above all sit on `:root[data-host="vscode"]`. Otherwise modals and dropdowns render with the default palette.

**8. Links.** Webviews don't follow `<a href="https://...">` by default. Intercept and `postMessage({ kind: 'open-external', url })` to the parent, which calls `vscode.env.openExternal`. Usually a handful of links in your app.

**9. Don't break the standalone build.** All of this is gated on `[data-host="vscode"]`. The non-webview build is unaffected — opening `localhost:5173` in a browser still uses the real palette and full padding.

### Order of attack for theming

1. Add the `?vscode=1` detection + `data-host` plumbing — five lines in `+layout.svelte`.
2. Wire the postMessage theme listener and have the extension send `{ kind, theme }` on load.
3. Add the `[data-host="vscode"]` palette block and tighten density.
4. Audit components for hardcoded colors (`#abc`, `slate-500`, etc.) — anywhere you bypassed your tokens needs converting to a token reference so the override block can reach it. This is usually where the cleanup time goes; everything else is plumbing.

For genuinely-native theming (mirrors the user's actual VS Code theme, not just light/dark): the extension reads `getComputedStyle(document.body).getPropertyValue('--vscode-...')` for the dozen-or-so variables you care about and forwards the values; the app applies them via `document.documentElement.style.setProperty(...)`. Polish pass, not first cut.
