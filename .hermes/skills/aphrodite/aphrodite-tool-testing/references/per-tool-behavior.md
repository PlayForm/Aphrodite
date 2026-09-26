# Per-tool behavior notes

- `aphrodite_compress` - `content` is required; the `type` hint wins over
  auto-detection. It returns `hash` + `marker`. Never retrieve to "verify
  storage", because the returned marker IS the proof of compression; retrieve
  only when the action needs the content.
- `aphrodite_retrieve` - `hash` is required for CCR resolution; requests with
  only `query` and no `hash` are rejected with a readable error (validation in
  `crates/aphrodite/src/proxy.rs`).
- `aphrodite_test` - `mode="quick"` runs 1 sample; `mode="full"` (any
  non-quick value) runs the 3-check round-trip set (source_code/build/
  json_array). There is no `matrix`/`pipeline` mode (CLAIM: no probe in the
  source). Expect `status="ok"`.
- `aphrodite_prefetch` - reads + compresses files in the background; markers
  are returned inline. `aphrodite_prefetch_status` shows loading/ready/errors.
  Use prefetch for batches of 3+ files.
- `aphrodite_reclassify` - retroactive metadata enrichment; omit `hash` to
  process all entries.
- `aphrodite_directive` - `action` in list/swap/add/remove/reset. The active
  directives are `focus` (targeted execution, preview-aware retrieval) and
  `foresight` (anticipate I/O: after `search_files`, prefetch the top 5-10
  results).
- `aphrodite_rebuild` - reports binary version + proxy health and a rebuild
  hint; run `aphrodite_rebuild` and the output is a report, never a rebuild or
  a restart of anything.
- `aphrodite_debug` - toggles per-session debug output, Rust-side only; it is
  not registered in `plugin.yaml` (runtime-derived; see the inventory note in
  SKILL.md).
