# Editor integration

This repository contains one extension implementation in `editors/vscode`. It provides Dadrock integration for VS Code, with a build/install target for Positron using the same extension source.

From the repository checkout, `make vscode` builds the binary and extension and installs it with the VS Code CLI. `make positron` targets Positron. These development commands require the editor CLI, Rust, Bun, Node/npm, and extension packaging dependencies.

This does not imply published marketplace packages or separate official extensions for Cursor, RStudio, or Neovim. You can edit DADR Markdown files in any editor and run `dadrock preview` alongside it.
