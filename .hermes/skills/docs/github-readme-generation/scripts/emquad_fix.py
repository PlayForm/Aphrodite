#!/usr/bin/env python3
"""Fix heading em-quads (U+2001) in README files.

The patch/write_file tool transport may silently remap U+2001 (EM QUAD) to
U+2003 (EM SPACE) on write - and inconsistently: the same input can survive
one write and be remapped on the next. This script restores the em-quad
header convention (reference README style) with line-based, regex-free
surgery: for every heading line, each space (or U+2003) adjacent to an emoji
run is replaced with U+2001.

Handles BOTH shapes:
  - trailing emoji run:  ## Install ⚡               (space BEFORE the run)
  - mid-line emoji run:  # Aphrodite 💋 Hermes Plugin (spaces on BOTH sides)
Emoji detection covers U+2600-U+27BF and U+FE0F - VS16 variants like
\u2699\ufe0f defeat naive >0x1F000 checks.

Usage: python3 emquad_fix.py [path ...]   (default: README.md)
"""

import sys

EM_QUAD = "\u2001"


def is_emoji(c: str) -> bool:
    o = ord(c)
    return o > 0x1F000 or 0x2600 <= o <= 0x27BF or c == "\ufe0f"


def fix(path: str) -> int:
    with open(path, encoding="utf-8") as fh:
        lines = fh.read().split("\n")
    fixed = 0
    out = []
    for line in lines:
        if line.startswith("#") and len(line) > 3:
            chars = list(line)
            i = 0
            while i < len(chars):
                if is_emoji(chars[i]):
                    j = i
                    while j < len(chars) and is_emoji(chars[j]):
                        j += 1
                    if i > 1 and chars[i - 1] in (" ", "\u2003"):
                        chars[i - 1] = EM_QUAD
                        fixed += 1
                    if j < len(chars) and chars[j] in (" ", "\u2003"):
                        chars[j] = EM_QUAD
                        fixed += 1
                    i = j
                else:
                    i += 1
            line = "".join(chars)
        out.append(line)
    with open(path, "w", encoding="utf-8") as fh:
        fh.write("\n".join(out))
    return fixed


def main() -> int:
    paths = sys.argv[1:] or ["README.md"]
    total = 0
    for p in paths:
        n = fix(p)
        print(f"{p}: fixed {n} separator(s)")
        total += n
    return 0 if total else 1


if __name__ == "__main__":
    sys.exit(main())
