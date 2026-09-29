#!/usr/bin/env python3
"""Port the 3 ruff VSCode keys from Current into Development's .vscode/settings.json.

Literal string replace (no regex). Preserves CRLF line endings of the working
tree (core.autocrlf=true checkout form). Refuses to run unless the anchor
snippet matches exactly once. Repo root derived from this script's location.
"""

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[5]
PATH = REPO / ".vscode/settings.json"

OLD = (
    '  "prettier.requireConfig": true,\n'
    '  "prettier.ignorePath": ".prettierignore",\n'
    "\n"
    '  "files.exclude": {'
)

NEW = (
    '  "prettier.requireConfig": true,\n'
    '  "prettier.ignorePath": ".prettierignore",\n'
    "  // Ruff keys ported from Current for editor parity (c5cbed8/589807a era).\n"
    "  // lineLength is redundant with ruff.toml (source of truth), kept for\n"
    "  // parity; organizeImports/showSyntaxErrors mirror Current's bindings.\n"
    '  "ruff.lineLength": 100,\n'
    '  "ruff.organizeImports": true,\n'
    '  "ruff.showSyntaxErrors": true,\n'
    "\n"
    '  "files.exclude": {'
)

with open(PATH, encoding="utf-8") as f:
    text = f.read()  # universal newlines -> LF in memory

hits = text.count(OLD)
if hits != 1:
    print(f"REFUSE: anchor matched {hits} times (expected exactly 1); file untouched", file=sys.stderr)
    sys.exit(1)

text = text.replace(OLD, NEW)

with open(PATH, "w", encoding="utf-8", newline="\r\n") as f:
    f.write(text)

print("OK: ruff keys inserted, CRLF preserved")