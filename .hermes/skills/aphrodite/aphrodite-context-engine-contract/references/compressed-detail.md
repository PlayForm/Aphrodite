# Compressed Detail (moved from SKILL.md)

Worked detail moved out of `SKILL.md` during a size-compression pass. Content is
verbatim from the source skill; line numbers remain observational snapshots.
SKILL.md keeps the one-line rule per item and points here.

## Optional / defaulted ContextEngine members

**Optional / defaulted (safe to override or ignore):**

- `update_model(model, context_length, base_url="", api_key="", provider="", api_mode="")` - model-switch handling: 7 params, no `**kw` in the current ABC signature; the historical reference note listed `**kw` - verify against `agent/context_engine.py` before matching.
- `select_context(request_messages, ...)` - request-only context replacement.
- `on_session_start(session_id, **kwargs)`, `on_session_end(session_id, messages)`, `on_session_reset()` - real session boundaries only, never per-turn.
- `should_compress_preflight(messages)`, `has_content_to_compress(messages)`.
- `get_tool_schemas()`, `handle_tool_call(name, args, **kwargs)`.
- `get_status()`, `on_turn_complete(...)`.
- Default attributes: `last_prompt_tokens`/`last_completion_tokens`/`last_total_tokens`/`threshold_tokens`/`context_length`/`compression_count` (0), `threshold_percent` (0.75), `protect_first_n` (3), `protect_last_n` (6), `emit_automatic_compaction_status` (True).

## Installed layout probe

**Installed layout is hooks-only:** `~/.hermes/plugins/aphrodite` holds ONLY `plugin.yaml` + `__init__.py` (the loader - there is no `_core/`, no `_hooks/`); everything else lives in `~/.hermes/aphrodite/` (`binaries/`, `aphrodite.toml`, the `BINARY_VERSION` pin, `ccr.db`, `directives/`). The repo-side `plugins/aphrodite/__init__.py` is the source of that loader. CLAIM: no probe command in the source for this layout; re-derive from `plugins/aphrodite/__init__.py`.

## APHRODITE_CONTEXT_ENGINE: two consumers

**The same env var name has two consumers** (`config_loader.rs:126`): `APHRODITE_CONTEXT_ENGINE` (or TOML `compression.context_engine`, **default true**) toggles the dylib's internal engine behavior, while the plugin's `_env_bool("APHRODITE_CONTEXT_ENGINE")` gates whether the engine is registered with Hermes at all. Do not assume one setting controls both, because they gate different layers: dylib behavior versus plugin registration.

## Env-driven upstream config and API-key export

**Env-driven upstream config (setup does not write it):** `aphrodite setup` parses `--api-key`/`--api-url`/`--model`, but the TOML template substitutes ONLY the proxy ports (cache 9797 / token 9798) - there are no placeholders for key/url/model. `api_url` and `model` are env-driven (`APHRODITE_API_URL` / `APHRODITE_MODEL`); the proxy's API key comes from the `APHRODITE_API_KEY` env var or `[defaults] api_key` in the TOML. Never claim setup writes them into the TOML, because the template has no placeholders for key/url/model.

**API key must be actually exported:** the proxy fails loudly with `no API key configured - set APHRODITE_API_KEY env var` when the variable is absent OR commented out in the environment file - a commented-out line behaves exactly like an absent var. Verify the var is exported (`env | grep APHRODITE_API_KEY`), never assume from the file's text.

## Plugin-side registration detail

Plugin side (`plugins/aphrodite/__init__.py`, `_register_context_engine`): registration is **opt-in** - only when `APHRODITE_CONTEXT_ENGINE` is truthy (`1`/`true`, `_env_bool`). It imports `agent.context_engine.ContextEngine` dynamically (the module is only importable inside the Hermes runtime) and registers `AphroditeContextEngine` with `name == "aphrodite"`, `should_compress -> False`, and `compress -> messages` unchanged (non-destructive: the transform hooks and proxy do the actual shrinking, so the engine itself never forces a compaction). A registration failure logs a warning and falls back to hooks + proxy.

## Engine selection steps (per agent build)

1. `context.engine` from the agent config; **default `"compressor"`** (built-in).
2. `"compressor"` -> `None` (built-in `ContextCompressor`; plugin engines are NOT auto-activated).
3. Otherwise: `plugins/context_engine/<name>/` loader first, then `get_plugin_context_engine()` (the general plugin system) - the candidate must have `.name == engine_name`.
4. The selected candidate is **deep-copied** (`copy.deepcopy`); an engine that cannot be copied (locks, DB connections) falls back to the built-in compressor with an accurate warning - the plugin engine should be deepcopy-clean (`AphroditeContextEngine` holds no uncopyable state).
5. Not found / name mismatch: warning "Context engine '<name>' not found - falling back to built-in compressor", built-in wins.

## Hook input structures

## Hook input structures (read-only, contract-level consequences)

- Hook input structures are read-only unless the framework documents mutability; `pre_llm_call`'s `conversation_history` is a copy - in-place edits are discarded, so message mutation belongs to the engine/transforms, never to in-place hook edits.
- A hook that replaces output must return the original output for pass-through; an empty string is a destructive replacement, never "no change". (Full rules: `aphrodite-boundaries`, context boundaries; per-hook return contracts: `aphrodite-hook-contracts`.)
