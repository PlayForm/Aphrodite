# AUDIT-REORG-2026-09-18 - .hermes/ audit, dedupe, reorganize

Date: 2026-09-18 (EEST) · Branch: `Development` · HEAD: `66d8fa1` · Repo:
PlayForm/Aphrodite (macOS). Author: audit agent (S1). Per instructions in
`.hermes/tmp/s1-hermes-audit-instructions.md`: inventory, dedupe by
consolidation (never blind deletion), re-categorize per the convention
`.hermes/<category>/<thing>.md`, fix internal links, refresh stale facts
where trivial. Scope: `notes/`, `notes/ops/`, `classification/`, `uml/`,
`release/`, `release-notes/`, `scripts/`. NOT touched: `skills/`, `tmp/`,
`plugins/aphrodite`, anything outside `.hermes/` (AGENTS.md received one
link-fix line, recorded below). No commits, no pushes.

## 1. Inventory summary (before)

118 tracked files in scope, ~1,059 KB total (all sizes bytes):

| Dir             | Files | Bytes   | Notes                                         |
| --------------- | ----- | ------- | --------------------------------------------- |
| classification/ | 6     | 266,144 | README + TAXONOMY + 4 pass files              |
| notes/ (root)   | 8     | 49,484  | 6 evergreen + 2 rewrite reports + README      |
| notes/ffi/      | 7     | 103,962 | 6 reports + INDEX                             |
| notes/issue11/  | 14    | 192,336 | 13 reports + INDEX                            |
| notes/ops/      | 12    | 122,361 | 10 reports + INDEX + optimization-clues.md    |
| notes/plugin/   | 6     | 44,067  | 5 reports + INDEX                             |
| notes/release/  | 9     | 118,916 | 8 reports + INDEX (3 reports missing from it) |
| notes/session/  | 5     | 42,515  | 4 reports + INDEX                             |
| release/        | 3     | 15,186  | template + scope + handoffs/ (1 file subdir)  |
| release-notes/  | 4     | 15,164  | v1.4.0-1.4.2 + v1.4.3-draft (shipped)         |
| scripts/        | 2     | 7,463   | build-watcher.py/.sh                          |
| uml/            | 12    | 67,198  | 01-11 + README                                |

Classification also carries pre-reorg path rows for notes-root files (fixed
in this pass, see §5).

## 2. Duplicate/overlap assessment

Consolidated (1 pair):

1. **`notes/release/MERGE-SUBMODULE.md` + `notes/release/MERGE-SUBMODULE-EXEC.md` →
   canonical `notes/release/MERGE-SUBMODULE-EXEC.md`.** The first is the
   pre-finalization draft of the same Phase B plugin sync-back record: same
   commit-disposition table, same verification table, but the intermediate
   pick hash `5fde801` (superseded when the pick was rebuilt). The EXEC
   record carries the final hash `098134e` (confirmed live in the submodule
   log). Draft archived (see §4) for its unique `git commit-tree` rebuild
   detail. Links updated: `notes/release/INDEX.md`, `SESSION-LOG` entry.

Assessed and kept (NOT duplicates):

- `HERMES-REWRITE-A/B.md` (notes root) vs `ops/REWRITE-A/B.md` - the first
  pair are the taxonomy/uml/classification + AGENTS/skills/tmp rewrite
  reports; the second pair are the wide-table expansion reports. Four
  distinct documents.
- `SPLIT-ADAPTATION.md` vs `REFACTOR-PLAN-1.5.0.md` (ops/) - research
  distillation vs concrete plan; they explicitly cross-reference each other.
- `MERGE-SUBMODULE-AUDIT.md` + `MERGE-SUBMODULE-EXEC.md` and
  `MERGE-PARENT-AUDIT.md` + `MERGE-PARENT-EXEC.md` - executor record +
  independent auditor report per operation (auditor has unique SHA-256
  pre/post captures); complementary, both kept.
- `ISSUE-11.md` (rollup) + `issue11/*` (details); `CEREMONY.md` (rollup) +
  `notes/release/*`; `HANDOFF.md` (current queue) + `session/HANDOFF-2026-09-17.md`
  (dated handoff) - rollup vs detail by design.
- `scripts/build-watcher.py` + `.sh` - same purpose, two implementations
  (wezterm-cli vs MCP variant); classified as two halted processes
  (SD6-01/SD6-02). Kept as a pair; `.py` is the maintained variant.
