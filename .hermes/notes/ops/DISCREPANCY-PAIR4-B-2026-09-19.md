# Discrepancy log - PAIR-4-B (docs/api/ + docs/metrics/)

Append-only log of verified-stale claims found while rewriting docs/api/ and
docs/metrics/ against live source (2026-09-19, binary 1.4.6).

- [2026-09-19] docs/api/health.md: example response showed `"version": "1.3.6"`; source returns `CARGO_PKG_VERSION` = 1.4.6 (crates/aphrodite/src/proxy.rs:2713).
- [2026-09-19] docs/api/retrieve.md: "Retrieve Flow" step 4 claimed a zstd-magic-byte decompression branch (0x28, 0xB5, 0x2F, 0xFD) with 500-on-failure; that branch is removed dead code - backends store/return content verbatim as UTF-8 `String`, no zstd anywhere on the path (crates/aphrodite/src/retrieve.rs:121-130).
- [2026-09-19] docs/api/retrieve.md: filter_content snippet truncated the query with a raw byte slice `&q[..512]`; source truncates char-boundary-safe via `floor_boundary(q, 512)` (crates/aphrodite/src/retrieve.rs:165).
- [2026-09-19] docs/api/retrieve.md: did not document hash normalization; `handle_retrieve` strips a `|type|size` marker-body suffix and whitespace via `marker::normalize_hash` before lookup (crates/aphrodite/src/retrieve.rs:53, crates/aphrodite/src/marker.rs:14).
- [2026-09-19] docs/api/ccr-endpoints.md: claimed "All three endpoints"; source routes FOUR management routes - /ccr/create, /ccr/list, /ccr/{hash}, plus POST /reload (hot reload), which was undocumented (crates/aphrodite/src/main.rs:591-594, crates/aphrodite/src/proxy.rs:2640).
- [2026-09-19] docs/api/ccr-endpoints.md: "Python Plugin Usage" table listed `_compress_via_proxy`, `_compress_handler`, `_store_conversation_turn`; no such functions exist - the plugin compresses/retrieves in-process via its dylib bindings and only calls the proxy HTTP surface for GET /health (plugins/aphrodite/__init__.py:846,876).
- [2026-09-19] docs/api/ccr-endpoints.md: create endpoint error table was missing 503 "CCR not enabled" and 500 "failed to store content in CCR backend" (crates/aphrodite/src/proxy.rs:2420-2436, 2538-2554).
- [2026-09-19] docs/api/retrieve.md, docs/metrics/prometheus.md, docs/metrics/queries.md: links pointed at `tree/Development`; default branch is `Current` (all rewritten links use tree/Development).
- [2026-09-19] docs/api/retrieve.md: pagination flow did not note that an empty stored document (`content: ""`) is a valid zero-line result, not an out-of-range offset (crates/aphrodite/src/retrieve.rs:203-205).