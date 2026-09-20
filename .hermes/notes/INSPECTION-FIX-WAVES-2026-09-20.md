# Inspection Fix Waves - 2026-09-20

Source: turn-by-turn code inspections (Turns 1-6) of `crates/aphrodite/src`
(state, flow, hooks, catalog, session, retrieve, config_loader, proxy review).
Every claim below was verified against the checked-out source before
dispatch. Wave 1 = 4 agents, disjoint file ownership.

## Corrections to the inspection ledger (verified in source)

- Fix 5 retracted: `state/files.rs` `record_file` already dedupes via
  `retain` before `push_front` (files.rs:6).
- Fix 8 retracted: `record_chain_split` calling `adapt_chain_split_threshold`
  is tested behavior - `test_adapt_raises_threshold_when_segments_ignored`
  (state/tests.rs:206) locks the record-time adaptation in.
- Fix 10 resolved: `expires_after_turn = push_turn + ttl`, inclusive;
  locked by `test_list_shows_ephemeral_and_reset_clears_them`.
- Fix 16 corrected: the inspection reads `.iter().rev()` as newest-first for
  `referenced_files`, but `record_file` uses `push_front` (files.rs:7), so
  reverse iteration selects the OLDEST entries - the delta misreports old
  files as new. Selection must come from the front; display in insertion
  order.

## Wave 1 ledger (open fixes, ownership)

| #   | File                                | Issue                                                                                 | Sev  | Agent |
| --- | ----------------------------------- | ------------------------------------------------------------------------------------- | ---- | ----- |
| 1   | state/store.rs + state/mod.rs       | O(n) `retain` LRU promotion → `lru::LruCache` order cache (dep already enabled)       | Med  | A     |
| 2   | state/mod.rs                        | `chain_split_enabled` struct default true vs shipped false                            | Low  | A     |
| 6   | state/mod.rs + state/markers.rs     | `Vec::remove(0)` O(n) eviction → `VecDeque::pop_front`                                | High | A     |
| 7   | state/chain_split.rs                | Linear scan in `note_split_retrieval` - document only (cap 16)                        | Low  | A     |
| 9   | state/mod.rs + events/markers/files | Hardcoded cap literals → named constants                                              | Low  | A     |
| 11  | flow.rs                             | `always_survive` inverted (`is_empty() as usize` counts absent)                       | High | A     |
| 12  | flow.rs                             | `push_nudge` `remove(0)` → `drain(..1)` (keep Vec)                                    | Low  | A     |
| 15  | state/tests.rs                      | Document eviction arithmetic                                                          | Low  | A     |
| 3a  | catalog.rs                          | `format_catalog_table` no sort before `.take(20)`                                     | Low  | B     |
| 3b  | catalog.rs                          | `by_type` HashMap → BTreeMap (serde_json preserve_order ON)                           | Low  | B     |
| 4   | session.rs                          | `catalog_summary` side-effect watermarks - commit after assembly, no rename this wave | Low  | B     |
| 16  | session.rs                          | File delta selects oldest, not newest (corrected above)                               | Low  | B     |
| 17  | session.rs                          | `chain_split_min_segments` leaks across sessions - reset to floor                     | Med  | B     |
| 18  | catalog.rs                          | Header/body count mismatch → truncation notice                                        | Low  | B     |
| 13  | hooks.rs                            | Archives last marker, not most significant (largest size)                             | Med  | C     |
| 14  | hooks.rs                            | Duplicate type-resolution blocks → shared helper                                      | Low  | C     |
| 19  | directives/build.rs                 | Mixed byte/char truncation guard                                                      | Med  | C     |
| 20  | retrieve.rs                         | Poisoned `inline_ccr` lock → silent miss; return 500                                  | Med  | D     |
| 21  | config_loader.rs                    | `get_bool` rejects `TRUE`/mixed case (case-insensitive fix only)                      | Low  | D     |
| 22  | config_loader.rs                    | Malformed `aphrodite.toml` skipped silently → `tracing::warn!` (load + load_from)     | Low  | D     |

