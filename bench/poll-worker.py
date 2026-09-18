#!/usr/bin/env python3
"""
Aphrodite session poll-worker - launch REAL Hermes sessions (actual LLM turns)
with the plugin active, monitor them live, and collect per-session compression
evidence.

Evidence sources (real, auditable):
  - session transcript (stdout saved per run) with `<<<CCR:` marker counts
  - ccr.db delta per session (entries + original bytes stored) - the engine's
    actual compression record
  - live proxy /metrics samples while the session runs (tool-relay path)

Defensive by design (repo convention): missing binary / unreachable proxy /
absent db yield explicit SKIPPED/ERROR - never a fabricated number. Stdlib-only.

Usage:
  python3 bench/poll-worker.py --sessions 1 --debug
  python3 bench/poll-worker.py --sessions 5 --prompt-file bench/conversational/prompts-session.txt
"""

import argparse
import json
import os
import re
import sqlite3
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
RESULTS_DIR = Path(
    os.environ.get("BENCH_RESULTS_DIR", REPO_ROOT / "bench" / "conversational" / "results")
)
PROXY_HEALTH = "http://127.0.0.1:9797/health"
PROXY_METRICS = "http://127.0.0.1:9797/metrics"
CCR_DB = Path(os.environ.get("APHRODITE_CCR_DB", Path.home() / ".hermes" / "aphrodite" / "ccr.db"))
MARKER_RE = re.compile(r"<<<CCR:[0-9a-f]{24,}\|[^|]+\|[^|]+>>>")

DEFAULT_PROMPTS = [
    "Run `ls -la` in the current directory and also `ls -laR bench/ | head -200`, then report the 10 largest files you see.",
    "Run `git log --oneline -50` in this repo and `git diff HEAD~1 --stat`, then summarize what the most recent commit changed.",
    "Read the ENTIRE file crates/aphrodite/src/proxy.rs (all lines) and summarize its main functions in under 150 words.",
    "Run `find bench/ -type f | head -50` and `wc -l bench/corpus/*`, then list the biggest corpus fixture and its line count.",
    "Read the full docs/ccr/marker-format.md and docs/ccr/content-types.md, then compare the marker hash format with the content-type taxonomy in under 150 words.",
]


def fetch(url: str, timeout: float = 3.0):  # noqa: UP007 - py3.9-compatible
    try:
        with urllib.request.urlopen(url, timeout=timeout) as r:
            return r.read().decode("utf-8", "replace")
    except Exception:
        return None


def parse_metrics(text):  # noqa: UP007 - py3.9-compatible
    if not text:
        return {"error": "metrics endpoint unreachable"}
    out: dict = {}
    for line in text.splitlines():
        if line.startswith("#"):
            continue
        key, _, val = line.partition(" ")
        for want in ("ccr", "requests_", "tokens_saved", "cache_"):
            if want in key:
                out[key] = val
    return out if out else {"error": "no CCR counters found"}


def db_stats() -> dict:
    """Read ccr.db entry count + total original bytes across known engine db
    paths (runtime home + dirs data dir). Best-effort: the session's in-process
    dylib may resolve its db elsewhere, in which case deltas stay 0 - the
    transcript marker count is the primary compression evidence."""
    paths = [CCR_DB, Path.home() / "Library" / "Application Support" / "aphrodite" / "ccr.db"]
    out: dict = {"paths": {str(p): "absent" for p in paths}}
    for p in paths:
        if not p.exists():
            continue
        try:
            con = sqlite3.connect(f"file:{p}?mode=ro", uri=True)
            row = con.execute(
                "SELECT count(*), COALESCE(sum(octet_length(original)),0) FROM ccr_entries"
            ).fetchone()
            con.close()
            out["paths"][str(p)] = {"entries": row[0], "original_bytes": row[1]}
        except Exception as e:
            out["paths"][str(p)] = f"unreadable: {e}"
    return out


def compute_db_delta(before: dict, after: dict) -> dict:
    """Per-path entry/byte deltas across both known db paths (best-effort)."""
    delta: dict = {}
    for p in before.get("paths", {}):
        b, a = before["paths"][p], after["paths"][p]
        if isinstance(b, dict) and isinstance(a, dict):
            delta[p] = {
                "entries": a.get("entries", 0) - b.get("entries", 0),
                "original_bytes": a.get("original_bytes", 0) - b.get("original_bytes", 0),
            }
    return delta if delta else {"note": "no db deltas (session db path may differ)"}


REAL_MARKER_RE = re.compile(r"<<<CCR:([0-9a-f]{24,})\|([a-z_]+)\|(\d+)>>>")


