#!/usr/bin/env python3
"""
emoji-placement-check.py
========================
Scans project README files for emoji placed before text without em-quad separator.

Emoji must always appear at the RIGHT end of a text segment, preceded by em-quad (&#x2001;).
This script checks prose context - mermaid blocks and tables are excluded.

Usage:
    python3 scripts/emoji-placement-check.py   # scan the current directory (repo root)
    python3 scripts/emoji-placement-check.py [path/to/repo]
    echo $?   # 0=clean, 1=issues found
"""

import os, re, sys

EM_QUAD = "\u2001"

EMOJI_RE = re.compile(
    "[\U0001f300-\U0001f9ff\U00002600-\U000026ff\U00002700-\U000027bf"
    "\U0001fa00-\U0001faff\U0001f600-\U0001f64f\U0001f680-\U0001f6ff"
    "\U00002194-\U00002199]"
)
FITZ_RE = re.compile("[\U0001f3fb-\U0001f3ff]")

BASE = os.getcwd()  # default: run from the repo root
target = sys.argv[1] if len(sys.argv) > 1 else BASE

SKIP_DIRS = {
    "node_modules",
    ".git",
    "target",
    "vendor",
    "dist",
    "build",
}
SKIP_PAT = [
    r"/vendor/",
    r"/plugins/aphrodite/",
    r"CHANGELOG\.md$",
    r"LICENSE",
    r"\.md\.orig$",
]


def is_inside_mermaid(lines, lineno):
    in_m = False
    for i in range(lineno):
        if "```mermaid" in lines[i]:
            in_m = True
        elif in_m and lines[i].strip() == "```":
            in_m = False
    return in_m


def is_inside_table(lines, lineno):
    in_table = False
    for i in range(lineno):
        s = lines[i].strip()
        if s.startswith("|") and i > 0 and lines[i - 1].strip() != "":
            in_table = True
        elif s == "" and in_table:
            in_table = False
    return in_table


issues = {}
total = 0

for root, dirs, files in os.walk(target):
    dirs[:] = sorted(d for d in dirs if d not in SKIP_DIRS)
    for fname in sorted(files):
        if not fname.endswith(".md"):
            continue
        fpath = os.path.join(root, fname)
        skip = False
        for pat in SKIP_PAT:
            if re.search(pat, fpath):
                skip = True
                break
        if skip:
            continue
        try:
            with open(fpath, "r", encoding="utf-8", errors="strict") as f:
                file_lines = f.readlines()
        except (UnicodeDecodeError, PermissionError):
            continue
        file_hits = []
        for lineno, line in enumerate(file_lines, 1):
            if is_inside_mermaid(file_lines, lineno) or is_inside_table(file_lines, lineno):
                continue
            for m in EMOJI_RE.finditer(line):
                emoji_pos = m.start()
                if FITZ_RE.match(line, emoji_pos):
                    continue
                if emoji_pos == 0:
                    continue
                prev = line[emoji_pos - 1]
                if prev in (" ", "\t", "\n", ""):
                    continue  # space before emoji without EM_QUAD - acceptable
                if prev == EM_QUAD:
                    continue  # correct: em-quad precedes emoji
                file_hits.append(
                    (
                        lineno,
                        line.rstrip(),
                        f"emoji '{m.group()}' at pos {emoji_pos} preceded by '{prev}' (no EM_QUAD)",
                    )
                )
                total += 1
        if file_hits:
            rel = os.path.relpath(fpath, os.path.dirname(target))
            issues[rel] = file_hits

if total == 0:
    print("=== CLEAN: 0 emoji-placement issues found ===")
    sys.exit(0)
else:
    print(f"=== FOUND {total} issue(s) across {len(issues)} file(s) ===\n")
    for rel in sorted(issues):
        for lineno, line, msg in issues[rel]:
            print(f"  {rel}:{lineno}  {msg}")
            print(f"    {line[:230]}")
    sys.exit(1)
