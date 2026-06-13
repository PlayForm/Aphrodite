#!/usr/bin/env python3
"""
Full-spectrum headroom benchmark — 6 payload types, 3 compression modes,
statistical aggregation, and proxy health monitoring.

Tests every headroom pipeline stage: ContentRouter, SmartCrusher, Kompress, Cache.

Usage:
    python3 tests/report.py              # Run all + render
    python3 tests/report.py --no-run     # Cached data only
    python3 tests/report.py --reset     # Clear session
"""

import json, os, random, subprocess, sys, time, urllib.request
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
VENV = REPO / ".venv"
OUT_DIR = REPO / ".hermes" / "tests"
TEMPLATE = REPO / "tests" / "report_template.html"
OUT_HTML = OUT_DIR / "comparison_report.html"
OUT_JSON = OUT_DIR / "comparison_data.json"
CACHE = OUT_DIR / "session_accumulator.json"

KEY = os.getenv("HEADROOM_DEEPSEEK_KEY", "")
PORT = 8787
LIMIT = int(next((a.split("=", 1)[1] for a in sys.argv if a.startswith("--limit=")), "40"))

if "--reset" in sys.argv and CACHE.exists():
    CACHE.unlink()
    print("Cache cleared.")


# ── Payload generators — 6 types covering all headroom stages ────────

FUNCS = ["setup", "teardown", "validate", "transform", "execute",
         "process", "handle", "dispatch", "resolve", "compute"]
TOPICS = ["compression", "optimization", "caching", "tokenization",
          "embedding", "transformer", "attention", "decoding"]


def _term():
    """Terminal listing — tests SmartCrusher (structured text dedup)."""
    n = random.randint(30, 150)
    return json.dumps({
        "output": "\n".join(
            f"  {random.choice(['file', 'dir', 'link']):4s}  "
            f"{random.randint(100, 99999):>6d}  "
            f"{random.choice(['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun']):3s} "
            f"{random.randint(1, 28):2d} {random.randint(2024, 2026)}  "
            f"{random.choice(['.py', '.js', '.ts', '.rs', '.go', '.md'])}"
            for _ in range(n)),
        "exit_code": random.choice([0, 0, 0, 1]),
    })


def _code():
    """Source code — tests CodeCompressor (AST-aware)."""
    n = random.randint(40, 200)
    return "\n".join(
        f"    def {random.choice(FUNCS)}_{i}(self, "
        f"{', '.join(f'a{j}' for j in range(random.randint(0, 3)))}):\n"
        f"        \"\"\"{random.choice(['Process', 'Handle', 'Validate', 'Execute'])} "
        f"item {i}.\"\"\"\n"
        f"        r = self.{random.choice(['_do', '_run', '_exec', '_call'])}"
        f"({', '.join(f'a{j}' for j in range(random.randint(0, 2))) or 'None'})\n"
        f"        return r\n"
        for i in range(n))


def _web():
    """Search results — tests Kompress (semantic compression)."""
    n = random.randint(3, 15)
    return json.dumps({
        "results": [{
            "title": f"{random.choice(TOPICS).title()} technique {i} — "
                     f"{random.choice(['improving', 'reducing', 'optimizing'])} LLM context",
            "url": f"https://example.com/article/{random.randint(1000, 9999)}",
            "snippet": f"Learn how {random.choice(TOPICS)} can "
                       f"{random.choice(['improve', 'reduce', 'optimize'])} "
                       f"token usage by {random.randint(20, 80)}% in production."
        } for i in range(n)]
    })


def _json():
    """Dense JSON — tests SmartCrusher compaction."""
    return json.dumps({
        "items": [{
            "id": random.randint(1000, 9999),
            "name": f"item_{i}",
            "tags": random.sample(TOPICS, k=random.randint(1, 4)),
            "meta": {
                "created": f"2026-{random.randint(1,12):02d}-{random.randint(1,28):02d}",
                "score": round(random.random() * 100, 4),
                "active": random.choice([True, False]),
            },
        } for i in range(random.randint(10, 40))],
        "total": random.randint(100, 9999),
    })


def _log():
    """Log output — tests ContentRouter (pattern matching)."""
    levels = ["DEBUG", "INFO", "WARN", "ERROR", "FATAL"]
    n = random.randint(20, 80)
    return "\n".join(
        f"{random.choice(levels):5s} [{random.randint(10000,99999)}] "
        f"{random.choice(['request', 'response', 'cache', 'db', 'auth', 'net'])}: "
        f"{' '.join(random.sample(TOPICS, k=random.randint(1, 3)))} "
        f"- latency={random.randint(1,999)}ms status={random.randint(200,599)}"
        for _ in range(n))


