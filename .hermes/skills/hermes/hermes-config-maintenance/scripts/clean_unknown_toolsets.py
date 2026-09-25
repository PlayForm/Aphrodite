#!/usr/bin/env python3
"""Clean dead/unknown toolset references from all Hermes config files.

Removes a set of "dead" toolset names from:
  - platform_toolsets.<platform>
  - known_plugin_toolsets.<platform>
across ~/.hermes/config.yaml and every ~/.hermes/profiles/*/config.yaml
(including the dev-aphrodite dev profile).

Typical use: after detaching the Aphrodite plugin or migrating the Hermes
config format, sweep every config so no toolset allowlist keeps a dead name.
The script never touches `plugins.*` - `aphrodite` (and friends) legitimately
stay listed there as plugin entries.

The top-level config.yaml is edited in place; profile configs too. Run it
through the terminal tool. It is idempotent and asserts YAML still parses.

Usage:
  python3 clean_unknown_toolsets.py            # dry run (prints changes)
  python3 clean_unknown_toolsets.py --apply    # actually write
"""

import argparse
import glob
import os
import sys

DEAD = {"a2a", "aphrodite", "hermes-google_chat", "hermes-teams"}

BASE = os.path.expanduser("~/.hermes")
FILES = [os.path.join(BASE, "config.yaml")] + sorted(
    glob.glob(os.path.join(BASE, "profiles", "*", "config.yaml"))
)


def clean_text(text):
    """Remove dead entries from the two toolset keys via line surgery."""
    lines = text.split("\n")
    out = []
    i = 0
    n = len(lines)
    current_key = None  # "platform_toolsets" / "known_plugin_toolsets"
    while i < n:
        line = lines[i]
        stripped = line.strip()
        if stripped in ("platform_toolsets:", "known_plugin_toolsets:"):
            current_key = stripped[:-1]
            out.append(line)
            i += 1
            continue
        # leave the key when we hit another top-level mapping
        if current_key and line and not line.startswith(" ") and stripped:
            current_key = None
        if current_key and stripped.startswith("- "):
            val = stripped[2:].strip()
            if val in DEAD:
                i += 1
                continue
        out.append(line)
        i += 1
    return "\n".join(out)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--apply", action="store_true", help="write changes (default: dry run)")
    args = ap.parse_args()

    for f in FILES:
        if not os.path.exists(f):
            continue
        before = open(f).read()
        after = clean_text(before)
        if after == before:
            print(f"unchanged : {f}")
            continue
        # sanity: must still parse as YAML-ish (basic check)
        try:
            import yaml  # type: ignore

            yaml.safe_load(after)
        except Exception as e:  # pragma: no cover
            print(f"YAML ERROR in {f}: {e}", file=sys.stderr)
            sys.exit(1)
        print(f"{'WOULD EDIT' if not args.apply else 'EDITED    '}: {f}")
        if args.apply:
            open(f, "w").write(after)


if __name__ == "__main__":
    main()
