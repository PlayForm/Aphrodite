#!/usr/bin/env python3
"""
Scan a Rust codebase for module-level TODO comments that haven't been
converted to structured sections yet. Module-level = within the first N
lines of each .rs file (default 25).
"""

import argparse, os, re, subprocess, sys
from pathlib import Path
from collections import defaultdict


def iter_rust_files(root: Path):
    """Yield all .rs files under root (excluding common vendor dirs)."""
    exclude = {".git", "target", "node_modules", "vendor", "Cargo.lock"}
    for dirpath, dirnames, filenames in os.walk(root):
        dirnames[:] = [d for d in dirnames if d not in exclude]
        for fn in filenames:
            if fn.endswith(".rs"):
                yield Path(dirpath) / fn


def count_module_todos(path: Path, threshold: int, hashtag_todos: bool) -> list[tuple[int, str]]:
    """Return (line_no, text) for any module-level TODO in the first `threshold` lines."""
    results = []
    try:
        with open(path, encoding="utf-8", errors="ignore") as fh:
            for i, line in enumerate(fh, 1):
                if i > threshold:
                    break
                stripped = line.rstrip()
                if hashtag_todos and re.search(r"//\s*TODO:", stripped):
                    results.append((i, stripped))
                if not hashtag_todos and re.search(r"//!\s*TODO:", stripped):
                    results.append((i, stripped))
    except Exception:
        pass
    return results


def main():
    ap = argparse.ArgumentParser(description="Find module-level TODO comments in Rust source")
    ap.add_argument(
        "root", nargs="?", default=".", help="Root directory to scan (default: current)"
    )
    ap.add_argument(
        "--lines", type=int, default=25, help="How many top lines to scan (default: 25)"
    )
    ap.add_argument(
        "--all",
        action="store_true",
        help="Match both //! TODO and // TODO (module-level heuristics)",
    )
    args = ap.parse_args()

    root = Path(args.root).resolve()
    found = defaultdict(list)

    for file in iter_rust_files(root):
        rel = file.relative_to(root)
        # Check module-level //! TODOs
        for lineno, text in count_module_todos(file, args.lines, hashtag_todos=False):
            found[str(rel)].append(f"  L{lineno}: {text.strip()}")
        if args.all:
            for lineno, text in count_module_todos(file, args.lines, hashtag_todos=True):
                found[str(rel)].append(f"  L{lineno}: {text.strip()}")

    if not found:
        print(
            "✅ No module-level TODO comments found in the first",
            args.lines,
            "lines of any .rs file",
        )
        return 0

    print(f"⚠️  Found module-level TODO comments in {len(found)} file(s):\n")
    for fname, lines in sorted(found.items()):
        print(f"  {fname}")
        for l in lines:
            print(l)
    print()
    return 1


if __name__ == "__main__":
    sys.exit(main())
