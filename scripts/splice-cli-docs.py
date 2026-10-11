#!/usr/bin/env python3
"""Splice `dadrock gen-cli-docs` output into the Software page.

Reads the generated Markdown on stdin, demotes its headings so the commands
sit under the page's "Command-line reference" section, and replaces whatever
is between the two marker comments in the target file.
"""

import re
import sys
from pathlib import Path

START = "<!-- cli-docs:start -->"
END = "<!-- cli-docs:end -->"


def demote(markdown: str) -> str:
    lines = markdown.splitlines()
    out = []
    for line in lines:
        if line.startswith("# "):
            continue  # the generated title; the page has its own heading
        if line.startswith("This document contains the help content"):
            continue
        if line.startswith("## "):
            line = "#" + line
        out.append(line)
    text = "\n".join(out).strip("\n")
    return re.sub(r"\n{3,}", "\n\n", text) + "\n"


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: splice-cli-docs.py <target.md> < generated.md", file=sys.stderr)
        return 2
    target = Path(sys.argv[1])
    page = target.read_text()
    if START not in page or END not in page:
        print(f"{target}: missing {START} / {END} markers", file=sys.stderr)
        return 1
    generated = demote(sys.stdin.read())
    head, rest = page.split(START, 1)
    _, tail = rest.split(END, 1)
    target.write_text(f"{head}{START}\n\n{generated}\n{END}{tail}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