- `release-notes/v1.4.3-draft.md` vs `release/RELEASE-SCOPE-1.4.3.md` vs
  `release/RELEASE-HANDOFF-1.4.3.md` - release notes (what shipped) vs
  manifest (IN/OUT) vs handoff (ceremony state); distinct artifacts, all
  historical for v1.4.3 (tag `Aphrodite/v1.4.3` exists).

## 3. Reorganizations (moves, git mv)

| From                                        | To                                                | Why                                                                                       |
| ------------------------------------------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `notes/ops/DUAL-STATE-MERGE-REVIEW.md`      | `notes/release/DUAL-STATE-MERGE-REVIEW.md`        | Sync-back review methodology; release/ = ceremony + sync-back; was missing from ops/INDEX |
| `notes/release/MERGE-SUBMODULE.md`          | `notes/ops/ARCHIVE-2026-09-18/MERGE-SUBMODULE.md` | Superseded draft (see §2.1)                                                               |
| `notes/ops/optimization-clues.md`           | `notes/ops/OPTIMIZATION-CLUES.md`                 | Case convention (`<thing>.md`; all siblings UPPERCASE)                                    |
| `release-notes/v1.4.3-draft.md`             | `release-notes/v1.4.3.md`                         | v1.4.3 shipped (tag exists); "draft" label stale                                          |
| `release/handoffs/RELEASE-HANDOFF-1.4.3.md` | `release/RELEASE-HANDOFF-1.4.3.md`                | Flatten single-file subdir; release/ holds <thing>.md                                     |

No other files were misplaced: the reverse-hierarchical taxonomy
(notes/ + category subdirs with INDEX.md) from the earlier 2026-09-18 pass is
sound.

## 4. Archive contents (`notes/ops/ARCHIVE-2026-09-18/`)

- `MERGE-SUBMODULE.md` (6,847 B) - consolidated draft, moved here by this
  pass (evidence rule: nothing deleted).
- `README.md` (new, by this pass) - archive index + provenance.
- `aphrodite-branch-release-flow/`, `aphrodite-upgrade-breakpoints/`,
  `aphrodite-v0.8.6-patterns/` - retired-skill snapshots placed by the
  skills-cleanup wave (2026-09-18), not by this pass; listed for
  completeness, not touched.

## 5. Stale-fact fixes

- **classification/DEV-knowledge-tests-bench.md**: 8 table rows for
  notes-root files updated to post-reorg paths (`ops/`, `session/`,
  `plugin/`, `release/`); `release/handoffs/RELEASE-HANDOFF-1.4.3.md` row →
  `release/RELEASE-HANDOFF-1.4.3.md`; `v1.4.3-draft.md` row renamed +
  annotation `@R` → `@A` (archival of the shipped note); uml README row
  `v1.3.4` → `v1.4.6`; prose §2.2 + §3 `CONTINUE-*`/draft references
  updated.
- **classification/DEV-engine-build.md**: `1.4.5` → `1.4.6` in 3 manifest
  rows + the version-pin prose (verified against both `Cargo.toml`s:
  `version = "1.4.6"`).
- **classification/CUR-plugin-loader.md**, **CUR-release-infra-identity.md**,
  **TAXONOMY.md**: `notes/RELEASE-METHODOLOGY.md` → `notes/release/...`.
- **notes/HANDOFF.md**: `SPLIT-MD-REWRITE.md (in this dir)` → `ops/SPLIT-ADAPTATION.md`;
  sanity-sweep test count 389 → 406 (matches the live re-run in
  `HERMES-REWRITE-B.md`).
- **notes/README.md**: category counts release 4 → 8, ops 7 → 10 (+ archive
  subdir); `HERMES-REWRITE-B.md` described as "the split.md rewrite" →
  "AGENTS.md + skills + tmp tidy pass"; `SPLIT-MD-REWRITE.md` →
  `ops/SPLIT-ADAPTATION.md` + `ops/REFACTOR-PLAN-1.5.0.md`.
- **notes/release/INDEX.md**: added the 3 missing rows (MERGE-PARENT-AUDIT,
  MERGE-SUBMODULE-AUDIT, MERGE-SUBMODULE-EXEC) + DUAL-STATE-MERGE-REVIEW;
  stale `5fde801` row → canonical EXEC `098134e`; archive pointer.
- **notes/ops/INDEX.md**: `optimization-clues` → `OPTIMIZATION-CLUES`;
  archive-dir row added.
