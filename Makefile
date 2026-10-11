.PHONY: help install web-deps build serve clean cli-docs kill window version bump release vscode positron editors vscode-sync-version vscode-stage-binary vscode-package-target vscode-package-universal vscode-publish

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
	@echo "Bumped workspace to $(VERSION)."
	@git diff --stat Cargo.toml Cargo.lock
	@echo ""
	@echo "Next: update CHANGELOG.md, commit Cargo.toml + Cargo.lock, then 'make release'."

release: ## Tag and push v$(VERSION); fires cargo-dist release workflow
	@test -z "$$(git status --porcelain)" || { echo "working tree is dirty; commit or stash first"; exit 1; }
	@echo "Tagging v$(VERSION) at $$(git rev-parse --short HEAD) and pushing..."
	git tag -a v$(VERSION) -m "Release v$(VERSION)"
	git push origin v$(VERSION)

cli-docs: ## Regenerate the command-line reference inside docs-src/dadrock.md
	cargo run --quiet -p dadrock --bin dadrock -- gen-cli-docs | python3 scripts/splice-cli-docs.py docs-src/dadrock.md

web-deps: ## Install Python deps (zensical) into .venv via uv
	uv sync

build: web-deps cli-docs ## Build the static documentation site to docs/
	uv run zensical build

DOCS_PORT ?= 8001

serve: web-deps ## Serve the docs site locally with live reload (DOCS_PORT=8001)
	uv run zensical serve --dev-addr 127.0.0.1:$(DOCS_PORT)

clean: ## Remove the generated docs/ directory
	rm -rf docs

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

vscode-publish: ## Publish packaged VSIX files to VS Marketplace and Open VSX
	@for v in $(VSIX_OUT)/*.vsix; do \
	  echo "Publishing $$v"; \
	  npx --prefix $(VSCODE_DIR) vsce publish --no-dependencies --packagePath $$v -p $$VSCE_PAT; \
	  npx --prefix $(VSCODE_DIR) ovsx publish $$v -p $$OVSX_PAT; \
	done
