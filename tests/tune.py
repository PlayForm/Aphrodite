#!/usr/bin/env python3
"""
Fine-tuning benchmark — 5 compression configs tested against the same cache.

Tests how protect_recent, min_tokens_to_compress, target_ratio affect savings.
All runs share a single session cache — no proxy, internal-only (Compress library).

Usage:  python3 tests/tune.py
"""

import json, random, sys, time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

REPO = Path(__file__).resolve().parent.parent
OUT_DIR = REPO / ".hermes" / "tests"
CACHE_FILE = OUT_DIR / "session_accumulator.json"
OUT_FILE = OUT_DIR / "tune_results.json"

LIMIT = 40

FUNCS = ["setup", "teardown", "validate", "transform", "execute",
         "process", "handle", "dispatch", "resolve", "compute"]
TOPICS = ["compression", "optimization", "caching", "tokenization",
          "embedding", "transformer", "attention", "decoding"]


def _gen(tool):
    if tool == "terminal":
        n = random.randint(30, 150)
        return json.dumps({
            "output": "\n".join(
                f"  {random.choice(['file','dir','link']):4s}  {random.randint(100,99999):>6d}  "
                f"{random.choice(['Jan','Feb','Mar','Apr','May','Jun']):3s} {random.randint(1,28):2d} "
                f"{random.randint(2024,2026)}  {random.choice(['.py','.js','.ts','.rs','.go','.md'])}"
                for _ in range(n)),
            "exit_code": random.choice([0, 0, 0, 1]),
        })
    if tool == "read_file":
        n = random.randint(40, 200)
        return "\n".join(
            f"    def {random.choice(FUNCS)}_{i}(self, {', '.join(f'a{j}' for j in range(random.randint(0,3)))}):\n"
            f"        \"\"\"{random.choice(['Process','Handle','Validate','Execute'])} item {i}.\"\"\"\n"
            f"        r = self.{random.choice(['_do','_run','_exec','_call'])}({', '.join(f'a{j}' for j in range(random.randint(0,2))) or 'None'})\n"
            f"        return r\n"
            for i in range(n))
    if tool == "web_search":
        n = random.randint(3, 15)
        return json.dumps({
            "results": [{
                "title": f"{random.choice(TOPICS).title()} technique {i}",
                "url": f"https://example.com/{random.randint(1000,9999)}",
                "snippet": f"Learn how {random.choice(TOPICS)} can {random.choice(['improve','reduce','optimize'])} token usage by {random.randint(20,80)}%."
            } for i in range(n)]
        })
    if tool == "execute_code":
        n = random.randint(10, 40)
        return json.dumps({
            "items": [{"id": random.randint(1000,9999), "name": f"item_{i}",
                       "tags": random.sample(TOPICS, k=random.randint(1,4)),
                       "meta": {"created": f"2026-{random.randint(1,12):02d}-{random.randint(1,28):02d}",
                                "score": round(random.random()*100,4), "active": random.choice([True,False])}}
                      for i in range(n)],
            "total": random.randint(100, 9999),
        })
    if tool == "search_files":
        n = random.randint(20, 80)
        levels = ["DEBUG","INFO","WARN","ERROR","FATAL"]
        return "\n".join(
            f"{random.choice(levels):5s} [{random.randint(10000,99999)}] "
            f"{random.choice(['request','response','cache','db','auth','net'])}: "
            f"{' '.join(random.sample(TOPICS,k=random.randint(1,3)))} "
            f"- latency={random.randint(1,999)}ms status={random.randint(200,599)}"
            for _ in range(n))
    return json.dumps({"ok": True, "n": random.randint(1, 100)})


def _load():
    return json.loads(CACHE_FILE.read_text()) if CACHE_FILE.exists() else []


def _save(msgs):
    if len(msgs) > LIMIT:
        msgs = msgs[-LIMIT:]
    CACHE_FILE.write_text(json.dumps(msgs, indent=2))


TOOLS = ["terminal", "read_file", "web_search", "execute_code", "search_files"]
#  5 configs to explore
# ═══════════════════════════════════════════════════════════════════════

CONFIGS = [
    {"label": "default",     "protect_recent": 1, "min_tokens": 100, "target_ratio": None},
    {"label": "aggressive",  "protect_recent": 0, "min_tokens": 50,  "target_ratio": 0.05},
    {"label": "conservative","protect_recent": 4, "min_tokens": 500, "target_ratio": None},
    {"label": "balanced+",   "protect_recent": 1, "min_tokens": 150, "target_ratio": 0.10},
    {"label": "maximum",     "protect_recent": 0, "min_tokens": 25,  "target_ratio": 0.01},
]

