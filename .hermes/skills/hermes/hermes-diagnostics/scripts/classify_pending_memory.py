#!/usr/bin/env python3
"""Classify ~/.hermes/pending/memory/*.json records against the CURRENT store.

Diagnostics step for Aphrodite-enabled sessions (dev-aphrodite profile):
for every staged record, simulate each op against the live MEMORY.md/USER.md
entries (whole-entry exact match first, else substring; distinct matches =
ambiguous) and print STALE / WOULD-APPLY verdicts per op, plus per-record and
total classes. Decision rule: records whose ops reference entry text that no
longer exists are stale - approving them either fails or REVERTS newer entries
to older drafts. Drop stale records; never bulk-approve a backlog.

Pure stdlib, read-only: it never writes memory or the pending store.
When pending records are listed through session tools in a compressed
session, expect <<<CCR:...>>> markers and resolve them with
aphrodite_retrieve(hash) before classifying.
Usage: python3 classify_pending_memory.py [pending_dir]
"""

import glob
import json
import os
import sys
from pathlib import Path

HOME = Path.home()
PENDING_DIR = Path(sys.argv[1]) if len(sys.argv) > 1 else HOME / ".hermes/pending/memory"
MEM_PATH = HOME / ".hermes/memories/MEMORY.md"
USER_PATH = HOME / ".hermes/memories/USER.md"
DELIM = "\n§\n"


def entries(p: Path):
    if not p.exists():
        return []
    return [e for e in (x.strip() for x in p.read_text(encoding="utf-8-sig").split(DELIM)) if e]


def find_unique(entries, old_text):
    """Mirror MemoryStore._find_unique_match: (idx, ambiguous)."""
    exact = [i for i, e in enumerate(entries) if e == old_text]
    matches = exact if exact else [i for i, e in enumerate(entries) if old_text in e]
    if len({entries[i] for i in matches}) > 1:
        return None, True
    return (matches[0] if matches else None), False


def dry_run(ops, target, mem, usr):
    working = list(mem if target == "memory" else usr)
    results = []
    for i, op in enumerate(ops or []):
        act = op.get("action")
        content = (op.get("content") or op.get("new_text") or "").strip()
        old = (op.get("old_text") or "").strip()
        if act == "add":
            if not content:
                results.append(f"op{i + 1} add: FAIL empty content")
            elif content in working:
                results.append(f"op{i + 1} add: noop (duplicate exists)")
            else:
                results.append(f"op{i + 1} add: WOULD ADD ({len(content)} chars)")
                working.append(content)
        else:
            idx, amb = find_unique(working, old)
            if amb:
                results.append(f"op{i + 1} {act}: FAIL ambiguous")
            elif idx is None:
                results.append(f"op{i + 1} {act}: FAIL no match (stale)")
            else:
                replaced = working[idx]
                if act == "replace":
                    results.append(
                        f"op{i + 1} replace: WOULD REPLACE ({len(replaced)}->{len(content)} chars)"
                    )
                    working[idx] = content
                else:
                    results.append(f"op{i + 1} remove: WOULD REMOVE ({len(replaced)} chars)")
                    working.pop(idx)
    return results


def main():
    mem, usr = entries(MEM_PATH), entries(USER_PATH)
    files = sorted(glob.glob(str(PENDING_DIR / "*.json")))
    if not files:
        print(f"no pending records in {PENDING_DIR}")
        return
    print(f"current memory: {len(mem)} entries, user: {len(usr)} entries; pending: {len(files)}\n")
    counts = {"ALL-STALE": 0, "PARTIAL": 0, "CLEAN": 0}
    for f in files:
        rec = json.load(open(f))
        payload = rec.get("payload", {})
        target = payload.get("target", "memory")
        ops = (
            payload.get("operations")
            if payload.get("operations") is not None
            else (
                [
                    {
                        "action": payload.get("action"),
                        "content": payload.get("content"),
                        "old_text": payload.get("old_text"),
                    }
                ]
                if payload.get("action")
                else None
            )
        )
        res = dry_run(ops, target, mem, usr)
        fails = [r for r in res if "FAIL" in r]
        tag = (
            "ALL-STALE"
            if (ops and all("FAIL" in r for r in res))
            else ("PARTIAL" if fails else "CLEAN")
        )
        counts[tag] += 1
        print(
            f"{os.path.basename(f)[:-5]} [{target}] {tag} ({len(ops or [])} ops, {len(fails)} stale)"
        )
        for r in res:
            print(f"    {r}")
    print(f"\n=== COUNTS === {counts}")


if __name__ == "__main__":
    main()
