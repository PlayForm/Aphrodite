# HERMES-REWRITE-B - .hermes/ dev-archive rewrite (owner B)

Date: 2026-09-18 · Branch: Development · Scope: `.hermes/AGENTS.md`,
`.hermes/skills/`, `.hermes/tmp/`, top-level `.hermes/` tidiness.
Partner owns `.hermes/notes/`, `.hermes/uml/`, `.hermes/classification/`
(untouched). Companion report: `REWRITE-A.md`.

## 1. AGENTS.md rewritten (current state, verified)

The previous AGENTS.md was stale (plugin-era boilerplate, wrong paths:
`.hermes/plans/` and `.hermes/RELEASE-TEMPLATE.md` did not exist). The new
file is a first-read map covering, all verified against the live tree:

- Binary 1.4.6 / plugin 2.1.4; plugin = pure ctypes loader with generated
  FFI bindings (`_bindings.py` via cbindgen → ctypesgen → finalize).
- Runtime home `~/.hermes/aphrodite/` (toml, binaries/, directives/,
  hotreload/, ccr.db) + layout self-heal.
- Dev-side skills auto-load via `skills.trusted_project_dirs` (verified in
  `~/.hermes/config.yaml`; config NOT modified).
- Issue #11 preview machinery (honest previews, caller-hint-wins, real
  total_count, `preview_max_chars` env > TOML > default 120).
- Ceremony incl. B4 branch-identity audit gate (I11); `.githooks` removed.
- Quality gates with CURRENT counts (all re-run live today, see §4).

## 2. Skills audit (13 dirs; 12 current + 1 historical)

- **`aphrodite-branch-release-flow` v1.1.0 - RETIRED in place**: deprecation
  banner at the top of SKILL.md + frontmatter description changed to
  "DEPRECATED - superseded by aphrodite-release-flow v2.0.0" so it never
  auto-triggers. Not deleted.
- **`aphrodite-release-flow` v2.0.0** - 2 patches: "deleted branch-release-flow"
  → "retired … (deprecated in place)"; B4 branch-identity audit gate (I11)
  added to the ceremony (points at RELEASE-METHODOLOGY.md `### B4`).
- **`aphrodite-testing-discipline` v1.0.0** - env-var-hermeticity lesson
  ADDED (was missing): the leaked `APHRODITE_PREVIEW_MAX_CHARS=20` shell
  export that failed 21-25 `cargo test -p aphrodite` runs with varying
  failures (env > TOML > default; config tests must remove_var/restore;
  a subagent's clean env does not prove the parent's). Checklist item added.
- **Stale-claim fixes** (9 older skills scanned):
    - `aphrodite-operations` - `/tmp/out.txt` → `.hermes/tmp/out.txt`.
    - `aphrodite-release-workflow` - branch-release-flow reference →
      release-flow v2.0.0; `.hermes/RELEASE-TEMPLATE.md` →
      `.hermes/release/RELEASE-TEMPLATE.md`; `.plans/release-notes/` →
      `.hermes/release-notes/`; `/tmp/notes.md` heredocs →
      `.hermes/tmp/notes.md`; dev symlink `~/.hermes/aphrodite/aphrodite` →
      `~/.hermes/aphrodite/binaries/aphrodite`.
    - `aphrodite-auto-expand-testing` - `~/.hermes/aphrodite.toml` →
      `~/.hermes/aphrodite/aphrodite.toml` (2 spots; old path verified gone).
    - `aphrodite-v0.8.6-patterns` - dangling `aphrodite-dev-workflow`
      references → `aphrodite-operations` (frontmatter + body).
    - `aphrodite-development-lessons` - template path corrected.
    - `aphrodite-hook-reference`, `aphrodite-benchmarking`,
      `aphrodite-cargo-upgrade`, `aphrodite-upgrade-breakpoints`,
      `aphrodite-tool-testing` - scanned, no stale claims found.

## 3. .hermes/tmp/ - compressed + gitignore verified

Contents compressed in place (nothing deleted, archives verified
`gzip -t` / `tar -tzf`):

- `aphrodite-setup/` (13 MB binary + 3.6 MB dylib) → `aphrodite-setup.tar.gz`
  (6.8 MB).
- All 11 `.log` files gzipped individually.
- Generated fixtures gzipped: `issue11_battery_after_generated.md`,
  `issue11_battery_after.tsv`, `issue11_final.json`, `issue11_final3.json`,
  and the 8 bulky generated bindings dumps (`_bindings.new.py`,
  `ffi_bad_raw.py`, `ffi_real_final.py`, `ffi_real_raw.py`, `t_raw.py`,
  `test_bindings.py`, `test_bindings2.py`, `viz_head.py`).
- Small evidence `.py` probes kept PLAIN (`aphrodite_probe*.py`,
  `sigserve_*.py`, `issue11_repro.py`, `fork_*.py`, `gen_battery_md.py`,
  `test_head.py`, `pypdfium2_ctypdescs.py`); small `.h` fixtures,
  `ffi-regress/`, `aphrodite-empty-hooks/` untouched.
- **Size: 16,696 KB → 6,956 KB (−9,740 KB, ~58%)**.
- Gitignore verified: `.hermes/tmp/*` (line 48) ignores every content file
  and every `.gz`/`.tar.gz` form; `*.tar.gz` (line 54) also matches
  `aphrodite-setup.tar.gz`; `.hermes/tmp/.gitkeep` is NOT ignored and stays
  tracked. No `.gitignore` edits needed. `git status` clean for tmp/.

## 4. Quality gates - live numbers (re-run today)

| gate                                        | result                                              |
| ------------------------------------------- | --------------------------------------------------- |
| `cargo test -p aphrodite`                   | 406 passed (377 lib + 29 bins), 0 failed, 1 ignored |
| `cargo test -p aphrodite-hermes`            | 52 passed                                           |
| `codegen/test_finalize_bindings.py`         | 23/23 OK                                            |
| `Maintain/tests/test_check_ffi_contract.py` | 13/13, 50 asserts                                   |
| checker `Maintain/check_ffi_contract.py`    | PASS, 0 violations                                  |
| drift-guard diff                            | identical                                           |
| repro (sigserve scratch)                    | SURVIVED, no crash                                  |

Note: lib count is now 377 (was 360 at the ISSUE-11-LANDED C2 sweep; +17
tests landed since) - AGENTS.md carries the fresh numbers.

## 5. Top-level .hermes/ tidiness

`release/`, `release-notes/`, `scripts/` naming is consistent; no stray
`plans/` dir; AGENTS.md now points at the real paths. Nothing renamed.

## 6. Constraints honored

- `.hermes/notes/`, `.hermes/uml/`, `.hermes/classification/` untouched
  (read-only for context: ISSUE-11-LANDED, PREVIEW-RACE-FIX-ROOTCAUSE,
  RELEASE-METHODOLOGY B4).
- No commits (auto-committer sweeps). No changes to `~/.hermes/config.yaml`
  or global skills (`branch-flow-protocol` still references the retired
  skill - flagged, out of repo scope).
- Anonymized: zero local absolute paths in any written .md.
- All written/edited `.md` prettier-clean (verified `npx prettier --check`).
