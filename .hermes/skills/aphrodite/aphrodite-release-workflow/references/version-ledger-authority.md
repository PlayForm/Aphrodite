# Version Ledger - canonical 5-row contract + authority paths

Owner: `aphrodite-release-workflow` (section 1 of the owning SKILL.md). The
same rows live in `.hermes/governance/VERIFICATION-MATRIX.md` - this skill is
their owner, keep them mirrored.

The binary release version, the plugin package version, and `BINARY_VERSION`
are **three different values with three different owners** - never described
as one generic "version bump." This ledger is the canonical 5-row contract.
Authority, earliest update, latest safe update, and verification are per row:

| Field                | Meaning                                | Authority            | Earliest update                            | Latest safe update                | Verification                                 |
| -------------------- | -------------------------------------- | -------------------- | ------------------------------------------ | --------------------------------- | -------------------------------------------- |
| Binary version       | Published Rust artifact identity       | Parent release owner | Development prep                           | Release-sync commit               | Binary `--version`, manifests, artifact name |
| Plugin version       | Plugin package identity                | Plugin owner         | Plugin prep                                | Plugin Current release commit     | `plugin.yaml`, install metadata              |
| `BINARY_VERSION`     | Binary expected by plugin installer    | Plugin release owner | Local prep only if artifact already exists | After artifact/tag availability   | Consumer download resolution                 |
| Parent gitlink       | Exact plugin commit consumed by parent | Parent Current owner | Parent sync                                | Before parent release tag         | Submodule status/tree entry                  |
| README badge/example | Documentation claim                    | Documentation owner  | After authoritative value changes          | Before release notes finalization | Render/source scan                           |

## Real authority paths (read live, never a doc number)

| Field            | Authority path                                                                                                                                                                                                                 | Verified value (2026-09-25)                                                                                                                                           |
| ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Binary version   | `crates/aphrodite/Cargo.toml` `version`; `crates/aphrodite-hermes/Cargo.toml` `version` **and** its `aphrodite = { path = "../aphrodite", version = "X", ... }` dep pin; `package.json` `version` (carries the binary version) | `1.6.2` in both crates + hermes pin; `package.json` must match (historical drift at `a81acab6`: lagged at `1.4.6` vs `1.5.0`); the four move together in one ceremony |
| Plugin version   | `plugins/aphrodite/plugin.yaml` `version` + the `install_message` block                                                                                                                                                        | `2.2.2`                                                                                                                                                               |
| `BINARY_VERSION` | `plugins/aphrodite/BINARY_VERSION` (read by `download.sh` / `download.ps1`)                                                                                                                                                    | `1.6.2`                                                                                                                                                               |
| Parent gitlink   | `git submodule status plugins/aphrodite`; `git ls-tree HEAD plugins/aphrodite`                                                                                                                                                 | re-derive live (submodule tracks the plugin's `Development` branch, remote `Source`)                                                                                  |
| README badge     | `README.md` release badge, plugin badge, example health output                                                                                                                                                                 | re-derive live; must equal the manifests (release badge = binary version, plugin badge = plugin version)                                                              |

Snapshot values are **evidence, not authority**: re-derive every row from its
authority path at release time. If a row disagrees with its authority (the
`package.json` lag above), fix the manifest before claiming a release; report
the drift.

## Ledger rules

- **Local bump can be safe, a public distribution pointer must not be early.**
  A local `BINARY_VERSION` bump ahead of the tag is safe when the referenced
  binaries already exist locally (the plugin's `_ensure_binaries` no-ops and
  `download.sh` has nothing to fetch). The "bump LAST" ceremony rule applies
  at **tag time**: a plugin whose `BINARY_VERSION` names a release that does
  not exist yet errors on every download. The plugin's
  `_check_version_published` warns before `download.sh` when the pinned
  `BINARY_VERSION` points at a release with no assets - treat that warning as
  a hard stop for tagging.
- **Two version tracks, never conflated:** binary `1.6.x` (parent
  crates + `package.json` + README badge) vs plugin `2.2.x` (
  `plugin.yaml`). A binary bump and a plugin bump are separate ceremonies
  (separate repos), even though they ride the same release.
- **Bump together (binary track):** both parent crates + the
  `aphrodite-hermes` dep pin + `package.json` must move in one ceremony
  (`cargo check` fails otherwise). Then `plugin.yaml` + `install_message` +
  README badges (they drift - the badge may lag two minors), then
  `BINARY_VERSION` LAST. The ceremony in `aphrodite-release-flow` executes
  this order; this ledger is the check it runs against.
- **Never reuse a claimed version:** check the registry before claiming a
  number (Gate R7 / owning SKILL.md section 3); a burned crates.io version
  is gone forever.