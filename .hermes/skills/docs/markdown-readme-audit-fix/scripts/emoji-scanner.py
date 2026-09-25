#!/usr/bin/env python3
"""
Pre-pass emoji audit for project README files.

Scans all project-owned README files and reports four types of emoji spacing issues:

  TYPE A - Regular ASCII space before emoji in a heading line.
           Fix: replace the space with an em-quad (U+2001 / &#x2001;).

  TYPE B - Double em-quad (two &#x2001; in a row).
           Fix: collapse to a single &#x2001;.

  TYPE C - Em-quad between a base emoji and its Fitzpatrick skin-tone modifier
           (e.g. &#x2001; between 🙏 and 🏻).
           Fix: remove the quad so the base and modifier are adjacent.

  TYPE D - Emoji appears on the LEFT of prose/diagram text.
           Fix: move the emoji to the RIGHT end of the text segment, preceded
           by a single em-quad.

Usage:
    python3 scripts/emoji-scanner.py [path/to/repo/root]
    # defaults to the current directory (run from the repo root)
"""

import os, re, sys

# ── Configuration ─────────────────────────────────────────────────────────────

# Primary README set (relative to repo root)
README_GLOBS = [
    "README.md",
    "docs/README.md",
] + [
    f"docs/{d}/README.md"
    for d in ("install", "config", "api", "proxy", "architecture", "plugin", "guides", "tool-relay")
]

# Sub-paths to exclude entirely
EXCLUDE_SUBSTRINGS = [
    "/vendor/",  # vendored submodule checkouts
    "/target/",  # cargo build output
    "/node_modules/",
    "/plugins/aphrodite/",  # plugin submodule
    "/.git/",
]

EM_QUAD = "\u2001"
EM_QUAD_ENTITY = "&#x2001;"

# Fitzpatrick modifiers U+1F3FB..U+1F3FF
FITZPATRICK_RE = re.compile("[\U0001f3fb\U0001f3fc\U0001f3fd\U0001f3fe\U0001f3ff]")

# Detect emoji (broad: any non-ASCII char outside common scripts)
# We use a simpler approach: look for the EM_QUAD_ENTITY + emoji pattern
EMOJI_CHARS_RE = re.compile(
    "[\U0001f300-\U0001f9ff\U00002600-\U000027bf\U0001f000-\U0001f02f"
    "\U0001f600-\U0001f64f\U0001f680-\U0001f6cf\U0001f900-\U0001f9ff"
    "\U00002300-\U000023ff\u2000-\u206f\u2100-\u214f]"
)


def should_skip(path: str) -> bool:
    return any(s in path for s in EXCLUDE_SUBSTRINGS)


def collect_readmes(repo_root: str) -> list[str]:
    results = []
    for rel in README_GLOBS:
        abs_path = os.path.join(repo_root, rel)
        if os.path.isfile(abs_path) and not should_skip(abs_path):
            results.append(abs_path)
    return sorted(results)


# ── Detection helpers ─────────────────────────────────────────────────────────


def check_type_a_diagram(line: str) -> bool:
    """Heading line with plain ASCII space before emoji: '# text 🚀'."""
    if line.startswith("#"):
        # Find space+emoji at end of heading text
        m = re.search(r" ([^\s])\s*$", line)
        if m:
            ch = m.group(1)
            if ch in EMOJI_CHARS_RE.pattern or ord(ch) > 0x1F300:
                return True
            # broad check: any high Unicode char that looks like emoji
            if ord(ch) > 0x2190:
                return True
    return False


def check_type_b(line: str) -> bool:
    """Double em-quad: &#x2001;&#x2001;."""
    return "&#x2001;&#x2001;" in line


def check_type_c(line: str) -> bool:
    """&#x2001; between base emoji and Fitzpatrick modifier."""
    # Normalise entity → actual char for the check
    norm = line.replace(EM_QUAD_ENTITY, EM_QUAD)
    # Look for: emoji_char + EM_QUAD + Fitzpatrick
    idx = 0
    while idx < len(norm):
        if norm[idx] == EM_QUAD:
            prev = norm[idx - 1] if idx > 0 else ""
            nxt = norm[idx + 1] if idx + 1 < len(norm) else ""
            if FITZPATRICK_RE.match(prev) and FITZPATRICK_RE.match(nxt):
                return True
        idx += 1
    return False


