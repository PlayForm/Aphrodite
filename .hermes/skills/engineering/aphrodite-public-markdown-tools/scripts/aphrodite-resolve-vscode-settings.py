#!/usr/bin/env python3
"""Resolve .vscode/settings.json squash conflict: keep Development's (HEAD)
structure as the base and graft in Current's unique [python] behavior
(editor.formatOnPaste: true + ruff explanatory comment).

Literal replaces only; asserts markers and anchor snippets exactly once.
Preserves CRLF line endings (autocrlf checkout form). Repo root derived
from this script's location.
"""

import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[5]
PATH = REPO / ".vscode/settings.json"
MARK_START = "<<<<<<< HEAD"
MARK_MID = "======="
MARK_END = ">>>>>>> Source/Current"

PY_OLD = (
    '  "[python]": {\n'
    '    "editor.defaultFormatter": "charliermarsh.ruff",\n'
    '    "editor.tabSize": 4,\n'
    '    "editor.insertSpaces": true\n'
    "  },\n"
)

PY_NEW = (
    "  // Python: use the repo's ruff (single formatter, explicit config in\n"
    "  // ruff.toml). This is the formatter CI's Check.yml uses, so Alt+F and CI\n"
    "  // produce identical output. Requires the Ruff VSCode extension\n"
    "  // (charliermarsh.ruff).\n"
    '  "[python]": {\n'
    '    "editor.defaultFormatter": "charliermarsh.ruff",\n'
    '    "editor.formatOnSave": true,\n'
    '    "editor.formatOnPaste": true,\n'
    '    "editor.tabSize": 4,\n'
    '    "editor.insertSpaces": true\n'
    "  },\n"
)

with open(PATH, encoding="utf-8") as f:
    text = f.read()  # universal newlines -> LF in memory

if text.count(MARK_START) != 1 or text.count(MARK_MID) != 1 or text.count(MARK_END) != 1:
    print("REFUSE: conflict markers not present exactly once; file untouched", file=sys.stderr)
    sys.exit(1)

head_body = text.split(MARK_START, 1)[1].split(MARK_MID, 1)[0]
incoming_body = text.split(MARK_MID, 1)[1].split(MARK_END, 1)[0]
tail = text.split(MARK_END, 1)[1]  # "\n}" (closing brace line)

if head_body.count(PY_OLD) != 1:
    print(f"REFUSE: python block anchor matched {head_body.count(PY_OLD)} times", file=sys.stderr)
    sys.exit(1)

head_body = head_body.replace(PY_OLD, PY_NEW)

# The conflict markers sit after "{\n"; strip the leading newline from tail.
merged = "{" + head_body + tail.lstrip("\n")
if merged.count("<<<<<<<") or merged.count(">>>>>>>") or merged.count("======="):
    print("REFUSE: markers remain in merged output", file=sys.stderr)
    sys.exit(1)

with open(PATH, "w", encoding="utf-8", newline="\r\n") as f:
    f.write(merged)

print("OK: merged - Development structure + Current's python formatOnPaste/comment")