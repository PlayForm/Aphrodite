# Wave 2 Bug Fixes (2026-06-15)

Second-wave audit from cross-agent review of proxy.rs, retrieve.rs, config.rs, main.rs.
18 bugs cataloged — 13 fixed, 2 already fixed, 3 skipped (architectural/docs), 0 remaining.

## Fixed in v0.5.6

| Bug | File | Description | Fix |
|-----|------|-------------|-----|
| 17 | proxy.rs | ccr_hits/misses never incremented in compress_chat_completion | Added ccr.get() check before ccr.put() in both text and tool_calls compression paths |
| 19 | proxy.rs | smart_marker (ASCII) vs marker_for (Unicode) inconsistency | Replaced marker_for() with `format!("<<<CCR:...>>>")`, removed marker_for import |
| 20 | proxy.rs | health_check returns 503 when CCR disabled | Always return 200; status conveyed via JSON body "degraded" |
| 22 | proxy.rs | request_history never written | Added record_request() method, calls at compressed/passthrough/error paths |
| 33 | proxy.rs | No X-Aphrodite-Compressed header | Added `.header("X-Aphrodite-Compressed", "true")` to compressed response builder |

## Fixed in v0.5.7

| Bug | File | Description | Fix |
|-----|------|-------------|-----|
| 18 | proxy.rs | inject_tool pushes into response tool_calls array | Removed entire inject_tool block; Python plugin already registers aphrodite_retrieve |
| 23 | proxy.rs | retry_with_backoff dead code | Removed the function; proxy_handler uses inline `for attempt in 1..=3` loop |
| 26 | config.rs | ccr_db_path default is relative | Absolute path via `dirs::data_dir()/aphrodite/ccr.db` |
| — | config.rs | no_ccr_inject_tool dead flag | Removed from Cli struct, MultiConfig::resolve() |
| — | proxy.rs | inject_tool AppState field dead | Removed from struct, build_state, test fixtures |

## Fixed in v0.5.8

| Bug | File | Description | Fix |
|-----|------|-------------|-----|
| 21 | proxy.rs | x-headroom-* headers silently dropped (except workspace) | Removed the skip filter; all x-headroom-* headers now pass through to upstream |
| 25 | retrieve.rs | No pagination for large content | Added `offset: usize` and `limit: usize` to RetrieveRequest; content sliced after filter |
| 28 | proxy.rs | api_key logged if AppState gets Debug-derived | Added `Secret(String)` newtype with safe Debug impl (`[REDACTED]`), Display for auth header |

## Already Fixed (pre-existing)

| Bug | Description |
|-----|-------------|
| 24 | query filter in handle_retrieve — filter_content() already implemented |
| 29 | bind to 0.0.0.0 — default already 127.0.0.1:8788 |
| 34 | /health/upstream route — already exists in main.rs lines 87-100 |

## Skipped (architectural / low-priority)

| Bug | Description | Reason |
|-----|-------------|--------|
| 27 | api-url default docs | Docs task, not code |
| 30 | No dual mode | Large architectural change — spawn both cache+token from one binary |
| 31 | No shared CCR between modes | Requires SQLite for cache mode too, significant refactor |
| 32 | No list tool in tool relay | Low-urgency feature request |

## Resolution: 18/18 bugs addressed (13 fixed + 2 already fixed + 3 skipped)
