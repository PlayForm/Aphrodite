# TABLES-B - Wide-table restore report (partner B)

**Scope:** 2 notes files owned by partner B, under `.hermes/notes/`. The label:value rewrite (commits `8c2386d` + `10dc03b`) is reverted to the ORIGINAL table format restored from git history, with readability improvements. No commits made; working tree left dirty by design.

**Date:** 2026-09-18

## Format applied

- **Restored source:** `ISSUE-11-PREVIEW-SYSTEM-AUDIT.md` restored from `6bad4cb` (the commit that added it, original table form); `RELEASE-METHODOLOGY.md` restored from `72283c8` (last pre-rewrite commit, original table form).
- All tables restored with their ORIGINAL column structure (header row + separator + rows): ISSUE-11 → 5 tables (Classifiers, Preview builders, Call sites, Per-type current-vs-ideal, Dead-config); RELEASE-METHODOLOGY → 2 tables (A3 ships-vs-never-ships, PART 4 CI triggers).
- File-level `#` title lines and the `---` section separators preserved byte-for-byte.
- **Long prose cells** broken into readable segments with `<br>` at sentence/segment boundaries (A3 list cells, classifier Outputs list, builder Location cells, Publish.yml "What it does", call-site row 8).
- **Cells containing code/output/preview content** rendered as full fenced `text` blocks directly AFTER their table, referenced from the cell (`→ P1`..`→ P19`, `M1`, `B4`); one fence per row for the per-type matrix (P-fences), adjacent to the table.
- **Nothing truncated:** every cell of every original table (including headers) re-appears in the rebuilt file - see preservation proof. Fence interiors carry the full preview strings with `\|` unescaped to `|`.
- Markdown delimiters that cannot render inside fences (`**` emphasis, backtick code-span markers) are dropped inside fence content; the words are preserved verbatim.
- Prose, headings, and the two already-narrow tables (Call sites, Dead-config) untouched.

## Per-file summary

| File                             | Restored from | Tables (rows)   | Cells before → after | Fenced blocks added | `<br>` cells | Backticks before → after | Prettier   |
| -------------------------------- | ------------- | --------------- | -------------------- | ------------------- | ------------ | ------------------------ | ---------- |
| ISSUE-11-PREVIEW-SYSTEM-AUDIT.md | `6bad4cb`     | 5 (45 rows)     | 214 → 214            | 20 (M1 + P1..P19)   | 6            | 794 → 830                | exit 0     |
| RELEASE-METHODOLOGY.md           | `72283c8`     | 2 (6 rows)      | 27 → 27              | 1 (B4)              | 2            | 278 → 282                | exit 0     |
| **Total**                        | -             | **7 (51 rows)** | **241 → 241**        | **21**              | **8**        | -                        | **exit 0** |

## Content-preservation proof

- Every cell of every original table (stripped of table padding; `\|` → `|`; `<br>` treated as whitespace; backtick/emphasis delimiters stripped on both sides) was re-extracted from the pre-rewrite git version and asserted present in the rebuilt file: **241 / 241 found (0 missing)**.
- Every non-table prose line from the restored source asserted present in the rebuilt file (same normalization): **634 / 634 found (0 missing)**.
- Two cells were intentionally transformed, both verified by their extracted content:
    - ISSUE-11 `ccr_marker` Builder cell: the marker template `<<<CCR:hash | type | size>>>` (with its in-span table-alignment padding artifact) extracted to fence `M1`, cell references it - template bytes confirmed in `M1`.
    - RELEASE-METHODOLOGY Publish.yml row: the mangled in-span artifact `|     |` (leftover from an earlier aligner) removed and the real condition `startsWith(github.ref, 'refs/tags/Aphrodite/')` extracted to fence `B4`, cell references it - condition bytes confirmed in `B4`.

## Backtick-balance / fence verification

- Global backtick totals are **even before and after** for both files (values in the per-file table; deltas come from retained backticks inside the added fences).
- Fence pairing: every ` ``` ` opener has a closer. Fence pairs: ISSUE-11 20 (19 P-fences + 1 M1), RELEASE-METHODOLOGY 31 (30 pre-existing + 1 B4). No stray single backticks introduced.
- Label:value block lines (`**Location:**`, `**Trigger:**`, `**What it does:**`, …) remaining in the rebuilt files: **0**.

## Prettier result

- Prettier 3.9.6 (repo `node_modules/prettier/bin/prettier.cjs`), repo `.prettierrc` auto-detected (tabs; printWidth 100; `proseWrap: preserve` for `*.md`).
- Run: `prettier --write` on both rebuilt files, then `prettier --check` → **exit 0, "All matched files use Prettier code style!"** Only table alignment padding and blank-line normalization changed; cell content verified above.

## Notes / caveats

- No local absolute paths anywhere in this report or in any rebuilt region (all paths are repo-relative `.hermes/notes/...`, `crates/...`, `preview.rs`, etc.).
- `<br>` is used inside table cells only; fences carry full code/preview content so no cell exceeds ~140 chars of prose.
- Restored-from commits: `6bad4cb` (ISSUE-11) and `72283c8` (RELEASE-METHODOLOGY) are the last versions whose tables were untouched by the `8c2386d`/`10dc03b` rewrites.
- No commits made; nothing staged.
