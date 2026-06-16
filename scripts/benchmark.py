#!/usr/bin/env python3
"""Aphrodite proxy benchmark - direct HTTP against :9798.

Endpoints:
  GET  /health      → {"status":"ok","checks":{"cache":…,"token":…}}
  GET  /stats       → {"mode":"token","requests":{…},"ccr":{…}}
  GET  /ccr/list    → {"entries":N,"backend":"sqlite","mode":"token"}
  POST /ccr/create  → body {"content":"…"}  → {"hash":"…","token_savings_ratio":…}
  POST /retrieve    → body {"hash":"…"}      → {"found":true,"content":"…","source":"ccr"}
"""

import json
import os
import random
import statistics
import string
import sys
import time
import urllib.error
import urllib.request

PROXY = os.environ.get("APHRODITE_PROXY", "http://127.0.0.1:9798")
API_KEY = os.environ.get("APHRODITE_API_KEY", "")

RESULTS: list[dict] = []


def req(method: str, endpoint: str, body: dict | None = None, timeout: int = 30) -> dict:
    url = f"{PROXY}{endpoint}"
    data = json.dumps(body).encode() if body else None
    r = urllib.request.Request(
        url, data=data,
        headers={"Content-Type": "application/json", "Authorization": f"Bearer {API_KEY}"},
        method=method,
    )
    start = time.monotonic()
    try:
        with urllib.request.urlopen(r, timeout=timeout) as resp:
            raw = resp.read()
            elapsed = time.monotonic() - start
            return {"body": json.loads(raw.decode()) if raw else {}, "status": resp.status, "elapsed": elapsed, "ok": True}
    except urllib.error.HTTPError as e:
        elapsed = time.monotonic() - start
        return {"body": e.read().decode(errors="replace"), "status": e.code, "elapsed": elapsed, "ok": False}
    except Exception as e:
        elapsed = time.monotonic() - start
        return {"body": str(e), "status": 0, "elapsed": elapsed, "ok": False}


def make_text(size: int) -> str:
    words = "the quick brown fox jumps over lazy dog".split()
    chunks = []
    remaining = size
    while remaining > 0:
        w = random.choice(words)
        chunks.append(w)
        remaining -= len(w) + 1
    return " ".join(chunks)[:size]


def make_code(size: int) -> str:
    lines, remaining = [], size
    snippets = [
        "fn process(data: &str) -> Result<(), Error> {",
        "    let x = data.parse::<u64>()?;",
        "    if x > threshold { return Err(Error::Overflow); }",
        "    cache.insert(key.clone(), Value::Compressed(ratio));",
        "    // TODO: handle edge case where ratio is NaN",
        "    Ok(())",
        "}",
    ]
    while remaining > 0:
        line = (" " * random.randint(0, 4)) + random.choice(snippets)
        lines.append(line)
        remaining -= len(line) + 1
    return "\n".join(lines)[:size]


def make_json(size: int) -> str:
    items, remaining = [], size
    while remaining > 0:
        key = "".join(random.choices(string.ascii_lowercase, k=8))
        val = "".join(random.choices(string.ascii_letters + string.digits, k=20))
        item = f'  "{key}": "{val}"'
        items.append(item)
        remaining -= len(item) + 2
    return "{\n" + ",\n".join(items) + "\n}" if items else "{}"


# ═══════════════════════════════════════════════════════════════════
# Benchmark phases
# ═══════════════════════════════════════════════════════════════════

def phase_health():
    r = req("GET", "/health", timeout=5)
    ok = r["ok"] and isinstance(r["body"], dict)
    detail = f"token={'alive' if ok else '?'}"
    if ok:
        checks = r["body"].get("checks", {})
        t = checks.get("token", {})
        detail = f"cache={checks.get('cache',{}).get('status','?')} token={t.get('status','?')}"
    return {"name": "health", "latency_ms": round(r["elapsed"]*1000, 1), "pass": ok, "status": r["status"], "detail": detail}


def phase_stats():
    r = req("GET", "/stats", timeout=5)
    ok = r["ok"] and isinstance(r["body"], dict)
    detail = ""
    if ok:
        b = r["body"]
        detail = f"mode={b.get('mode','?')} requests={b.get('requests',{}).get('total',0)} compressed={b.get('requests',{}).get('compressed',0)} tokens_saved={b.get('tokens_saved',0)} ccr_hits={b.get('ccr',{}).get('hits',0)} ccr_created={b.get('ccr',{}).get('created',0)}"
    return {"name": "stats", "latency_ms": round(r["elapsed"]*1000, 1), "pass": ok, "detail": detail}


