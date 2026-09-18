# DOC-REWRITE-PLUGIN-2026-09-18 - plugin repo comments + docs on Development (S4)

Scope: `plugins/aphrodite` (submodule PlayForm/Aphrodite-Hermes, branch
Development, HEAD 47cb321, clean). Comments/docs in `__init__.py` (loader,
verified only - see constraint below), `download.sh`, `download.ps1`,
`layout_check.py`, `layout_schema.json`, `README.md`, `plugin.yaml`
(comments only), `tests/**`. Plus the BINARY_VERSION removal evaluation.
All edits are comment/doc-only; zero behavior change. Nothing committed or
staged; superproject gitlink untouched.

## (a) Per-file rewrite summary

1. **README.md** - 6 fixes (details in (c)): stale line counts, a false
   "nothing creates the plugin link automatically" claim (contradicted by
   layout_check.py's plugin-symlink self-heal), a stale binary size, a
   health-endpoint comment pointing at the wrong badge, and the Dev Install
   section (wrong crate / wrong dylib name / wrong profile).
2. **download.sh** - header comment: the version auto-detection chain was
   wrong ("falls back to plugin.yaml" - the script never reads plugin.yaml;
   actual order is BINARY_VERSION -> Cargo.toml -> GitHub API).
3. **layout_check.py** - module docstring dropped the dead design-notes
   pointer (`.hermes/notes/LAYOUT-SELF-HEAL.md` does not exist in the
   superproject notes tree); schema reference kept.
4. ****init**.py (1257 lines)** - read in full; comments verified accurate
   against the code (loader architecture, ctypes FFI via generated
   `_bindings.py`, hot-reload copy machinery, subprocess smoke-test,
   auto-download, proxy pre-launch probe, per-process state holder,
   `_check_version` / `_check_version_published` reading BINARY_VERSION at
   lines 690 and 757). NO edits: the file must stay byte-identical to the
   untouchable superproject template `crates/aphrodite/templates/__init__.py`
   (drift guard verified green after the pass).
5. **download.ps1** - read in full; header version chain (`BINARY_VERSION /
Cargo.toml / GitHub API`) is already accurate to `Resolve-Version`; asset
   naming and checksum flow match download.sh. No changes needed.
6. **layout_schema.json** - read in full; descriptions accurate (runtime home
   contents, overrides, heal rules, BINARY_VERSION listed as plugin-source
   file to quarantine). No changes needed.
7. **plugin.yaml** - comments verified (engine-threshold semantics, skills
   ship dev-side, install_message tool/hook counts all match the code and
   README). No changes needed.
8. **tests/** (5 files) - all comments verified accurate against the code
   (issue-5 shallow-path regression, hotreload reaping, perf probe, reaper
   prefix contract, Windows multi-home load path + proxy probe). No changes
   needed.

## (b) BINARY_VERSION decision: KEEP on Development

Removal is NOT sound - the loader on Development actively reads the file:

- `__init__.py:690` (`_check_version`) - loads the plugin's own pin and
  warns when the loaded dylib's version disagrees with it (contract-change
  guard at registration).
- `__init__.py:757` (`_check_version_published`) - resolves the GitHub
  release tag from it and warns before auto-download when that release has
  no assets (404 crash-loop guard).
- `download.sh:41-43` / `download.ps1:36-39` - BINARY_VERSION is the FIRST
  source for the version to fetch; the auto-download mechanism
  (`_ensure_binaries` -> download.sh) keys off it.
- `layout_check.py:37` + `layout_schema.json:30,117` - the file is part of
  the plugin-source set the layout self-heal quarantines from the runtime
  home; removing it changes the schema contract.

The dev flow uses locally built binaries (runtime home
`~/.hermes/aphrodite/binaries/` verified populated with the 1.4.6 binary +
dylib; env overrides `APHRODITE_BINARY_PATH` / `APHRODITE_HERMES_DYLIB_PATH`
honored first), but the version handshake and the release-assets guard read
the pin even when no download happens. Keeping the file matches AGENTS.md
("live distribution pointer - bump LAST at tag time"; it points at binary
1.4.6, the current release). Removal would silently disable the contract
mismatch warning and the crash-loop guard on Development. File left as-is.

## (c) Stale items fixed

- README architecture diagram + Files tree: "**init**.py 1005-line" x2 ->
  1257-line (actual `wc -l`).
- README One-command install: "nothing in Aphrodite creates that link
  automatically" -> self-heal can only recreate the link once Hermes has
  already loaded the plugin (layout_check.py `_ensure_symlink` for
  `~/.hermes/plugins/aphrodite`).
- README runtime tree: binary size "~12 MB" -> "~35 MB" (download.sh's own
  range is "~10-40MB"; live binary 37.8 MB).
- README /health example: "see the badge above" -> "see BINARY_VERSION"
  (badge shows the PLUGIN version v2.1.4; /health reports the BINARY
  version).
- README Dev Install: `cargo build -p aphrodite` + "Dylib at
  target/debug/libaphrodite.dylib - auto-detected by plugin" -> build
  `-p aphrodite-hermes`, dylib is `libaphrodite_hermes.dylib`, copy it into
  the canonical runtime home the loader resolves first; added the proxy-reuse
  note (`_start_proxy` pre-launch probe). Wrong on all three axes before:
  crate (dylib comes from aphrodite-hermes, not aphrodite), filename
  (loader never loads `libaphrodite.dylib`), profile (loader only ever
  checks `target/release`, never `target/debug`).
- download.sh header: version chain corrected to BINARY_VERSION ->
  Cargo.toml -> GitHub API (script never consulted plugin.yaml).
- layout_check.py docstring: removed dead `.hermes/notes/LAYOUT-SELF-HEAL.md`
  pointer (file absent from the superproject notes tree).

## (d) Code-vs-doc mismatches (fix would need a code change - NOT changed)

1. **README.md:16** - license badge links `LICENSE`, but the plugin repo
   tracks no LICENSE file (git ls-files). Either the file must be added or
   the badge dropped; licensing intent (CC0-1.0) not verifiable here - left
   as-is.
2. ****init**.py:1089** (`register` docstring) - "Targets the Hermes v0.17.0
   PluginContext API" vs `plugin.yaml:5` `min_hermes_version: "0.16.0"` and
   README's "0.16.0+" badge. Doc-vs-doc inconsistency; the 0.16.0 floor may
   simply be conservative. Would need Hermes release history to settle -
   left as-is (also byte-stable template pair).
3. ****init**.py:319-334** - "monorepo target/release fallbacks" comment: for
   the actual submodule layout (`<repo>/plugins/aphrodite`), `parents[2]` /
   `parents[3]` are the repo root's ancestors, never the monorepo root, so
   these fallbacks cannot hit `<repo>/target/release`; they only fire for a
   different checkout shape (e.g. a 5-level-deep path). Comment states
   intent, code is guarded and tested as-is; byte-stable template pair -
   left as-is.
4. **tests/test_dylib_candidates.py:41-44,47-52** - "partner's edit has not
   landed yet" lazy-skip scaffolding is dead code now (the helper exists);
   harmless test-only comment - left as-is.

## (e) Observations

- `plugins/aphrodite/plugins/aphrodite/` - untracked EMPTY directory in the
  plugin repo (leftover from an old layout); not in git ls-files, harmless.

## Verification (real outputs)

- `diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py`
  -> identical (drift guard green, run from superproject root).
- `bash -n plugins/aphrodite/download.sh` -> exit 0.
- Import smoke (spec_from_file_location exec of `__init__.py`) -> exit 0,
  no registration triggered.
- `ruff check plugins/aphrodite/` (superproject config) -> "All checks
  passed!", exit 0.
- `git status` (plugin repo): 3 modified files (README.md, download.sh,
  layout_check.py); nothing staged; no commits/pushes; superproject gitlink
  untouched.
