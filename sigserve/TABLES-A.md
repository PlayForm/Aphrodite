# TABLES-A - Wide-table restore report (partner A)

**Scope:** 4 notes files owned by partner A, under `.hermes/notes/`. The label:value rewrite (commits `8c2386d` + `10dc03b` + `4bb050e`) is reverted to the ORIGINAL table format restored from git history, with readability improvements (multiline cells, fenced code/output). No manual commits made by the agent; the environment's auto-committer swept the rebuilt files into commit `96010b7` mid-run - a post-fix working-tree delta remains on `CEREMONY-AUDIT.md` only (see Notes).

**Date:** 2026-09-18

## Format applied

- **Restored source:** `CEREMONY-AUDIT.md` restored from `72283c8` (last pre-rewrite commit with the original table); `DEVELOP.md` + `DOCS-UPDATE.md` restored from `d87bb8c`; `HERMES-UPDATE-ERROR-FORENSICS.md` restored from `c38199e` (its pre-rewrite commit). All four predate the `4bb050e` label:value rewrite.
- All tables restored with their ORIGINAL column structure (header row + separator + rows): CEREMONY-AUDIT → 1 table (17 rows: File | Branch | Verdict | Evidence); DEVELOP → 2 tables (7-row report-item matrix, 11-row verification matrix: Check | Result); DOCS-UPDATE → 2 tables (12-row `.hermes/uml/` matrix, 2-row "Other files": File | What changed); HERMES-UPDATE-ERROR-FORENSICS → 1 table (12-row timeline: Time | Event).
- File-level `#` title lines (e.g. `# DEVELOP.md - FFI pipeline implementation (research findings -> code)`) preserved byte-for-byte; all non-table prose sections preserved verbatim (re-wrapped only where Prettier's `proseWrap: preserve` already had them).
- **Long prose cells** broken into readable segments with `<br>` at sentence/segment boundaries (CEREMONY 6 cells, DEVELOP 5 cells, DOCS-UPDATE 11 cells, FORENSICS 6 cells).
- **Cells containing code/output content** rendered as full fenced `text` blocks directly AFTER their table, referenced from the cell (`→ P1`..`→ P3` in DEVELOP's verification matrix: the direct-finalize validation message, the bind_to replay signature map, the artifact-shape grep summary). All other code stays in inline backtick spans, intact.
- **Nothing truncated:** every original cell's full text re-appears in the rebuilt file (see preservation proof). Fence interiors carry the moved output verbatim with backtick code-span delimiters dropped (words preserved), per the established convention.
- Path redaction in the forensics file follows the convention already set by the `4bb050e` rewrite: user-home roots → `~`, machine-specific path prefixes → `…/` (e.g. `…/PlayForm/Aphrodite/...`, `…/Application/...`). Verbatim log/crash excerpts keep every other byte.

## Per-file summary

| File                             | Restored from | Tables (rows)   | Cells preserved | `<br>` cells | Fenced blocks added | Backticks before → after | Prettier   |
| -------------------------------- | ------------- | --------------- | --------------- | ------------ | ------------------- | ------------------------ | ---------- |
| CEREMONY-AUDIT.md                | `72283c8`     | 1 (19 rows)     | 72              | 6            | 0 (4 existing kept) | 226 → 226                | exit 0     |
| DEVELOP.md                       | `d87bb8c`     | 2 (22 rows)     | 40              | 5            | 3 (P1..P3)          | 296 → 312                | exit 0     |
| DOCS-UPDATE.md                   | `d87bb8c`     | 2 (18 rows)     | 32              | 11           | 0 (1 existing kept) | 302 → 302                | exit 0     |
| HERMES-UPDATE-ERROR-FORENSICS.md | `c38199e`     | 1 (14 rows)     | 26              | 6            | 0 (4 existing kept) | 192 → 192                | exit 0     |
| **Total**                        | -             | **6 (73 rows)** | **170**         | **28**       | **3**               | -                        | **exit 0** |

## Content-preservation proof

- Every original line (including every table cell), normalized identically on both sides (table padding stripped; `\|` → `|`; `<br>` treated as whitespace; backtick/emphasis delimiters stripped; path roots redacted per the convention above), asserted present in the rebuilt file.
- **CEREMONY-AUDIT: 0 missing** (line level + n-gram sweep n=2/3/6). **DOCS-UPDATE: 0 missing. HERMES-UPDATE-ERROR-FORENSICS: 0 missing.**
- **DEVELOP: 3 verification-matrix result cells intentionally relocated** to fences P1..P3 (rows "Direct finalize run", "bind_to replay vs real release dylib", "Artifact shape"); the cell now references `→ Pn` and the full original result text is confirmed byte-for-byte (modulo dropped backtick delimiters) inside the corresponding fence. All other lines: 0 missing.
- The only n-grams absent from the rebuilt files are row-boundary transitions that the fence relocation splits (e.g. `exports) exit`, `OK Artifact shape`) - content itself is present in the fences.

## Fence / backtick verification

- Fence pairing: every fence opener has a closer. Pairs: CEREMONY 4, DEVELOP 3 (all new P-fences), DOCS-UPDATE 1, FORENSICS 4. No stray single backticks introduced; backtick totals are even before and after for every file.
- Label:value block lines (`**File:**`, `**Branch:**`, `**Verdict:**`, `**What changed:**`, `**Check:**`, `**Result:**`, `**Time:**`, `**Event:**`) remaining in the rebuilt files: **0**.
- Zero local absolute paths in any rebuilt file (machine-path-root scan of all four notes files: 0 hits).

## Prettier result

- Prettier 3.9.6 (repo `node_modules/prettier/bin/prettier.cjs`), repo `.prettierrc` auto-detected (tabs; printWidth 100; `proseWrap: preserve` for `*.md`).
- Run: `prettier --write` on all four rebuilt files (twice - once per pass, second run idempotent), then `prettier --check` → **exit 0, "All matched files use Prettier code style!"** Only table alignment padding and blank-line normalization changed; cell content verified above.

## Notes / caveats

- Restored-from commits (`72283c8`, `d87bb8c`, `c38199e`) are the last versions whose tables were untouched by the `8c2386d`/`10dc03b`/`4bb050e` rewrites.
- The environment's auto-committer committed the rebuilt files as `96010b7` mid-run; a later fix to the `PART 7` diff-block spacing (`+I11` vs `+ I11`, aligning the fence byte-for-byte with `72283c8`) is still an uncommitted working-tree change on `CEREMONY-AUDIT.md` only. Working tree is otherwise clean for these four files; nothing else was committed by the agent.
- `<br>` is used inside table cells only; fences carry the full output content, so no cell exceeds ~140 chars of prose.
- No local absolute paths anywhere in this report (all paths are repo-relative `.hermes/notes/...`, `crates/...`, `~/.hermes/...`, `…/PlayForm/...`).