def session_db_markers(session_id: str) -> dict:
    """Query state.db for the session's messages and count REAL CCR markers
    (hex hash + concrete type + numeric size) - the authoritative evidence;
    the CLI transcript display omits tool-result markers."""
    try:
        con = sqlite3.connect(f"file:{Path.home() / '.hermes' / 'state.db'}?mode=ro", uri=True)
        rows = con.execute(
            "SELECT content FROM messages WHERE session_id=? AND content LIKE '%<<<CCR:%'",
            (session_id,),
        ).fetchall()
        con.close()
    except Exception as e:
        return {"error": f"state.db unreadable: {e}"}
    ms = [m for r in rows for m in REAL_MARKER_RE.findall(r[0] or "")]
    return {"markers": len(ms), "types": sorted({t for _, t, _ in ms})}


def run_session(prompt: str, debug: bool, index: int, total: int) -> dict:
    result = {"index": index, "prompt": prompt[:140], "status": "RUNNING", "started": time.time()}
    db_before = db_stats()
    cmd = ["hermes", "chat", "-q", prompt]
    env = {**os.environ, "TERMINAL_CWD": str(REPO_ROOT)}
    try:
        proc = subprocess.Popen(
            cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, env=env
        )
    except FileNotFoundError:
        result.update(status="ERROR", error="hermes binary not found")
        return result

    metrics_samples = []
    while proc.poll() is None:
        time.sleep(2.0)
        metrics_samples.append(parse_metrics(fetch(PROXY_METRICS)))
        if debug:
            print(
                f"  [poll {index}/{total}] alive {time.time() - result['started']:.0f}s", flush=True
            )

    out, err = proc.communicate(timeout=30)
    db_after = db_stats()
    # Markers can be line-wrapped by the 80-col CLI output - strip whitespace
    # before matching so wrapped markers still count.
    markers = MARKER_RE.findall(re.sub(r"\s+", "", out))
    transcript_path = RESULTS_DIR / f"transcript-{index:02d}.txt"
    transcript_path.write_text(out, encoding="utf-8", errors="replace")
    session_id = None
    m = re.search(r"Session:\s+(\S+)", out)
    if m:
        session_id = m.group(1)
    result.update(
        duration=round(time.time() - result["started"], 2),
        exit_code=proc.returncode,
        stdout_len=len(out),
        transcript_path=str(transcript_path),
        marker_count=len(markers),
        marker_types=sorted({m.split("|")[1] for m in markers}),
        session_id=session_id,
        session_db_markers=session_db_markers(session_id)
        if session_id
        else {"error": "no session id parsed"},
        db_before=db_before,
        db_after=db_after,
        db_delta=compute_db_delta(db_before, db_after),
        proxy_health=fetch(PROXY_HEALTH) or "unreachable",
        status="OK" if proc.returncode == 0 else "FAILED",
    )
    return result


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--sessions", type=int, default=1)
    ap.add_argument("--prompt-file", type=Path, default=None)
    ap.add_argument("--debug", action="store_true")
    args = ap.parse_args()

    prompts = DEFAULT_PROMPTS
    if args.prompt_file and args.prompt_file.exists():
        prompts = [p.strip() for p in args.prompt_file.read_text().splitlines() if p.strip()]
    if args.sessions > len(prompts):
        prompts = (prompts * ((args.sessions // len(prompts)) + 1))[: args.sessions]

    if not fetch(PROXY_HEALTH):
        print("SKIPPED: proxy not reachable on :9797 (plugin/proxy not running)", file=sys.stderr)
        return 2

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    runs = [
        run_session(p, args.debug, i + 1, args.sessions)
        for i, p in enumerate(prompts[: args.sessions])
    ]

    summary = {
        "tool": "poll-worker.py v2 (transcript + ccr.db + proxy)",
        "sessions": len(runs),
        "ok": sum(1 for r in runs if r["status"] == "OK"),
        "failed": sum(1 for r in runs if r["status"] == "FAILED"),
        "total_markers": sum(r.get("marker_count", 0) for r in runs),
        "total_db_entries_stored": sum(r.get("db_delta", {}).get("entries", 0) for r in runs),
        "avg_duration_s": round(sum(r.get("duration", 0) for r in runs) / max(len(runs), 1), 2),
        "runs": runs,
    }
    out_path = RESULTS_DIR / f"poll-worker-{time.strftime('%Y%m%d-%H%M%S')}.json"
    out_path.write_text(json.dumps(summary, indent=2))
    print(
        f"\n=== POLL-WORKER SUMMARY: {summary['ok']}/{summary['sessions']} OK, "
        f"{summary['total_markers']} markers, {summary['total_db_entries_stored']} entries stored, "
        f"avg {summary['avg_duration_s']}s -> {out_path}"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
