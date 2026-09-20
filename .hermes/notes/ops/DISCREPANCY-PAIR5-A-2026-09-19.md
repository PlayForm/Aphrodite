# Discrepancy log - PAIR-5-A (docs/plugin/) - 2026-09-19

Append-only log of verified-stale claims found while rewriting
docs/plugin/hooks.md, docs/plugin/directives.md, docs/plugin/context-engine.md.
Format: `- [date] file: claim X stale; source says Y (file:line)`.

- [2026-09-19] docs/plugin/hooks.md: claimed FIVE hook handlers and self-labeled "historical" (retired fat-Python plugin); the live plugin registers SIX hooks per-hook - `pre_tool_call` was missing from the old list (plugins/aphrodite/plugin.yaml:7-13; crates/aphrodite-hermes/src/lib.rs:506-523; registered at plugins/aphrodite/**init**.py:1168-1182).
- [2026-09-19] docs/plugin/hooks.md: tool-result thresholds TOOL_THRESHOLD_TOKEN=1,024 / TOOL_THRESHOLD_CACHE=8,192 / INLINE_THRESHOLD=4,096 stale; the dylib path uses a single tool threshold, default 4,096 via APHRODITE_TOOL_THRESHOLD_TOKEN / [compression] tool_threshold_token (crates/aphrodite/src/config_loader.rs:141-142).
- [2026-09-19] docs/plugin/hooks.md: TERMINAL_THRESHOLD default 2,048 stale; default is 1,024 via APHRODITE_TERMINAL_THRESHOLD / [compression] terminal_threshold (crates/aphrodite/src/config_loader.rs:143-144).
- [2026-09-19] docs/plugin/hooks.md: "Dev Mode" passthrough (APHRODITE_PASSTHROUGH / HERMES_DEV) no longer exists anywhere in crates/ or plugins/ (0 grep hits).
- [2026-09-19] docs/plugin/hooks.md: terminal "build output detection" (collapse repeated lines, extract error patterns) no longer exists; the terminal path is telemetry -> chain-split (opt-in) -> threshold gate -> classify -> inline store -> marker (crates/aphrodite/src/hooks.rs:249-397).
- [2026-09-19] docs/plugin/directives.md: "six directives baked into the binary" stale; the built-in set is SEVEN, including `lazy-eval` (crates/aphrodite/src/directives.rs:25-35; 7 .md materialized at ~/.hermes/aphrodite/directives/).
- [2026-09-19] docs/plugin/directives.md: injection "appended after the catalog summary" stale; directives are assembled FIRST in flow::build_turn_context (never dropped under budget pressure) and the recall catalog is the last/dropped-first section (crates/aphrodite/src/flow.rs:51-59, 87-99).
- [2026-09-19] docs/plugin/context-engine.md: threshold percent default 50 stale; default is 45 (crates/aphrodite/src/config_loader.rs:127-128).
- [2026-09-19] docs/plugin/context-engine.md: protect_first_n default 1 stale; default is 2 (crates/aphrodite/src/config_loader.rs:130-131).
- [2026-09-19] docs/plugin/context-engine.md: protect_last_n default 1 stale; default is 5 (crates/aphrodite/src/config_loader.rs:132-133).
- [2026-09-19] docs/plugin/context-engine.md: min_messages_to_compress default 4 stale; engine_min_msgs default is 8 (crates/aphrodite/src/config_loader.rs:129).
- [2026-09-19] docs/plugin/context-engine.md: the entire "Compress Algorithm" (middle-message packing, editing detection, orphan tool sweep, proxy/inline compression, `aphrodite_engine_compressed` hook, get_status, engine session lifecycle) is stale; the current engine is a thin opt-in pass-through whose should_compress() always returns False and compress() returns the transcript unchanged (plugins/aphrodite/**init**.py:1242-1255) - compression is done by the transform hooks + proxy, never the engine.
- [2026-09-19] docs/plugin/context-engine.md: registration claimed driven by plugin.yaml alone; the engine is registered only when APHRODITE_CONTEXT_ENGINE=1 (plugins/aphrodite/**init**.py:1206), while plugin.yaml provides_context_engine: true (plugins/aphrodite/plugin.yaml:28) is the manifest capability flag.