- **notes/SESSION-LOG-2026-09-17-18.md**: `ops/optimization-clues.md` →
  `ops/OPTIMIZATION-CLUES.md`; Phase B entry link → `MERGE-SUBMODULE-EXEC.md`
  and hash `5fde801` → `098134e`.
- **notes/session/CONTINUE-2026-09-15.md**: `release/handoffs/...` →
  `../../release/RELEASE-HANDOFF-1.4.3.md`; `release/RELEASE-SCOPE-1.4.3.md`
  → `../../release/...`; retired skill ref `aphrodite-branch-release-flow` →
  `aphrodite-release-flow`.
- **release/RELEASE-TEMPLATE.md**: `Maintain/CHANGELOG.md` →
  `../CHANGELOG.md` (verified: `Maintain/CHANGELOG.md` does not exist;
  CHANGELOG.md is at repo root on both lines).
- **AGENTS.md** (out of scope, one link-fix line only, recorded here):
  `(v1.4.0…v1.4.3-draft)` → `(v1.4.0…v1.4.3)`.

Deliberately left as historical (recorded, not changed): `MERGE-PARENT-EXEC.md`
row 12 and `CONTINUE-2026-09-15.md` line 123 tree mentions (describe the tree
at their time); `HERMES-REWRITE-A.md` `SPLIT-MD-REWRITE.md` mentions
(point-in-time report); `CLAUDE.md` classification row (file deleted
2026-09-17, commit `24c8258`; row is a snapshot record).

## 6. Gaps noted (no fabrication)

- `release-notes/` has no v1.4.4 / v1.4.5 entries (tags `Aphrodite/v1.4.4`
  and `v1.4.5` exist); v1.4.6 is un-released (no tag; bumps local only per
  HANDOFF). Drafting those notes is out of this pass's scope.
- `AGENTS.md` still lists 13 skills while the archive dir now holds 3 retired
  snapshots; skills content is owned by the skills-cleanup wave.

## 7. Final tree outline (in-scope, after)

```
.hermes/
├── AGENTS.md                          (1 line fixed: v1.4.3-draft → v1.4.3)
├── classification/                    6 files - README, TAXONOMY, CUR-plugin-loader,
│                                      CUR-release-infra-identity, DEV-engine-build,
│                                      DEV-knowledge-tests-bench (paths/versions refreshed)
├── notes/
│   ├── README.md                      catalog (counts + blueprint refs fixed)
│   ├── ARCHITECTURE.md  CEREMONY.md  HANDOFF.md  ISSUE-11.md
│   ├── SESSION-LOG-2026-09-17-18.md  HERMES-REWRITE-A.md  HERMES-REWRITE-B.md
│   ├── ffi/      INDEX + 6 reports
│   ├── issue11/  INDEX + 13 reports
│   ├── release/  INDEX + 8 reports (incl. DUAL-STATE-MERGE-REVIEW moved in)
│   ├── session/  INDEX + 4 reports
│   ├── plugin/   INDEX + 5 reports
│   └── ops/      INDEX + 10 reports (OPTIMIZATION-CLUES.md renamed)
│       └── ARCHIVE-2026-09-18/        README + MERGE-SUBMODULE.md + 3 retired-skill dirs
├── uml/          README + 01-11 (unchanged, verified current)
├── release/      RELEASE-TEMPLATE.md, RELEASE-SCOPE-1.4.3.md,
│                 RELEASE-HANDOFF-1.4.3.md (flattened from handoffs/)
├── release-notes/ v1.4.0.md  v1.4.1.md  v1.4.2.md  v1.4.3.md (renamed from draft)
└── scripts/      build-watcher.py  build-watcher.sh (unchanged)
```

## 8. Verification

- Prettier: all touched .md files pass `node node_modules/prettier/bin/prettier.cjs --check` (repo `.prettierrc`: tabs, width 100, proseWrap preserve).
- Link sweep: grep for the old paths (`notes/optimization-clues`,
  `notes/MERGE-SUBMODULE.md`, `release/handoffs/`, `v1.4.3-draft`) leaves only
  intentional historical mentions (recorded in §5).
- Git: 5 renames staged (git mv); no commits, no pushes made.
- Version facts verified against source: crates at 1.4.6
  (`crates/aphrodite/Cargo.toml`, `crates/aphrodite-hermes/Cargo.toml` pin
  `version = "1.4.6"`); tags `Aphrodite/v1.4.0`…`v1.4.5` exist, `v1.4.6` does
  not; submodule log contains `098134e` (final sync-back pick).
