# Publishing `vendor/headroom` (`aphrodite-headroom-core`) - evidence note

Owner: `aphrodite-release-workflow` (publishing gates; this file is the
evidence for the owned headroom publish). The ceremony that executes the
publish is `Publish.yml`, triggered via `workflow_run` on Build completion
(tag `Aphrodite/v*` → Build success → Publish; no dispatch input exists)

- never run `cargo publish` locally without `CARGO_REGISTRY_TOKEN`. The
  release ceremony (`aphrodite-release-flow` Step I5) carries the fork into
  EVERY parent release cycle; this note is the evidence + checklist for that
  leg.

## Irreversibility (governs everything below)

- crates.io package versions are **immutable**: a version burned by a bad
  publish can never be reused or retracted. Wrong published version ⇒ release
  a new version; never re-tag, never re-publish.
- `cargo publish` needs `CARGO_REGISTRY_TOKEN` (CI secret). Never run it
  locally without that token.

## Facts (live state, re-derive at release time)

- `vendor/headroom` is a git submodule → `PlayForm/Headroom.git`, branch
  `Current` (remote `Source`).
- Fork HEAD (observed 2026-09-20, parent Development `a81acab6`):
  `02706ea1a3dcd9956ae8cbba4d17e19d6ab174f1` (`02706ea1`).
- Publishable crate: `vendor/headroom/crates/headroom-core/Cargo.toml`,
  published under package name **`aphrodite-headroom-core`** (its `name`
  field). Crate version on the fork tree: **`0.1.2`** (live on crates.io,
  published 2026-07-14; 0.1.1 on 07-13). **`0.1.3` is the planned next
  bump** - the fork holds ~14 unpublished fork commits since the 0.1.2 bump.
- Last published fork commit: `c6b61470` ("fix(package): update repository
  URL and bump version to 0.1.2") - the delta-check comparison base and the
  version the crates.io index serves.
- Parent pin: `crates/aphrodite/Cargo.toml` (line 55 - observational):
  `headroom-core = { package = "aphrodite-headroom-core", path = "../../vendor/headroom/crates/headroom-core", version = "0.1.2", default-features = false }`.
- Sibling crates inside the fork reference it via
  `headroom-core = { package = "aphrodite-headroom-core", path = "../headroom-core" }`
  (headroom-ffi, headroom-parity, headroom-proxy, headroom-py).
- `Cargo.lock` regenerates on the next `cargo build` (it tracks the package
  name, not the path); no manual edit needed.
- Fork release tags use the fork's OWN scheme `aphrodite-vX.Y.Z` (last:
  `aphrodite-v0.9.4`); **proposed next fork tag: `aphrodite-v0.10.0`** -
  NEVER the parent `Aphrodite/v*` scheme.

## Mandatory release-cycle tracking (every parent release)

- EVERY parent release cycle compares the fork's current HEAD against the
  last published commit and carries any delta into the release:
  `git -C vendor/headroom log c6b61470..HEAD --oneline` (last published =
  the commit carrying the version live on crates.io).
- The fork change ledger: `vendor/headroom/CHANGELOG.md` +
  `vendor/headroom/RELEASE-CYCLE.md` - the fork's release record; update
  both per cycle.

## Bump BEFORE the release chain (or CI skips you)

`Publish.yml` job `Publish-Headroom-Core` (`needs: [Test]`,
`working-directory: vendor/headroom`, `environment: Release`) reads the fork
crate version and checks it against the crates.io index; the publish step is
gated on the REMOVED `workflow_dispatch publish_crates` input and never
fires (lines 159-177 - observational; read the `if:` conditions at the tag
commit):

```yaml
- name: Check if aphrodite-headroom-core version is already published
  if: ${{ startsWith(github.event.workflow_run.head_branch, 'Aphrodite/v') }}
  id: check
  working-directory: vendor/headroom
  run: |
      VERSION=$(grep '^version' crates/headroom-core/Cargo.toml | head -1 | sed 's/.*"\(.*\)"/\1/')
      echo "version=$VERSION" >> "$GITHUB_OUTPUT"
      if curl -sL "https://index.crates.io/ap/hr/aphrodite-headroom-core" | grep -q "\"vers\":\"$VERSION\""; then
        echo "published=true" >> "$GITHUB_OUTPUT"
      else
        echo "published=false" >> "$GITHUB_OUTPUT"
      fi

- name: Publish aphrodite-headroom-core to crates.io (opt-in)
  if: ${{ github.event_name == 'workflow_dispatch' && inputs.publish_crates && steps.check.outputs.published == 'false' }}
  working-directory: vendor/headroom
  run: cargo publish -p aphrodite-headroom-core --no-verify
  env:
      CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
```

Rule: **bump the fork crate version BEFORE the release chain (before the
parent tag push).** The check reads `crates/headroom-core/Cargo.toml` at the
parent-recorded gitlink the tag pins; if that version is already in the
index the publish step is skipped (`published == 'false'` is false) - and
even if it is not, the publish step is unreachable (removed dispatch input).
A stale fork version therefore publishes NOTHING - the job goes green and
the release silently ships the old crate (or `Publish-Aphrodite` fails when
the bumped version is not live). Bump fork crate + parent pin together,
float the gitlink, THEN tag.

