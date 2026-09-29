#!/usr/bin/env python3
"""Inventory relative/partial links in Development's public markdown.

Reads from git (Development ref), extracts markdown links, classifies them,
and checks whether relative targets exist on the branch. Prints a compact
table. Read-only - no writes. Repo root derived from this script's location.
"""

import re
import subprocess
import sys
from pathlib import Path

PROJECT = "PlayForm/Aphrodite"
REF = "Development"
REPO = Path(__file__).resolve().parents[5]

def git(*args):
    return subprocess.run(
        ["git", "-C", REPO, *args],
        capture_output=True, text=True, check=True,
    ).stdout

def tree_files():
    return set(git("ls-tree", "-r", "--name-only", REF).splitlines())

files_of_interest = [
    "README.md", "CHANGELOG.md", "aphrodite.toml.example",
    "crates/aphrodite/README.md", "crates/aphrodite-hermes/README.md",
]
files_of_interest += sorted(
    f for f in tree_files() if f.startswith("docs/") and f.endswith(".md")
)

link_re = re.compile(r"\]\(([^)]+)\)")
tree = tree_files()
rel_count = 0
abs_tree_count = 0
abs_other_count = 0
rows = []

for path in files_of_interest:
    if path not in tree:
        continue
    content = git("show", f"{REF}:{path}")
    for m in link_re.finditer(content):
        target = m.group(1).strip()
        # strip optional title, anchors, query
        base = target.split(" ")[0].split("#")[0].split("?")[0]
        if base.startswith(("http://", "https://")):
            if f"github.com/{PROJECT}/tree/Current/" in base:
                abs_tree_count += 1
            else:
                abs_other_count += 1
            continue
        rel_count += 1
        # resolve relative to the file's dir
        if base.startswith("/"):
            resolved = base.lstrip("/")
        else:
            resolved = path.rsplit("/", 1)[0] + "/" + base if "/" in path else base
            resolved = resolved.replace("/./", "/")
            # collapse ../
            parts = []
            for p in resolved.split("/"):
                if p == "..":
                    if parts:
                        parts.pop()
                elif p == ".":
                    continue
                else:
                    parts.append(p)
            resolved = "/".join(parts)
        exists = "OK " if resolved in tree else "MISSING"
        rows.append((path, target, resolved, exists))

print(f"total relative links: {rel_count} | abs tree/Current: {abs_tree_count} | abs other: {abs_other_count}")
print(f"{'file':<45} {'link':<55} {'resolved':<45} {'status'}")
for path, target, resolved, exists in rows:
    print(f"{path:<45} {target:<55} {resolved:<45} {exists}")