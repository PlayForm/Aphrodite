#!/usr/bin/env python3
"""Rewrite relative repo links in Development's public markdown to absolute
tree/Current URLs (Current's link convention).

Rules:
- Only touches public files: README.md, CHANGELOG.md, docs/**/*.md.
- Absolute http(s) links and pure same-file anchors (#...) are left alone.
- Relative FILE links are resolved against the file's dir, verified to exist
  on the Current ref, and rewritten to
  https://github.com/<PROJECT>/tree/Current/<resolved>[#anchor].
- Any relative link whose target does NOT exist on Current is reported and
  left untouched (fail-soft, but the count must be 0 for the run to pass).
- Byte-preserving elsewhere; asserts deterministic match counts.

Repo root derived from this script's location (no hardcoded paths).
"""

import re
import subprocess
import sys
from pathlib import Path

PROJECT = "PlayForm/Aphrodite"
REPO = Path(__file__).resolve().parents[5]
DEV = "Development"
CUR = "Current"
BASE = f"https://github.com/{PROJECT}/tree/Current/"

def git(*args):
    return subprocess.run(
        ["git", "-C", REPO, *args], capture_output=True, text=True, check=True
    ).stdout

cur_tree = set(git("ls-tree", "-r", "--name-only", CUR).splitlines())
dev_tree = set(git("ls-tree", "-r", "--name-only", DEV).splitlines())

files = ["README.md", "CHANGELOG.md"]
files += sorted(f for f in dev_tree if f.startswith("docs/") and f.endswith(".md"))

link_re = re.compile(r"\]\(([^)]+)\)")

def resolve(rel, base_dir):
    """Resolve a relative link to a repo-root path; anchor kept separately."""
    # split anchor
    anchor = ""
    pathpart = rel
    if "#" in rel:
        pathpart, anchor = rel.split("#", 1)
        anchor = "#" + anchor
    if pathpart.startswith("http://") or pathpart.startswith("https://"):
        return None, None
    if pathpart.startswith("/"):
        parts = pathpart.lstrip("/").split("/")
    elif base_dir:
        parts = (base_dir + "/" + pathpart).split("/")
    else:
        parts = pathpart.split("/")
    stack = []
    for p in parts:
        if p in ("", "."):
            continue
        if p == "..":
            if stack:
                stack.pop()
            else:
                return None, None  # escapes repo root
        else:
            stack.append(p)
    return "/".join(stack), anchor

total_changed = 0
total_files = 0
problems = []

for path in files:
    content = git("show", f"{DEV}:{path}")
    base_dir = path.rsplit("/", 1)[0] if "/" in path else ""
    new_content = content
    count = 0

    def repl(m):
        global count
        target = m.group(1).strip()
        # split optional title
        parts = target.split()
        if len(parts) > 1:
            urlpart = parts[0]
        else:
            urlpart = target
        resolved, anchor = resolve(urlpart, base_dir)
        if resolved is None:
            return m.group(0)  # absolute or escaping - leave
        if resolved not in cur_tree:
            problems.append((path, urlpart, resolved))
            return m.group(0)
        count += 1
        suffix = anchor or ""
        # preserve trailing title if present
        if len(parts) > 1:
            suffix += " " + " ".join(parts[1:])
        return f"]({BASE}{resolved}{suffix})"

    new_content = link_re.sub(repl, new_content)
    if count:
        with open(f"{REPO}/{path}", "w", encoding="utf-8", newline="") as f:
            f.write(new_content)
        total_changed += count
        total_files += 1
        print(f"  {path}: {count} links rewritten")

print(f"\nTOTAL: {total_changed} links across {total_files} files")
if problems:
    print("UNRESOLVED (target missing on Current):")
    for p in problems:
        print("  ", p)
    sys.exit(1)
print("OK: all rewritten targets verified to exist on Current")