def phase_compress(label: str, generator, iterations: int = 3):
    latencies, ratios, hashes = [], [], []
    orig_size = 1
    for _ in range(iterations):
        content = generator()
        orig_size = len(content.encode())
        r = req("POST", "/ccr/create", body={"content": content}, timeout=30)
        latencies.append(r["elapsed"] * 1000)
        if r["ok"] and isinstance(r["body"], dict):
            h = r["body"].get("hash", "")
            if h:
                hashes.append(h)
                cr = r["body"].get("token_savings_ratio", 0)
                ratios.append(cr)

    return {"name": f"compress/{label}", "iterations": len(latencies), "latency_ms": round(statistics.mean(latencies),1) if latencies else 0,
            "latency_p50": round(statistics.median(latencies),1) if latencies else 0,
            "latency_p95": round(sorted(latencies)[int(len(latencies)*0.95)] if len(latencies)>=2 else (latencies[0] if latencies else 0),1),
            "ratio_mean": round(statistics.mean(ratios),1) if ratios else 0, "ratio_median": round(statistics.median(ratios),1) if ratios else 0,
            "original_size": orig_size, "hashes": hashes, "pass": len(hashes) > 0}


def phase_retrieve(hashes: list[str]):
    latencies, found, errors = [], 0, 0
    sample = hashes[:min(10, len(hashes))]
    for h in sample:
        r = req("POST", "/retrieve", body={"hash": h}, timeout=30)
        latencies.append(r["elapsed"] * 1000)
        if r["ok"] and isinstance(r["body"], dict) and r["body"].get("found"):
            found += 1
        else:
            errors += 1
    return {"name": f"retrieve x{len(sample)}", "iterations": len(sample), "found": found, "errors": errors,
            "latency_ms": round(statistics.mean(latencies),1) if latencies else 0,
            "latency_p50": round(statistics.median(latencies),1) if latencies else 0,
            "latency_p95": round(sorted(latencies)[int(len(latencies)*0.95)] if len(latencies)>=2 else (latencies[0] if latencies else 0),1),
            "pass": errors == 0}


def phase_catalog():
    r = req("GET", "/ccr/list", timeout=30)
    ok = r["ok"] and isinstance(r["body"], dict)
    entries = r["body"].get("entries", 0) if ok else 0
    return {"name": "catalog", "latency_ms": round(r["elapsed"]*1000, 1), "entry_count": entries, "pass": ok}


