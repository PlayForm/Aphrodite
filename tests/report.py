#!/usr/bin/env python3
"""
Template-based compression report populator.

Reads tests/report_template.html, replaces %PLACEHOLDER% markers with
live benchmark data, writes .hermes/tests/comparison_report.html.

Flags: --reset (clear cache)  --no-run (use cached JSON)  --limit=N
"""

import json, os, random, re, subprocess, sys, time, urllib.request
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
VENV = REPO / ".venv"
OUT_DIR = REPO / ".hermes" / "tests"
TEMPLATE = REPO / "tests" / "report_template.html"
OUT_HTML = OUT_DIR / "comparison_report.html"
OUT_JSON = OUT_DIR / "comparison_data.json"
CACHE_FILE = OUT_DIR / "session_accumulator.json"

KEY = os.getenv("HEADROOM_DEEPSEEK_KEY", "")
PORT = 8787
LIMIT = int(next((a.split("=", 1)[1] for a in sys.argv if a.startswith("--limit=")), "40"))

if "--reset" in sys.argv and CACHE_FILE.exists():
    CACHE_FILE.unlink()
    print("Cache cleared.")


# ── Generators ───────────────────────────────────────────────────────

FUNCS = ["setup", "teardown", "validate", "transform", "execute",
         "process", "handle", "dispatch", "resolve", "compute"]
TOPICS = ["compression", "optimization", "caching", "tokenization",
          "embedding", "transformer", "attention", "decoding"]


def _term():
    n = random.randint(20, 120)
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
    n = random.randint(30, 150)
    return "\n".join(
        f"    def {random.choice(FUNCS)}_{i}(self, "
        f"{', '.join(f'arg{j}' for j in range(random.randint(0, 3)))}):\n"
        f"        \"\"\"{random.choice(['Process', 'Handle', 'Validate', 'Execute'])} "
        f"item {i}.\"\"\"\n"
        f"        r = self.{random.choice(['_do', '_run', '_exec', '_call'])}"
        f"({', '.join(f'arg{j}' for j in range(random.randint(0, 2))) or 'None'})\n"
        f"        return r\n"
        for i in range(n))


def _web():
    n = random.randint(3, 12)
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


GEN = {"terminal": _term, "read_file": _code, "web_search": _web}
TOOLS = list(GEN) + ["execute_code", "search_files"]


# ── Cache ────────────────────────────────────────────────────────────

def _load(): return json.loads(CACHE_FILE.read_text()) if CACHE_FILE.exists() else []


def _save(msgs):
    if len(msgs) > LIMIT:
        msgs = msgs[-LIMIT:]
    CACHE_FILE.write_text(json.dumps(msgs, indent=2))


# ── Runners ──────────────────────────────────────────────────────────