Retracted/resolved: 5, 8, 10. No new files created - HPC classification of
existing files is unchanged; this note is the wave record.

## Agent ownership (disjoint, hard constraint)

- Agent A: state/mod.rs, state/store.rs, state/markers.rs, state/events.rs,
  state/files.rs, state/chain_split.rs, state/tests.rs, flow.rs (fixes 1, 2,
  6, 7, 9, 11, 12, 15).
- Agent B: catalog.rs, session.rs (fixes 3a, 3b, 4, 16, 17, 18).
- Agent C: hooks.rs, directives/build.rs (fixes 13, 14, 19).
- Agent D: retrieve.rs, config_loader.rs (fixes 20, 21, 22).

## Wave rules (given to every agent verbatim)

- Do not commit/push; an external auto-committer may sweep - ignore it, leave
  changes uncommitted.
- read_file before editing; patch/write_file only, never sed/awk.
- Match tab indentation; no whole-file reformatting.
- Verify: `cargo test -p aphrodite` then `cargo clippy -p aphrodite --lib -D
warnings`; report ACTUAL command output and test counts.
- Scratch only in `.hermes/tmp/`; resolve CCR markers with
  `aphrodite_retrieve` before acting.
- Read `.hermes/skills/aphrodite-testing-discipline/SKILL.md` and
  `.hermes/skills/aphrodite-boundaries/SKILL.md`; reason about small
  beneficial improvements inside owned files, keep changes minimal and
  behavior-preserving unless the fix says otherwise.

## Deferred (Turn 6 proxy.rs review observations - no numbered fixes)

- AppState 47 pub fields: config / tunable / counters not separated by type.
- std Mutex vs tokio Mutex discipline - comment co-location suggested.
- `build_state` `/tmp` fallback for missing `$HOME` in SQLite path - WARN or
  bail suggested.
- `body_wants_stream` full JSON parse per request - memchr scan suggested.
- Retry loop `1..=3` with `attempt < 3` = 2 retries; comment says 3 - clarify
  comment.
- `proxy_detect_content_type` linter arm precedence - add parentheses; error
  arm `contains` vs `starts_with` false-positive risk.
- `estimate_compressed_size` 0.97 coefficient bias - document direction.
- `compute_fill_pct` hardcoded 9000 vs derived - call at end of build_state.
- `response_cache_get` peek-vs-get recency distinction - comment.

## Wave 1 results (2026-09-20, delegation deleg_f2e68426)

All 19 open fixes landed on disk and were verified site-by-site by the parent.
Agents: A completed (418 lib+bin tests green, clippy clean on its files); B, C,
D died late (429/401 provider quota) with full deliverables on disk - no
re-dispatch needed; parent completed C's one-line test assertion (build.rs
`ends_with` em-dash) and fixed the 3 sibling clippy lints (catalog.rs
sort_by_key, config_loader.rs if-let x2).