def _mixed():
    """Mixed content — tests full pipeline end-to-end."""
    sections = []
    if random.random() < 0.5:
        sections.append(f"# Build output\n\n{_log()}")
    if random.random() < 0.5:
        sections.append(f"# Test results\n\n{_json()}")
    if random.random() < 0.3:
        sections.append(f"# Source diff\n\n{_code()}")
    return "\n\n".join(sections) if sections else _log()


# All 6 payload types with which headroom stage they exercise
PAYLOADS = {
    "terminal":      (_term,  "SmartCrusher"),
    "read_file":     (_code,  "CodeCompressor"),
    "web_search":    (_web,   "Kompress"),
    "execute_code":  (_json,  "SmartCrusher+Compaction"),
    "search_files":  (_log,   "ContentRouter"),
    "cronjob":       (_mixed, "Full Pipeline"),
}

TOOLS = list(PAYLOADS)


# ── Session cache ────────────────────────────────────────────────────

def _load():
    return json.loads(CACHE.read_text()) if CACHE.exists() else []


def _save(msgs):
    if len(msgs) > LIMIT:
        msgs = msgs[-LIMIT:]
    CACHE.write_text(json.dumps(msgs, indent=2))


# ── Inline runner — tests all 6 payload types ────────────────────────

def run_inline():
    from hermes_compress._compress import Compress, CompressOption
    comp = Compress(option=CompressOption(
        Enabled=True, Mode="inline", ProtectRecent=1, MinTokensToCompress=100,
    ), model="deepseek-v4-pro")

    cache = _load()
    rows = []

    for tool in TOOLS:
        gen, stage = PAYLOADS[tool]
        content = gen()

        msg = {"role": "tool", "content": content,
               "tool_call_id": f"tc_{tool}_{random.randint(1000, 9999)}", "name": tool}
        session = cache[-30:] + [{"role": "user", "content": f"bench {len(cache)}"}, msg]

        try:
            r = comp.compress(session)
            post = ""
            for m in r.messages:
                if m.get("tool_call_id") == msg["tool_call_id"]:
                    post = m.get("content", "")
            rows.append({
                "tool": tool, "stage": stage, "cache_n": len(cache),
                "cpre": len(content), "cpost": len(post),
                "tb": r.tokens_before, "ta": r.tokens_after,
                "ts": r.tokens_saved, "pct": round(r.compression_ratio * 100, 1),
                "ms": round(r.duration_ms, 1),
                "xf": r.transforms_applied,
                "err": r.error,
                "pre": content, "post": post,
            })
            if r.tokens_saved > 0:
                print(f"  {tool:15s} [{stage:20s}] cache={len(cache):2d}: {r.tokens_before:>5d}→{r.tokens_after:<5d}t (-{r.tokens_saved}t, {round(r.compression_ratio*100,1)}%)")
            else:
                print(f"  {tool:15s} [{stage:20s}] cache={len(cache):2d}: {r.tokens_before:>5d}→{r.tokens_after:<5d}t (no savings)")
        except Exception as e:
            rows.append({"tool": tool, "stage": stage, "cache_n": len(cache),
                         "cpre": len(content), "err": str(e)})
            print(f"  {tool:15s} [{stage:20s}] ERR: {e}")

        cache.append(msg)
        if random.random() < 0.4:
            cache.append({"role": "user", "content": f"continue {len(cache)}"})

    _save(cache)
    return rows


# ── Proxy runner — tests 4 random payloads ───────────────────────────

def _proxy_ok():
    try:
        with urllib.request.urlopen(f"http://127.0.0.1:{PORT}/health", timeout=3) as r:
            return json.loads(r.read()).get("ready", False)
    except Exception:
        return False


