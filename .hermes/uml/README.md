# Aphrodite UML - Runtime Flow Traces (v1.3.4)

Mermaid UML diagrams tracing every runtime flow of Aphrodite (CCR compression
proxy + Hermes plugin), anchored to real modules/functions with `file:line`.
Two runtime worlds are covered: the **proxy binary** (dual loopback listeners,
HTTP response compression) and the **Hermes plugin** (Python shim → C-ABI dylib
→ shared core crate). They share marker/CCR concepts but run in separate
processes with separate state.

| # | File | What it traces |
|---|------|----------------|
| 01 | [01-startup.md](01-startup.md) | Process startup: `main` → config resolution → bind-before-spawn of `:9797` cache / `:9798` token listeners → CCR store init → hot-reload watcher (sequence + port-override flowchart). |
| 02 | [02-chat-compression.md](02-chat-compression.md) | Core value path: chat response → classify → EMA-tuned threshold → BLAKE3 → store → `<<<CCR:…>>>` marker + preview; shows the `tool_calls`-not-compressed branch (sequence + flowchart). |
| 03 | [03-retrieve.md](03-retrieve.md) | `/retrieve` (inline→backend, filter, pagination, byte-exact) + recursive `resolve::expand` (depth limit, cycle-safe, no write-back). |
| 04 | [04-hook-ffi.md](04-hook-ffi.md) | Hermes → Python ctypes → C-ABI → core hooks; the `pre_llm_call` → `flow::build_turn_context` choke point; bridge-vs-core parity. |
| 05 | [05-ccr-lifecycle.md](05-ccr-lifecycle.md) | State machine of a CCR entry (created→stored→previewed→{retrieved/decayed/evicted/expired}→GC) + the EMA threshold state. |
| 06 | [06-sse-streaming.md](06-sse-streaming.md) | `text/event-stream` detection → timeout-free client → chunk passthrough (no compression) → mid-stream byte/error accounting. |
| 07 | [07-config-resolution.md](07-config-resolution.md) | env > TOML > default precedence across `config.rs` / `config_loader.rs`; live vs inert/reserved keys. |
| 08 | [08-dylib-hotreload.md](08-dylib-hotreload.md) | mtime detection → unique temp-path copy → `CDLL` reload → per-image state reset. |
| 09 | [09-release-ci.md](09-release-ci.md) | Tag push → `Build.yml` (release-once → 4-target matrix → Finalize) + `Publish.yml` (Test → opt-in crates.io chain); Windows long pole. |
| 10 | [10-component.md](10-component.md) | Top-level component diagram: crates + Python shim + proxies + store backends + Hermes host, with C-ABI and HTTP boundaries labeled. |
| 11 | [11-data-model.md](11-data-model.md) | Class diagram: `AphroditeState`, `AppState`, `MarkerEntry`, `ToolEvent`, `CcrStore` trait + backends, directives. |

## Cross-cutting findings noted while tracing
- The **HTTP proxy path and the Hermes FFI path are two separate compression
  pipelines**. The proxy uses `proxy_detect_content_type` + `proxy_build_preview`
  and never calls `transforms::detect` / `stage2` / `struct_extract`; those run
  only on the FFI hook path (`hooks::transform_*_inner`). They share `compute_key`
  (BLAKE3) and the marker wire format but nothing else.
- **Two marker layouts** exist for the same `<<<CCR:hash|type|size>>>` token:
  `render_marker` (marker.rs, FFI path) puts the marker line **first**;
  `proxy_format_ccr_output` (proxy.rs, proxy path) puts it **last**.
- **`flow.rs` is fully implemented** (not mid-flux) and is the single directive
  choke point that all three `pre_llm_call` entry points converge on.
- **Dead/removed paths** confirmed in-tree: the `{hash}#stage2` shadow lookup
  (resolve.rs:68, deleted), the zstd-magic decompress branch (retrieve.rs:119,
  removed as unreachable), `RedisCcrStore` (compiled, no `build_state` call
  site). The brief's "no-finalize-job gap" is **closed** - `Build.yml:205` now
  has a `Finalize` asset-completeness job.
