# Editors

DADR records are plain Markdown files. You can write and edit them in any editor and run `dadrock preview` alongside it to review, accept, and validate them in the [web interface](web.md).

## VS Code and Positron

The repository contains one extension implementation in `editors/vscode`. It opens the DADRock web interface inside an editor panel: it starts `dadrock preview` for the current workspace, shows the local page in a webview, and stops the server when asked. Positron uses the same extension source.

From a repository checkout, build the binary and the extension and install it with the editor's command-line tool:

```sh
make vscode      # VS Code
make positron    # Positron
make editors     # both
```

These targets need the editor CLI, Rust, Bun, and Node. The extension they build bundles the `dadrock` binary. Reload the editor window afterwards to pick up the new extension.

### Commands

| Command | Description |
|---|---|
| `Dadrock: Start` | Start the preview server for the current workspace. |
| `Dadrock: Stop` | Stop the preview server. |
| `Dadrock: Restart` | Stop and start again. |
| `Dadrock: Show Panel` | Reveal the DADRock panel. |
| `Dadrock: Init` | Run `dadrock init` in the current workspace. |

### Settings

| Setting | Default | Description |
|---|---|---|
| `dadrock.binaryPath` | `""` | Absolute path to the `dadrock` binary. Empty means the bundled binary, then `dadrock` on `PATH`. |
| `dadrock.autoStart` | `false` | Start DADRock automatically when a workspace containing `dadr/inbox`, `dadr/ledger`, or `dadr/archive` opens. |

The extension is not published on the Visual Studio Marketplace or Open VSX. GitHub releases attach its VSIX files: one per platform with the `dadrock` binary bundled (macOS arm64 and x64, Linux x64, Windows x64), and a universal one that needs `dadrock` on `PATH` or `dadrock.binaryPath`. Install one with the editor's `--install-extension` option. There are no separate official extensions for Cursor, RStudio, or Neovim.