def run_proxy():
    if not _proxy_ok():
        return [{"tool": "proxy", "err": "Proxy not running"}]
    if not KEY:
        return [{"tool": "proxy", "err": "API key not set"}]

    cache = _load()
    rows = []

    for _ in range(4):
        tool = random.choice(TOOLS)
        gen, stage = PAYLOADS[tool]
        content = gen()
        chars = len(content)

        ctx = [m for m in cache[-20:] if m.get("role") != "tool"] + [
            {"role": "system",
             "content": "You are a test harness. Reply with ONLY a single integer: the character count of the tool output in the next message. No other text."},
            {"role": "user", "content": f"Tool: {tool}\n\n{content[:6000]}"},
        ]

        llm = ""
        try:
            data = json.dumps({
                "model": "deepseek-chat",
                "messages": ctx,
                "max_tokens": 100,
            }).encode()
            req = urllib.request.Request(
                f"http://127.0.0.1:{PORT}/v1/chat/completions",
                data=data, method="POST",
                headers={"Content-Type": "application/json", "Authorization": f"Bearer {KEY}"})
            with urllib.request.urlopen(req, timeout=90) as r:
                body = json.loads(r.read())

            usage = body.get("usage", {})
            llm = body.get("choices", [{}])[0].get("message", {}).get("content", "")
            rows.append({
                "tool": tool, "stage": stage, "cache_n": len(cache), "cpre": chars,
                "tb": usage.get("prompt_tokens", 0),
                "ct": usage.get("completion_tokens", 0),
                "tt": usage.get("total_tokens", 0),
                "llm": llm,
                "err": body.get("error", {}).get("message"),
                "pre": content,
            })
            empty = "(empty)" if not llm.strip() else llm[:60]
            print(f"  {tool:15s} [{stage:20s}] cache={len(cache):2d}: prompt={usage.get('prompt_tokens',0):>4d}t comp={usage.get('completion_tokens',0):>2d}t total={usage.get('total_tokens',0):>4d}t | {empty}")
        except Exception as e:
            rows.append({"tool": tool, "stage": stage, "cache_n": len(cache),
                         "cpre": chars, "err": str(e)[:200]})
            print(f"  {tool:15s} [{stage:20s}] ERR: {e}")

        cache.append({"role": "user", "content": f"Tool: {tool}, chars: {chars}"})
        cache.append({"role": "assistant", "content": llm or "(no response)"})

    _save(cache)
    return rows


# ── HTML builders ────────────────────────────────────────────────────

TT = {"terminal": "tt-te", "read_file": "tt-rf", "web_search": "tt-ws",
      "execute_code": "tt-ec", "search_files": "tt-sf", "cronjob": "tt-sf"}


def _esc(s):
    return (s or "").replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def _snip(s, n=500):
    s = str(s)
    return s if len(s) <= n else s[:n] + f"\n\n  ... [+{len(s) - n} more chars]"


def _bar(pct):
    if pct >= 50: return "g", "tg"
    if pct >= 20: return "y", "ty"
    return "r", "tr"


def build_inline_rows(rows):
    out = []
    for i, r in enumerate(rows):
        tb, ta, ts = r.get("tb", 0), r.get("ta", 0), r.get("ts", 0)
        pct = round(ts / tb * 100, 1) if tb else 0
        col, cc = _bar(pct)
        w = min(abs(pct), 100)
        cp, ca = r.get("cpre", 0), r.get("cpost", 0)
        tag = TT.get(r.get("tool", ""), "")
        err = f' <span class="tr">ERR</span>' if r.get("err") else ""
        out.append(
            f'<tr onclick="toggle(\'i_{i}\')" style="cursor:pointer">'
            f'<td class="n">{i+1}</td>'
            f'<td class="cn"><span class="tool-tag {tag}">{r.get("tool","?")}</span></td>'
            f'<td class="lbl" style="font-size:10px">{r.get("stage","")}</td>'
            f'<td class="n">{r.get("cache_n",0)}</td>'
            f'<td class="n">{tb:,}</td><td class="n">{ta:,}</td>'
            f'<td class="n {cc}">{ts:,}</td>'
            f'<td><span class="sb"><span class="br"><span class="bf g{col}" style="width:{w}%"></span></span>'
            f'<span class="pct p{col}">{pct:+.0f}%</span></span></td>'
            f'<td class="lbl">{cp:,}&rarr;{ca:,}</td>'
            f'<td class="lbl">{r.get("ms",0)}ms</td>{err}</tr>')
    return "\n".join(out)


