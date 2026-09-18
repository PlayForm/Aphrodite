# HANDOFF - current pending queue (2026-09-18)

Read this FIRST when resuming work on the Aphrodite monorepo. State: branch
`Development`, binary **1.4.6** / plugin **2.1.4**. Issue #11 WS1 + WS2 + WS4

- residuals 1/2/4 landed; WS3 + residual 3 pending. Skills are dev-side
  (`.hermes/skills/`, never shipped). No `.githooks` exist - branch discipline
  is manual. The auto-committer sweeps working-tree changes; do not commit
  manually unless asked; no rebase ever.

## Verified-done recap (this wave)

- SIGSEGV fixed + FFI hardened (probe sentinel, forced `c_void_p` restype,
  same-handle free, `free_string` argtypes fix); repro SURVIVED (6 threads x
  300, exit 0).
- FFI codegen pipeline generated + validated: `_bindings.py` byte-stable,
  checker PASS 0 violations, codegen self-tests 23/23, checker self-tests
  13/13, drift-guard identical.
- Issue #11 WS1/2/4 landed; battery-after: direct 65% -> 26.7%, hook 55% ->
  30%; residual rewrite landed (`3b8b5d3`/`1d202b2`/`7674456`).
- FFI CI fixes landed (Pair A): ruff on generated artifact + N812, checker
  self-test rewrite, ffi-check.yml trigger paths.
- Ceremony B4 audit executed: **Current's `Auto.yml` leak found** (see 2.2).

## 1. Release: Aphrodite/v1.4.6 does NOT exist on GitHub yet

- Latest published is v1.4.5; bumps are local only. `_check_version_published`
  now warns instead of 404ing. The ceremony is pending per
  `release/RELEASE-METHODOLOGY.md` (and the condensed `CEREMONY.md`):
    1. Run the **B4 branch-identity audit** (mandatory pre-tag gate, I11) on
       both refs - currently NOT clean (see 2.2).
    2. Phase A: plugin sync first (S-Development -> S-Current), then parent
       sync (P-Development -> P-Current), identity files restored, tags
       (`v2.1.4` plugin, `Aphrodite/v1.4.6` parent) at the END, `gh release
create --notes-file`, Build.yml Finalize 12 assets.
    3. BINARY_VERSION bumped LAST (live download pointer - the 2026-09-17 404
       was exactly this).
    4. Phase B sync-back after publish; re-run B4.
- Chain-split rides the 1.4.6 release: opt-in, default OFF in shipped config
  (invariant D2; `git grep -c 'chain_split'` per-track decision at A2).

## 2. Open items

### 2.1 Fix Current's Auto.yml leak (ceremony blocker, ABORT-class)

`git show Source/Current:.github/workflows/Auto.yml` line 68 pushes
`branch: Development` - inverts the branch-owned identity (Development is the
append-only workshop line). Development's own copy pushes `branch: Current`
(sanctioned heartbeat, touches only CI-ignored `.github/Update.md`).
Fix Current separately (read-only ref access only; no checkout of Current
without the ceremony window), re-run the B4 audit clean. Detail:
`release/CEREMONY-AUDIT.md`.

### 2.2 Minor ceremony notes

- `.githooks/*` still tracked on Current (stale comment + divergence after
  the repo-wide removal) - cleanup candidate.
- `Maintain/scripts/release/auto-release.sh:27` derives `RELEASE_BRANCH` from
  HEAD with a `Current` fallback on detached HEAD - latent risk, not active.

### 2.3 1.5.0 backlog (from research + notes)

- **WS3 proxy-pipeline parity** (deferred from Issue #11; `proxy.rs` parallel
  preview arms -> route through `build_preview`) - first on the list.
- **Residual defect 3**: `search_shape_array` hook case - hits label counts
  the `matches` array (1) not `total_count` (150); truncated-total signal
  still invisible. (`matches_text`, the real Hermes shape, is fixed.)
- Structured hook payloads (hermes-agent rich kwargs; plugin flattens with
  `default=str`).
- Rust teardown export for authoritative dylib unload (roto pattern).
- Stress-test the pipeline against rust-bindgen corpus shapes before adding
  new exports.
- CI: exercise the real codegen path (install ctypesgen; byte-identity check
  vs committed `_bindings.py`) - CI still tests only the fallback path.
- `proxy_health` export decision (dead for the plugin; HTTP /health used).
- `model_family` / `code_structure_map` / `rust_preview_lines`: wire or
  remove (only `preview_max_chars` was wired).
- `ops/SPLIT-ADAPTATION.md` holds the 1.5.0 blueprint for the
  `detect_semantic_type` / preview.rs regex refactor (fancy-regex vs
  regex/aho-corasick research + isolated 7-session execution order).

### 2.4 Housekeeping

- Documentation repo: `Module/Astro` phantom self-referential gitlink (same
  auto-committer class as the plugin submodule's) - cleanup there.
- Pre-existing CI blockers (NOT FFI, unchanged): `kompress_parity` E0432
  (`ml`-feature gate in vendor/headroom) blocks the dylib build + pytest FFI
  steps in CI; clippy doc-lazy-continuation at
  `crates/aphrodite-hermes/src/lib.rs:539`.

## 3. How to resume (checklist)

1. `cd` to the monorepo, branch Development; submodule `plugins/aphrodite` on
   Development; verify no phantom gitlink (`git ls-files -s` shows no 160000).
2. Sanity sweep: `python3 Maintain/check_ffi_contract.py` (PASS),
   `python3 sigserve/repro.py` (SURVIVED), `cargo test -p aphrodite` (406),
   `cargo test -p aphrodite-hermes` (52),
   `python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py` (23),
   `python3 Maintain/tests/test_check_ffi_contract.py` (13/13), drift-guard
   `diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py`.
3. Next task: 1 (release ceremony incl. the B4 gate + fixing 2.1), then 2.3
   (WS3 first).
4. Working style: pairs of 2 subagents per task, disjoint file ownership;
   scratch in the Temporary dir, durable records in `.hermes/notes/`
   (prettier-clean, anonymized).
