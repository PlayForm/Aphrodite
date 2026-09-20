# Aphrodite Governance - ARCHIVE-INDEX.md

Catalog of archived Aphrodite skills and historical snapshots. Archived
material stays searchable but is structurally prevented from steering new
work. **Nothing in this index is live content.**

Archive location: `.hermes/notes/ops/ARCHIVE-2026-09-18/` (preserved, never
deleted - evidence rule; the authoritative ledger of every move is
`.hermes/notes/ops/AUDIT-REORG-2026-09-18.md`).

## Archived skills

| Skill                           | Status                          | Successor                               | Historical cutoff | Archive path                                                          | Notes                                                                                                                    |
| ------------------------------- | ------------------------------- | --------------------------------------- | ----------------- | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| `aphrodite-branch-release-flow` | archived / do_not_execute: true | `aphrodite-release-flow` (v2.0.0)       | 2026-09-18        | `.hermes/notes/ops/ARCHIVE-2026-09-18/aphrodite-branch-release-flow/` | Deprecated in place (v1.1.0); competing release ceremony. Never appears in the active skill index or suggested workflow. |
| `aphrodite-v0.8.6-patterns`     | archived / do_not_execute: true | none (historical only)                  | 2026-09-18        | `.hermes/notes/ops/ARCHIVE-2026-09-18/aphrodite-v0.8.6-patterns/`     | Explicitly archival; v0.8.5→v0.8.6 cycle context only.                                                                   |
| `aphrodite-upgrade-breakpoints` | archived / do_not_execute: true | absorbed into `aphrodite-cargo-upgrade` | 2026-09-18        | `.hermes/notes/ops/ARCHIVE-2026-09-18/aphrodite-upgrade-breakpoints/` | Historical skill snapshot; its breakpoint knowledge lives on in `aphrodite-cargo-upgrade`.                               |

## Historical snapshot files (same archive dir)

| File                 | Origin                                                                                      | Canonical replacement                                                  |
| -------------------- | ------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| `MERGE-SUBMODULE.md` | Pre-finalization draft of the Phase B plugin sync-back record (intermediate `5fde801` hash) | `.hermes/notes/release/MERGE-SUBMODULE-EXEC.md` (final pick `098134e`) |
| `README.md`          | Archive directory index (2026-09-18 cleanup)                                                | This file supersedes it as the skill-side index                        |

## Rules

- An archived skill's metadata (`status: archived`, `do_not_execute: true`,
  `successor`, `historical_cutoff: 2026-09-18`) is mandatory; an archive
  without a successor named is a broken archive.
- Updating an archive is prohibited **except to correct archival metadata**
  (e.g. fixing a successor pointer). Historical content is never edited.
- No active skill lists an archive under "Related" unless the link label says
  **Historical only-do not execute**.
- Re-activation of an archived skill is a manifest change
  (SKILL-MANIFEST.md maintenance rules) plus a fresh version and a
  CONTRADICTION-REGISTER.md entry. Do not simply delete the archive first.
- If an active skill's row in SKILL-MANIFEST.md points here, the pointer
  means "historical evidence only."
- Archives carry `do_not_execute: true` in their frontmatter; an agent that
  loads one must treat it as context, never as a procedure.
