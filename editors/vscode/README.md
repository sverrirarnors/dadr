# Dadrock

Data Analysis Decision Records browser and editor for VS Code and Positron.

Dadrock opens the existing local web workbench inside an editor panel. It starts `dadrock preview` against the current workspace, embeds the localhost preview in a webview iframe, and stops the preview server when requested.

## Features

- Browse, review, validate, and manage DADR records from inside the editor.
- Run the same Dadrock preview UI used by the standalone localhost server.
- Initialize a workspace with `Dadrock: Init`.
- Bundled binary on supported platform VSIX builds; falls back to `dadrock` on `$PATH`.

## Requirements

The extension shells out to the `dadrock` binary. Platform VSIX builds bundle the binary. Universal builds require `dadrock` on `$PATH` or `dadrock.binaryPath` set in settings.

## Commands

| Command | Description |
| --- | --- |
| `Dadrock: Start` | Start the preview server for the current workspace. |
| `Dadrock: Stop` | Stop the preview server. |
| `Dadrock: Restart` | Stop and start again. |
| `Dadrock: Show Panel` | Reveal the Dadrock workbench panel. |
| `Dadrock: Init` | Run `dadrock init` in the current workspace. |

## Settings

| Setting | Default | Description |
| --- | --- | --- |
| `dadrock.binaryPath` | `""` | Absolute path to the `dadrock` binary. Empty means use the bundled binary, then `$PATH`. |
| `dadrock.autoStart` | `false` | Start Dadrock automatically when a DADR workspace opens. |

## License

MIT.