The publish step is gated on
`github.event_name == 'workflow_dispatch' && inputs.publish_crates &&
published == 'false'` - and `Publish.yml` no longer declares
`workflow_dispatch` (the input was removed at 677a7b3), so the condition is
ALWAYS false: **the chain NEVER publishes headroom-core**.
`aphrodite`/`aphrodite-hermes` publish steps are gated on
`startsWith(github.event.workflow_run.head_branch, 'Aphrodite/v')` and DO
run on the tag chain - see the trigger table in the owning SKILL.md, Gate
R7.

Trigger:

NO dispatch exists. `Publish.yml` runs via `workflow_run` on Build
completion, gated on `conclusion == 'success' && startsWith(head_branch,
'Aphrodite/v')`, with checkouts pinned to the triggering tag. The
headroom-core publish step never fires under this chain.

Dependency order enforced by `needs:`: Test → Publish-Headroom-Core →
Publish-Aphrodite → Publish-Hermes. If the fork version is bumped and is not
live on crates.io, `Publish-Aphrodite` fails (its `path + version` dep
strips the `path` key on publish, so the matching version must already exist
on crates.io) - the chain cannot publish the fork itself.

## Fork tag convention (create BEFORE the release chain)

- Fork tags use `aphrodite-vX.Y.Z` in the fork repo (PlayForm/Headroom,
  branch `Current`) - NEVER the parent `Aphrodite/v*` scheme (last fork tag:
  `aphrodite-v0.9.4`; proposed next: `aphrodite-v0.10.0`).
- Create the fork tag BEFORE the release chain - the parent tag push freezes
  the gitlink CI publishes, so the parent gitlink must float to the tagged
  fork commit carrying the new version (see Gitlink trap below).

## 1.5.0 gap - canonical failure (the published-version trap)

- `aphrodite-headroom-core` 0.1.2 published to crates.io **2026-07-14**
  (0.1.1 on 07-13).
- The 1.5.0 release published `aphrodite` 1.5.0 + `aphrodite-hermes` 1.5.0
  but SKIPPED the headroom publish: the version check saw 0.1.2 already live
  → `published=true` → the publish step condition
  (`workflow_dispatch && publish_crates && published == 'false'`) evaluated
  false → skip.
- The fork meanwhile held ~14 unpublished commits since the 0.1.2 bump
  (`c6b61470` → `02706ea1`): upstream `headroomlabs-ai/headroom@main` merge
  `43dc9836` (Aug 7), ml feature default-off, package-name reference fix,
  ml-cluster dep pin, Sep 18-19 nightly-toolchain syntax batch.
- **Nothing failed, no red job** - a silent gap. External consumers of
  `aphrodite` 1.5.0 resolved the OLD 0.1.2. The version number alone lies:
  "already published" ≠ "fork tree published". This is why the delta check is
  mandatory.

## Version availability check (read-only, before claiming a number)

- Index (path = first 2 / next 2 chars of the crate name):
  `https://index.crates.io/ap/hr/aphrodite-headroom-core` - `404` = not
  published (CI will attempt it); `200` containing `"vers":"X.Y.Z"` = that
  version is live (CI skips it).
- API: `curl -A <ua> https://crates.io/api/v1/crates/aphrodite-headroom-core`
  → `max_version` (the API rejects requests without a User-Agent).

Never reuse a claimed version; if a parallel release won the race, claim the
next number.

## Gitlink trap (critical)

CI checks out the submodule at the **parent's recorded gitlink**, not the
local submodule HEAD. To publish a changed headroom tree:

```bash
cd <Aphrodite>
git add vendor/headroom
git commit -m "build(submodule): point vendor/headroom at <new headroom commit>"
git push Source Current
```

If you skip this, `Publish.yml` publishes the previously-recorded commit's
tree. This is the ceremony's submodule-first rule applied to the fork: bump +
commit in the submodule, then float the parent gitlink before the release
chain (the parent tag push).

## Post-event consumer verification (publisher's claim is not proof)

- Index serves the EXACT NEW version - the check that catches the trap
  recurring (seeing only the old 0.1.2 means the skip fired again):
  `curl -fsS https://index.crates.io/ap/hr/aphrodite-headroom-core | grep '"vers"'`.
- API `max_version` equals the released version:
  `curl -A <ua> https://crates.io/api/v1/crates/aphrodite-headroom-core | grep max_version`.
- A fresh resolve succeeds without the path-only fallback:
  `cargo update -p aphrodite-headroom-core --precise <version>` then
  `cargo tree -i aphrodite-headroom-core` in a clean checkout.
- Binary consumers are unaffected (headroom is a build-time dependency baked
  into the `aphrodite` binary - no runtime download path references it).

## Rename rule

The package name stays `aphrodite-headroom-core` - never rename it. A rename
would orphan the parent pin, break the sibling `package =` aliases, and burn
the crates.io namespace history.