def build_inline_cards(rows):
    out = []
    for i, r in enumerate(rows):
        pre, post = r.get("pre", ""), r.get("post", "")
        if not pre: continue
        cpre, cpost = len(pre), len(post)
        out.append(
            f'<div class="ccard" id="i_{i}">'
            f'<div class="ccard-head" onclick="toggle(\'i_{i}_b\')">'
            f'<h3>#{i+1} {r.get("tool","?")} '
            f'<span style="font-weight:400;color:#888;font-size:12px">'
            f'[{r.get("stage","")}] cache: {r.get("cache_n",0)} msgs</span></h3>'
            f'<span class="st">{r.get("tb",0):,}t &rarr; {r.get("ta",0):,}t</span></div>'
            f'<div class="ccard-body collapsed" id="i_{i}_b">'
            f'<div class="diff-wrap">'
            f'<div class="diff-col before">'
            f'<h4><span class="badge">BEFORE</span> {cpre:,} chars &nbsp; {r.get("tb",0):,} tokens</h4>'
            f'<div class="cbox pre">{_esc(_snip(pre))}</div></div>'
            f'<div class="arr"><div class="arrow-line"></div></div>'
            f'<div class="diff-col after">'
            f'<h4><span class="badge">AFTER</span> {cpost:,} chars &nbsp; {r.get("ta",0):,} tokens</h4>'
            f'<div class="cbox post">{_esc(_snip(post))}</div></div></div>'
            f'<div class="stats-bar">'
            f'<span class="kv"><span class="k">Tokens</span> <span class="v">{r.get("tb",0):,} &rarr; {r.get("ta",0):,}</span></span>'
            f'<span class="saved">Saved {r.get("ts",0):,}t ({r.get("pct",0)}%)</span>'
            f'<span class="ms">&mid; {r.get("ms",0)}ms</span>'
            f'<span class="xf">Stage: {r.get("stage","")} &middot; Transforms: {", ".join(r.get("xf",[]) or ["none"])}</span>'
            f'</div></div></div>')
    return "\n".join(out)


def build_proxy_rows(rows):
    out = []
    for i, r in enumerate(rows):
        tb, ct, tt = r.get("tb", 0), r.get("ct", 0), r.get("tt", 0)
        llm = (r.get("llm") or "")
        tag = TT.get(r.get("tool", ""), "")
        empty_flag = ' <span class="tr" style="font-size:9px">(empty)</span>' if not llm.strip() else ""
        err = f' <span class="tr">ERR: {r["err"][:40]}</span>' if r.get("err") else ""
        out.append(
            f'<tr onclick="toggle(\'p_{i}\')" style="cursor:pointer">'
            f'<td class="n">{i+1}</td>'
            f'<td class="cn"><span class="tool-tag {tag}">{r.get("tool","?")}</span></td>'
            f'<td class="lbl" style="font-size:10px">{r.get("stage","")}</td>'
            f'<td class="n">{r.get("cache_n",0)}</td>'
            f'<td class="n">{tb:,}</td><td class="n">{ct:,}</td><td class="n">{tt:,}</td>'
            f'<td class="lbl">{r.get("cpre",0):,}</td>'
            f'<td class="lbl">{llm[:60]}{empty_flag}</td>{err}</tr>')
    return "\n".join(out)


def build_proxy_cards(rows):
    out = []
    for i, r in enumerate(rows):
        pre = r.get("pre", "")
        if not pre or r.get("err"): continue
        llm = r.get("llm", "") or "(no response)"
        empty_note = ' <span style="color:var(--r);font-size:9px">⚠ empty response</span>' if not r.get("llm","").strip() else ""
        out.append(
            f'<div class="ccard" id="p_{i}">'
            f'<div class="ccard-head" onclick="toggle(\'p_{i}_b\')">'
            f'<h3>#{i+1} {r.get("tool","?")} '
            f'<span style="font-weight:400;color:#888;font-size:12px">'
            f'[{r.get("stage","")}] cache: {r.get("cache_n",0)} msgs</span></h3>'
            f'<span class="st">{r.get("tb",0):,} prompt tokens</span></div>'
            f'<div class="ccard-body collapsed" id="p_{i}_b">'
            f'<div class="diff-wrap">'
            f'<div class="diff-col before">'
            f'<h4><span class="badge">CONTENT SENT</span> {len(pre):,} chars &nbsp; {r.get("tb",0):,} prompt tokens</h4>'
            f'<div class="cbox pre">{_esc(_snip(pre))}</div></div>'
            f'<div class="arr"><div class="arrow-line"></div></div>'
            f'<div class="diff-col after">'
            f'<h4><span class="badge">LLM RESPONSE</span> {r.get("ct",0):,} completion tokens{empty_note}</h4>'
            f'<div class="cbox post">{_esc(llm[:400])}</div></div></div>'
            f'<div class="stats-bar">'
            f'<span class="kv"><span class="k">Prompt</span> <span class="v">{r.get("tb",0):,}t</span></span>'
            f'<span class="kv"><span class="k">Completion</span> <span class="v">{r.get("ct",0):,}t</span></span>'
            f'<span class="kv"><span class="k">Total billed</span> <span class="v">{r.get("tt",0):,}t</span></span>'
            f'</div></div></div>')
    return "\n".join(out)


