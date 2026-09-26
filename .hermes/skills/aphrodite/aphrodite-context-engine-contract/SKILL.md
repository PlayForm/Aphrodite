---
name: aphrodite-context-engine-contract
description: "Use when configuring, debugging, or verifying the Aphrodite context engine. Single-compression-owner activation contract: inheritance, registration, selection, configuration ownership."
version: 1.2.0
author: Hermes Agent
license: MIT
platforms: [macos]
category: aphrodite
category_taxonomy: aphrodite/aphrodite-context-engine-contract
date: 2026-09-25
metadata:
    hermes:
        tags: [aphrodite, context-engine, compression, ownership, configure, debug, verify]
        related_skills:
            [
                aphrodite-hook-contracts,
                aphrodite-boundaries,
                aphrodite-orientation,
                aphrodite-compression-safety,
            ]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
    branches:
        - Development
    runtime_modes:
        - source
        - installed
owns:
    - The single-compression-owner activation contract (7-condition checklist)
    - ContextEngine inheritance and registration conditions (isinstance gate, one-engine rule, name @property)
    - Engine selection (context.engine config, name match, deepcopy path)
    - Configuration ownership for context.engine, compression.enabled, compression.context_engine, APHRODITE_CONTEXT_ENGINE
    - The boundary between Hermes built-in compression and Aphrodite compression
    - The hooks-only state definition (activation failure)
depends_on:
    - aphrodite-boundaries (context boundaries, failure-behavior policy)
    - aphrodite-orientation (preflight gate before any live probe)
supersedes: []
verification:
    source_of_truth:
        - ~/.hermes/hermes-agent/agent/context_engine.py (ContextEngine ABC)
        - ~/.hermes/hermes-agent/agent/agent_init.py (_select_context_engine)
        - ~/.hermes/hermes-agent/hermes_cli/plugins.py (register_context_engine, get_plugin_context_engine)
        - ~/.hermes/hermes-agent/cli.py + tui_gateway/session_compression.py (compression.enabled default)
        - plugins/aphrodite/__init__.py (_register_context_engine)
        - crates/aphrodite/src/config_loader.rs (dylib-side toggle)
mutation_level: read-only
---

# Aphrodite Context Engine Contract

The contract for activating Aphrodite's context engine as the **single compression owner** of a Hermes session, and the boundary between Hermes built-in compression and Aphrodite compression. Source-derived facts, not prose: every claim names the file to re-derive from; line numbers are observational snapshots, never durable coordinates.

**Stop if:** a claim below disagrees with the checked-out source named in `verification.source_of_truth`, or a line number is being treated as a durable coordinate.
**Recovery:** re-derive the fact from the named source file, update the claim and its row in the claim-to-test matrix, then continue.

## The single-compression-owner principle

Exactly **one** system may mutate conversation context per session. If Aphrodite is active, Hermes built-in compression must be disabled; otherwise two independently mutating systems are present and the resulting "Compacting context" behavior cannot be attributed reliably. This skill owns the activation checklist that makes the owner unambiguous. (State boundary owner: `aphrodite-boundaries`.)

## Boundary: Hermes built-in compression vs Aphrodite compression

| Dimension         | Hermes built-in                                                                                                                     | Aphrodite                                                                            |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------ |
| Compaction engine | `ContextCompressor` (fallback when no engine is selected)                                                                           | `AphroditeContextEngine` (selected via `context.engine`)                             |
| Actual shrinking  | LLM summarization inside `compress()`                                                                                               | Rust transform hooks (tool/terminal) + proxy; engine is the ownership placeholder    |
| Enable switch     | `compression.enabled` - **default true** (`cli.py:405`; `tui_gateway/session_compression.py:86`, `gateway/run_turn.py:637` read it) | `context.engine` selection + plugin registration; dylib `compression.context_engine` |
| Activation model  | Preflight compaction near the token threshold                                                                                       | Per-tool-output marker compression, threshold-free                                   |
| Failure behavior  | N/A (host-owned)                                                                                                                    | Fail-open transforms; engine itself never raises                                     |

**Never leave `compression.enabled: true` when Aphrodite is active** because the built-in preflight/auto-compaction would then run independently of the Aphrodite engine, producing constant `Compacting context` messages and two mutating systems. Verify with `hermes config get compression.enabled` and set it false.

