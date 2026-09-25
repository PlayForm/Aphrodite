# Compressed detail - worked mechanics moved out of SKILL.md

Detail stripped from SKILL.md during the compression pass. SKILL.md holds the
one-line rule; this file holds the worked mechanics, probes, and matrices.

## Version ledger drift (worked example)

Ledger rows drift - every row must equal its authority path (Step P1). At
`a81acab6` `package.json` lagged at `1.4.6` while both crates read `1.5.0` -
fix the manifest before claiming a release and report the drift. Ledger:
`aphrodite-release-workflow` §1 (5 rows).

## Bump order (worked detail)

The two parent crates + the `aphrodite-hermes` dep pin
`aphrodite = { path = .., version = "X" }` + `package.json` move TOGETHER
(cargo check fails otherwise); then `plugin.yaml` + `install_message` +
README badges (the badge may lag two minors); then `BINARY_VERSION` LAST at
tag time. A LOCAL `BINARY_VERSION` bump ahead of the tag is safe only when
the binaries already sit in `~/.hermes/aphrodite/binaries` (`_ensure_binaries`
no-ops) - the "bump LAST" rule applies at TAG time, not local prep. Grep the
OLD version strings after; update any test that pins them.

## B4 branch-identity audit (expanded)

MANDATORY before ANY sync or tag. Contract points: I1 before staging, I4
after the controlled restore, R1 before tag; canonical text:
`.hermes/notes/release/RELEASE-METHODOLOGY.md` `### B4`, invariant I11. It
scans workflow triggers + push targets, `.gitmodules` branch fields, gitlink
resolution, keywords - zero hits required. ANY hit ABORTS the ceremony;
record in `.hermes/notes/release/CEREMONY-AUDIT.md`; fix the offending branch
separately; never proceed past a hit "because it is only the heartbeat";
never checkout the other branch. Commands: references/ceremony-steps.md Step
I1.

## Headroom fork leg (expanded)

Fork delta → fork crate + parent pin TOGETHER (cargo check fails otherwise),
fork tag BEFORE dispatch (`aphrodite-vX.Y.Z`, never `Aphrodite/v*`), gitlink
to the TAGGED fork commit (CI publishes the parent-recorded gitlink tree, not
local submodule HEAD). A stale fork version makes CI skip the publish
silently (1.5.0 trap). Tracking: `vendor/headroom/RELEASE-CYCLE.md` +
`vendor/headroom/CHANGELOG.md`. Dispatch in R6 only after this leg.

## Artifact matrix (expanded)

Finalize fails loudly on missing assets; 12 = 4 targets × (`aphrodite-<t>` +
`libaphrodite_hermes-<t>.{so,dylib,dll}` + `SHA256SUMS-<t>.txt`). BODY
amendable, tag not; `--notes-file`, never inline backticks.

## Hotfix sync-back pick/skip lists (expanded)

PICK real fixes: setup.rs changes, config-template refresh, docs URL fixes,
release-notes finalization, version bumps that ride the line. SKIP:
gitlink-only bumps (the gitlink is branch-owned, Development floats its own),
style-only commits on rewritten files, snapshots that re-add removed content
(e.g. `directives/`) or delete test files. Never merge Current wholesale;
never rebase picks.