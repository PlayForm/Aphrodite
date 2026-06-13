<br>
<p align="center">
	<picture>
		<source media="(prefers-color-scheme: dark)" srcset="https://PlayForm.Cloud/Dark/Image/GitHub/Hermes.svg">
		<source media="(prefers-color-scheme: light)" srcset="https://PlayForm.Cloud/Image/GitHub/Hermes.svg">
		<img width="28" alt="Hermes" src="https://PlayForm.Cloud/Image/GitHub/Hermes.svg">
	</picture>
</p>

<br>
<br>

# [Hermes Compress] 🗜️

**Headroom‑powered context compression for** **[`Hermes Agent`][Hermes]** **and
any Python application.**

Slash LLM token usage by 25‑60 % per API call. Works standalone, as a Hermes
plugin, or anywhere you call an LLM.

<br>

<p align="center">
	<a href="https://GitHub.Com/PlayForm/HermesCompress/actions/workflows/Node.yml" target="_blank">
		<picture>
			<source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/github/actions/workflow/status/PlayForm/HermesCompress/Node.yml?branch=Current&label=Build&logo=python&color=black&labelColor=black&logoColor=white&logoWidth=0">
			<source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/github/actions/workflow/status/PlayForm/HermesCompress/Node.yml?branch=Current&label=Build&logo=python&color=white&labelColor=white&logoColor=black&logoWidth=0">
			<img src="https://img.shields.io/github/actions/workflow/status/PlayForm/HermesCompress/Node.yml?branch=Current&label=Build&logo=python&color=black&labelColor=black&logoColor=white&logoWidth=0" alt="Build" title="Build">
		</picture>
	</a>
	<br>
	<a href="https://PyPI.Org/project/hermes-compress" target="_blank">
		<picture>
			<source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/pypi/v/hermes-compress?label=Version&logo=pypi&color=black&labelColor=black&logoColor=white&logoWidth=0">
			<source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/pypi/v/hermes-compress?label=Version&logo=pypi&color=white&labelColor=white&logoColor=black&logoWidth=0">
			<img src="https://img.shields.io/pypi/v/hermes-compress?label=Version&logo=pypi&color=black&labelColor=black&logoColor=white&logoWidth=0" alt="Version" title="Version">
		</picture>
	</a>
	<br>
	<a href="https://GitHub.Com/PlayForm/HermesCompress" target="_blank">
		<picture>
			<source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/github/stars/PlayForm/HermesCompress?style=flat&label=Star&logo=github&color=black&labelColor=black&logoColor=white&logoWidth=0">
			<source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/github/stars/PlayForm/HermesCompress?style=flat&label=Star&logo=github&color=white&labelColor=white&logoColor=black&logoWidth=0">
			<img src="https://img.shields.io/github/stars/PlayForm/HermesCompress?style=flat&label=Star&logo=github&color=black&labelColor=black&logoColor=white&logoWidth=0" alt="Star">
		</picture>
	</a>
	<br>
	<a href="https://GitHub.Com/PlayForm/HermesCompress/blob/Current/LICENSE" target="_blank">
		<picture>
			<source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/github/license/PlayForm/HermesCompress?label=License&color=black&labelColor=black&logoColor=white&logoWidth=0">
			<source media="(prefers-color-scheme: light)" srcset="https://img.shields.io/github/license/PlayForm/HermesCompress?label=License&color=white&labelColor=white&logoColor=black&logoWidth=0">
			<img src="https://img.shields.io/github/license/PlayForm/HermesCompress?label=License&color=black&labelColor=black&logoColor=white&logoWidth=0" alt="License">
		</picture>
	</a>
</p>

<br>

---

<br>

## Install 🚀

```bash
pip install hermes-compress
```

Hermes Agent integration (one command):

```bash
hermes-compress install   # patches hermes‑agent core
hermes-compress uninstall # reverts all changes
hermes-compress status    # check patch status
```

Enable in `~/.hermes/config.yaml`:

```yaml
compression:
 headroom:
  enabled: true
```

Restart Hermes.

<br>