| #   | Fix                                              | Status | Evidence                          |
| --- | ------------------------------------------------ | ------ | --------------------------------- |
| 1   | store.rs LRU via lru::LruCache order cache       | ✅     | store.rs:23/48/56 pop_lru/put/get |
| 2   | chain_split_enabled default false                | ✅     | mod.rs:232                        |
| 3a  | catalog sort_by_key Reverse(turn)                | ✅     | catalog.rs:128                    |
| 3b  | by_type BTreeMap                                 | ✅     | catalog.rs:6/44                   |
| 4   | catalog_summary watermarks commit after assembly | ✅     | session.rs:160-161                |
| 6   | recent_markers VecDeque                          | ✅     | mod.rs:82, markers.rs:23          |
| 7   | note_split_retrieval doc (cap 16)                | ✅     | chain_split.rs:50                 |
| 9   | named caps TOOL/RECENT_MARKERS/REFERENCED_FILES  | ✅     | mod.rs:46-50                      |
| 11  | always_survive counts present sections           | ✅     | flow.rs:113-116 + regression test |
| 12  | push_nudge drain(..1)                            | ✅     | flow.rs:167                       |
| 13  | archive largest-size marker                      | ✅     | hooks.rs:464 max_by_key           |
| 14  | shared resolve_type_and_classify_content         | ✅     | hooks.rs:132-141                  |
| 15  | eviction arithmetic documented                   | ✅     | state/tests.rs:95/110             |
| 16  | file delta front-selection + insertion order     | ✅     | session.rs:125-129                |
| 17  | chain_split_min_segments reset to floor          | ✅     | session.rs:23 + test              |
| 18  | table truncation notice                          | ✅     | catalog.rs:140-142                |
| 19  | char-count truncation gate                       | ✅     | build.rs:48 + em-dash tests       |
| 20  | poisoned lock → 500                              | ✅     | retrieve.rs:83-88 + test          |
| 21  | get_bool case-insensitive                        | ✅     | config_loader.rs:96/100           |
| 22  | warn on TOML parse failure (load + load_from)    | ✅     | config_loader.rs:43/75 + test     |

## Post-wave notes

- `cargo test -p aphrodite` green (final count recorded below when the parent
  re-run completes); clippy re-run pending after the lib suite.
- External parallel edits detected OUTSIDE the wave: `crates/aphrodite-hermes/
src/{lib,schemas,tools}.rs` carry rustfmt-only diffs not made by any wave
  agent (no `cargo fmt` in any transcript) - attributed to the user's parallel
  tooling/session; left untouched per the parallel-work rule.
- Agent B redirected one check output to `/tmp/cc_out.txt` (scratch rule
  violation - should be `.hermes/tmp/`); transient file, noted only.
- No commits made by any agent; tree left uncommitted for the auto-committer
  sweep. Verify `git log --oneline -5` at close.

## Deferred (Turn 6 proxy.rs review observations - no numbered fixes)

- AppState 47 pub fields: config / tunable / counters not separated by type.
- std Mutex vs tokio Mutex discipline - comment co-location suggested.
- `build_state` `/tmp` fallback for missing `$HOME` in SQLite path - WARN or
  bail suggested.
- `body_wants_stream` full JSON parse per request - memchr scan suggested.
- Retry loop `1..=3` with `attempt < 3` = 2 retries; comment says 3 - clarify
  comment.
- `proxy_detect_content_type` linter arm precedence - add parentheses; error
  arm `contains` vs `starts_with` false-positive risk.
- `estimate_compressed_size` 0.97 coefficient bias - document direction.
- `compute_fill_pct` hardcoded 9000 vs derived - call at end of build_state.
- `response_cache_get` peek-vs-get recency distinction - comment.

## Post-wave verification (parent, mandatory)

1. Full `cargo test -p aphrodite` in the parent shell - DONE: lib 389 passed,
   0 failed, 1 ignored (up from 377 pre-wave, +12 new tests); bin suites green
   (roundtrip_regression 4, serde_json_features 2, doc-tests 0).
2. `cargo clippy -p aphrodite --lib -- -D warnings` - DONE: clean; one
   pre-existing manifest advisory (toml version semver metadata, Cargo.toml
   line 90, not wave code, left untouched).
3. Grep each fix site to confirm the change landed on disk. - DONE (table
   above).
4. `git log --oneline -5` + `git status --short` - DONE: HEAD unchanged
   (cd858a0), no auto-commits; wave files uncommitted as intended.
5. Update this ledger with wave results. - DONE.

WAVE 1 COMPLETE - all 19 open fixes landed, verified, green.

---

## Wave 2 - Turn 6 proxy.rs deferred observations (2026-09-20)

All items are in `crates/aphrodite/src/proxy.rs` - single file, single agent
ownership (never split one file across siblings). All claims verified against
source before dispatch.