def check_type_d(line: str) -> bool:
    """Emoji LEFT of prose text in the same line (non-diagram context)."""
    # Skip mermaid / code fences pre-filter; caller must do that
    # Skip table rows and diagram node labels - they're handled separately
    stripped = line.strip()

    # Skip if this is a mermaid code line or table row
    if stripped.startswith("|") or stripped.startswith("graph "):
        return False
    # Skip classDef / subgraph / --> lines
    if re.match(r"^\s*(classDef|subgraph|-->|---|\.\.\.)\s", stripped):
        return False

    # Look for: ... text ... <space-or-emquad> <emoji> <text continues>
    # Stricter: emoji followed within reasonable distance by non-space prose
    # Normalise entity
    norm = stripped.replace(EM_QUAD_ENTITY, EM_QUAD)
    # Pattern: whitespace + emoji + whitespace + regular-text char
    m = re.search(
        r"[\s"
        + re.escape(EM_QUAD)
        + r"]+([\U0001F300-\U0001F9FF\U00002600-\U000027BF])[\s"
        + re.escape(EM_QUAD)
        + r"]+([^\s\]\)}\u2001&#<])",
        norm,
    )
    if m:
        return True
    return False


def check_type_d_mermaid(line: str) -> bool:
    """Emoji LEFT of text inside a mermaid node label or subgraph title."""
    stripped = line.strip()
    # Node labels: NodeName["..."] or subgraph "..."
    content_match = re.search(r'\["(.*?)"\]', stripped) or re.search(r'"([^"]*)"', stripped)
    if not content_match:
        return False
    label = content_match.group(1)
    norm = label.replace(EM_QUAD_ENTITY, EM_QUAD)
    # emoji before text
    m = re.search(
        r"([\U0001F300-\U0001F9FF\U00002600-\U000027BF])[\s"
        + re.escape(EM_QUAD)
        + r"]+([^\s\]\)}\)])",
        norm,
    )
    return bool(m)


# ── Main scan ─────────────────────────────────────────────────────────────────


def scan_file(path: str) -> list[dict]:
    issues = []
    with open(path, encoding="utf-8") as f:
        lines = f.readlines()

    in_mermaid = False
    for lineno, raw in enumerate(lines, 1):
        line = raw.rstrip("\n").rstrip("\r")

        if line.strip().startswith("```mermaid"):
            in_mermaid = True
            continue
        if line.strip().startswith("```") and in_mermaid:
            in_mermaid = False
            continue

        if not line:  # skip blank
            continue

        ctx = "mermaid" if in_mermaid else "prose"

        if ctx == "mermaid":
            if check_type_d_mermaid(line):
                issues.append({"type": "TYPE-D-MERMAID", "line": lineno, "text": line.strip()})
            # (TYPE A/B/C are not typical inside mermaid node labels)
        else:
            # prose
            if check_type_a_diagram(line):
                issues.append({"type": "TYPE-A", "line": lineno, "text": line.strip()})
            if check_type_b(line):
                issues.append({"type": "TYPE-B", "line": lineno, "text": line.strip()})
            if check_type_c(line):
                issues.append({"type": "TYPE-C", "line": lineno, "text": line.strip()})
            if check_type_d(line):
                issues.append({"type": "TYPE-D", "line": lineno, "text": line.strip()})

    return issues


def main():
    repo_root = sys.argv[1] if len(sys.argv) > 1 else os.getcwd()
    readmes = collect_readmes(repo_root)
    total_issues = 0
    for path in readmes:
        rel = os.path.relpath(path, repo_root)
        issues = scan_file(path)
        if issues:
            print(f"\n[FILE:{rel}] {len(issues)} issue(s):")
            for iss in issues:
                print(f"  L{iss['line']} ({iss['type']}): {iss['text'][:100]}")
                total_issues += 1

    if total_issues == 0:
        print("\n=== CLEAN - no issues found ===")
    else:
        print(
            f"\n=== DONE: {total_issues} issue(s) across {sum(1 for p in readmes if scan_file(p))} file(s) ==="
        )


if __name__ == "__main__":
    main()
