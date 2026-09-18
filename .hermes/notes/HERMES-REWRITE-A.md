# HERMES-REWRITE-A - .hermes notes taxonomy + uml + classification pass

Date: 2026-09-18 (EEST) - branch `Development`, binary 1.4.6 / plugin 2.1.4.
Scope: consolidate `.hermes/notes/` into a reverse hierarchical taxonomy,
refresh `.hermes/uml/`, verify `.hermes/classification/` consistency.
Nothing committed by the agent (the auto-committer swept the moves + scrubs
into `bc0e548` mid-run; that commit also carried the sibling's SPLIT-MD-REWRITE
deliverable).

## 1. What was done

### notes/ - reverse hierarchical taxonomy (no content deleted)

The 39 per-task reports were moved into six coarse-to-fine category
directories, each with an INDEX.md (one-line description per file):

```
.hermes/notes/
├── README.md                    <- catalog + reading order (NEW)
├── ARCHITECTURE.md              <- evergreen: FFI pipeline, runtime home,
│                                    plugin loader, preview system (NEW)
├── ISSUE-11.md                  <- evergreen: full bug-family history,
│                                    landed WS1/2/4 + residuals, what remains (NEW)
├── CEREMONY.md                  <- evergreen: Phase A/B + B4 audit gate (NEW)
├── SESSION-LOG-2026-09-17-18.md <- evergreen: chronological record absorbing
│                                    every report as a dated entry (NEW)
├── HANDOFF.md                   <- evergreen: current pending queue (NEW)
├── HERMES-REWRITE-A.md          <- this report (NEW)
├── HERMES-REWRITE-B.md          <- sibling report (untouched)
├── SPLIT-MD-REWRITE.md          <- sibling research/blueprint (untouched)
├── ffi/        (6)  pipeline, codegen, research, hardening, FFI CI
├── issue11/    (13) root-cause, fixdesign, batteries, landed, residual
├── release/    (4)  ceremony, methodology, sync-back, CI
├── session/    (4)  continuations, handoffs, dispatch
├── plugin/     (5)  loader, layout, directives, failure forensics
└── ops/        (7)  operational + meta notes
```

Every report moved verbatim (a few gained anonymization-only edits - see 4);
zero files deleted. The root evergreen docs link DOWN into the taxonomy; the
per-category INDEX.md files are the library catalog.

### uml/ - refresh (drift fixed)

- `07-config-resolution.md`: the "Live vs inert" diagram claimed
  `preview_max_chars` was declared-never-read. WS4 wired it end-to-end
  (verified in source: `main.rs:128`, `proxy.rs:2660`, `config_loader.rs:303`,
  `preview.rs:579/953`, hermes `lib.rs:70`). Added a LIVE node L8 for the cap
  and trimmed the INERT node to `model_family` / `code_structure_map` /
  `rust_preview_lines` (still inert).
- `04-hook-ffi.md`: annotated the `unwrap_hermes_result` sequence note with
  the Issue #11 WS1 changes (ok-collapse removed, single-key guards,
  caller-hint-wins); the "original content hashed verbatim" invariant
  unchanged.
- All other uml files verified current (DOCS-UPDATE pass was already
  source-verified for 1.4.6 / 2.1.4; preview-related line refs checked).

### classification/ - consistency verified, dated amendment added

- TAXONOMY.md + the four pass files still describe the tree: the Issue #11
  preview/detection rewrite changed the CONTENT of already-classified
  layer-1/2 processes (tools.rs PD2-02, preview.rs PD1-16, config.rs PD1-05,
  config_loader.rs PD1-06, main.rs, proxy.rs, hermes lib.rs PD2-01) with no
  phase/layer/code moves - all codes remain valid.
- Added one dated amendment row (2026-09-18) to TAXONOMY.md section 6:
  post-snapshot note recording the preview rewrite + `preview_max_chars`
  going live + the B4 gate (I11) + the Current Auto.yml leak. No pass file
  restructured.

## 2. Resulting tree (verification)

- `.hermes/notes/`: 6 evergreen docs + this report + 2 sibling files at
  root; 6 category dirs (39 reports) each with INDEX.md.
- Counts: ffi 6, issue11 13, release 4, session 4, plugin 5, ops 7 = 39
  reports (before: 39 loose files at root).
- `.hermes/uml/`: 12 files + README, only 07 + 04 edited.
- `.hermes/classification/`: untouched except one amendment-log row.
- Untouched per constraints: `.hermes/skills/`, `.hermes/tmp/`, plugins/,
  crates/, docs/, Maintain/.
- Anonymization: zero local absolute paths remain in notes/, uml/,
  classification/ (the user-home prefix -> `~`, the drive prefix ->
  `…/`, the Temporary scratch -> `…/.playform/Temporary`; 5 files
  scrubbed).
- Prettier: all touched .md files pass `npx prettier --check` (repo
  `.prettierrc`: tabs, width 100, proseWrap preserve).

## 3. Before/after footprint

| Metric                                         | Before                 | After                                                 |
| ---------------------------------------------- | ---------------------- | ----------------------------------------------------- |
| Loose .md at notes/ root                       | 39 reports + 0 indexes | 6 evergreen + 1 report + 2 sibling files + 6 INDEX.md |
| Category dirs                                  | none                   | 6 (ffi, issue11, release, session, plugin, ops)       |
| Files deleted                                  | -                      | 0                                                     |
| Absolute-path hits in notes/uml/classification | 10 (5 files)           | 0                                                     |
| uml drift                                      | 1 stale claim (07)     | fixed                                                 |

## 4. Notes / caveats

- The auto-committer committed the moves + scrubs into `bc0e548`
  ("docs(.hermes): reorganize notes taxonomy...") mid-run, bundled with the
  sibling's SPLIT-MD-REWRITE addition and an AGENTS.md refresh; my remaining
  working-tree delta after that commit: the REWRITE-A.md scrub finalization
    - this pass's INDEX/evergreen/uml/classification edits (all docs).
- Sibling deliverables (`HERMES-REWRITE-B.md`, `SPLIT-MD-REWRITE.md`) left
  untouched; the split.md blueprint will fit `ops/` or `ffi/` when the
  sibling's wave closes.
- `SPLIT-MD-REWRITE.md` is the 1.5.0 blueprint for the `detect_semantic_type`
  / preview.rs regex refactor (fancy-regex vs regex/aho-corasick research +
  isolated 7-session execution order) - referenced from HANDOFF.md.
