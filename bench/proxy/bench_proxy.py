#!/usr/bin/env python3
"""End-to-end proxy benchmark for aphrodite (stdlib only).

Starts target/release/aphrodite on override ports 19797 (cache) / 19798
(token) using bench/proxy/bench.toml, waits for /health, then for every
corpus file x both proxies measures:

  - compression ratio  (original bytes in vs hash/marker bytes out, from
    POST /ccr/create's documented wire contract)
  - p50/p95/p99 create latency over N iterations
  - retrieve round-trip correctness (create -> hash -> POST /retrieve ->
    byte-identical?)  mismatches are recorded as FINDINGS, not crashes
  - /metrics counter deltas across the file's whole iteration batch

Writes machine-readable JSON to bench/results/ (timestamped) and prints a
human summary table. Kills the spawned proxy on exit (atexit + signal).
"""

import atexit
import datetime
import json
import os
import signal
import statistics
import subprocess
import sys
import time
import urllib.error
import urllib.request

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.abspath(os.path.join(HERE, "..", ".."))
BINARY = os.path.join(REPO, "target", "release", "aphrodite")
CORPUS = os.path.join(REPO, "bench", "corpus")
RESULTS = os.path.join(REPO, "bench", "results")
TMP = os.path.join(RESULTS, "tmp")
ITERATIONS = int(os.environ.get("BENCH_ITERATIONS", "50"))
PROXIES = {"cache": 19797, "token": 19798}
METRIC_KEYS = (
    "aphrodite_ccr_created_total",
    "aphrodite_ccr_hits_total",
    "aphrodite_ccr_misses_total",
    "aphrodite_requests_compressed_total",
    "aphrodite_tokens_saved_total",
    "aphrodite_ccr_store_entries",
)

proc = None


def cleanup():
    global proc
    if proc and proc.poll() is None:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()
            proc.wait()
    proc = None


def http(method, port, path, body=None, timeout=30):
    url = f"http://127.0.0.1:{port}{path}"
    data = None
    headers = {}
    if body is not None:
        data = json.dumps(body).encode()
        headers["Content-Type"] = "application/json"
    req = urllib.request.Request(url, data=data, headers=headers, method=method)
    try:
        with urllib.request.urlopen(req, timeout=timeout) as r:
            return r.status, r.read()
    except urllib.error.HTTPError as e:
        return e.code, e.read()


def wait_health(port, deadline=20.0):
    t0 = time.time()
    while time.time() - t0 < deadline:
        try:
            status, raw = http("GET", port, "/health", timeout=2)
            if status == 200:
                return json.loads(raw)
        except (urllib.error.URLError, OSError, TimeoutError):
            time.sleep(0.2)
    raise RuntimeError(f"proxy on :{port} never became healthy")


def scrape_metrics(port):
    _, raw = http("GET", port, "/metrics")
    out = {}
    for line in raw.decode().splitlines():
        parts = line.split()
        if len(parts) == 2:
            name = parts[0].split("{")[0]
            if name in METRIC_KEYS:
                try:
                    out[name] = out.get(name, 0) + float(parts[1])
                except ValueError:
                    pass
    return out


def pct(sorted_vals, p):
    if not sorted_vals:
        return None
    k = min(len(sorted_vals) - 1, max(0, round(p / 100 * (len(sorted_vals) - 1))))
    return sorted_vals[k]


