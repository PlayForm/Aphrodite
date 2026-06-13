#!/usr/bin/env python3
"""
Proxy compression benchmark — uses headroom's own /stats for ground truth.

Launch:  bash scripts/proxy-launch.sh [cache|token]
Run:     python3 tests/bench_proxy.py [--mode cache] [--rounds 3]
"""

import json, os, subprocess, sys, time, urllib.request
from pathlib import Path

REPO_DIR = Path(__file__).resolve().parent.parent
HEADROOM_BIN = REPO_DIR / ".venv" / "bin" / "headroom"
RESULTS_DIR = REPO_DIR / ".hermes" / "tests" / "proxy"
RESULTS_DIR.mkdir(parents=True, exist_ok=True)
CACHE_FILE = REPO_DIR / ".hermes" / "cache" / "session.json"

DEEPSEEK_KEY = os.getenv("HEADROOM_DEEPSEEK_KEY", "")
DEEPSEEK_URL = "https://api.deepseek.com"

# Load real messages from session cache
if CACHE_FILE.exists():
    cache = json.loads(CACHE_FILE.read_text())
    msgs = [m for m in cache["messages"] if m["role"] in ("user","assistant","system")]
    MESSAGES = msgs[:8]
else:
    MESSAGES = [{"role":"user","content":"Say hello"}]


def proxy_stats(port):
    """Query headroom proxy /stats for ground-truth compression."""
    try:
        req = urllib.request.Request(f"http://127.0.0.1:{port}/stats", method="GET")
        with urllib.request.urlopen(req, timeout=3) as r:
            return json.loads(r.read())
    except Exception:
        return {}


def start_proxy(mode="cache", port=8787):
    subprocess.run(["pkill", "-f", f"headroom proxy.*{port}"], capture_output=True)
    time.sleep(0.5)
    p = subprocess.Popen(
        [str(HEADROOM_BIN), "proxy", "--port", str(port), "--mode", mode,
         "--backend", "openai", "--openai-api-url", DEEPSEEK_URL],
        env={**os.environ,
             "OPENAI_BASE_URL": DEEPSEEK_URL,
             "OPENAI_API_KEY": DEEPSEEK_KEY},
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    for _ in range(12):
        time.sleep(1)
        try:
            urllib.request.urlopen(
                urllib.request.Request(f"http://127.0.0.1:{port}/health", method="GET"),
                timeout=2)
            return p
        except Exception: continue
    p.terminate(); return None


def stop_proxy(port):
    subprocess.run(["pkill", "-f", f"headroom proxy.*{port}"], capture_output=True)


def call_proxy(port, payload):
    data = json.dumps(payload).encode()
    req = urllib.request.Request(
        f"http://127.0.0.1:{port}/v1/chat/completions",
        data=data, method="POST",
        headers={"Content-Type":"application/json",
                 "Authorization":f"Bearer {DEEPSEEK_KEY}"})
    with urllib.request.urlopen(req, timeout=60) as r:
        return json.loads(r.read())


def run_round(mode, port, payload, round_num):
    """One benchmark round: pre-stats → API call → post-stats → delta."""
    pre = proxy_stats(port)
    comp_before = pre.get("summary",{}).get("compression",{}).get("total_tokens_removed",0)

    start = time.time()
    body = call_proxy(port, payload)
    elapsed = time.time() - start

    post = proxy_stats(port)
    comp_after = post.get("summary",{}).get("compression",{}).get("total_tokens_removed",0)
    requests = post.get("summary",{}).get("compression",{}).get("requests_compressed",0)
    cache_hits = post.get("summary",{}).get("cache",{}).get("total_hits",0)

    usage = body.get("usage",{})
    return {
        "round": round_num,
        "mode": mode,
        "elapsed_s": round(elapsed, 1),
        "prompt_tokens": usage.get("prompt_tokens", 0),
        "completion_tokens": usage.get("completion_tokens", 0),
        "total_tokens": usage.get("total_tokens", 0),
        "headroom_tokens_removed": comp_after - comp_before,
        "headroom_requests_compressed": requests,
        "headroom_cache_hits": cache_hits,
        "response_preview": body.get("choices",[{}])[0].get("message",{}).get("content","")[:100],
        "error": body.get("error",{}).get("message","")[:100] if body.get("error") else None,
    }


def main():
    mode_filter = next((a.split("=",1)[1] for a in sys.argv if a.startswith("--mode=")), None)
    rounds = int(next((a.split("=",1)[1] for a in sys.argv if a.startswith("--rounds=")), "3"))
    modes = [mode_filter] if mode_filter else ["cache", "token"]

    payload = {"model": "deepseek-v4-flash", "messages": MESSAGES, "max_tokens": 1000}

    for mode in modes:
        port = 8787 if mode == "cache" else 8788
        print(f"\n[{mode}] {len(MESSAGES)} messages, {rounds} rounds")

        proxy = start_proxy(mode, port)
        if not proxy: print(f"  FAIL"); continue
        print(f"  proxy healthy", end="", flush=True)

        all_rounds = []
        for i in range(rounds):
            r = run_round(mode, port, payload, i+1)
            all_rounds.append(r)
            if r.get("error"):
                print(f"\n  R{i+1}: FAIL {r['error']}")
                break
            removed = r["headroom_tokens_removed"]
            cached = r["headroom_cache_hits"]
            print(f"\n  R{i+1}: {r['elapsed_s']}s, prompt={r['prompt_tokens']}t, "
                  f"removed={removed}t, cached={cached}", end="", flush=True)
        stop_proxy(port)

        if all_rounds and not all_rounds[-1].get("error"):
            removed = [r["headroom_tokens_removed"] for r in all_rounds]
            cached = [r["headroom_cache_hits"] for r in all_rounds]
            print(f"\n  Total: removed={sum(removed)}t, cached={sum(cached)}hits")

        ts = time.strftime("%Y%m%d_%H%M%S")
        (RESULTS_DIR / f"{mode}_{ts}.json").write_text(json.dumps(all_rounds, indent=2))

    print(f"\nResults: {RESULTS_DIR}")


if __name__ == "__main__":
    main()
