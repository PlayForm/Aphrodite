#!/usr/bin/env python3
"""Doc-rewrite final-pass link scan: wrong-branch links + dead relative links.

Usage: python3 doc-link-scan.py [repo_root]
Scans <root>/docs/**/*.md and <root>/README.md. Exit 0 = clean.
Prints DEAD/WRONG counts; nonzero exit if any found.

Aphrodite context: Development is the repo's working line, so
`tree/Development` is the correct link target for docs viewed from that
branch; the scan still reports every occurrence so a per-repo check can
confirm it (see the doc-rewrite pitfall in the skill's SKILL.md).
"""

import re
import sys
from pathlib import Path

root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.cwd()
targets = sorted((root / "docs").rglob("*.md")) + [root / "README.md"]

wrong = []
for p in targets:
    txt = p.read_text(errors="replace")
    for m in re.finditer(r"tree/Development", txt):
        # report line number, not char offset
        line = txt.count("\n", 0, m.start()) + 1
        wrong.append(f"{p}:{line}")

# Development is the Aphrodite working line, so the hardcoded scan target is
# correct for this repo; verify per repo anyway (`git remote show <origin>
# | grep HEAD`) - the skill's doc-rewrite pitfall treats a hardcoded branch
# that doesn't match the file's branch as the stale tell.

dead = []
for p in targets:
    txt = p.read_text(errors="replace")
    for m in re.finditer(r"\[[^\]]*\]\(([^)#]+)(#[^)]*)?\)", txt):
        target = m.group(1)
        if target.startswith(("http", "mailto:", "#")):
            continue
        if not (p.parent / target).resolve().exists():
            dead.append(f"{p}: {target}")

print(f"WRONG-BRANCH:{len(wrong)}")
for w in wrong[:30]:
    print(" ", w)
print(f"DEAD:{len(dead)}")
for d in dead[:30]:
    print(" ", d)

sys.exit(1 if (wrong or dead) else 0)