def run_inline():
    from hermes_compress._compress import Compress, CompressOption
    comp = Compress(option=CompressOption(
        Enabled=True, Mode="inline", ProtectRecent=1, MinTokensToCompress=100,
    ), model="deepseek-v4-pro")

    cache = _load()
    rows = []

    for _ in range(6):
        tool = random.choice(TOOLS)
        gen = GEN.get(tool)
        content = gen() if gen else json.dumps({"ok": True, "n": random.randint(1, 100)})

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
                "tool": tool, "cache_n": len(cache),
                "cpre": len(content), "cpost": len(post),
                "tb": r.tokens_before, "ta": r.tokens_after,
                "ts": r.tokens_saved, "pct": round(r.compression_ratio * 100, 1),
                "ms": round(r.duration_ms, 1),
                "xf": r.transforms_applied,
                "err": r.error,
                "pre": content, "post": post,
            })
        except Exception as e:
            rows.append({"tool": tool, "cache_n": len(cache), "cpre": len(content), "err": str(e)})

        cache.append(msg)
    _save(cache)
    return rows


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
        gen = GEN.get(tool)
        content = gen() if gen else "ok"
        chars = len(content)

        ctx = [m for m in cache[-20:] if m.get("role") != "tool"] + [
            {"role": "system", "content": "Test mode. Reply with the character count of the tool output. Number only."},
            {"role": "user", "content": f"Tool: {tool}\n\n{content[:6000]}"},
        ]

        llm = ""
        try:
            data = json.dumps({"model": "deepseek-v4-flash", "messages": ctx, "max_tokens": 50}).encode()
            req = urllib.request.Request(
                f"http://127.0.0.1:{PORT}/v1/chat/completions",
                data=data, method="POST",
                headers={"Content-Type": "application/json", "Authorization": f"Bearer {KEY}"})
            with urllib.request.urlopen(req, timeout=90) as r:
                body = json.loads(r.read())
            usage = body.get("usage", {})
            llm = body.get("choices", [{}])[0].get("message", {}).get("content", "")
            rows.append({
                "tool": tool, "cache_n": len(cache), "cpre": chars,
                "tb": usage.get("prompt_tokens", 0),
                "ct": usage.get("completion_tokens", 0),
                "tt": usage.get("total_tokens", 0),
                "llm": llm,
                "err": body.get("error", {}).get("message"),
                "pre": content,
            })
        except Exception as e:
            rows.append({"tool": tool, "cache_n": len(cache), "cpre": chars, "err": str(e)[:200]})

        cache.append({"role": "user", "content": f"Tool: {tool}, chars: {chars}"})
        cache.append({"role": "assistant", "content": llm or "(no response)"})

    _save(cache)
    return rows


# ── HTML builders ────────────────────────────────────────────────────

TT = {"terminal": "tt-te", "read_file": "tt-rf", "web_search": "tt-ws",
      "execute_code": "tt-ec", "search_files": "tt-sf"}


def _esc(s):
    return (s or "").replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")


def _snip(s, n=600):
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
        tool = r.get("tool", "?")
        tag = TT.get(tool, "")
        err = f' <span class="tr">ERR</span>' if r.get("err") else ""
        out.append(
            f'<tr onclick="toggle(\'i_{i}\')" style="cursor:pointer">'
            f'<td class="n">{i+1}</td>'
            f'<td class="cn"><span class="tool-tag {tag}">{tool}</span></td>'
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
            f'cache: {r.get("cache_n",0)} messages</span></h3>'
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
            f'<span class="xf">Transforms: {", ".join(r.get("xf",[]) or ["none"])}</span>'
            f'</div></div></div>')
    return "\n".join(out)


def build_proxy_rows(rows):
    out = []
    for i, r in enumerate(rows):
        tb, ct, tt = r.get("tb", 0), r.get("ct", 0), r.get("tt", 0)
        llm = (r.get("llm") or "")[:80]
        tool = r.get("tool", "?")
        tag = TT.get(tool, "")
        err = f' <span class="tr">ERR: {r["err"][:40]}</span>' if r.get("err") else ""
        out.append(
            f'<tr><td class="n">{i+1}</td>'
            f'<td class="cn"><span class="tool-tag {tag}">{tool}</span></td>'
            f'<td class="n">{r.get("cache_n",0)}</td>'
            f'<td class="n">{tb:,}</td><td class="n">{ct:,}</td><td class="n">{tt:,}</td>'
            f'<td class="lbl">{r.get("cpre",0):,}</td>'
            f'<td class="lbl">{llm}</td>{err}</tr>')
    return "\n".join(out)