> **Note**
>
> The first API call of any session may add **10‑15 seconds** while headroom
> loads compression models (Kompress ONNX). Subsequent calls are fast
> (~50‑80 ms).

<br>

---

<br>

## Usage 💡

### Standalone (Any Python App)

```python
from hermes_compress import Compress

c = Compress(model = "deepseek-v4-pro", enabled = True)
result = c.compress(messages)
messages = result.messages  # compressed, 25‑60 % smaller
```

### CLI

```bash
hermes-compress proxy --port 8787
echo '{"items": [1,2,3]}' | hermes-compress compress --json
hermes-compress compress "$(cat large-log.txt)"
```

### Hermes Plugin

```bash
git clone https://GitHub.Com/PlayForm/HermesCompress.git \
	 ~/.hermes/plugins/hermes-compress
```

<br>

---

<br>

## Modes ⚙️

| Mode                 | How                     | Latency          | Savings                  | When                                |
| -------------------- | ----------------------- | ---------------- | ------------------------ | ----------------------------------- |
| **inline** (default) | Library call in‑process | 8‑10 ms warm     | Full pipeline (7 phases) | Default — fastest, best compression |
| **proxy**            | Separate server, HTTP   | +5‑20 ms network | Headroom only            | Zero code changes, multi‑client     |

Both share the same headroom pipeline (CacheAligner → ContentRouter →
SmartCrusher → Kompress). Inline mode adds 6 extra pre‑processing phases that
the raw proxy does not have, giving **additional savings on top of headroom**.
Use inline unless you cannot modify the codebase.

### Inline

```python
from hermes_compress import Compress

c = Compress(enabled = True, mode = "inline")
result = c.compress(messages)
```

### Proxy

```python
from hermes_compress import Proxy

proxy = Proxy(port = 8787)
proxy.start()
# Point provider base_url → http://127.0.0.1:8787
```

Or: `hermes-compress proxy --port 8787`

<br>

---

<br>

## Compression Pipeline 🔬

Eight phases run before every LLM API call:

1. **Pre‑process** 🧹 — Strip ANSI codes, collapse repeated log lines, remove
   debug noise, compress repeated patterns
2. **Optimize** ✨ — Whitespace normalization, JSON number rounding, path
   normalization (~/), timestamp shortening, boilerplate stripping. Zero
   fidelity loss — only formatting changes.
3. **Strategies** 🎯 — Per‑tool compression tier selection
   (aggressive / balanced / code / prose / minimal / skip)
4. **Truncation** ✂️ — Smart head‑and‑tail, JSON‑aware, line‑based truncation
   for outputs over 50 K chars
5. **Deduplication** 🪞 — Skip identical tool results across turns
6. **Pre‑compress** 🔄 — Double‑pass: compress each large tool output
   individually before the full list
7. **Headroom** 🧠 — CacheAligner → ContentRouter → SmartCrusher → Kompress
8. **Stats** 📊 — Per‑call metrics, dry‑run mode, backpressure simulation

### How the pipeline works

When Hermes calls the LLM, the conversation loop injects compression just before
the API request. The compressor receives `api_messages` — the complete message
history that will be sent to the provider — and applies each phase in order.

**Phase 1 (Pre‑process)** strips formatting waste that headroom cannot detect as
cleanly. ANSI color codes from terminal output, repeated identical log lines
from build runs, Python tracebacks beyond the first frame, and npm/Docker
verbose output are all removed before they reach the compressor. This is pure
win: the tokens removed carry no semantic value.

**Phase 2 (Optimize)** handles the remaining formatting overhead. Whitespace is
normalized without changing structure (tabs become spaces, multiple consecutive
blank lines collapse to two). Floating‑point numbers in JSON are rounded to 4
decimal places — semantically identical for all LLM purposes, but fewer tokens.
Absolute file paths are shortened to `~/`‑relative form. ISO 8601 timestamps
like `2026‑06‑13T07:28:20.609Z` become `20260613‑072820`. Standard tool output
headers and session metadata footers are stripped.

