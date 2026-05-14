# adr-tools utility functions

Internal helper scripts (prefixed `_adr_`) that the user-facing `adr` commands compose. All live in `src/`.

This document also includes a **comparison with the competing [marouni/adr](https://github.com/marouni/adr) package** at the bottom.

---

## adr-tools (this package) — utilities

## `_adr_add_link`
Appends a Markdown link of the form `LINK_TYPE [Target Title](target.md)` to the `## Status` section of a source ADR.
Args: `SOURCE LINK_TYPE TARGET`.

## `_adr_autocomplete`
Produces shell completion suggestions for the `adr` CLI. Lists top-level commands, then subcommands derived from `_adr_<cmd>_*` scripts.

## `_adr_commands`
Lists all available `adr-*` executables in the bin directory, stripping the `adr-` prefix. Used to enumerate commands for help and autocomplete.

## `_adr_dir`
Resolves the ADR directory by walking up from the current directory looking for either a `.adr-dir` marker file or an existing `doc/adr/` directory. Falls back to `doc/adr`.

## `_adr_file`
Resolves a partial reference (number or substring) to a full ADR file path by grepping `adr list` and returning the first match.

## `_adr_generate_graph`
Emits a Graphviz `dot` description of the ADR link graph. Each node links back to the rendered ADR document.
Options: `-e LINK-EXTENSION` (default `.html`), `-p LINK_PREFIX`.

## `_adr_generate_toc`
Emits a Markdown table of contents for all ADRs to stdout.
Options: `-i INTRO`, `-o OUTRO`, `-p LINK_PREFIX`.

## `_adr_help`
Dispatches `adr help` requests. Prints the list of commands when called bare; otherwise runs a dedicated `_adr_help_<cmd>` script if one exists, or extracts `##`-prefixed comment lines from the target command file.

## `_adr_help_new`
Dedicated long-form help text for `adr new`, documenting the `-s` (supercedes) and `-l` (link) options and the template lookup behavior.

## `_adr_links`
Parses the `## Status` section of an ADR and emits `NUMBER=LINK_TYPE` lines, one per outgoing link to another ADR.

## `_adr_remove_status`
Removes a specific status line (e.g. `Accepted`) from the `## Status` section of an ADR, collapsing extra blank lines that result.
Args: `FROM FILE`.

## `_adr_status`
Prints the body of the `## Status` section of an ADR (non-blank, non-heading lines).

## `_adr_title`
Prints the title of an ADR by reading the first line of the file and stripping the leading `# `.

---

## Competition: `marouni/adr`

A minimalist Go reimplementation of the same idea, intentionally much smaller in scope.

### Features

- **Two commands only**: `init` and `new` (with aliases `i` and `c`).
- **Global configuration**: stores config as JSON in `~/.adr/config.json` rather than a per-repo `.adr-dir` file. Holds the base directory and the current ADR counter.
- **Single shared template**: `~/.adr/template.md`, generated on `init`. Includes Number, Title, Date, Status, Context, Decision, and Consequences sections.
- **Built-in status enum**: `Proposed`, `Accepted`, `Deprecated`, `Superseded` — but new ADRs are always created in `Proposed` state and there is no command to change status.
- **Auto-incrementing counter**: tracked in the global config rather than derived from filenames on disk.
- **Colored terminal output** via `fatih/color`.
- **Default base folder**: `~/adr/` if no path is passed to `init`.

### Comparison with adr-tools

| Capability                          | adr-tools (this repo)                        | marouni/adr                          |
|-------------------------------------|----------------------------------------------|--------------------------------------|
| Implementation                      | Bash scripts                                 | Single Go binary                     |
| Commands                            | `init`, `new`, `link`, `list`, `generate`, `upgrade-repository`, `help`, `config` | `init`, `new`                |
| Linking ADRs                        | Yes (`adr link`, supersedes via `-s`)        | No                                   |
| Status mutation                     | Yes (supersede rewrites prior ADR's status)  | No                                   |
| Table of contents generation        | Yes (`adr generate toc`)                     | No                                   |
| Graphviz link graph                 | Yes (`adr generate graph`)                   | No                                   |
| Per-project ADR location            | Yes (`.adr-dir` marker, walks up tree)       | No (single global base dir)          |
| Per-project template override       | Yes (`templates/template.md`)                | No (single global template)          |
| Shell autocomplete                  | Yes                                          | No                                   |
| Opens new ADR in `$EDITOR`/`$VISUAL`| Yes                                          | No (just writes the file)            |
| ADR numbering source                | Highest existing filename + 1                | Counter persisted in config JSON     |
| Filename format                     | `NNNN-kebab-title.md` (4-digit, zero-padded) | `N-kebab-title.md` (no padding)      |

---

## Competition: `adr-tools-python` (Victor Sluiter)

A Python port of the original Bash adr-tools, distributed on PyPI as `adr-tools-python`. Aims for command-line and on-disk compatibility with `npryce/adr-tools`.

### Commands (installed as console_scripts via `setup.py`)

- **`adr-init [DIRECTORY]`** — creates the ADR directory (default `doc/adr/`); writes a `.adr-dir` marker if a non-default location is used; auto-creates ADR 0001 "record-architecture-decisions".
- **`adr-new [-s SUPERSEDED] [-l TARGET:LINK:REVERSE-LINK] TITLE...`** — creates a numbered ADR. Supports multiple `-s` and `-l` flags. Same CLI surface as the Bash version.
- **`adr-list`** — prints all ADR file paths in sorted order.
- **`adr-config`** — prints the resolved config as `key="value"` lines (mirrors the Bash version's `eval`-friendly output).
- **`-v` / `--verbose`** flag is available on every command.

### Internal helper functions (`adr_func/adr_util.py`)

- **`adr_config()`** — returns a dict with `adr_bin_dir` and `adr_template_dir` (both resolve to the package directory). Replaces the original Bash `eval "$(adr-config)"` pattern.
- **`adr_init(config, localpath, dirname)`** — creates the ADR directory, writes `.adr-dir` if non-default, and seeds the first ADR.
- **`adr_new(config, localpath, title, superseded, links)`** — copies `template.md` to a new `NNNN-title.md`, rewrites the header/date/status, and processes `-s` / `-l` link arguments by calling `_adr_add_link` and `_adr_remove_status`.
- **`adr_write_number_and_header(dst, idx, title)`** — rewrites lines 1, 3, 7 of the new file in place to set the title, today's date, and the `Accepted` status.
- **`adr_list(dir)`** — returns a sorted list of full ADR paths under the resolved ADR directory.
- **`_adr_dir()`** — walks up from CWD looking for `doc/adr/` or `.adr-dir`; falls back to `doc/adr/`. Returns a path relative to CWD.
- **`_adr_file(adr_text)`** — resolves a number or substring to a `(number, path)` tuple by searching `adr_list`.
- **`_adr_title(text)`** — reads the first line of an ADR and strips `# ` prefix and trailing newline.
- **`_adr_add_link(source, linktype, target)`** — inserts `linktype [Title](target.md)` into the `## Status` section of the source ADR via a `fileinput`-based state machine.
- **`_adr_remove_status(status, adr)`** — removes a status line (e.g. `Accepted`) from the `## Status` section. The author's own comment calls it "a very ugly state machine, based on the original awk".
- **`find_alternate_dir(dir)`** — reads `.adr-dir` and returns the configured directory.
- **`set_adr_verbosity(b)` / `get_adr_verbosity()` / `adr_print(text)`** — module-level verbosity flag implemented as a single-element list (mutable singleton); `adr_print` only prints when verbose is on.

### Comparison with adr-tools

| Capability                          | adr-tools (this repo)                        | adr-tools-python                          |
|-------------------------------------|----------------------------------------------|-------------------------------------------|
| Implementation                      | Bash + awk                                   | Python 3 (argparse, fileinput, shutil)    |
| Distribution                        | Manual / Homebrew                            | `pip install adr-tools-python`            |
| Commands                            | `init`, `new`, `link`, `list`, `generate`, `upgrade-repository`, `help`, `config` | `init`, `new`, `list`, `config` |
| `adr link` standalone command       | Yes                                          | No (linking only via `adr-new -l`)        |
| `adr generate toc`                  | Yes                                          | No                                        |
| `adr generate graph`                | Yes                                          | No                                        |
| `adr upgrade-repository`            | Yes                                          | No                                        |
| `adr help` subsystem                | Yes (per-command help, autocomplete)         | No (only argparse `--help`)               |
| Per-project `.adr-dir` marker       | Yes                                          | Yes (compatible)                          |
| Per-project template override       | Yes (`templates/template.md`)                | No (uses bundled `adr_func/template.md`)  |
| Supersede rewrites prior status     | Yes (removes `Accepted`, adds back-link)     | Yes (mirrors Bash behavior)               |
| Multiple `-s` / `-l` flags          | Yes                                          | Yes                                       |
| Opens new ADR in `$EDITOR`/`$VISUAL`| Yes                                          | No (writes file and exits)                |
| Verbose / debug mode                | No                                           | Yes (`-v`)                                |
| Filename format                     | `NNNN-kebab-title.md`                        | `NNNN-kebab-title.md` (compatible)        |
