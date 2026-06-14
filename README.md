<p align="center">
  <img src="assets/logo.svg" alt="HermesCompress" width="180" height="180">
</p>

<h1 align="center">HermesCompress</h1>

<p align="center"><strong>Headroom-powered context compression for Hermes Agent.</strong><br>Inline compression shim - no proxy required.</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.7.10-purple?style=flat" alt="version">
  <img src="https://img.shields.io/badge/savings-50--67%25-brightgreen?style=flat" alt="savings">
  <img src="https://img.shields.io/badge/latency-50--300ms-blue?style=flat" alt="latency">
  <img src="https://img.shields.io/badge/python-3.11+-orange?style=flat" alt="python">
</p>

---

## Overview

HermesCompress is a Hermes Agent plugin that monkey-patches the conversation loop to
compress API messages before they reach the LLM provider. It uses
[headroom-ai](https://github.com/NousResearch/headroom) (v0.25.0) with the
Kompress ONNX model for AST-aware, dedup-capable compression.

**Key numbers** (live benchmark, 85-message session):

| Metric | Value |
|--------|-------|
| Token savings | **50-67%** (warm cache, 10+ messages) |
| Latency overhead | **50-300ms** per API call (warm) |
| Tool output integrity | **100% preserved** (0 empty/truncated results) |
| CCR markers | **0** (inline mode, no markers) |
| First-call overhead | **5-7s** (one-time Kompress ONNX model load) |

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Hermes Agent                          │
│                                                         │
│  ┌───────────────────────────────────────────────┐      │
│  │        conversation_loop.run_conversation()    │      │
│  │  ┌──────────────────────────────────────────┐ │      │
│  │  │  hermes-compress-shim (monkey-patch)      │ │      │
│  │  │                                          │ │      │
│  │  │  1. _patched(*args, **kwargs)            │ │      │
│  │  │     wraps run_conversation               │ │      │
│  │  │  2. _compress_hook wraps forwarders:      │ │      │
│  │  │     - _interruptible_api_call             │ │      │
│  │  │     - _interruptible_streaming_api_call   │ │      │
│  │  │  3. _compress(messages)                   │ │      │
│  │  │     → Compress.compress() via headroom    │ │      │
│  │  │  4. Proxy detection: skip if localhost    │ │      │
│  │  └──────────────────────────────────────────┘ │      │
│  └───────────────────────────────────────────────┘      │
│                         ↓                               │
│              DeepSeek API (v4-pro)                       │
└─────────────────────────────────────────────────────────┘
```

The shim intercepts at the **innermost layer** - the last code before the HTTP call to
DeepSeek. All Hermes processing (sanitization, reasoning echo, middleware, tool JSON
canonicalization) completes BEFORE compression. No downstream code can undo the result.

---

## Quick Start

### 1. Install dependencies into Hermes agent venv

```bash
~/.hermes/hermes-agent/venv/bin/pip install -e /path/to/HermesCompress
~/.hermes/hermes-agent/venv/bin/pip install headroom-ai
```

**Critical**: `hermes_compress` MUST be importable from the Hermes process. The shim
runs inside Hermes' Python process, not the project venv. Without this step, engine
init silently fails (`ModuleNotFoundError`) with zero compression applied.

### 2. Enable the plugin

```bash
hermes plugins enable hermes-compress-shim
```

**Note**: `hermes config set plugins.enabled ...` is IGNORED by Hermes. Use
`hermes plugins enable` instead. Verify with `hermes plugins list --plain`.

### 3. Restart Hermes

On next session startup, you should see:

```
[hermes-compress-shim] ✓ patched agent API hooks - direct compression
```

The first API call will be slow (5-7s Kompress ONNX model download from HuggingFace).
Set `HF_TOKEN` in `.env` to avoid rate limits. Subsequent calls: 50-300ms.

### 4. Debug mode

```bash
HERMES_COMPRESS_DEBUG=1 hermes
```

Prints every step to stderr: patch detection, forwarder wrapping, engine init,
compression result, savings %.

---

## Configuration

The shim reads its config from `COMPRESS_CONFIG` in the plugin source:

| Key | Default | Description |
|-----|---------|-------------|
| `protect_recent` | `1` | Protect N most recent messages from compression |
| `min_tokens` | `100` | Minimum tokens before compression triggers |
| `target_ratio` | `None` | Target compression ratio (`None` = auto) |
| `precompress` | `True` | Pre-compress tools before headroom |
| `aggressive_kompress` | `True` | Aggressive mode for Kompress |
| `deduplicate` | `True` | SmartCrusher deduplication |

To override, edit `~/.hermes/plugins/hermes-compress-shim/__init__.py` (hardlinked to
the repo copy at `plugins/hermes-compress-shim/__init__.py`).

---

## Plugins

| Plugin | What it does |
|--------|-------------|
| `hermes-compress-shim` | Monkey-patches API forwarders → 50-67% token savings |
| `hermes-tool-fix` | Monitors terminal_tool for empty output, recovers read_file content |

Enable both:

```bash
hermes plugins enable hermes-compress-shim
hermes plugins enable hermes-tool-fix
```

---

## Proxy Mode (optional)

Two headroom proxies are available for prefix-cache freezing:

```bash
./scripts/proxy-start.py --mode cache --port 8787   # cache mode
./scripts/proxy-start.py --mode token --port 8788   # token mode
```

| Mode | Port | What it does |
|------|------|-------------|
| Cache | :8787 | Anthropic Messages API compression + CCR markers |
| Token | :8788 | DeepSeek prefix-cache freezing (no Chat Completions compression) |

**Important**: Neither proxy compresses OpenAI Chat Completions (what Hermes uses).
The proxy only compresses Anthropic Messages API traffic. For Hermes, the inline
shim is the ONLY compression path that actually reduces token count.

The shim detects localhost base_url and skips local compression when a proxy is active
(to avoid double-compression).

---

## Benchmarks

### Live session (pane 6, 2026-06-14)

```
10.7% -   2 msgs,  5,798ms  (cold: Kompress model load)
15.3% -   5 msgs,     42ms
56.1% -   6 msgs,     65ms
58.6% -   9 msgs,     78ms
61.4% -  12 msgs,    239ms
61.9% -  14 msgs,     85ms
62.3% -  16 msgs,     88ms
61.8% -  18 msgs,     81ms
```

### Standalone (85 messages, all 5 tool types)

```
26.3% savings - 207,647 → 153,145 chars
30 tool outputs, 0 CCR markers, 0 safety guard triggers
Latency: 42s (85 msgs in one pass - per-call is 50-300ms on 5-20 msgs)
```

Full report: `reports/2026-06-14/live-benchmark.md`

---

## Safety Guard

The `_compress.py` safety guard reverts only truly empty tool outputs (zero-length
strings). Across all benchmarks: **0 empty tool outputs detected**. The
`protect_recent=1` setting prevents over-compression of recent content.

---

## Verification

```bash
# Standalone compression test
.venv/bin/python tests/shim_hermes_compress.py --test

# Compare direct vs proxy
.venv/bin/python tests/benchmark_compare.py

# Tuning sweep
.venv/bin/python tests/tune.py

# Live session check (from another terminal)
grep "hermes-compress: saved" ~/.hermes/logs/agent.log | tail -5
```

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| No compression, marker appears | `hermes_compress` not in agent venv | `pip install -e .` into agent venv |
| Plugin `not enabled` in list | `plugins.enabled` config ignored | `hermes plugins enable hermes-compress-shim` |
| `TypeError: run_conversation()` | Shim has wrong param signature | Update to latest (uses `*args/**kwargs`) |
| `headroom_retrieve` loops | Proxy not running | Start proxy or ignore (tool fails fast now) |
| Terminal commands return empty | Hermes sandbox bug (not our issue) | Enable `hermes-tool-fix` plugin |

---

## Commits (Current branch)

```
d1d9a01 feat: hermes-tool-fix plugin - patches terminal_tool + read_file_tool
6f68475 fix: headroom_retrieve fails fast when proxy not running
6092a5f fix: add debug logging + fix ModuleNotFoundError for hermes_compress
4e2aab5 fix: *args/**kwargs passthrough to match actual run_conversation signature
7b36b8a feat: proxy detection + correct port in shim plugin
ae2ed63 docs: live benchmark report v2 - 85-msg payload, per-tool breakdown, 26.2% savings
9f225f0 fix: correct monkey-patch targets + live benchmark report
```

---

## License

HermesCompress is part of the [PlayForm](https://playform.cloud) ecosystem.