## ContextEngine ABC (the inheritance contract)

Base class: `agent/context_engine.py` (`ContextEngine(ABC)`).

**Abstract (MUST implement):**

- `name` - **`@property`**, short identifier string. A class attribute is NOT accepted because it is an abstract property on the ABC; property lookup fails.
- `update_from_response(usage: dict) -> None` - token tracking after each API response.
- `should_compress(prompt_tokens=None) -> bool` - return True to trigger compaction.
- `compress(messages, current_tokens=None, focus_topic=None, force=False, memory_context="") -> list` - main compaction entry; returns the (possibly shortened) message list. The host filters optional parameters by signature, so older engines may omit them.

**Optional / defaulted (safe to override or ignore):** full signatures, defaults, and selection order: `references/compressed-detail.md`, `references/context-engine-api.md`; silent-rejection pitfalls: `references/context-engine-pitfalls.md`.

- **One-engine rule:** only one context engine may be registered per plugin manager; a second registration is rejected with "Only one context engine plugin is allowed."
- **name must be `@property`** and match the `context.engine` config value at selection time.
- Success logs: `Plugin 'aphrodite' registered context engine: aphrodite`.

Plugin side (`plugins/aphrodite/__init__.py`, `_register_context_engine`): registration is opt-in via `APHRODITE_CONTEXT_ENGINE` (truthy, `_env_bool`); failure logs a warning and falls back to hooks + proxy. Details: `references/compressed-detail.md`.

**Installed layout is hooks-only:** `~/.hermes/plugins/aphrodite` holds only the loader (`plugin.yaml` + `__init__.py`); runtime state lives in `~/.hermes/aphrodite/`. Full layout + CLAIM: `references/compressed-detail.md`.

**`APHRODITE_CONTEXT_ENGINE` has two consumers** - dylib toggle (TOML `compression.context_engine`, `config_loader.rs:126`) and plugin registration gate (`_env_bool`); they gate different layers. Details: `references/compressed-detail.md`.

## Engine selection (configuration ownership)

Selection runs per agent build (`agent/agent_init.py`, `_select_context_engine`):

`context.engine` (default `"compressor"`) -> built-in `ContextCompressor` unless a registered engine with matching `.name` is found; candidates are deep-copied (`copy.deepcopy` - engine must be deepcopy-clean); copy failure or name mismatch falls back to built-in with a warning. Details: `references/compressed-detail.md`.

Configuration ownership table:

| Setting                        | Where                                | Owner / default                                         | Aphrodite requirement                        |
| ------------------------------ | ------------------------------------ | ------------------------------------------------------- | -------------------------------------------- |
| `context.engine`               | Hermes config.yaml                   | Hermes; default `compressor`                            | `aphrodite`                                  |
| `compression.enabled`          | Hermes config.yaml                   | Hermes; **default true**                                | `false`                                      |
| `compression.context_engine`   | `~/.hermes/aphrodite/aphrodite.toml` | Aphrodite dylib; **default true**                       | leave true (or `APHRODITE_CONTEXT_ENGINE=1`) |
| `APHRODITE_CONTEXT_ENGINE` env | environment                          | two consumers (plugin registration gate + dylib toggle) | truthy when the engine is wanted             |

Config changes apply to **new sessions** - changing `context.engine` mid-session does not swap the running engine.

**Setup writes only proxy ports, never key/url/model:** `api_url`/`model` are env-driven (`APHRODITE_API_URL`/`APHRODITE_MODEL`); the proxy key comes from `APHRODITE_API_KEY` or TOML `[defaults] api_key` and must be actually exported - a commented-out line behaves like an absent var. Details: `references/compressed-detail.md`.

## Activation checklist (7 conditions - all must hold)

Aphrodite's context engine is **active** only when every condition below is confirmed. If ANY condition fails, the state is **hooks-only** - never "partially active context engine". Hooks-only means: plugin loads, hooks and proxy run, but no engine owns compression; if `compression.enabled` stays true, Hermes built-in compression is the owner - two systems once the engine is enabled on top.

