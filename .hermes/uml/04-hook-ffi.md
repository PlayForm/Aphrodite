# 04 - Hermes Hook & FFI Path

The Hermes plugin path: Hermes fires a hook → Python ctypes shim
(`__init__.py`) resolves the dylib fresh each call → C ABI in
`aphrodite-hermes/src/lib.rs` (`aphrodite_hermes_call_hook`, panic-guarded) →
core `aphrodite` crate hook bodies. The `pre_llm_call` arm is the **choke
point**: bridge, core `hooks::pre_llm_call`, and the `context_engine_pre_llm`
tool all converge on the single assembler `flow::build_turn_context`, so
directive injection can never fork between paths again (bug class 01-F3).

## Tool-result / terminal-output transform (bridge path)

```mermaid
sequenceDiagram
    autonumber
    participant H as Hermes host
    participant PY as __init__.py _hook_dispatch
    participant DL as _load_dylib() (mtime check + hot-reload)
    participant CJ as _call_json (FFI + free_string)
    participant CH as aphrodite_hermes_call_hook (hermes/lib.rs:289)
    participant UW as tools::unwrap_hermes_result (tools.rs:66)
    participant TI as hooks::transform_tool_result_with_meta (hooks.rs:67)
    participant IN as transform_tool_result_inner (hooks.rs:119)
    participant ST as inline_store + recent_markers (AphroditeState)
    participant RF as replacement_from (hermes/lib.rs:106)

    H->>PY: transform_tool_result(tool_name, result, status, error_*, duration_ms)
    PY->>DL: _load_dylib()  (fresh CDLL if mtime changed)
    PY->>CJ: aphrodite_hermes_call_hook("transform_tool_result", args_json)
    CJ->>CH: extern "C" (guarded - panic → {"error":...})
    CH->>UW: unwrap_hermes_result(tool_content) → (classify_content, type)
    Note over UW: unwraps {output,exit_code}/{diff}/{error}/{matches}/{content}<br/>ORIGINAL content still hashed verbatim - only type/preview affected
    CH->>TI: transform_tool_result_with_meta(state, content, tool, classify, meta)
    TI->>IN: inner pipeline
    IN->>ST: record_tool_event_from_meta (always - telemetry)
    IN->>IN: gates: empty / essential-tool / self-tool / below tool_threshold
    alt over threshold, compressible
        IN->>ST: inline_store_put(hash, content) · record_marker
        IN-->>CH: {compressed:true, hash, type, size, preview, marker}
    else skipped
        IN-->>CH: {compressed:false, reason}
    end
    CH->>RF: replacement_from(result)
    RF-->>CJ: marker string  (or Value::Null if not compressed)
    CJ->>CJ: aphrodite_hermes_free_string(ptr)
    CJ-->>PY: json
    PY-->>H: string → Hermes swaps tool output for marker (or leaves it on null)
```

## pre_llm_call - the directive-injection choke point

```mermaid
flowchart TD
    subgraph entries["Three entry points converge"]
      A["bridge: call_hook('pre_llm_call') (hermes/lib.rs:360)"]
      B["core FFI: aphrodite_dispatch('pre_llm_call') → hooks::pre_llm_call (hooks.rs:350)"]
      C["context_engine_pre_llm tool (tools.rs:511)"]
    end
    A --> D["flow::build_turn_context(state, est_bytes) (flow.rs:39)"]
    B --> D
    C --> D
    D --> E["directives = build_directive_context (directives.rs:80) - NEVER dropped"]
    D --> F["nudges = render_nudges - ≤2 inline [nudge:…], NEVER dropped"]
    D --> G["recall = session::catalog_summary - droppable FIRST"]
    E --> H["assemble top-down under state.flow_budget_chars (default 4000)"]
    F --> H
    G --> H
    H --> I{"joined len > budget?"}
    I -->|yes| J["pop last section (recall block) - directives+nudges survive"]
    I -->|no| K["emit context"]
    J --> I
    K --> L["injected into model turn:<br/>[directives]…\n[nudge:…]\n[recall]…\nretrieve hint"]
```

`post_llm_call` (hooks.rs:370) is the mirror choke point on both paths:
`archive_turn` (last marker of the turn → `conv_index`) → `next_turn` (turn++)
→ `flow::purge_expired_nudges` (after the counter advances, so a ttl=1 nudge
renders exactly once).

## Parity: bridge path vs core (handle-based) path

```mermaid
flowchart LR
    subgraph py["Python shim (__init__.py)"]
      T["_make_handler → dispatch_tool"]
      HK["_hook_dispatch → call_hook"]
    end
    subgraph bridge["aphrodite-hermes (process-global 1 state)"]
      DT["aphrodite_hermes_dispatch_tool (lib.rs:216)"]
      CH2["aphrodite_hermes_call_hook (lib.rs:289)"]
    end
    subgraph core["aphrodite core (handle-based ABI)"]
      DP["aphrodite_dispatch (lib.rs:530)"]
      IH["hooks::{transform_*, pre_llm_call, post_llm_call}"]
    end
    T --> DT
    HK --> CH2
    DT --> R["tools::dispatch registry"]
    CH2 --> IH
    DP --> IH
    R --> SH["with_shared → AphroditeState"]
    IH --> SH
    Note1["pre_llm_call: bridge + core + context_engine_pre_llm<br/>ALL call flow::build_turn_context - single assembler"]
    style Note1 fill:#eef
```

Both `guarded()` wrappers (core `lib.rs:137`, hermes `lib.rs:206`) catch panics
across the FFI boundary and return `{"error":"internal error: panicked…"}`
instead of aborting the process. `_with_meta` variants exist only on the bridge
because the handle-based core ABI has no Hermes `status`/`error_type` telemetry
to plumb.

## Key call sites
- Python: `_load_dylib` (hot-reload), `_call_json`, `_hook_dispatch`, `register` - `crates/aphrodite/templates/__init__.py:74,172,368,350` (mirror: `plugins/aphrodite/__init__.py`)
- `aphrodite_hermes_call_hook` (bridge hooks) / `replacement_from` - `crates/aphrodite-hermes/src/lib.rs:289,106`
- `tools::unwrap_hermes_result` / `tools::dispatch` - `crates/aphrodite-hermes/src/tools.rs:66,19`
- `hooks::transform_tool_result_inner` / `pre_llm_call` / `post_llm_call` - `crates/aphrodite/src/hooks.rs:119,350,370`
- `flow::build_turn_context` (assembler) - `crates/aphrodite/src/flow.rs:39`
- core C ABI `guarded` / `aphrodite_dispatch` - `crates/aphrodite/src/lib.rs:137,530`
