#!/usr/bin/env python3
"""Scan all mermaid fences in .md files for GitHub-render-breaking labels.

READ-ONLY: prints file:line for every risky unquoted node label; never edits.

Why: mermaid flowchart/state labels with unquoted braces are parsed as
shape tokens - `B[HintContext: {Code(rust)}]` dies on GitHub with
"Expecting ..., got 'DIAMOND_START'" (the `{` reads as a diamond start).
The fix is quoting: `B["HintContext: {Code(rust)}"]`. Non-ASCII in labels
(e.g. the x multiplication sign) is also risky. Run over the whole tree
after any diagram-touching doc wave - not just the files each child touched.

Usage:  python3 mermaid-render-lint.py [path ...]
        (defaults to ./ recursively; pass explicit files/dirs to limit)
Exit:   0 = clean, 1 = risky labels found.
"""

import re
import sys
import pathlib

# Unquoted node-label risk: X[label] where the label contains chars mermaid
# reads as shape/edge syntax, or any non-ASCII. Also flags --label--> edges.
LABEL_RE = re.compile(r'(?<!["\w])([A-Za-z0-9_]+)\[([^"]*?)\]')
EDGE_RE = re.compile(r"--([^>]*?)-->")
RISKY = re.compile(r"[{}()<>#=;]|[^\x00-\x7f]")


def scan(path: pathlib.Path) -> list:
    hits = []
    txt = path.read_text()
    for m in re.finditer(r"```mermaid\n(.*?)```", txt, re.S):
        # enumerate gives TRUE line numbers - never m.start()+i (that is a
        # character offset and reports nonsense lines for long files)
        for i, line in enumerate(m.group(1).splitlines(), 1):
            s = line.strip()
            if not s or s.startswith(("style ", "classDef ", "%%", "subgraph", "end")):
                continue
            for mm in LABEL_RE.finditer(s):
                if RISKY.search(mm.group(2)):
                    hits.append((path, m.start() + i, s[:110]))
            for mm in EDGE_RE.finditer(s):
                if RISKY.search(mm.group(1).strip()):
                    hits.append((path, m.start() + i, s[:110]))
    return hits


def main(argv: list) -> int:
    roots = [pathlib.Path(a) for a in argv] or [pathlib.Path(".")]
    files = []
    for r in roots:
        if r.is_file():
            files.append(r)
        else:
            files.extend(sorted(r.rglob("*.md")))
    hits = []
    for f in files:
        hits.extend(scan(f))
    for path, line, snippet in hits:
        print(f"{path}:{line}: {snippet}")
    print(f"{len(hits)} risky label(s) in {len(files)} file(s)")
    return 1 if hits else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