def main():
    if not os.path.exists(BINARY):
        sys.exit(f"missing binary: {BINARY} - build with `cargo build --release -p aphrodite`")

    os.makedirs(TMP, exist_ok=True)
    db_path = os.path.join(TMP, "ccr-bench.db")
    for suffix in ("", "-wal", "-shm"):
        try:
            os.remove(db_path + suffix)
        except FileNotFoundError:
            pass

    # Render config with the absolute sqlite path.
    with open(os.path.join(HERE, "bench.toml")) as f:
        cfg = f.read().replace("@CCR_DB_PATH@", db_path)
    rendered = os.path.join(TMP, "bench-rendered.toml")
    with open(rendered, "w") as f:
        f.write(cfg)

    global proc
    atexit.register(cleanup)
    for sig in (signal.SIGINT, signal.SIGTERM):
        signal.signal(sig, lambda *_: (cleanup(), sys.exit(130)))

    env = dict(os.environ, APHRODITE_CONFIG_PATH=rendered, RUST_LOG="warn")
    log_path = os.path.join(TMP, "proxy.log")
    with open(log_path, "w") as log:
        proc = subprocess.Popen([BINARY], cwd=TMP, env=env, stdout=log, stderr=log)

    health = {}
    for mode, port in PROXIES.items():
        health[mode] = wait_health(port)
        print(f"[bench] {mode} proxy healthy on :{port}: {health[mode]}")

    corpus = sorted(
        f
        for f in os.listdir(CORPUS)
        if os.path.isfile(os.path.join(CORPUS, f))
        and f != "gen_corpus.py"
        and not f.startswith(".")
    )
    findings = []
    results = []

    for fname in corpus:
        with open(os.path.join(CORPUS, fname), "rb") as f:
            raw = f.read()
        content = raw.decode("utf-8")

        for mode, port in PROXIES.items():
            rec = {"file": fname, "proxy": mode, "bytes_in": len(raw), "iterations": ITERATIONS}
            before = scrape_metrics(port)

            lat, hash_val, resp = [], None, None
            create_err = None
            for _ in range(ITERATIONS):
                t0 = time.perf_counter()
                status, body = http("POST", port, "/ccr/create", {"content": content})
                lat.append((time.perf_counter() - t0) * 1000)
                if status != 200:
                    create_err = f"HTTP {status}: {body[:200]!r}"
                    break
                resp = json.loads(body)
                hash_val = resp["hash"]
            if create_err:
                rec["create_error"] = create_err
                findings.append(f"{mode}/{fname}: /ccr/create failed: {create_err}")
                results.append(rec)
                continue

            lat.sort()
            rec.update(
                hash=hash_val,
                original_size=resp["original_size"],
                compressed_size=resp["compressed_size"],
                marker_size=resp["marker_size"],
                token_savings_ratio=resp["token_savings_ratio"],
                latency_ms={
                    "p50": round(pct(lat, 50), 3),
                    "p95": round(pct(lat, 95), 3),
                    "p99": round(pct(lat, 99), 3),
                    "mean": round(statistics.fmean(lat), 3),
                },
            )

            # Retrieve round-trip (limit=0 -> no pagination).
            status, body = http("POST", port, "/retrieve", {"hash": hash_val})
            if status != 200:
                rec["retrieve_error"] = f"HTTP {status}: {body[:200]!r}"
                findings.append(f"{mode}/{fname}: /retrieve failed: {rec['retrieve_error']}")
            else:
                got = json.loads(body).get("content")
                identical = got == content
                rec["roundtrip_identical"] = identical
                if not identical:
                    detail = "content is None"
                    if got is not None:
                        detail = f"len {len(got.encode('utf-8'))} vs {len(raw)}"
                        for i, (a, b) in enumerate(zip(got, content)):
                            if a != b:
                                detail += f"; first diff at char {i}: {a!r} vs {b!r}"
                                break
                    rec["roundtrip_detail"] = detail
                    findings.append(f"{mode}/{fname}: retrieve NOT byte-identical ({detail})")

            after = scrape_metrics(port)
            rec["metrics_delta"] = {
                k: after.get(k, 0) - before.get(k, 0)
                for k in METRIC_KEYS
                if k in after or k in before
            }
            results.append(rec)
            print(
                f"[bench] {mode:5s} {fname:24s} ratio={rec.get('token_savings_ratio', 0):9.1f}x "
                f"p50={rec['latency_ms']['p50']:7.2f}ms p95={rec['latency_ms']['p95']:7.2f}ms "
                f"roundtrip={'OK' if rec.get('roundtrip_identical') else 'FAIL'}"
            )

    cleanup()

    ts = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    out = {
        "timestamp": ts,
        "binary": BINARY,
        "iterations": ITERATIONS,
        "ports": PROXIES,
        "health": health,
        "results": results,
        "findings": findings,
    }
    out_path = os.path.join(RESULTS, f"proxy-bench-{ts}.json")
    with open(out_path, "w") as f:
        json.dump(out, f, indent=1)

    # Human summary.
    print("\n===== aphrodite proxy bench summary =====")
    print(
        f"{'proxy':6s} {'file':24s} {'in(B)':>8s} {'ratio':>10s} {'p50ms':>8s} {'p95ms':>8s} {'p99ms':>8s} {'rt':>4s}"
    )
    for r in results:
        if "create_error" in r:
            print(
                f"{r['proxy']:6s} {r['file']:24s} {r['bytes_in']:8d}  CREATE ERROR: {r['create_error']}"
            )
            continue
        lm = r["latency_ms"]
        rt = "OK" if r.get("roundtrip_identical") else "FAIL"
        print(
            f"{r['proxy']:6s} {r['file']:24s} {r['bytes_in']:8d} {r['token_savings_ratio']:9.1f}x "
            f"{lm['p50']:8.2f} {lm['p95']:8.2f} {lm['p99']:8.2f} {rt:>4s}"
        )
    for mode in PROXIES:
        rs = [r for r in results if r["proxy"] == mode and "latency_ms" in r]
        if rs:
            med_ratio = statistics.median(r["token_savings_ratio"] for r in rs)
            worst_p95 = max(r["latency_ms"]["p95"] for r in rs)
            print(
                f"[{mode}] median ratio {med_ratio:.1f}x, worst p95 {worst_p95:.2f}ms over {len(rs)} files"
            )
    if findings:
        print(f"\nFINDINGS ({len(findings)}):")
        for f_ in findings:
            print(f"  - {f_}")
    else:
        print("\nno correctness findings - all round-trips byte-identical")
    print(f"\nresults written to {out_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
