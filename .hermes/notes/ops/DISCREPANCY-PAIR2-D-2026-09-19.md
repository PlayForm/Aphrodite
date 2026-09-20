# DISCREPANCY-PAIR2-D-2026-09-19

Append-only log of verified-stale claims found while rewriting
docs/ccr/content-types.md -> docs/classification/content-types.md (child D of
PAIR-2, classification category).

- [2026-09-19] docs/ccr/content-types.md: claimed 26 content types in the
  registry table; the verified pipeline emits 30 distinct types across its
  classifier stages - headroom base 7 (content_detector.rs enum:33-56), semantic
  15 (preview.rs detect_semantic_type:37-251), proxy 14 native
  (proxy.rs proxy_detect_content_type:1325-1469), bridge unwrap 4
  (tools.rs unwrap_hermes_result:67-233: terminal, build_error, build_output,
  search).
- [2026-09-19] docs/ccr/content-types.md: described a "Python plugin"
  classifier with its own registry (build_error, search_results,
  process_output, search_files, tabular, json_list, tool, terminal,
  aphrodite, context, build, compress) and a 5,000-char detection order; no
  Python classifier exists - plugins/aphrodite/**init**.py is a thin ctypes
  shim (grep classify: 0 hits in plugin and templates/**init**.py mirror);
  classification is entirely Rust-side. build_error/terminal/search survive
  only via tools.rs unwrap_hermes_result.
- [2026-09-19] docs/ccr/content-types.md: "Code (×4)" threshold stale; code
  multiplier defaults to 3.0 and is live-configurable via
  compression.code_multiplier / APHRODITE_CODE_MULTIPLIER
  (proxy.rs:466,492-494).
- [2026-09-19] docs/ccr/content-types.md: Detection Order (Rust) omitted the
  detect_semantic_type passthrough stage, which runs between the code step
  and the error step (proxy.rs:1397-1399); git status/log, ls, grep, test,
  table, markdown, yaml, xml, csv, html shapes were unaccounted for.
- [2026-09-19] docs/ccr/content-types.md: threshold claim "noisy types
  compress at base/2" was already self-corrected in the old doc (kept at
  base); verified threshold_for returns base immediately for
  linter/build_output/log before auto-tune (proxy.rs:470-501).
- [2026-09-19] docs/README.md (orchestrator-owned, NOT edited): still claims
  "26-type classifier" and a Python `_classify_content`
  (docs/README.md:7,33,39,41); orchestrator should update to the 30-type
  pipeline and the new docs/classification/ paths during the link pass.
