#!/usr/bin/env python3
"""
Aphrodite benchmark report - live binary latency + machine-readable JSON.

Sibling of benchmark-eval.py. Adds what the pure-python eval cannot measure:
  - per-corpus and overall compression ratio (raw bytes vs marker bytes,
    reported by the REAL binary's /ccr/create response)
  - median / p95 end-to-end latency of the real binary's compress path
    (POST /ccr/create on a spawned cache-mode proxy, wall-clock)
  - per-content-type breakdown using the same content_detector labels
    the binary uses (json_array, source_code, search, build, diff, html,
    tabular, structured_config, text)
  - a machine-readable JSON summary written under
    ~/Developer/.playform/Temporary/hermes/benchmark-results.json (NOT the repo;
    override with BENCH_RESULTS_DIR)

Defensive by design (repo convention): if the aphrodite binary is missing or
the proxy cannot start, latency is reported as an explicit
"SKIPPED (binary unavailable)" - never a fabricated number. Missing optional
deps (requests/matplotlib) degrade to stdlib urllib; nothing is installed.

Usage:
  python3 benchmark-report.py [--json-only] [--skip-live]
"""

from __future__ import annotations

import json
import os
import shutil
import socket
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

# Import the shared corpus logic from the sibling evaluator (filename has a
# hyphen, so load via runpy instead of a regular import).
SCRIPT_DIR = Path(__file__).resolve().parent
import runpy  # noqa: E402

_eval_ns = runpy.run_path(str(SCRIPT_DIR / "benchmark-eval.py"))
REPO_ROOT = _eval_ns["REPO_ROOT"]
classify = _eval_ns["classify"]
load_corpus = _eval_ns["load_corpus"]
marker_bytes_estimate = _eval_ns["marker_bytes_estimate"]

RESULTS_DIR = Path(
    os.environ.get(
        "BENCH_RESULTS_DIR", str(Path.home() / "Developer" / ".playform" / "Temporary" / "hermes")
    )
)
RESULTS_FILE = RESULTS_DIR / "benchmark-results.json"

BENCH_PORT = 59797
SKIP_REASON_BINARY = "SKIPPED (binary unavailable)"
SKIP_REASON_START = "SKIPPED (proxy failed to start)"


# ── HTTP: requests when present, stdlib urllib fallback (no installs) ────────
try:
    import requests  # type: ignore[import-not-found]

    _HTTP_BACKEND = "requests"
except ImportError:
    import urllib.request  # type: ignore[import-not-found]

    _HTTP_BACKEND = "urllib"


def http_post_json(url: str, payload: dict, timeout: float = 30.0) -> dict:
    """POST JSON, return parsed dict, or {"error": ...} on any failure."""
    body = json.dumps(payload).encode("utf-8")
    if _HTTP_BACKEND == "requests":
        try:
            resp = requests.post(url, json=payload, timeout=timeout)  # type: ignore[attr-defined]
            return (
                resp.json()
                if resp.status_code < 300
                else {"error": f"HTTP {resp.status_code}: {resp.text[:200]}"}
            )
        except Exception as e:  # noqa: BLE001
            return {"error": str(e)}
    try:
        req = urllib.request.Request(  # type: ignore[attr-defined]
            url, data=body, headers={"Content-Type": "application/json"}, method="POST"
        )
        with urllib.request.urlopen(req, timeout=timeout) as r:  # type: ignore[attr-defined]
            return json.loads(r.read().decode("utf-8"))
    except Exception as e:  # noqa: BLE001
        return {"error": str(e)}


def resolve_binary() -> str | None:
    """Find the aphrodite binary: APHRODITE_BIN env, then target dirs."""
    env_bin = os.environ.get("APHRODITE_BIN")
    if env_bin and Path(env_bin).exists():
        return env_bin
    for profile in ("release", "debug"):
        cand = REPO_ROOT / "target" / profile / "aphrodite"
        if cand.exists():
            return str(cand)
    return None


def binary_version(bin_path: str) -> str:
    try:
        out = subprocess.run([bin_path, "--version"], capture_output=True, text=True, timeout=10)
        if out.returncode == 0 and out.stdout.strip():
            return out.stdout.strip().splitlines()[0]
    except Exception:  # noqa: BLE001
        pass
    mtime = datetime.fromtimestamp(Path(bin_path).stat().st_mtime, tz=timezone.utc)
    return f"built {mtime.isoformat()}"


