# Discrepancy log - PAIR-5-B (docs/tool-relay) - 2026-09-19

Append-only log of stale claims found while rewriting docs/tool-relay/tools.md
and docs/tool-relay/callbacks.md against live source. Format:
`- [date] file: claim X stale; source says Y (file:line)`.

## tools.md

- [2026-09-19] docs/tool-relay/tools.md: stats example `"version": "1.2.1"` stale; version is the crate's CARGO_PKG_VERSION, currently 1.4.6 (crates/aphrodite-hermes/Cargo.toml:3, crates/aphrodite-hermes/src/tools.rs:351)
- [2026-09-19] docs/tool-relay/tools.md: stats example `terminal_threshold: 1024` stale; shipped default is 256 (crates/aphrodite/src/state.rs:244)
- [2026-09-19] docs/tool-relay/tools.md: catalog "default full mode adds {turn}" stale; full mode adds {turn} AND {center} (crates/aphrodite-hermes/src/tools.rs:393-396)
- [2026-09-19] docs/tool-relay/tools.md: reclassify schema advertises an `action` param ("all"); stale - the implementation takes only an optional `hash`, and omitting it reclassifies every entry in the session (crates/aphrodite-hermes/src/tools.rs:504-530, schemas.rs:288-297)
- [2026-09-19] docs/tool-relay/tools.md: test description "compress, retrieve, search, stats, files, diff, proxy health" stale; the tool only round-trips compress -> retrieve on built-in samples and reports proxy health (crates/aphrodite-hermes/src/schemas.rs:232-239, tools.rs:533-573)
- [2026-09-19] docs/tool-relay/tools.md: "active directive bodies injected ... appended after the catalog summary" stale; directives are an always-survive section assembled BEFORE the droppable recall catalog (crates/aphrodite/src/flow.rs:51-58, 91-104)
- [2026-09-19] docs/tool-relay/tools.md: "markers stay resolvable for the life of the session" needs the caveat that markers do NOT survive a dylib hot-reload (crates/aphrodite-hermes/src/schemas.rs:68-69)
- [2026-09-19] docs/tool-relay/tools.md: link to tree/Development/docs/ccr/content-types.md stale; the content-type taxonomy moved to docs/classification/content-types.md and public links must use tree/Current (default branch is Current)
- [2026-09-19] docs/tool-relay/tools.md: troubleshooting links used tree/Development; stale, default branch is Current

## callbacks.md

- [2026-09-19] docs/tool-relay/callbacks.md: "callback POST uses Bearer auth" stale for the tool-relay leg; the async callback POST sends no auth header at all - only the ccr-created notification adds `Authorization: Bearer {notify_key}` when a key is configured (crates/aphrodite/src/proxy.rs:2264-2270 vs 2483-2486)
- [2026-09-19] docs/tool-relay/callbacks.md: "invalid callback_url silently dropped, proxy returns a normal synchronous-style response" stale; a non-https callback_url now returns HTTP 400 with `{success: false, error: "callback_url must use the https scheme"}` (crates/aphrodite/src/proxy.rs:2235-2244)
- [2026-09-19] docs/tool-relay/callbacks.md: "Both notify_url and notify_key must be set for callbacks to fire" stale; only notify_url is checked - notify_key is optional and merely adds the Bearer header when present (crates/aphrodite/src/proxy.rs:2466, 2484-2486)
- [2026-09-19] docs/tool-relay/callbacks.md: metrics named `aphrodite_notify_success`/`aphrodite_notify_failure` stale; the Prometheus names are `aphrodite_notify_success_total`/`aphrodite_notify_failure_total` (crates/aphrodite/src/main.rs:553-554)
- [2026-09-19] docs/tool-relay/callbacks.md: "Fires when a new CCR entry is created" imprecise; the notify webhook fires only on `POST /ccr/create` (crates/aphrodite/src/proxy.rs:2466), never on chat-compression or tool-side compress paths
- [2026-09-19] docs/tool-relay/callbacks.md: "success/failure tracked via notify counters" applied to the tool-relay callback is stale; the async callback POST's outcome is deliberately ignored (`let _ =`) and relay results are counted separately as `tool_relay` success/failure (crates/aphrodite/src/proxy.rs:2259-2270, 418-422)
- [2026-09-19] docs/tool-relay/callbacks.md: config section only showed TOML; notify_url/notify_key are also CLI flags and env vars APHRODITE_NOTIFY_URL/APHRODITE_NOTIFY_KEY, with env taking precedence over TOML (crates/aphrodite/src/config.rs:196-201, 389-390)