# ═══════════════════════════════════════════════════════════════════
def main():
    print("═" * 70)
    print(" APHRODITE BENCHMARK v2")
    print(f" Proxy: {PROXY}  |  {time.strftime('%Y-%m-%d %H:%M:%S')}")
    print("═" * 70)

    # ── Phase 1: Health + Stats ──
    print("\n── Phase 1: Proxy ──")
    for fn in [phase_health, phase_stats]:
        r = fn()
        RESULTS.append(r)
        tag = "✓" if r["pass"] else "✗"
        print(f"  {tag} {r['name']:<10} {r['latency_ms']:>7.1f}ms  {r.get('detail','')}")

    # ── Phase 2: Compression across sizes and types ──
    print("\n── Phase 2: Compression ──")
    sizes = {"1KB": 1024, "10KB": 10240, "50KB": 51200, "100KB": 102400, "500KB": 512000}
    types = {"text": make_text, "code": make_code, "json": make_json}
    all_hashes = []

    for slabel, sbytes in sizes.items():
        for tname, genfn in types.items():
            iters = 5 if sbytes <= 10240 else 3
            r = phase_compress(f"{slabel}/{tname}", lambda sz=sbytes, fn=genfn: fn(sz), iters)
            RESULTS.append(r)
            all_hashes.extend(r["hashes"])
            tag = "✓" if r["pass"] else "✗"
            print(f"  {tag} {r['name']:<26} avg={r['latency_ms']:>7.1f}ms  p50={r['latency_p50']:>7.1f}ms  ratio={r['ratio_mean']:>8.1f}x  ({r['original_size']}B→hash)")
    print(f"  → {len(all_hashes)} hashes stored")

    # ── Phase 3: Retrieve ──
    print("\n── Phase 3: Retrieve ──")
    if all_hashes:
        r = phase_retrieve(all_hashes)
        RESULTS.append(r)
        tag = "✓" if r["pass"] else "✗"
        print(f"  {tag} {r['name']:<26} avg={r['latency_ms']:>7.1f}ms  p50={r['latency_p50']:>7.1f}ms  p95={r['latency_p95']:>7.1f}ms  found={r['found']}/{r['iterations']}")
    else:
        print("  SKIP (no hashes)")

    # ── Phase 4: Catalog ──
    print("\n── Phase 4: Catalog ──")
    r = phase_catalog()
    RESULTS.append(r)
    tag = "✓" if r["pass"] else "✗"
    print(f"  {tag} {r['name']:<26} latency={r['latency_ms']:>7.1f}ms  entries={r['entry_count']}")

    # ── Summary ──
    passes = sum(1 for r in RESULTS if r["pass"])
    total = len(RESULTS)
    comp_lats = [r["latency_ms"] for r in RESULTS if r["name"].startswith("compress/") and r["latency_ms"] > 0]
    retr_lats = [r["latency_ms"] for r in RESULTS if r["name"].startswith("retrieve") and r["latency_ms"] > 0]
    ratios = [r["ratio_mean"] for r in RESULTS if r.get("ratio_mean", 0) > 0]

    print("\n" + "═" * 70)
    print(" SUMMARY")
    print("═" * 70)
    print(f"  Results:      {passes}/{total} passed ({total-passes} failed)")
    if comp_lats:
        print(f"  Compress avg: {statistics.mean(comp_lats):.1f}ms  min={min(comp_lats):.1f}ms  max={max(comp_lats):.1f}ms")
    if retr_lats:
        print(f"  Retrieve avg: {statistics.mean(retr_lats):.1f}ms  min={min(retr_lats):.1f}ms  max={max(retr_lats):.1f}ms")
    if ratios:
        print(f"  Ratio range:  {min(ratios):.0f}x - {max(ratios):.0f}x (median {statistics.median(ratios):.0f}x)")

    # ── Previous run comparison ──
    hist_path = os.path.abspath(os.path.join(os.path.dirname(__file__) or ".", "..", ".hermes", "benchmark-history.jsonl"))
    prev = None
    if os.path.exists(hist_path):
        try:
            with open(hist_path) as f:
                lines = [l for l in f if l.strip()]
            if lines:
                prev = json.loads(lines[-1])
        except Exception:
            prev = None
    if prev:
        ps = prev.get("summary", {})
        print(f"  Previous run: {prev.get('timestamp','?')}")
        if comp_lats and ps.get("compress_avg_ms") is not None:
            d = statistics.mean(comp_lats) - ps["compress_avg_ms"]
            print(f"  Compress Δ:   {d:+.1f}ms vs previous")
        if retr_lats and ps.get("retrieve_avg_ms") is not None:
            d = statistics.mean(retr_lats) - ps["retrieve_avg_ms"]
            print(f"  Retrieve Δ:   {d:+.1f}ms vs previous")

    # Save timestamped result & append to cumulative history
    ts_iso = time.strftime("%Y-%m-%dT%H:%M:%S")
    ts_safe = time.strftime("%Y-%m-%dT%H-%M-%S")
    base = os.path.abspath(os.path.join(os.path.dirname(__file__) or ".", "..", ".hermes"))
    os.makedirs(base, exist_ok=True)
    out_json = os.path.join(base, f"benchmark-{ts_safe}.json")
    run_data = {"timestamp": ts_iso, "proxy": PROXY,
                "summary": {"total": total, "passed": passes, "failed": total-passes,
                             "compress_avg_ms": round(statistics.mean(comp_lats),1) if comp_lats else None,
                             "retrieve_avg_ms": round(statistics.mean(retr_lats),1) if retr_lats else None},
                "results": RESULTS}
    with open(out_json, "w") as f:
        json.dump(run_data, f, indent=2)
    print(f"\n  Results → {out_json}")

    # Append a compact entry to the cumulative JSONL history
    hist_entry = {"timestamp": ts_iso, "proxy": PROXY, "summary": run_data["summary"],
                  "file": f"benchmark-{ts_safe}.json"}
    with open(hist_path, "a") as f:
        f.write(json.dumps(hist_entry) + "\n")
    print(f"  History → {hist_path}")
    return 0 if passes == total else 1


if __name__ == "__main__":
    sys.exit(main())