def wait_ready(port: int, timeout: float = 10.0) -> bool:
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.5):
                return True
        except OSError:
            time.sleep(0.1)
    return False


def start_proxy(bin_path: str) -> subprocess.Popen | None:
    """Spawn a cache-mode proxy on an isolated bench port. None on failure."""
    env = os.environ.copy()
    # Isolate from any repo aphrodite.toml that would override CLI flags.
    env["APHRODITE_CONFIG_PATH"] = "/nonexistent/aphrodite-bench.toml"
    try:
        proc = subprocess.Popen(
            [
                bin_path,
                "--mode",
                "cache",
                "--listen",
                f"127.0.0.1:{BENCH_PORT}",
                "--api-url",
                "http://127.0.0.1:1",
                "--api-key",
                "bench",
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env=env,
        )
    except Exception as e:  # noqa: BLE001
        print(f"  [warn] proxy spawn failed: {e}")
        return None
    if not wait_ready(BENCH_PORT):
        try:
            proc.kill()
        except Exception:  # noqa: BLE001
            pass
        return None
    if proc.poll() is not None:
        # Child exited (e.g. port already bound by a foreign listener) - we
        # would otherwise be measuring a proxy we don't own. Fail safe.
        return None
    return proc


def stop_proxy(proc: subprocess.Popen | None) -> None:
    if proc is None:
        return
    try:
        proc.terminate()
        proc.wait(timeout=5)
    except Exception:  # noqa: BLE001
        try:
            proc.kill()
        except Exception:  # noqa: BLE001
            pass


def pct(values: list[float], q: float) -> float:
    if not values:
        return 0.0
    s = sorted(values)
    k = (len(s) - 1) * q
    lo = int(k)
    hi = min(lo + 1, len(s) - 1)
    frac = k - lo
    return s[lo] + (s[hi] - s[lo]) * frac


def main() -> None:
    import argparse

    ap = argparse.ArgumentParser(description="Aphrodite benchmark report (live binary + JSON)")
    ap.add_argument("--json-only", action="store_true", help="print only the JSON summary")
    ap.add_argument(
        "--skip-live", action="store_true", help="skip the live binary latency measurement"
    )
    args = ap.parse_args()

    corpora = load_corpus()
    samples = []
    for corpus, items in corpora.items():
        for s in items:
            samples.append({**s, "corpus": corpus})

    bin_path = resolve_binary()
    binary_available = bin_path is not None and not args.skip_live
    proxy = None
    if binary_available:
        print(f"[report] binary: {bin_path}")
        print(f"[report] starting cache-mode proxy on :{BENCH_PORT} (mode=cache)")
        proxy = start_proxy(bin_path)
        if proxy is None:
            binary_available = False
            print(f"[report] {SKIP_REASON_START}")

    live_entries = []
    per_corpus_lat: dict[str, list[float]] = {}
    per_type_lat: dict[str, list[float]] = {}
    errors = []

    try:
        for idx, s in enumerate(samples):
            raw = s["content"]
            raw_bytes = len(raw.encode("utf-8", errors="replace"))
            ctype = classify(raw)
            s["ctype"] = ctype
            s["raw_bytes"] = raw_bytes
            entry = {
                "corpus": s["corpus"],
                "name": s["name"],
                "type": ctype,
                "raw_bytes": raw_bytes,
                "marker_bytes": marker_bytes_estimate(raw, ctype),
                "latency_ms": None,
                "binary_ratio": None,
                "binary_marker_bytes": None,
                "token_savings_ratio": None,
                "status": "ok",
            }
            if binary_available and proxy is not None:
                t0 = time.perf_counter()
                resp = http_post_json(
                    f"http://127.0.0.1:{BENCH_PORT}/ccr/create",
                    {"content": raw, "type": ctype},
                    timeout=30.0,
                )
                elapsed_ms = (time.perf_counter() - t0) * 1000.0
                if "error" in resp:
                    entry["status"] = "error"
                    entry["error"] = resp["error"]
                    errors.append(f"{s['corpus']}:{s['name']}: {resp['error']}")
                else:
                    entry["latency_ms"] = round(elapsed_ms, 3)
                    entry["binary_marker_bytes"] = resp.get("marker_size")
                    entry["token_savings_ratio"] = resp.get("token_savings_ratio")
                    ms = resp.get("marker_size") or 0
                    entry["binary_ratio"] = round(raw_bytes / ms, 3) if ms else None
                    per_corpus_lat.setdefault(s["corpus"], []).append(elapsed_ms)
                    per_type_lat.setdefault(ctype, []).append(elapsed_ms)
                print(
                    f"  {s['corpus']:16} {s['name']:28} {raw_bytes:7,}B → marker "
                    f"{resp.get('marker_size') if 'error' not in resp else 'ERR':>4} "
                    f"({elapsed_ms:7.2f} ms)"
                )
            else:
                entry["status"] = "skipped"
                entry["skip_reason"] = (
                    SKIP_REASON_BINARY if binary_available is False else "skip_live"
                )
            live_entries.append(entry)
    finally:
        stop_proxy(proxy)

    # ── Aggregate ─────────────────────────────────────────────────────────
    corpora_agg = {}
    for corpus in sorted({s["corpus"] for s in live_entries}):
        cs = [s for s in live_entries if s["corpus"] == corpus]
        rb = sum(s["raw_bytes"] for s in cs)
        mb = sum(s["marker_bytes"] for s in cs)
        mb_bin = sum(
            s["binary_marker_bytes"] for s in cs if s.get("binary_marker_bytes") is not None
        )
        lat = [s["latency_ms"] for s in cs if s["latency_ms"] is not None]
        corpora_agg[corpus] = {
            "samples": len(cs),
            "raw_bytes": rb,
            "marker_bytes": mb,
            "binary_marker_bytes": mb_bin or None,
            "ratio": round(rb / mb, 3) if mb else None,
            "binary_ratio": round(rb / mb_bin, 3) if mb_bin else None,
            "latency": _latency_summary(lat, corpus, binary_available),
        }

    types_agg = {}
    for ctype in sorted({s["type"] for s in live_entries}):
        ts = [s for s in live_entries if s["type"] == ctype]
        rb = sum(s["raw_bytes"] for s in ts)
        mb = sum(s["marker_bytes"] for s in ts)
        mb_bin = sum(
            s["binary_marker_bytes"] for s in ts if s.get("binary_marker_bytes") is not None
        )
        lat = [s["latency_ms"] for s in ts if s["latency_ms"] is not None]
        types_agg[ctype] = {
            "samples": len(ts),
            "raw_bytes": rb,
            "marker_bytes": mb,
            "binary_marker_bytes": mb_bin or None,
            "ratio": round(rb / mb, 3) if mb else None,
            "binary_ratio": round(rb / mb_bin, 3) if mb_bin else None,
            "latency": _latency_summary(lat, ctype, binary_available),
        }

    total_rb = sum(s["raw_bytes"] for s in live_entries)
    total_mb = sum(s["marker_bytes"] for s in live_entries)
    total_mb_bin = sum(
        s["binary_marker_bytes"] for s in live_entries if s.get("binary_marker_bytes") is not None
    )
    total_lat = [s["latency_ms"] for s in live_entries if s["latency_ms"] is not None]
    overall = {
        "samples": len(live_entries),
        "raw_bytes": total_rb,
        "marker_bytes": total_mb,
        "binary_marker_bytes": total_mb_bin or None,
        "ratio": round(total_rb / total_mb, 3) if total_mb else None,
        "binary_ratio": round(total_rb / total_mb_bin, 3) if total_mb_bin else None,
        "latency": _latency_summary(total_lat, "overall", binary_available),
    }

    summary = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "tool": "benchmark-report.py",
        "http_backend": _HTTP_BACKEND,
        "binary": {
            "path": bin_path,
            "available": binary_available,
            "version": binary_version(bin_path) if bin_path else None,
            "mode": "cache",
            "endpoint": "/ccr/create",
        },
        "content_types_present": sorted({s["ctype"] for s in samples}),
        "corpora": corpora_agg,
        "per_content_type": types_agg,
        "overall": overall,
        "samples": live_entries,
        "errors": errors,
        "notes": [
            "compression ratio = raw_bytes / marker_bytes (marker format <<<CCR:40hex|type|size>>>)",
            "binary marker_size / token_savings_ratio come from the real /ccr/create response; the binary reports marker_size=40 (the BLAKE3 key length), so binary_ratio = raw_bytes / 40 is the stored-bytes ratio per the binary's own metric",
            "latency = wall-clock of POST /ccr/create to the spawned real binary (cache mode), median/p95 over samples",
            "pure-python marker_bytes use the exact repo marker format; only live mode has real BLAKE3 hashes",
        ],
    }

    RESULTS_DIR.mkdir(parents=True, exist_ok=True)
    with open(RESULTS_FILE, "w", encoding="utf-8") as f:
        json.dump(summary, f, indent=2, ensure_ascii=False)
        f.write("\n")

    # ── Human report ──────────────────────────────────────────────────────
    if not args.json_only:
        print()
        print("═" * 78)
        print("  Aphrodite Benchmark Report - live binary (cache mode)")
        print("═" * 78)
        print()
        print("| Corpus | Samples | Raw (B) | Marker (B) | Ratio | Lat med (ms) | Lat p95 (ms) |")
        print("|--------|---------|---------|------------|-------|--------------|--------------|")
        for corpus, c in corpora_agg.items():
            lat = c["latency"]
            ratio = c["binary_ratio"] if c["binary_ratio"] is not None else c["ratio"]
            print(
                f"| {corpus:20} | {c['samples']:7} | {c['raw_bytes']:7,} | {c['marker_bytes']:10,} | "
                f"{ratio if ratio is not None else '-':>5} | "
                f"{_fmt_lat(lat['median_ms'], lat['status']):>12} | {_fmt_lat(lat['p95_ms'], lat['status']):>12} |"
            )
        ratio = overall["binary_ratio"] if overall["binary_ratio"] is not None else overall["ratio"]
        print(
            f"| {'**TOTAL**':20} | {overall['samples']:7} | {overall['raw_bytes']:7,} | {overall['marker_bytes']:10,} | "
            f"{ratio if ratio is not None else '-':>5} | "
            f"{_fmt_lat(overall['latency']['median_ms'], overall['latency']['status']):>12} | "
            f"{_fmt_lat(overall['latency']['p95_ms'], overall['latency']['status']):>12} |"
        )
        print()
        print(f"## Content Types in Corpus ({len(types_agg)}): {', '.join(sorted(types_agg))}")
        print()
        print("| Type | Samples | Raw (B) | Marker (B) | Ratio | Lat med (ms) | Lat p95 (ms) |")
        print("|------|---------|---------|------------|-------|--------------|--------------|")
        for ctype, t in sorted(types_agg.items()):
            lat = t["latency"]
            ratio = t["binary_ratio"] if t["binary_ratio"] is not None else t["ratio"]
            print(
                f"| {ctype:16} | {t['samples']:7} | {t['raw_bytes']:7,} | {t['marker_bytes']:10,} | "
                f"{ratio if ratio is not None else '-':>5} | "
                f"{_fmt_lat(lat['median_ms'], lat['status']):>12} | {_fmt_lat(lat['p95_ms'], lat['status']):>12} |"
            )
        print()
        if errors:
            print(f"## Errors ({len(errors)})")
            for e in errors:
                print(f"  - {e}")
        print()
        print(f"JSON summary: {RESULTS_FILE}")
    else:
        print(json.dumps(summary, indent=2, ensure_ascii=False))


def _latency_summary(lat: list[float], label: str, binary_available: bool) -> dict:
    if not lat:
        return {
            "median_ms": None,
            "p95_ms": None,
            "count": 0,
            "status": SKIP_REASON_BINARY if binary_available is False else "no samples measured",
        }
    return {
        "median_ms": round(statistics.median(lat), 3),
        "p95_ms": round(pct(lat, 0.95), 3),
        "count": len(lat),
        "status": "measured",
    }


def _fmt_lat(v: float | None, status: str) -> str:
    if v is not None:
        return f"{v:.2f}"
    return (
        status.replace("SKIPPED (", "").replace(")", "") if status.startswith("SKIPPED") else status
    )


if __name__ == "__main__":
    main()
