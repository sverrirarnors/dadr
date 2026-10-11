.PHONY: help install web-deps build serve clean cli-docs screenshot demo kill window version bump release vscode positron editors vscode-sync-version vscode-stage-binary vscode-package-target vscode-package-universal vscode-publish

.DEFAULT_GOAL := help

help: ## Display this help screen
	@printf "\033[1mAvailable commands:\033[0m\n\n"
	@grep -E '^[a-zA-Z0-9_.-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}' | sort

VERSION := $(shell awk -F'"' '/^version/ { print $$2; exit }' Cargo.toml)

install: ## Install the dadrock binary with Cargo
	@if command -v bun >/dev/null 2>&1; then \
		cargo install --path crates/dadrock --bin dadrock --force; \
	else \
		printf "bun not found; installing dadrock without embedded web UI assets. Preview will show a no-assets notice.\n"; \
		cargo install --path crates/dadrock --bin dadrock --force --features no-js-build; \
	fi

version: ## Print the current workspace version
	@echo $(VERSION)

bump: ## Bump workspace version (usage: make bump VERSION=x.y.z)
	@if [ -z "$(VERSION)" ] || [ "$(VERSION)" = "$(shell awk -F'"' '/^version/ { print $$2; exit }' Cargo.toml)" ]; then \
	    echo "usage: make bump VERSION=x.y.z  (must differ from current $(shell awk -F'"' '/^version/ { print $$2; exit }' Cargo.toml))"; \
	    exit 1; \
	fi
	@sed -i.bak -E 's/^version = "[^"]*"/version = "$(VERSION)"/' Cargo.toml && rm Cargo.toml.bak
	@cargo update -w >/dev/null
	@# The desktop app is a Cargo workspace of its own, so it does not inherit
	@# the version; release-desktop.yml refuses a tag that does not match it.
	@# Only the first `version =` line is the package's own.
	@awk -v v="$(VERSION)" '!done && /^version = "/ { sub(/"[^"]*"/, "\"" v "\""); done = 1 } { print }' \
	    desktop/Cargo.toml > desktop/Cargo.toml.tmp && mv desktop/Cargo.toml.tmp desktop/Cargo.toml
	@if [ -f desktop/Cargo.lock ]; then cargo update -w --manifest-path desktop/Cargo.toml >/dev/null; fi
	@echo "Bumped workspace and desktop app to $(VERSION)."
	@git diff --stat Cargo.toml Cargo.lock desktop/Cargo.toml desktop/Cargo.lock
	@echo ""
	@echo "Next: update CHANGELOG.md, commit Cargo.toml, Cargo.lock and desktop/Cargo.*, then 'make release'."

release: ## Tag and push v$(VERSION); fires cargo-dist release workflow
	@test -z "$$(git status --porcelain)" || { echo "working tree is dirty; commit or stash first"; exit 1; }
	@echo "Tagging v$(VERSION) at $$(git rev-parse --short HEAD) and pushing..."
	git tag -a v$(VERSION) -m "Release v$(VERSION)"
	git push origin v$(VERSION)

cli-docs: ## Regenerate the command-line reference inside docs-src/dadrock/cli.md
	cargo run --quiet -p dadrock --bin dadrock -- gen-cli-docs | python3 scripts/splice-cli-docs.py docs-src/dadrock/cli.md

web-deps: ## Install Python deps (zensical) into .venv via uv
	uv sync

build: web-deps cli-docs ## Build the static documentation site to docs/
	uv run zensical build

DOCS_PORT ?= 8001

serve: web-deps ## Serve the docs site locally with live reload (DOCS_PORT=8001)
	uv run zensical serve --dev-addr 127.0.0.1:$(DOCS_PORT)

screenshot: ## Regenerate the web UI screenshot on the landing page (docs-src/assets/dadrock-review-*.webp)
	./scripts/screenshot-web-ui.sh

clean: ## Remove the generated docs/ directory
	rm -rf docs

DEMO_DIR ?= /tmp/dadr-demo
DEMO_EXAMPLE ?= candidate-review
DEMO_PORT ?= 4004
DEMO_URL := http://127.0.0.1:$(DEMO_PORT)/
ifeq ($(shell uname -s),Darwin)
  OPEN ?= open
else
  OPEN ?= xdg-open
endif

demo: ## Seed a throwaway demo project and open it in your browser (DEMO_EXAMPLE=candidate-review DEMO_DIR=/tmp/dadr-demo DEMO_PORT=4004)
	@if [ -e "$(DEMO_DIR)" ] && [ ! -d "$(DEMO_DIR)/dadr" ]; then \
		echo "refusing to replace $(DEMO_DIR): it is not a dadrock demo project"; exit 1; \
	fi
	@rm -rf "$(DEMO_DIR)"
	cargo run --quiet -p dadrock --bin dadrock -- demo --example $(DEMO_EXAMPLE) "$(DEMO_DIR)"
	@( if command -v curl >/dev/null 2>&1; then \
	     for i in $$(seq 1 120); do \
	       curl -fsS "$(DEMO_URL)api/project" >/dev/null 2>&1 && break; sleep 0.5; \
	     done; \
	   else sleep 3; fi; \
	   echo "Opening $(DEMO_URL)"; $(OPEN) "$(DEMO_URL)" >/dev/null 2>&1 || echo "Could not open a browser; visit $(DEMO_URL)" ) &
	cargo run --quiet -p dadrock --bin dadrock -- preview --port $(DEMO_PORT) "$(DEMO_DIR)"

kill: ## Kill running dadrock preview servers
	@pkill -f '[d]adrock.*preview' || true

window: ## Resize the active window to 1920x1080 at 100,100 after a short delay
	sleep 3 && wmctrl -r :ACTIVE: -e 0,100,100,1920,1080
	xdotool getactivewindow getwindowgeometry

.PHONY: web-dev web-build web-test web-lint

web-dev: ## Run the Svelte web UI dev server
	cd crates/dadrock/web && bun run dev

web-build: ## Build the Svelte web UI
	cd crates/dadrock/web && bun run build

web-test: ## Run Svelte web UI unit tests
	cd crates/dadrock/web && bun run test:unit

web-lint: ## Run Svelte web UI lint and type checks
	cd crates/dadrock/web && bun run lint && bun run check

VSCODE_DIR := editors/vscode
VSIX_OUT   := $(VSCODE_DIR)/dist

vscode-sync-version:
	@node -e "const fs=require('fs'); \
	  const p='$(VSCODE_DIR)/package.json'; const j=JSON.parse(fs.readFileSync(p)); \
	  j.version='$(VERSION)'; fs.writeFileSync(p, JSON.stringify(j, null, 2)+'\n');"

vscode-stage-binary:
	@test -n "$(BIN_PATH)" || { echo "BIN_PATH must be set"; exit 1; }
	@rm -rf $(VSCODE_DIR)/bin
	@mkdir -p $(VSCODE_DIR)/bin
	@cp "$(BIN_PATH)" $(VSCODE_DIR)/bin/
	@chmod +x $(VSCODE_DIR)/bin/dadrock* 2>/dev/null || true

vscode: vscode-sync-version ## Build and install the VS Code extension with bundled dadrock
	DADROCK_REQUIRE_JS_BUILD=1 cargo build --release -p dadrock --bin dadrock
	rm -f $(VSCODE_DIR)/dadrock-*.vsix
	rm -rf $(VSCODE_DIR)/bin
	mkdir -p $(VSCODE_DIR)/bin
	cp target/release/dadrock $(VSCODE_DIR)/bin/
	chmod +x $(VSCODE_DIR)/bin/dadrock
	cd $(VSCODE_DIR) && npm install --no-audit --no-fund --silent
	cd $(VSCODE_DIR) && npx tsc -p ./ && npx vsce package --no-dependencies
	code --uninstall-extension VincentArel-Bundock.dadrock 2>/dev/null || true
	code --install-extension $(VSCODE_DIR)/dadrock-$(VERSION).vsix --force
	@echo ">>> Reload VS Code window to pick up the new extension <<<"

UNAME_S := $(shell uname -s)
ifeq ($(UNAME_S),Darwin)
  POSITRON_CLI := /Applications/Positron.app/Contents/Resources/app/bin/code
else
  POSITRON_CLI := positron
endif

positron: vscode-sync-version ## Build and install the Positron extension with bundled dadrock
	DADROCK_REQUIRE_JS_BUILD=1 cargo build --release -p dadrock --bin dadrock
	rm -f $(VSCODE_DIR)/dadrock-*.vsix
	rm -rf $(VSCODE_DIR)/bin
	mkdir -p $(VSCODE_DIR)/bin
	cp target/release/dadrock $(VSCODE_DIR)/bin/
	chmod +x $(VSCODE_DIR)/bin/dadrock
	cd $(VSCODE_DIR) && npm install --no-audit --no-fund --silent
	cd $(VSCODE_DIR) && npx tsc -p ./ && npx vsce package --no-dependencies
	$(POSITRON_CLI) --uninstall-extension VincentArel-Bundock.dadrock 2>/dev/null || true
	$(POSITRON_CLI) --install-extension $(VSCODE_DIR)/dadrock-$(VERSION).vsix --force

editors: vscode positron ## Install all editor extensions (VS Code + Positron)

vscode-package-target: vscode-sync-version
	@test -n "$(TARGET)" || { echo "TARGET must be set (e.g. darwin-arm64)"; exit 1; }
	@mkdir -p $(VSIX_OUT)
	cd $(VSCODE_DIR) && npm install --no-audit --no-fund --silent
	cd $(VSCODE_DIR) && npx tsc -p ./
	cd $(VSCODE_DIR) && npx vsce package --no-dependencies \
	  --target $(TARGET) -o dist/dadrock-$(TARGET)-$(VERSION).vsix

vscode-package-universal: vscode-sync-version
	@mkdir -p $(VSIX_OUT)
	@rm -rf $(VSCODE_DIR)/bin
	cd $(VSCODE_DIR) && npm install --no-audit --no-fund --silent
	cd $(VSCODE_DIR) && npx tsc -p ./
	cd $(VSCODE_DIR) && npx vsce package --no-dependencies \
	  -o dist/dadrock-universal-$(VERSION).vsix

vscode-publish: ## Publish packaged VSIX files to VS Marketplace and Open VSX (needs VSCE_PAT and OVSX_PAT)
	@set -e; \
	missing=""; \
	[ -n "$${VSCE_PAT:-}" ] || missing="$$missing VSCE_PAT"; \
	[ -n "$${OVSX_PAT:-}" ] || missing="$$missing OVSX_PAT"; \
	if [ -n "$$missing" ]; then \
	  echo "vscode-publish: not set:$$missing; nothing was published" >&2; \
	  exit 1; \
	fi; \
	set -- $(VSIX_OUT)/*.vsix; \
	if [ ! -e "$$1" ]; then \
	  echo "vscode-publish: no VSIX files in $(VSIX_OUT); run vscode-package-target or vscode-package-universal first" >&2; \
	  exit 1; \
	fi; \
	for v in "$$@"; do \
	  echo "Publishing $$v"; \
	  npx --prefix $(VSCODE_DIR) vsce publish --no-dependencies --packagePath "$$v" -p "$$VSCE_PAT"; \
	  npx --prefix $(VSCODE_DIR) ovsx publish "$$v" -p "$$OVSX_PAT"; \
	done

# ------------------------------------------------------------------------------
# Linux packages (packaging/linux/, scripts/make-linux-packages.sh)
#
# Requires nfpm on PATH. The release workflow (linux-packages.yml, called by
# release-desktop.yml) runs the same script against the binaries already
# published, for both architectures. On NixOS the script refuses the local
# binaries, whose ELF interpreter lives in /nix/store.
# ------------------------------------------------------------------------------

.PHONY: linux-packages

linux-packages: ## Build dist/ .deb, .rpm and .pkg.tar.zst for dadrock, and dadrock-desktop if built (Linux only)
	DADROCK_REQUIRE_JS_BUILD=1 cargo build --release -p dadrock --bin dadrock
	./scripts/make-linux-packages.sh dadrock
	@if [ -f desktop/target/release/dadrock-desktop ]; then \
		./scripts/make-linux-packages.sh dadrock-desktop; \
	else \
		echo "desktop/target/release/dadrock-desktop is not built; skipping the dadrock-desktop packages"; \
	fi

# ------------------------------------------------------------------------------
# Desktop app (desktop/): a Tauri window around `dadrock preview`.
#
# desktop/ is a Cargo workspace of its own. On NixOS its WebKitGTK build
# inputs and GSettings schemas come from desktop/shell.nix; elsewhere install
# Tauri's prerequisites (https://v2.tauri.app/start/prerequisites/) and the
# CLI with `cargo install tauri-cli --version "^2" --locked`.
# ------------------------------------------------------------------------------

.PHONY: desktop-dev desktop-demo desktop desktop-icons

ifneq ($(wildcard /etc/NIXOS),)
  DESKTOP_ENV := nix-shell $(CURDIR)/desktop/shell.nix --run
else
  DESKTOP_ENV := sh -c
endif
HOST_TRIPLE = $(shell rustc -vV | sed -n 's/^host: //p')

desktop-dev: ## Run the desktop app with a debug dadrock (PROJECT=path opens it directly)
	cargo build --quiet -p dadrock --bin dadrock
	$(DESKTOP_ENV) 'cd desktop && DADROCK_BIN="$(CURDIR)/target/debug/dadrock" cargo run -- $(PROJECT)'

desktop-demo: ## Seed the demo project (like `make demo`) and open it in the desktop app
	@if [ -e "$(DEMO_DIR)" ] && [ ! -d "$(DEMO_DIR)/dadr" ]; then \
		echo "refusing to replace $(DEMO_DIR): it is not a dadrock demo project"; exit 1; \
	fi
	@rm -rf "$(DEMO_DIR)"
	cargo run --quiet -p dadrock --bin dadrock -- demo --example $(DEMO_EXAMPLE) "$(DEMO_DIR)"
	$(MAKE) desktop-dev PROJECT="$(DEMO_DIR)"

# macOS gets a .dmg with dadrock inside the app; Linux gets the bare binary,
# which `make linux-packages` turns into .deb/.rpm packages that depend on the
# dadrock package instead. Release builds come from release-desktop.yml.
desktop: ## Build the desktop app for this machine (macOS .dmg, Linux binary)
	DADROCK_REQUIRE_JS_BUILD=1 cargo build --release -p dadrock --bin dadrock
ifeq ($(UNAME_S),Darwin)
	cp target/release/dadrock desktop/binaries/dadrock-$(HOST_TRIPLE)
	cd desktop && cargo tauri build --bundles dmg
else
	$(DESKTOP_ENV) 'cd desktop && cargo tauri build --no-bundle'
endif

desktop-icons: ## Regenerate desktop/icons/ from desktop/icons/icon.svg
	$(DESKTOP_ENV) 'set -e; tmp=$$(mktemp -d); \
	  rsvg-convert -w 1024 -h 1024 desktop/icons/icon.svg -o "$$tmp/icon.png"; \
	  cd desktop && cargo tauri icon "$$tmp/icon.png" --output icons; \
	  rm -rf icons/android icons/ios icons/Square*Logo.png icons/StoreLogo.png "$$tmp"'