**Phase 3 (Strategies)** selects the optimal compression tier per tool type. The
system ships with six tiers, each tuned to a content category. `search_files`
and `web_search` get the aggressive tier (target ratio 0.10, protect recent 0)
because JSON arrays compress extremely well under SmartCrusher. `read_file` and
`patch` get the code tier (target ratio 0.20) to preserve function signatures
and import blocks. `terminal` and `execute_code` get the balanced tier with
default settings. Tiny tools like `write_file`, `memory`, and `clarify` are
skipped entirely because their output is rarely worth the compression overhead.

**Phase 4 (Truncation)** activates only for outputs over 50 K characters. For
code content, it keeps the first 5 K and last 3 K characters. For terminal logs,
it keeps the first 100 and last 50 lines. For JSON, it truncates arrays to the
first 100 items and preserves the object structure. The truncation threshold is
deliberately high — it only fires on genuinely oversized outputs that would slow
down the compression pipeline.

**Phase 5 (Deduplication)** maintains an LRU cache of the last 50 tool results.
When a tool returns content identical to a previous call in the same session,
the message is replaced with a short reference. This is common with `read_file`
re‑reads, repeated `search_files` queries, and idempotent `patch` operations. A
50‑entry cache is small enough to be nearly free but large enough to catch most
duplicate patterns.

**Phase 6 (Pre‑compress)** is the double‑pass strategy. Each large tool output
(>500 characters) is individually compressed via headroom before the full
message list is compressed. The individual pass uses `protect_recent=0` (no
protection) and `min_tokens_to_compress=100` to aggressively shrink each output.
The full pass then works on the partially‑compressed list, often finding
additional savings because the ContentRouter can now detect patterns across the
shrunken outputs.

**Phase 7 (Headroom)** is the core compression engine. It runs four sub‑phases
internally. CacheAligner stabilises message prefixes for KV‑cache hits (disabled
for DeepSeek and OpenAI, which do not support this). ContentRouter detects the
content type of each message and routes it to the appropriate compressor.
SmartCrusher handles JSON arrays with pattern detection and structural
compression. CodeCompressor is AST‑aware for Python, JavaScript, TypeScript,
Rust, Go, and Java. Kompress uses a quantised ONNX model for ML‑based prose
compression. Each compressor returns a ratio (fraction of original kept), and
the ContentRouter records the transform chain.

**Phase 8 (Stats)** runs only when dev mode is active or `verbose_stats` is
enabled in config. It records per‑call token counts, latency breakdowns by
phase, transform chains, and per‑tool savings rates. The `StatsCollector` can
replay the last N calls as a formatted table and compute averages across the
session.

### Provider‑specific behaviour

Different LLM providers support different compression features. Hermes Compress
delegates model routing to headroom, which detects the provider and adjusts
automatically.

| Provider      | CacheAligner      | SmartCrusher | CodeCompressor | Kompress |
| ------------- | ----------------- | ------------ | -------------- | -------- |
| **DeepSeek**  | Disabled          | Active       | Active         | Active   |
| **Anthropic** | Active (KV cache) | Active       | Active         | Active   |
| **OpenAI**    | Disabled          | Active       | Active         | Active   |
| **Google**    | Disabled          | Active       | Active         | Active   |

**DeepSeek** is the primary target. The provider's 1 M token context window
means conversations can grow very large before summarisation kicks in.
Compression is therefore essential for cost control — 59‑64 % savings at 8‑10 ms
per call translates to a 2.5× longer conversation in the same budget. DeepSeek
does not support Anthropic‑style prompt caching, so CacheAligner is disabled.
All three content compressors remain fully active.

**Anthropic** (Claude) benefits from CacheAligner because the provider's prompt
caching API (`cache_control` breakpoints) allows re‑using KV cache entries
across turns. Hermes Compress stabilises message prefixes to maximise cache hit
rates. With prompt caching enabled, the effective token cost of the compressed
context can drop to near‑zero for cache hits — only the new, uncompressed
messages are billed.

**OpenAI** and **Google** work identically to DeepSeek: CacheAligner is skipped,
and content compressors handle the full load. Both providers charge per input
token, so compression savings translate directly to cost reduction.