def build_proxy_cards(rows):
    out = []
    for i, r in enumerate(rows):
        pre = r.get("pre", "")
        if not pre or r.get("err"): continue
        out.append(
            f'<div class="ccard" id="p_{i}">'
            f'<div class="ccard-head" onclick="toggle(\'p_{i}_b\')">'
            f'<h3>#{i+1} {r.get("tool","?")} '
            f'<span style="font-weight:400;color:#888;font-size:12px">'
            f'cache: {r.get("cache_n",0)} messages</span></h3>'
            f'<span class="st">{r.get("tb",0):,} prompt tokens</span></div>'
            f'<div class="ccard-body collapsed" id="p_{i}_b">'
            f'<div class="diff-wrap">'
            f'<div class="diff-col before">'
            f'<h4><span class="badge">CONTENT SENT</span> {len(pre):,} chars &nbsp; {r.get("tb",0):,} prompt tokens</h4>'
            f'<div class="cbox pre">{_esc(_snip(pre))}</div></div>'
            f'<div class="arr"><div class="arrow-line"></div></div>'
            f'<div class="diff-col after">'
            f'<h4><span class="badge">LLM RESPONSE</span> {r.get("ct",0):,} completion tokens</h4>'
            f'<div class="cbox post">{_esc((r.get("llm") or "(no response)")[:400])}</div></div></div>'
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
        print(f"Cache: {len(_load())} messages (limit {LIMIT})\n[inline]")
        inline = run_inline()
        for r in inline:
            e = f" ERR: {r['err']}" if r.get("err") else ""
            print(f"  {r['tool']} cache={r['cache_n']}: {r.get('tb',0)}→{r.get('ta',0)}t (-{r.get('ts',0)}t, {r.get('pct',0)}%){e}")

        print("\n[proxy]")
        proxy = run_proxy()
        for r in proxy:
            e = f" ERR: {r['err']}" if r.get("err") else ""
            print(f"  {r['tool']} cache={r['cache_n']}: prompt={r.get('tb',0)}t comp={r.get('ct',0)}t total={r.get('tt',0)}t {r.get('llm','')[:50]}{e}")

        cn = len(_load())
        print(f"\nCache: {cn} messages")
        json.dump({"inline": inline, "proxy": proxy, "cache_n": cn},
                  open(OUT_JSON, "w"), indent=2, default=str)
    else:
        d = json.load(open(OUT_JSON))
        inline, proxy, cn = d["inline"], d["proxy"], d.get("cache_n", len(_load()))
        print(f"Loaded: {cn} messages")

    # ── Populate template ────────────────────────────────────────────
    template = TEMPLATE.read_text()

    itb = sum(r.get("tb", 0) for r in inline)
    its = sum(r.get("ts", 0) for r in inline)
    ptt = sum(r.get("tt", 0) for r in proxy if not r.get("err"))
    ipct = round(its / itb * 100, 1) if itb else 0
    i_total_pct = min(round(its / itb * 100, 1), 100) if itb else 0

    ptb = sum(int(r.get("tb", 0)) for r in proxy if not r.get("err"))
    pct_sum = sum(int(r.get("ct", 0)) for r in proxy if not r.get("err"))
    ptt_sum = sum(int(r.get("tt", 0)) for r in proxy if not r.get("err"))

    n_ok = sum(1 for r in inline if not r.get("err"))
    p_ok = sum(1 for r in proxy if not r.get("err"))

    replacements = {
        "%VERSION%": "0.7.3",
        "%LIMIT%": str(LIMIT),
        "%TIMESTAMP%": now,
        "%CACHE_N%": str(cn),
        "%N_INLINE%": str(n_ok),
        "%N_PROXY%": str(p_ok),
        "%INLINE_PCT%": f"{ipct:+.0f}%",
        "%INLINE_TB%": f"{itb:,}",
        "%INLINE_TA%": f"{itb - its:,}",
        "%PROXY_TT%": f"{ptt:,}",
        "%INLINE_ROWS%": build_inline_rows(inline),
        "%INLINE_CARDS%": build_inline_cards(inline),
        "%INLINE_TOTAL_TB%": f"{itb:,}",
        "%INLINE_TOTAL_TA%": f"{sum(r.get('ta',0) for r in inline):,}",
        "%INLINE_TOTAL_TS%": f"{its:,}",
        "%INLINE_TOTAL_PCT%": str(i_total_pct),
        "%PROXY_ROWS%": build_proxy_rows(proxy),
        "%PROXY_CARDS%": build_proxy_cards(proxy),
        "%PROXY_TOTAL_TB%": f"{ptb:,}",
        "%PROXY_TOTAL_CT%": f"{pct_sum:,}",
        "%PROXY_TOTAL_TT%": f"{ptt_sum:,}",
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
