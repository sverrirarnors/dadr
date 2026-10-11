# Dadrock desktop app

A [Tauri 2](https://v2.tauri.app) window around `dadrock preview`. The app
doesn't reimplement anything. Each project window starts its own `dadrock
preview` process and shows the web UI that process serves, exactly as the VS
Code extension does. So the browser, the editor extensions and the desktop app
always agree about a project.

## How it works

- **Finding the CLI.** The app runs the first of these it finds:
  `$DADROCK_BIN`, a `dadrock` next to its own executable, or `dadrock` on
  `PATH`. The macOS and Windows bundles carry their own copy (Tauri
  `externalBin`, staged as `binaries/dadrock-<target-triple>`). On Linux the
  `dadrock-desktop` package depends on the `dadrock` package, which puts it at
  `/usr/bin/dadrock`.
- **One server per window.** The port is derived from the project path
  (41000–42999, falling through to the next free one). A project gets the same
  origin each time, so the UI's `localStorage` (theme, resolved diagnostics)
  survives restarts.
- **No orphaned servers.** The app sets `DADROCK_PREVIEW_EXIT_ON_STDIN_EOF=1`
  and holds the server's stdin. The server exits when that pipe closes, which
  happens even if the app crashes or is killed.
- **PATH for agents.** Apps started from the Dock or a desktop launcher get a
  minimal `PATH`. On macOS and Linux the app asks the user's login shell for
  its `PATH` and passes it to `dadrock`, so ACP agents such as
  `claude-agent-acp` are found.
- **Links.** A project window only navigates within its own server. Other
  links, including Markdown links in records, open in the system browser.
- **Welcome window.** `welcome/` is a static page bundled into the app. It
  lists recent projects (stored in the app's config directory), opens a
  folder picker, and accepts a dropped folder. It is the only page with Tauri
  IPC. Project pages get no capabilities.
- **Menus.** macOS gets the full native menu bar. On Linux and Windows the
  menu bar is drawn inside each window, so it is a single File menu: Open
  Project (Ctrl+O), Recent Projects, Reload (Ctrl+R), Close, Quit.

## Develop

```bash
make desktop-dev                      # welcome window, debug dadrock from this repo
make desktop-dev PROJECT=~/analysis   # open a project directly
make desktop-demo                     # seed the demo project and open it
```

`desktop/` is its own Cargo workspace, so the root `cargo test --workspace`
never needs WebKitGTK. To check the app:

```bash
cd desktop && cargo clippy --all-targets && cargo test
```

Prerequisites:

- **NixOS:** the Makefile targets enter `desktop/shell.nix` automatically. It
  provides WebKitGTK, the Tauri CLI, and the GSettings schemas GTK's file
  picker needs.
- **Other Linux distributions:** install
  [Tauri's prerequisites](https://v2.tauri.app/start/prerequisites/#linux)
  and `cargo install tauri-cli --version "^2" --locked`.
- **macOS:** Xcode command-line tools and the Tauri CLI.

## Build

```bash
make desktop          # macOS: .dmg with dadrock inside; Linux: target/release/dadrock-desktop
make linux-packages   # Linux: .deb/.rpm/.pkg.tar.zst for dadrock and dadrock-desktop
make desktop-icons    # regenerate icons/ from icons/icon.svg
```

On NixOS, `make linux-packages` refuses to package local binaries, because
their ELF interpreter lives in `/nix/store`.

## Release

`make bump VERSION=x.y.z` updates `desktop/Cargo.toml` along with the
workspace, and `make release` pushes the tag. After cargo-dist's **Release**
workflow publishes the CLI, **Release Desktop**
(`.github/workflows/release-desktop.yml`) builds the app on macOS (arm64 and
x86_64), Windows and Linux (x86_64 and arm64). It reuses cargo-dist's
`dadrock` binaries as the bundled CLI, attaches the results to the same GitHub
release, and then runs:

- `linux-packages.yml`: `.deb`, `.rpm` and Arch packages for `dadrock` and
  `dadrock-desktop`, then a nudge to the
  [apt](https://github.com/vincentarelbundock/apt) and
  [rpm](https://github.com/vincentarelbundock/rpm) repositories
  (`APT_REPO_TOKEN`, `RPM_REPO_TOKEN`; optional, otherwise they pick it up
  nightly).
- `update-homebrew-tap.yml`: the `dadrock` formula and the `dadrock-desktop`
  cask (`HOMEBREW_TAP_GITHUB_TOKEN`).
- `update-scoop-bucket.yml`: the `dadrock` and `dadrock-desktop` manifests
  (`SCOOP_BUCKET_GITHUB_TOKEN`).

macOS signing is optional. With the `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`,
`APPLE_PASSWORD` and `APPLE_TEAM_ID` secrets, Tauri signs and notarizes the
app. Without them it is ad-hoc signed: it runs on Apple silicon, but
Gatekeeper blocks the first launch until the user allows it, and the cask's
caveats explain how. The Windows installer isn't code-signed either, so
SmartScreen warns on first run.
