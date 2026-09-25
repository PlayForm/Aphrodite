#!/usr/bin/env python3
"""
dash-normalizer.py - scan .md files for irregular Unicode dashes and curly quotes.

Usage:
    python3 dash-normalizer.py                  # scan mode (check), print findings, exit 1 if any
    python3 dash-normalizer.py --check          # same as bare
    python3 dash-normalizer.py --check --full   # full 24-char hook regex set
    python3 dash-normalizer.py --check --strict mode=repo
    python3 dash-normalizer.py --check --strict mode=ranges   # broadest, include generated/vendor trees
    python3 dash-normalizer.py --help

This script is read-only. It does NOT modify any files. Use it with `patch` tool for edits.
"""

import argparse, os, re, sys

BASE = os.getcwd()  # default: run from the repo root
RANGES = {
    "target",
    "vendor",
    "node_modules",
    "build",
    "dist",
}
RANGE_SKIP_RE = re.compile(r"/(?:target|vendor|node_modules|build|dist)/")
GENERATED_SKIP_RE = re.compile(r"(?:CHANGELOG|LICENSE|README-lib|\.md\.orig|CHANGELOG\.md)$")

# Minimal 6-char set (always scanned)
ALL_MINIMAL = {
    "\u2014": "EM_DASH",
    "\u2013": "EN_DASH",
    "\u2018": "L_SQ",
    "\u2019": "R_SQ",
    "\u201c": "L_DQ",
    "\u201d": "R_DQ",
}

# Full 24-char set (normalize-dashes hook: ~/.hermes/agent-hooks/normalize-dashes.sh)
FULL_RANGES = [
    (0x058A, 0x058A),
    (0x05BE, 0x05BE),
    (0x1400, 0x1400),
    (0x1806, 0x1806),
    (0x2010, 0x2015),
    (0x2E17, 0x2E17),
    (0x2E1A, 0x2E1A),
    (0x2E3A, 0x2E3B),
    (0x2E40, 0x2E40),
    (0x2E5D, 0x2E5D),
    (0x301C, 0x301C),
    (0x3030, 0x3030),
    (0x30A0, 0x30A0),
    (0xFE31, 0xFE32),
    (0xFE58, 0xFE58),
    (0xFE63, 0xFE63),
    (0xFF0D, 0xFF0D),
]
ALL_FULL = {}
for lo, hi in FULL_RANGES:
    for cp in range(lo, hi + 1):
        try:
            ch = chr(cp)
            ALL_FULL[ch] = f"U+{cp:04X}"
        except ValueError:
            pass
ALL_FULL.update({k: v for k, v in ALL_MINIMAL.items() if k not in ALL_FULL})


def should_skip(path):
    if RANGE_SKIP_RE.search(path):
        return True
    if GENERATED_SKIP_RE.search(path):
        return True
    return False


def scan(char_set, label, skip_ranges=True):
    issues = []
    total = 0
    skip_re = RANGE_SKIP_RE if skip_ranges else None
    for root, dirs, files in os.walk(BASE):
        dirs.sort()
        for fname in sorted(files):
            if not fname.endswith(".md"):
                continue
            fpath = os.path.join(root, fname)
            if skip_re and skip_re.search(fpath):
                continue
            if GENERATED_SKIP_RE.search(fpath):
                continue
            rel = os.path.relpath(fpath, os.getcwd())
            try:
                with open(fpath, "r", encoding="utf-8", errors="strict") as f:
                    lines = f.readlines()
            except (UnicodeDecodeError, PermissionError):
                continue
            file_hits = []
            for lineno, line in enumerate(lines, 1):
                hits = [(ch, lbl) for ch, lbl in char_set.items() if ch in line]
                if hits:
                    file_hits.append((lineno, hits, line.rstrip()))
                    for _, lbl in hits:
                        total += 1
            if file_hits:
                issues.append((rel, file_hits))
    if issues:
        for rel, fhits in issues:
            for lineno, hits, line in fhits:
                hit_names = ", ".join(lbl for _, lbl in hits)
                print(f"{rel}:{lineno} [{hit_names}]: {line[:255]}")
        return 1
    print(f"No irregular characters found ({label}).")
    return 0


def scan_ranges(args):
    rc = scan(args.get("chars", ALL_FULL), args.get("label", "RANGES"), skip_ranges=False)
    return rc


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument(
        "--full", action="store_true", help="use full 24-char hook regex instead of minimal 6-char"
    )
    p.add_argument("--check", action="store_true", help="scan-only (default)")
    p.add_argument(
        "--ranges", action="store_true", help="scan generated/vendor directories too (broader)"
    )
    args = p.parse_args()

    if args.ranges:
        mode = {
            "chars": ALL_FULL if args.full else ALL_MINIMAL,
            "label": "FULL-HOOK+RANGES" if args.full else "MINIMAL+RANGES",
        }
        sys.exit(scan_ranges(mode))
    elif args.full:
        sys.exit(scan(ALL_FULL, "FULL-HOOK"))
    else:
        sys.exit(scan(ALL_MINIMAL, "MINIMAL"))


if __name__ == "__main__":
    main()