TOOLS = ["terminal", "read_file", "web_search", "execute_code", "search_files"]


def run_config(label, protect_recent, min_tokens, target_ratio):
    from hermes_compress._compress import Compress, CompressOption
    comp = Compress(option=CompressOption(
        Enabled=True, Mode="inline",
        ProtectRecent=protect_recent,
        MinTokensToCompress=min_tokens,
        TargetRatio=target_ratio,
    ), model="deepseek-v4-pro")

    cache = _load()
    results = []

    for tool in TOOLS:
        content = _gen(tool)
        msg = {"role": "tool", "content": content,
               "tool_call_id": f"tc_{tool}_{random.randint(1000,9999)}", "name": tool}
        session = cache[-30:] + [{"role": "user", "content": f"tune {len(cache)}"}, msg]

        try:
            r = comp.compress(session)
            post = ""
            for m in r.messages:
                if m.get("tool_call_id") == msg["tool_call_id"]:
                    post = m.get("content", "")
            results.append({
                "tool": tool,
                "tb": r.tokens_before, "ta": r.tokens_after,
                "ts": r.tokens_saved,
                "pct": round(r.compression_ratio * 100, 1),
                "ms": round(r.duration_ms, 1),
                "cpre": len(content), "cpost": len(post),
                "err": r.error,
            })
        except Exception as e:
            results.append({"tool": tool, "err": str(e)})

        cache.append(msg)

    _save(cache)
    return results


# ═══════════════════════════════════════════════════════════════════════

def main():
    print("HermesCompress — Fine-Tuning (5 Configs, Same Cache)")
    print("=" * 75)

    all_results = {}

    for cfg in CONFIGS:
        label = cfg["label"]
        print(f"\n── {label} ──")
        print(f"   protect_recent={cfg['protect_recent']}  min_tokens={cfg['min_tokens']}  "
              f"target_ratio={cfg['target_ratio']}")
        print(f"   {'Tool':<15} {'Tokens':>20} {'Saved':>8} {'Rate':>7} {'Chars':>14} {'Time'}")
        print(f"   {'-'*15} {'-'*20} {'-'*8} {'-'*7} {'-'*14} {'-'*6}")

        rows = run_config(**cfg)

        all_results[label] = {
            "config": cfg,
            "rows": rows,
            "total_tb": sum(r.get("tb", 0) for r in rows),
            "total_ts": sum(r.get("ts", 0) for r in rows),
            "total_pct": round(sum(r.get("ts", 0) for r in rows) / sum(r.get("tb", 0) for r in rows) * 100, 1) if sum(r.get("tb", 0) for r in rows) else 0,
        }

        for r in rows:
            e = f" ERR: {r['err']}" if r.get("err") else ""
            print(f"   {r.get('tool','?'):<15} {r.get('tb',0):>6}→{r.get('ta',0):<6}t "
                  f"{r.get('ts',0):>6}t {r.get('pct',0):>5.0f}% "
                  f"{r.get('cpre',0):>6}→{r.get('cpost',0):<6}c {r.get('ms',0)}ms{e}")

        total_tb = all_results[label]["total_tb"]
        total_ts = all_results[label]["total_ts"]
        total_pct = all_results[label]["total_pct"]
        print(f"   {'─'*60}")
        print(f"   {'TOTAL':<15} {total_tb:>6}t saved {total_ts:>6}t ({total_pct}%)\n")

    # ── Summary comparison ────────────────────────────────────────────
    print("=" * 75)
    print(f"{'Config':<15} {'Tokens In':>10} {'Saved':>8} {'Rate':>7} {'Assessment'}")
    print(f"{'─'*15} {'─'*10} {'─'*8} {'─'*7} {'─'*35}")

    best_label, best_pct = "", 0
    for label, data in all_results.items():
        tb = data["total_tb"]
        ts = data["total_ts"]
        pct = data["total_pct"]
        cfg = data["config"]
        assess = ""
        if pct > 80:
            assess = "⚠ may over-compress (check for data loss)"
        elif pct > 60:
            assess = "✓ strong savings, safe"
        elif pct > 30:
            assess = "~ moderate, good for code-heavy sessions"
        else:
            assess = "~ conservative, preserves context"
        if pct > best_pct:
            best_pct, best_label = pct, label
        print(f"   {label:<15} {tb:>10,d} {ts:>8,d} {pct:>5.0f}%   {assess}")

    print(f"\n   Best: {best_label} ({best_pct}% savings)")

    OUT_FILE.write_text(json.dumps(all_results, indent=2, default=str))
    print(f"\nResults: {OUT_FILE}")


if __name__ == "__main__":
    main()