| #    | Item                                                                                                                                                                      | Type                                                                                  | Anchor                   |
| ---- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- | ------------------------ |
| W2-1 | AppState 47-field doc: categorize config / tunable / counters; structural AppConfig/AppCounters split EXPLICITLY deferred                                                 | doc                                                                                   | proxy.rs:180             |
| W2-2 | Mutex discipline comment: std Mutex never held across `.await` (!Send)                                                                                                    | doc                                                                                   | proxy.rs:212-213         |
| W2-3 | `build_state` `/tmp` fallback for missing `$HOME` - WARN, degrade, never bail                                                                                             | code                                                                                  | proxy.rs:666-667         |
| W2-4 | `body_wants_stream` byte-exact fast path (`"stream":true`, `"stream": true`); false positives impossible (JSON-string quotes are escaped); full parse stays the authority | code                                                                                  | proxy.rs:768-773         |
| W2-5 | Retry loop comment: up to 3 attempts (2 retries); loop logic unchanged                                                                                                    | doc                                                                                   | proxy.rs:1027-1088       |
| W2-6 | `proxy_detect_content_type`: explicit parens on the `                                                                                                                     | `-linter arm; error arm markers `contains`→`starts_with` (false-positive fix) + tests | code                     | proxy.rs:1360-1379 |
| W2-7 | `estimate_compressed_size`: comment on conservative 0.97 direction                                                                                                        | doc                                                                                   | proxy.rs:338-369         |
| W2-8 | `fill_pct` initial value derived from `INITIAL_RATIO_EMA` const (kills manual 9000 sync) in build_state + test fixtures                                                   | code                                                                                  | proxy.rs:734, 2552, 3071 |
| W2-9 | `response_cache_get`: peek-no-promote vs get-promote comment                                                                                                              | doc                                                                                   | proxy.rs:825-838         |

Verification: `cargo test -p aphrodite` (proxy:: plus full suite), `cargo
clippy -p aphrodite --lib -- -D warnings`. Same wave rules as Wave 1 (no
commits, patch/write_file only, tabs, report actual output).

## Wave 2 results (delegation deleg_14301d25)

First dispatch died instantly (8s, HTTP 401 first call - documented transient
pattern); solo re-dispatch landed all 9 items (98 API calls, 1013s).

| #    | Item                                                                                                                                                      | Status                  |
| ---- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------- |
| W2-1 | AppState field-category doc comment (structural split deferred)                                                                                           | ✅                      |
| W2-2 | std-Mutex-never-across-await rule comment                                                                                                                 | ✅                      |
| W2-3 | `/tmp` fallback WARN (HOME unset), degrade not bail                                                                                                       | ✅                      |
| W2-4 | `body_wants_stream` byte-exact fast path; agent corrected the spec: real JSON is `"stream":true` (key quote included), unquoted pattern could never match | ✅                      |
| W2-5 | retry comment: up to 3 attempts (at most 2 retries)                                                                                                       | ✅                      |
| W2-6 | error arm `contains` → `starts_with` + parens on `                                                                                                        | ` arm + regression test | ✅  |
| W2-7 | conservative 0.97 direction comment                                                                                                                       | ✅                      |
| W2-8 | `INITIAL_RATIO_EMA` const + `initial_fill_pct()` const fn at all 3 init sites                                                                             | ✅                      |
| W2-9 | peek-no-promote vs get-promote comment                                                                                                                    | ✅                      |

Parent verification: lib 393 passed / 0 failed / 1 ignored (+4 new proxy
tests); clippy `-- -D warnings` clean (pre-existing toml manifest advisory
only); all 9 sites grepped on disk.

## Auto-committer note (2026-09-20)

The external auto-committer swept Wave 1 into commit `a0dec1e` ("apply
inspection wave of state, catalog, and session correctness fixes", 18 files,
+836/-121) - including the ledger note and the three external aphrodite-hermes
rustfmt diffs. Not fought (legitimate session work). Wave 2 (`proxy.rs`) was
still uncommitted at close; the sweep picks it up or the user's Save tool does.
