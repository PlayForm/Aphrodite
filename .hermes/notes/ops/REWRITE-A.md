# REWRITE-A - Wide-table expansion (8 files owned by worker A)

Date: 2026-09-18. Repo: PlayForm/Aphrodite, branch `Development`.
Method: every wide table (any cell >140 chars raw, plus the explicitly-mandated
battery tables) rewritten into expanded per-row blocks: column semantics kept
as bold labels, every cell text preserved verbatim, code/output/preview cells
in full untruncated `text` code fences, `---` horizontal rules between rows
only (never after the last), prose/headings/narrow tables untouched.
Prettier-clean (repo `.prettierrc`: tabs, width 100, proseWrap preserve).
Nothing committed.

## Spec compliance notes (applies to all files)

- **Cell text verbatim:** table padding (alignment spaces) collapsed to single
  spaces; content characters unchanged. Pipes inside backtick code spans
  (e.g. `[tool_result:1L 17B | {"success": true}]`) handled when splitting.
- **Backtick balance:** verified - global parity even, no odd-count lines
  outside fences, every fence closed. One pre-existing stray fixed:
  `ISSUE-11-AUDIT.md` L22 Returns cell was missing a closing backtick after
  `"N total (truncated)"` (line had 19 backticks); fixed to 20.
- **Before/after pairs:** none of the 8 files contained before/after code
  blocks; before (1.4.5) and after (1.4.6) values are columns and were kept
  as labeled fields per row.
- **Blank-line-every-field rule:** every `**label:**` line is its own
  paragraph (blank line before/after); fences sit after a blank line; verified
  zero joined label lines. Pre-existing bold-label front matter
  (Date/Event/Method/Repo/Scope/Predecessors) was also separated with blank
  lines (rendering fix, zero content change).
- **Anonymization (zero local absolute paths):** `HERMES-UPDATE-ERROR-FORENSICS.md`
  rewritten: `/Users/nikola` -> `~`, the local drive prefix + repo root -> `…/`
  (e.g. `…/PlayForm/Aphrodite`), the Temporary scratch -> `…/.playform/Temporary`.
  No local absolute path remains in any file.
- **Location note:** `ISSUE-11-BATTERY-AFTER.md` does not exist in
  `.hermes/notes/`; it lives in the sigserve scratch dir
  (`…/.playform/Temporary/sigserve/`). It was rewritten in place there.
- **Pre-existing, kept verbatim:** `DEVELOP.md` prose has a code span split
  across two lines (`for _lib in` / `_libs.values():`) - globally balanced,
  left as authored.

## Per-file summary + content-preservation proof

### CEREMONY-AUDIT.md

- 1 table rewritten (Findings table, 17 rows x 4 cols). Evidence cells in
  `text` fences (commands + snippets); File/Branch/Verdict inline.
- Cells before: 68 == cells after: 68. Fences: 42 (21 pairs, all closed).
- Backticks: 328 (even). Prettier --check: pass.

### DEVELOP.md

- 2 tables rewritten: §1 report items (7 x 2) and §5 verification matrix
  (11 x 2). All cells inline (prose with inline code).
- Cells before: 36 == after: 36. Backticks: 296 (even).
- Prettier --check: pass.

### DOCS-UPDATE.md

- 2 tables rewritten: `.hermes/uml/` files (12 x 2) and "Other files"
  (2 x 2). All cells inline.
- Cells before: 28 == after: 28. Backticks: 302 (even). Prettier --check: pass.

### HERMES-UPDATE-ERROR-FORENSICS.md

- 1 table rewritten (20:33 timeline, 12 x 2), plus full-file path
  anonymization (see above).
- Cells before: 24 == after: 24. Backticks: 192 (even). Prettier --check: pass.

### HERMES-UPDATE-ERROR-PLUGIN-SIDE.md

- 1 table rewritten (observed timeline, 10 x 2). Cells inline.
- Cells before: 20 == after: 20. Backticks: 326 (even). Prettier --check: pass.

### ISSUE-11-AUDIT.md

- 2 tables rewritten: branch table (10 x 6; Returns + False-positive-risk
  cells in `text` fences) and history timeline (6 x 4; all inline).
- Stray-backtick fix applied (see compliance notes).
- Cells before: 84 == after: 84. Backticks: 530 (even).
- Prettier --check: pass.

### ISSUE-11-FIXDESIGN.md

- 1 table rewritten (fix candidates, 7 x 6; all inline) + 1 narrow table
  kept untouched (test matrix, 10 x 10, max cell 75 chars).
- Cells before: 142 == after: 142. Backticks: 348 (even).
- Prettier --check: pass.

### ISSUE-11-BATTERY-AFTER.md (sigserve scratch; FULLY RE-CONSTRUCTED)

- 3 tables rewritten: fix-claims (6 x 3; evidence cells in `text` fences),
  Table A full battery (75 x 6; every preview in a full untruncated `text`
  fence) and Table B hook path (20 x 6; same). 2 narrow score tables kept
  (4 x 4 each).
- Cells before: 620 == after: 620 (588 expanded + 32 kept).
- Backticks: 1014 (even); 202 fences (101 pairs, all closed); zero joined
  label lines; no `---` after the last row.
- Prettier --check: pass (run with `--config .prettierrc` - file is outside
  the repo tree, so the config must be passed explicitly).

## Verification summary

- Cell counts: before == after in every file (asserted per table).
- Backtick balance: all 8 files even parity, no unbalanced lines outside
  fences, all fences closed.
- Blank-line rule: zero joined label lines across all files.
- `---` separators: between rows only, never after the last row.
- Local absolute paths: zero remaining.
- Prettier: `All matched files use Prettier code style!` (exit 0) for all 8.
- Git: 7 `.hermes/notes/` files modified (unstaged), nothing committed;
  partner-owned files untouched.