1. **Plugin loads successfully** - `register()` completes; dylib loads; the log shows the registered hook count (a missing/broken dylib disables the plugin entirely).
2. **Engine subclasses `ContextEngine`** - `AphroditeContextEngine` extends the dynamically imported `agent.context_engine.ContextEngine`.
3. **Registration accepted, not silently ignored** - the log shows "Plugin 'aphrodite' registered context engine: aphrodite"; no "does not inherit from ContextEngine" or "already registered" warning.
4. **Engine selected** - `context.engine: aphrodite` in Hermes config; no "falling back to built-in compressor" warning at agent build.
5. **Hermes built-in compression disabled** - `compression.enabled` is false (default is true; verify live with `hermes config get compression.enabled`, never assume).
6. **One controlled compress/retrieve round trip succeeds** - compress a known payload and retrieve it once (e.g. `aphrodite_test`); normalized content matches the source.
7. **Exactly one active compression owner** - engine selected AND built-in compression off AND no second engine registered by any plugin.

## Hook input structures

Read-only unless documented mutable; `pre_llm_call`'s `conversation_history` is a copy - in-place edits are discarded (mutation belongs to engine/transforms). Output-replacing hooks must return the original output for pass-through; an empty string is a destructive replacement, never "no change". (Full rules: `aphrodite-boundaries`, `aphrodite-hook-contracts`; details: `references/compressed-detail.md`.)

## References

Evidence notes moved here from `aphrodite-hook-reference` (keep as evidence; line numbers are v0.16.0-era snapshots that have drifted - the contract above re-derives from current source):

- `references/context-engine-api.md` - ContextEngine API shape, abstract methods, selection order
- `references/context-engine-integration.md` - registration flow, required interface, engine-to-plugin hooks
- `references/context-engine-pitfalls.md` - silent rejection cases, name @property, update_model signature
- `references/compressed-detail.md` - optional/defaulted signatures, installed-layout probe, env-mapping detail
- `references/session-discoveries-20260615.md` - session findings incl. the isinstance bug, copy semantics, tool-chain boundary split

## Claim-to-test matrix

| Claim                                          | Evidence source                                            | Test                                                                 | Pass condition                                                       | Failure response                                        |
| ---------------------------------------------- | ---------------------------------------------------------- | -------------------------------------------------------------------- | -------------------------------------------------------------------- | ------------------------------------------------------- |
| Engine registration is isinstance-gated        | `hermes_cli/plugins.py` register_context_engine            | Register a plain class; watch the log                                | Warning + engine ignored, registration continues                     | Fix the engine to subclass ContextEngine                |
| Only one engine per manager                    | `hermes_cli/plugins.py:705`                                | Register two engines                                                 | Second registration rejected with warning                            | Keep exactly one engine registration                    |
| Engine selected only when name matches         | `agent/agent_init.py` `_select_context_engine`             | Set `context.engine: aphrodite`; observe agent build                 | `_select_context_engine` returns non-None; no fallback warning       | Enter hooks-only; re-check config name and registration |
| Deepcopy is required for selection             | `agent/agent_init.py:1807`                                 | Engine holds a lock; attempt selection                               | Accurate "could not be safely copied" fallback warning               | Implement `__deepcopy__` or keep engine copyable        |
| `compression.enabled` defaults true            | `cli.py:405`                                               | Fresh config; `hermes config get compression.enabled`                | Reads true                                                           | Re-derive from cli.py defaults                          |
| Built-in compression off with Aphrodite active | `tui_gateway/session_compression.py:86`                    | Run a session with `context.engine: aphrodite` + enabled compression | Exactly one mutator: no host "Compacting context" messages           | Disable `compression.enabled`; stay hooks-only until so |
| Aphrodite engine is a non-destructive owner    | `plugins/aphrodite/__init__.py` `_register_context_engine` | Call `should_compress` and `compress` on the instance                | `should_compress` False; `compress` returns the list unchanged       | Update the plugin engine                                |
| Activation requires all 7 conditions           | This contract                                              | Run the checklist; break one condition at a time                     | Any single failure yields hooks-only state, never "partially active" | Re-run checklist; report exact failing condition        |
| pre_llm_call history edits are discarded       | `agent/turn_context.py:702` (`list(messages)`)             | Mutate the copy in a hook; assert live transcript                    | Live transcript unchanged                                            | Move mutation to engine/transforms; fix the hook        |
