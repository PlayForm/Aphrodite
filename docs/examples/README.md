# Examples

Real tool results captured from live sessions against the Aphrodite 1.4.6
binary, shown the way the model sees them: verbatim markers, rendered previews,
and the byte economics of compression. Where the rest of the docs describe how
the pieces work, the examples show what actually lands in a session's context.

| Page                                      | What it shows                                                                                                                                                                      |
| ----------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [What the LLM Actually Sees](llm-view.md) | The two-line marker-plus-preview block, seven worked scenarios (code reads, build errors, JSON, diffs, retrieval, directives, preview families), and a real token-economics table. |

## What the captures demonstrate

The examples are organized by the behavior they exercise, mirroring the
configuration knobs behind them:

| Behavior area               | Captured variants                                                                                                                                                                                                                             | Documented in                                                              |
| --------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- |
| Compression thresholds      | `tool_threshold_token` 512 / 4096, `terminal_threshold` 512 / 1024 / 4096, `inline_threshold` 512 / 2048, `code_multiplier` 1.0 / 3.0, `engine_threshold_pct` 20 / 45 / 100, `engine_protect_first`, `engine_protect_last`, `engine_min_msgs` | [aphrodite-toml](../config/aphrodite-toml.md)                              |
| Storage and expansion       | `auto_expand` true / false, `auto_expand_limit`, `chain_split` true / false, `catalog_mode` tool / compact / full, `prefetch`, `poll_worker`                                                                                                  | [aphrodite-toml](../config/aphrodite-toml.md)                              |
| Preview rendering           | `model_family` compact / code_first / balance, `code_structure_map` true / false, `preview_max_chars` 0 / 120                                                                                                                                 | [aphrodite-toml](../config/aphrodite-toml.md)                              |
| Content types and previews  | `ls`, `terminal`, `source_code`, `diff`, `json`, `build_error`, `text` markers with rendered previews                                                                                                                                         | [Content Types](../classification/content-types.md)                        |
| Session orientation         | `directives.active` `["focus","foresight"]` / `["explore"]` / `[]`                                                                                                                                                                            | [Directives](../plugin/directives.md)                                      |
| Flow budget                 | `budget_chars` 2600 / 10000                                                                                                                                                                                                                   | [aphrodite-toml](../config/aphrodite-toml.md)                              |
| Prompt templates            | `retrieve_guidance` minimal / standard / verbose, `catalog_intent_hints`, `session_inject`, marker hint wiring                                                                                                                                | [aphrodite-toml](../config/aphrodite-toml.md)                              |
| Environment overrides       | `APHRODITE_ENGINE_THRESHOLD_PCT`, `TERMINAL_THRESHOLD`, `TOOL_THRESHOLD_TOKEN`, `INLINE_THRESHOLD`, `CCR_TTL_SECONDS`, `DIRECTIVES_DIR`, `FLOW_BUDGET_CHARS`, alternate proxy ports                                                           | [env-vars](../config/env-vars.md)                                          |
| Proxies                     | default cache (9797) and token (9798) listeners, health and version endpoints                                                                                                                                                                 | [Proxy Architecture](../proxy/architecture.md)                             |
| Marker format and lifecycle | marker wire format, retrieve round-trips, TTL / expiry                                                                                                                                                                                        | [Marker Format](../ccr/marker-format.md), [Lifecycle](../ccr/lifecycle.md) |

## Token economics at a glance

Real captured payloads and the marker-plus-preview block that replaced them in
the relayed result (full details in [What the LLM Actually Sees](llm-view.md)):

| Content             | Raw bytes | Marker + preview | Ratio |
| ------------------- | --------- | ---------------- | ----- |
| Directory listing   | 5,889     | ~106 chars       | ~56x  |
| Rust source file    | 2,645     | ~115 chars       | ~23x  |
| JSON tool output    | 1,728     | ~104 chars       | ~17x  |
| Git diff            | 987       | ~114 chars       | ~9x   |
| Build error (rustc) | 801       | ~117 chars       | ~7x   |

The pattern behind every row: the model sees the shape and summary of the
content for the price of a few hundred characters, and pays the full token
cost only when it calls `aphrodite_retrieve`.

## Reading a marker

```
<<<CCR:hash|type|size>>>
[preview line]
```

The marker line names the stored entry (BLAKE3 hash, detected content type,
original byte size); the preview line summarizes it. Retrieval by hash restores
the original content byte-for-byte, and because hashing is content-addressed,
the same output always produces the same marker.
