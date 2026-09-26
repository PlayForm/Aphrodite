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
fork tag before the release chain (`aphrodite-vX.Y.Z`, never `Aphrodite/v*`),
gitlink to the TAGGED fork commit (CI publishes the parent-recorded gitlink
tree, not local submodule HEAD). A stale fork version makes CI skip the
publish silently (1.5.0 trap). Tracking: `vendor/headroom/RELEASE-CYCLE.md` +
`vendor/headroom/CHANGELOG.md`. No dispatch exists: Publish.yml is
`workflow_run`-chained and headroom-core's publish step is unreachable
(`inputs.publish_crates` gate, input removed at 677a7b3) -
references/headroom-publish.md.

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

## Version claims (last recorded - re-derive live)

CLAIM (re-derive live at ceremony time): binary `1.6.2`, plugin `2.2.2`,
`BINARY_VERSION` `1.6.2`.

## Phase rules - full text (moved from SKILL.md)

SKILL.md holds the one-line rules; this section keeps the full bullets moved
out during the compression pass.

### Prepare (Development)

- Version must be free: a burned crates.io version is gone forever - claim the NEXT number; never tag before free. Ledger rows = authority path (Step P1).
- Binary track moves in ONE ceremony (cargo check fails if one crate moved alone); `BINARY_VERSION` moves LAST at tag time.
- Templates match live config: `plugins/aphrodite/__init__.py` byte-identical to `crates/aphrodite/templates/__init__.py` (setup.rs asserts it). Crate README links render as blob/HEAD - write absolute to the file's OWN branch.

### Validate

- Record ACTUAL gate output ("406 passed, 0 failed") - never "should pass"; a red gate voids the release claim.
- Runtime evidence from a FRESH process: a stale dylib is not evidence; restart, then re-probe (`aphrodite_stats`/`aphrodite_test`/`aphrodite_rebuild`).

### Integrate (Development → Current)

- **B4 branch-identity audit - MANDATORY before ANY sync or tag.** Scans workflow triggers + push targets, `.gitmodules` branch fields, gitlink resolution - zero hits required; ANY hit ABORTS (record in `.hermes/notes/release/CEREMONY-AUDIT.md`; never checkout the other branch). Commands: ceremony-steps.md Step I1.
- **Submodule-first (bottom-up):** plugin FIRST, then parent; validate the plugin commit before the parent gitlink moves; never float the gitlink to a non-released plugin commit.
- **Controlled restore is identity-only:** `git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite` - ceremony invariant, NOT repair; never blanket checkout/reset.
- **Headroom fork leg (mandatory):** fork delta → fork crate + parent pin TOGETHER (cargo check fails otherwise), fork tag before the release chain (the parent tag push freezes the gitlink CI publishes; `aphrodite-vX.Y.Z`, never `Aphrodite/v*`), gitlink to the TAGGED fork commit (CI publishes the parent-recorded gitlink tree). Stale fork version → CI skips publish silently (1.5.0 trap). Tracking: compressed-detail.md.

### Release

- **4 irreversible events, never combined:** (1) release-sync commit (I4), (2) immutable tag (R4), (3) artifacts (R5, Build.yml attaches), (4) registry (R6, cargo publish). Each requires identity confirmation, version availability, intended list, Gate R7, human approval (`Ready for approval` pause), consumer verification.
- **Gate R7 at the exact tag commit:** read the ACTUAL workflow files - never trust remembered behavior. Tag push publishes `aphrodite` + `aphrodite-hermes` (no already-published check); `aphrodite-headroom-core`'s publish step is unreachable - gated on the REMOVED `workflow_dispatch publish_crates` input, so the tag chain never publishes it. Unexpected tag-reachable publish → stop. Commands: release-steps.md Step R2.
- **Never re-tag; never move the tag.** Immutable evidence on the exact release-sync commit; re-tagging re-fires Build/Publish.
- **`BINARY_VERSION` bumps LAST (after assets exist):** a pre-asset bump 404s every download; `_check_version_published` warning = hard stop.
- **Registry:** the tag → Build → Publish chain already ran `cargo publish` for `aphrodite`/`aphrodite-hermes` - verify via crates.io API `max_version`, never re-trigger (commands: release-steps.md Step R6; headroom-core's publish step is unreachable - headroom-publish.md); failed publish → release a NEW version.
- **Artifacts:** Finalize fails loudly on missing assets; 12 = 4 targets × (`aphrodite-<t>` + `libaphrodite_hermes-<t>.{so,dylib,dll}` + `SHA256SUMS-<t>.txt`). BODY amendable, tag not; `--notes-file`, never inline backticks.

### Observe

- `aphrodite_stats` is ground truth; the banner is NOT - never declare the release verified on a banner alone (stale dylib).
- Round trips: `aphrodite_test` (quick=1/full=3), `aphrodite_catalog`, `aphrodite_diff`, `aphrodite_directive list`. `catalog` populated + `diff` empty is EXPECTED; counter reset after the bump is NORMAL. Terminal output is CCR-compressed - scan for `<<<CCR:` and retrieve before reading on.
- Proxies down + inline-only is the user's ACCEPTED state - no proxy-restart default fix. Misleading preview = BUG, not cosmetic (preview-quality-debugging.md).

### Recover

- Classify by ONE observed symptom first (runbooks: engine-health-debugging.md, plugin-lifecycle.md, preview-quality-debugging.md); never change more than one dimension at a time.
- Repair taxonomy (`aphrodite-boundaries`): wrong content → edit directly; identity crossed → restore named protected paths; conflict → resolve + marker sweep; empty pick → verify + skip; phantom → remove the indexed mode-160000 entry; wrong version → release a new version.
- Destructive shortcuts prohibited: blanket checkout/reset, force-push, retag, hook re-creation, phantom-gitlink ignore.
- Behavior ≠ docs → update the canonical owner skill + test matrix.