### Troubleshooting

**Compression is not active.** Check that `compression.headroom.enabled` is
`true` in `~/.hermes/config.yaml`. Run `hermes-compress status` to verify the
install patcher applied all files. If you see `already patched` for all three
files, compression is wired into the conversation loop. Restart Hermes for
config changes to take effect.

**First API call is slow (10‑15 seconds).** This is expected. Headroom loads the
Kompress ONNX model on first use. A warning is logged:
`hermes-compress: first call -- headroom is loading compression models (Kompress ONNX). This may add 10-15 seconds to this request. Subsequent calls will be fast (~50-80ms).`
If the delay is unacceptable, disable Kompress by setting `mode: token` (forces
SmartCrusher + CodeCompressor only, no ML model load).

**Compression ratio is lower than expected.** Check `protect_recent` in config.
The default of 1 protects only the most recent message. If you see low savings,
the tool outputs may be genuinely unique content that headroom cannot compress
without losing fidelity. JSON‑heavy sessions (web search, file search) compress
best. Prose‑heavy sessions (web extraction, documentation) compress less.

**Headroom is not installed.** Run `pip install hermes-compress` — headroom‑ai
is a dependency. Compression is silently disabled if headroom is missing.
Messages pass through unchanged.

**The install patcher failed.** Run `hermes-compress uninstall` to restore from
`.bak` backups, then re‑run `hermes-compress install`. If the backups are
missing, run `git checkout` in the hermes‑agent directory to restore the
original files, then re‑install.

<br>

---

<br>

## Configuration 🎛️

### Python API

```python
from hermes_compress import CompressOption

option = CompressOption(
 Enabled           = True,
 Mode              = "inline",
 ProtectRecent       = 1,
 TargetRatio         = None,   # most aggressive (~15 % kept)
 MinTokensToCompress = 250,
 PrecompressTools    = True,
 AggressiveKompress  = True,
 DeduplicateResults  = True,
 VerboseStats       = True,
)
c = Compress(model = "deepseek-v4-pro", option = option)
```

### Hermes `config.yaml`

```yaml
compression:
 headroom:
  enabled: true
  mode: inline
  protect_recent: 1
  target_ratio: null
  min_tokens_to_compress: 250
  precompress_tools: true
  aggressive_kompress: true
  deduplicate_results: true
  verbose_stats: true
```

<br>

---

<br>

## Benchmarks 📈

## Benchmarks 📈

## Benchmarks 📈

### Production Session (DeepSeek v4-pro, max compression)

| Metric                | Value               |
| --------------------- | ------------------- |
| **Calls**             | 9                   |
| **Tokens saved**      | 5,361               |
| **Total latency**     | 204 ms (23 ms/call) |
| **Compression ratio** | ~60% per call       |

### Dev Session (same model, dev bypass active)

| Metric            | Value               |
| ----------------- | ------------------- |
| **Calls**         | 7                   |
| **Tokens saved**  | 4,817               |
| **Total latency** | 178 ms (25 ms/call) |

### Unit Benchmark (raw headroom, 18 configs × 4 content types)

| Content          | Configs | Latency (warm) | Token savings        |
| ---------------- | ------- | -------------- | -------------------- |
| JSON (34K chars) | 18      | 18 ms          | 0% (structural only) |
| Code (2.9K)      | 18      | 1 ms           | 0%                   |
| Mixed (3.7K)     | 18      | 2 ms           | 0%                   |
| Prose (4.6K)     | 18      | 2 ms           | 0%                   |

> Raw headroom reports 0% because SmartCrusher restructures content without
> reducing token count. The full 8-phase pipeline (pre-process, optimize,
> deduplicate, pre-compress) adds 43-60% real savings on top.

### vs API Baseline

Without compression, 863 messages cost ~2.3M tokens per call. With
hermes-compress at `protect_recent=1`: **~77% reduction** in effective token
usage through prompt caching + compression. For DeepSeek's 1M context window,
this means ~3× longer conversations before summarization.

