# 10 - Component Diagram

Top-level components and the two boundaries that matter: the **C-ABI** (Python
shim ↔ Rust dylib) and the **HTTP** boundary (LLM clients / Hermes ↔ the two
loopback proxies ↔ upstream API).

```mermaid
graph TB
    subgraph host["Hermes host process"]
        HERMES["Hermes runtime<br/>(registers hooks/tools/skills)"]
        subgraph plugin["Python plugin (thin loader)"]
            SHIM["__init__.py ctypes shim<br/>_load_dylib · _call_json · register"]
        end
    end

    subgraph dylib["libaphrodite_hermes.{dylib,so,dll}"]
        BR["aphrodite-hermes (bridge crate)<br/>process-global STATE (OnceLock)<br/>call_hook · dispatch_tool · schemas · skills"]
    end

    subgraph corelib["aphrodite core crate (rlib, linked into dylib + binary)"]
        HOOKS["hooks (transform_*, pre/post_llm_call)"]
        FLOW["flow::build_turn_context (assembler)"]
        DIRS["directives · session · catalog"]
        CCRMOD["marker · resolve · preview · stage2 · struct_extract"]
        COREABI["core C-ABI: aphrodite_init/dispatch (HANDLES)"]
    end

    subgraph vendored["vendor/headroom-core (aphrodite-headroom-core)"]
        TRAIT["CcrStore trait · compute_key (BLAKE3)"]
        BSQL["SqliteCcrStore"]
        BMEM["InMemoryCcrStore (LRU+TTL)"]
        BRED["RedisCcrStore (present, no proxy call site)"]
    end

    subgraph binproc["aphrodite proxy binary (separate process)"]
        MAIN["main.rs · dual axum listeners"]
        subgraph proxies["proxies"]
            CACHEP[":9797 cache proxy (InMemory CCR)"]
            TOKENP[":9798 token proxy (Sqlite CCR)"]
        end
        PH["proxy_handler · compress_chat_completion"]
        INLINE["inline_ccr LRU (1024) · response_cache LRU"]
    end

    LLMCLIENT["OpenAI/Anthropic-compatible client"]
    UPSTREAM["Upstream LLM API (DeepSeek/OpenAI/…)"]

    HERMES -->|register/invoke| SHIM
    SHIM -.->|C-ABI ctypes| BR
    BR --> HOOKS
    BR --> FLOW
    HOOKS --> DIRS
    HOOKS --> CCRMOD
    BR --> COREABI

    SHIM -.->|"subprocess: aphrodite (auto-launch)"| MAIN
    SHIM -->|HTTP /health| CACHEP
    LLMCLIENT -->|HTTP /v1/chat/completions| CACHEP
    LLMCLIENT -->|HTTP| TOKENP
    CACHEP --> PH
    TOKENP --> PH
    PH --> INLINE
    PH -->|HTTP forward| UPSTREAM

    PH --> TRAIT
    HOOKS -.->|inline_store| corelib
    TRAIT --> BSQL
    TRAIT --> BMEM
    TRAIT --> BRED
    TOKENP --> BSQL
    CACHEP --> BMEM
    CCRMOD --> TRAIT

    classDef boundary stroke-dasharray: 5 5;
```

Boundary notes:
- **C-ABI (dashed ctypes edges):** `aphrodite-hermes` exposes
  `aphrodite_hermes_*` process-global functions; the core crate additionally
  exposes handle-based `aphrodite_*` functions. Both guard panics via
  `guarded()` so no unwind crosses FFI.
- **HTTP boundary:** clients and Hermes talk to the two loopback listeners;
  `loopback_only` + `require_mgmt_token` middleware gate management routes; the
  catch-all `/{*path}` forwards to upstream.
- **Store backends:** token proxy → SQLite, cache proxy → in-memory LRU; Redis
  is compiled in the vendored crate but wired by no `build_state` arm.
- The **core crate is linked into both** the dylib (Hermes path) and the proxy
  binary - the marker/resolve/preview logic is shared, but the two run in
  separate processes with separate CCR state.

## Key call sites
- bridge crate exports - `crates/aphrodite-hermes/src/lib.rs`
- core C-ABI + hook bodies - `crates/aphrodite/src/{lib.rs,hooks.rs,flow.rs}`
- proxy listeners + handler - `crates/aphrodite/src/{main.rs,proxy.rs}`
- `CcrStore` trait + backends - `vendor/headroom/crates/headroom-core/src/ccr/{mod.rs,backends/}`
- Python shim + subprocess launch - `crates/aphrodite/templates/__init__.py`
