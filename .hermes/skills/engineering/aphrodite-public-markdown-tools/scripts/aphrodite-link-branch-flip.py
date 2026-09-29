#!/usr/bin/env python3
"""Flip <PROJECT> tree/Current URLs to tree/Development in the
working tree's public markdown (Development's docs link to Development).

Literal replace only; asserts every occurrence is inside a
<PROJECT>/tree/Current URL; refuses on drift. Repo root derived from
this script's location (no hardcoded <WORKSPACE> paths).
"""

import subprocess
import sys
from pathlib import Path

PROJECT = "PlayForm/Aphrodite"
REPO = Path(__file__).resolve().parents[5]
OLD = f"github.com/{PROJECT}/tree/Current/"
NEW = f"github.com/{PROJECT}/tree/Development/"

def git(*args):
    return subprocess.run(
        ["git", "-C", REPO, *args], capture_output=True, text=True, check=True
    ).stdout

dev_tree = set(git("ls-tree", "-r", "--name-only", "Development").splitlines())

files = ["README.md", "CHANGELOG.md",
         "crates/aphrodite/README.md", "crates/aphrodite-hermes/README.md"]
files += sorted(f for f in dev_tree if f.startswith("docs/") and f.endswith(".md"))

total = 0
for path in files:
    with open(f"{REPO}/{path}", encoding="utf-8") as f:
        text = f.read()
    hits = text.count(OLD)
    if not hits:
        continue
    # sanity: no occurrences outside the full URL form
    stray = text.replace(OLD, "").count("tree/Current")
    if stray:
        print(f"REFUSE {path}: {stray} stray tree/Current occurrences outside {PROJECT} URLs")
        sys.exit(1)
    with open(f"{REPO}/{path}", "w", encoding="utf-8", newline="") as f:
        f.write(text.replace(OLD, NEW))
    total += hits
    print(f"  {path}: {hits} flipped")

print(f"TOTAL: {total} links flipped to tree/Development")