# ── Main ─────────────────────────────────────────────────────────────

def main():
    run = "--no-run" not in sys.argv
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")

    if run:
        cn_start = len(_load())
        print(f"Cache: {cn_start} messages (limit {LIMIT})\n")

        print(f"{'TOOL':<16} {'STAGE':<22} {'CACHE':>5} {'TOKENS':>14} {'RESULT'}")
        print("-" * 80)
        print("[inline]")
        inline = run_inline()

        print("\n[proxy]")
        proxy = run_proxy()

        cn = len(_load())
        print(f"\nCache: {cn_start} → {cn} messages")

        json.dump({"inline": inline, "proxy": proxy, "cache_n": cn, "limit": LIMIT},
                  open(OUT_JSON, "w"), indent=2, default=str)
    else:
        d = json.load(open(OUT_JSON))
        inline, proxy, cn = d["inline"], d["proxy"], d.get("cache_n", len(_load()))
        print(f"Loaded cached: {cn} messages")

    # ── Populate template ────────────────────────────────────────────
    template = TEMPLATE.read_text()

    itb = sum(r.get("tb", 0) for r in inline)
    its = sum(r.get("ts", 0) for r in inline)
    ita_sum = sum(r.get("ta", 0) for r in inline)
    ptt_sum = sum(r.get("tt", 0) for r in proxy if not r.get("err"))
    ipct = round(its / itb * 100, 1) if itb else 0
    i_total_pct = min(round(its / itb * 100, 1), 100) if itb else 0

    ptb = sum(int(r.get("tb", 0)) for r in proxy if not r.get("err"))
    pct_sum = sum(int(r.get("ct", 0)) for r in proxy if not r.get("err"))
    ptt = sum(int(r.get("tt", 0)) for r in proxy if not r.get("err"))

    n_ok = sum(1 for r in inline if not r.get("err"))
    p_ok = sum(1 for r in proxy if not r.get("err"))

    empty_count = sum(1 for r in proxy if not r.get("err") and not (r.get("llm") or "").strip())

    # Add empty-response warning to note if any
    empty_warning = ""
    if empty_count > 0:
        empty_warning = (
            f'<br><span style="color:var(--r)">⚠ {empty_count} of {p_ok} proxy responses were empty. '
            f'This usually means max_tokens=50 was too restrictive for that payload size. '
            f'Re-run with a larger cache for better context.</span>'
        )

    replacements = {
        "%VERSION%": "0.7.4",
        "%LIMIT%": str(LIMIT),
        "%TIMESTAMP%": now,
        "%CACHE_N%": str(cn),
        "%N_INLINE%": str(n_ok),
        "%N_PROXY%": str(p_ok),
        "%INLINE_PCT%": f"{ipct:+.0f}%",
        "%INLINE_TB%": f"{itb:,}",
        "%INLINE_TA%": f"{itb - its:,}",
        "%PROXY_TT%": f"{ptt_sum:,}",
        "%INLINE_ROWS%": build_inline_rows(inline),
        "%INLINE_CARDS%": build_inline_cards(inline),
        "%INLINE_TOTAL_TB%": f"{itb:,}",
        "%INLINE_TOTAL_TA%": f"{ita_sum:,}",
        "%INLINE_TOTAL_TS%": f"{its:,}",
        "%INLINE_TOTAL_PCT%": str(i_total_pct),
        "%PROXY_ROWS%": build_proxy_rows(proxy),
        "%PROXY_CARDS%": build_proxy_cards(proxy),
        "%PROXY_TOTAL_TB%": f"{ptb:,}",
        "%PROXY_TOTAL_CT%": f"{pct_sum:,}",
        "%PROXY_TOTAL_TT%": f"{ptt:,}",
        "%EMPTY_WARNING%": empty_warning,
    }

    for key, val in replacements.items():
        template = template.replace(key, val)

    OUT_HTML.write_text(template)
    print(f"Report: {OUT_HTML} ({OUT_HTML.stat().st_size:,} bytes)")

    import webbrowser
    webbrowser.open(f"file://{OUT_HTML}")
    print("Opened.")


if __name__ == "__main__":
    main()