<br>

---

<br>

## Hermes Tools 🔧

Five tools registered when loaded as a Hermes plugin:

| Tool                    | Description                          |
| ----------------------- | ------------------------------------ |
| `headroom_stats`        | Session compression statistics       |
| `headroom_compress`     | Manually compress text / JSON / code |
| `headroom_proxy_start`  | Start the proxy server               |
| `headroom_proxy_stop`   | Stop the proxy server                |
| `headroom_proxy_status` | Check proxy health                   |

<br>

---

<br>

## API Reference 📖

### `Compress`

```python
class Compress(model: str = "", option: CompressOption = None):
    def compress(messages) -> CompressResult
    def update_model(model)
    # Properties: enabled, mode, stats
```

### `Proxy`

```python
class Proxy(port: int = 8787, host: str = "127.0.0.1", mode: str = "token"):
    def start() -> bool
    def stop()
    # Properties: running, healthy, base_url
```

### `CompressResult`

```python
@dataclass
class CompressResult:
    messages: list[dict]
    tokens_before: int
    tokens_after: int
    tokens_saved: int
    compression_ratio: float
    duration_ms: float
    transforms_applied: list[str]
    error: str | None
    compressed: bool  # True when tokens were saved
```

### `CompressOption`

```python
@dataclass
class CompressOption:
    Enabled: bool = False
    Mode: str = "inline"
    ProtectRecent: int = 1
    TargetRatio: float | None = None
    MinTokensToCompress: int = 250
    PrecompressTools: bool = False
    AggressiveKompress: bool = False
    DeduplicateResults: bool = False
    VerboseStats: bool = False
    ProxyPort: int = 8787
    ProxyHost: str = "127.0.0.1"
    ProxyAutoStart: bool = False
```

<br>

---

<br>

## Dev Mode 🧪

```bash
HERMES_COMPRESS_DEV = 1 hermes ...

# Feature flags
HERMES_COMPRESS_FLAGS = dry_run = 1, verbose_stats = 1, precompress_tool_outputs = 1

# Simulate backpressure
HERMES_COMPRESS_FLAGS = simulate_backpressure = 1, backpressure_delay_ms = 50
```

Hot‑reload: edit any `.py` file in the plugin directory — changes are picked up
on the next API call without restart.

<br>

---

<br>

## FAQ ❓

**Does this replace the built‑in context compression?** No. Hermes' context
compressor handles conversation length (summarization). Hermes Compress handles
message content — compressing tool outputs and prose before they enter context.
They are complementary.

**Does this need a separate server?** Not by default. Inline mode runs headroom
as a library in‑process. Proxy mode is optional.

**What if headroom‑ai is not installed?** Compression is silently disabled —
messages pass through unchanged.

**Does this modify conversation history?** No. Only `api_messages` are
compressed — the per‑call copy sent to the LLM. Persistent history is preserved.

<br>

---

<br>

## Changelog 📝

See [`CHANGELOG.md`](CHANGELOG.md).

<br>

---

<br>

## License ⚖️

CC0 1.0 Universal — see [`LICENSE`](LICENSE).

<br>

<br>

<p align="center">
	<a href="https://PlayForm.Cloud" target="_blank">
		<picture>
			<source media="(prefers-color-scheme: dark)" srcset="https://PlayForm.Cloud/Dark/Image/GitHub/PlayForm.svg">
			<source media="(prefers-color-scheme: light)" srcset="https://PlayForm.Cloud/Image/GitHub/PlayForm.svg">
			<img width="200" alt="PlayForm" src="https://PlayForm.Cloud/Image/GitHub/PlayForm.svg">
		</picture>
	</a>
</p>

[Hermes]: https://GitHub.Com/NousResearch/hermes-agent
[Hermes Compress]: https://GitHub.Com/PlayForm/HermesCompress
[SmartCrusher]: https://GitHub.Com/chopratejas/headroom
[CodeCompressor]: https://GitHub.Com/chopratejas/headroom
[Kompress]: https://HuggingFace.Com/chopratejas/kompress-v2-base
