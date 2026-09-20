# Publishing `vendor/headroom` (`aphrodite-headroom-core`) - evidence note

Owner: `aphrodite-release-workflow` (publishing gates; this file is the
evidence for the owned headroom publish). The ceremony that executes the
publish is `Publish.yml` via `gh workflow run Publish -f publish_crates=true`

- never run `cargo publish` locally without `CARGO_REGISTRY_TOKEN`.

## Irreversibility (governs everything below)

- crates.io package versions are **immutable**: a version burned by a bad
  publish can never be reused or retracted. Wrong published version ⇒ release
  a new version; never re-tag, never re-publish.
- `cargo publish` needs `CARGO_REGISTRY_TOKEN` (CI secret). Never run it
  locally without that token.

## Facts (verified 2026-09-20 at parent commit `a81acab6`)

- `vendor/headroom` is a git submodule → `PlayForm/Headroom.git`, branch
  `Current`.
- Publishable crate: `vendor/headroom/crates/headroom-core/Cargo.toml`,
  published under package name **`aphrodite-headroom-core`** (its `name`
  field). Verified version: **`0.1.2`**.
- Parent pin: `crates/aphrodite/Cargo.toml` (line 55):
  `headroom-core = { package = "aphrodite-headroom-core", path = "../../vendor/headroom/crates/headroom-core", version = "0.1.2", default-features = false }`.
- Sibling crates inside the fork reference it via
  `headroom-core = { package = "aphrodite-headroom-core", path = "../headroom-core" }`
  (headroom-ffi, headroom-parity, headroom-proxy, headroom-py).
- `Cargo.lock` regenerates on the next `cargo build` (it tracks the package
  name, not the path); no manual edit needed.

## Publish trigger (dispatch-gated ONLY)

`Publish.yml` job `Publish-Headroom-Core` (`needs: [Test]`,
`working-directory: vendor/headroom`, `environment: Release`):

- The version-check step runs on both events (`workflow_dispatch` with
  `publish_crates` OR tag push).
- The actual publish step is gated on
  `workflow_dispatch && inputs.publish_crates && published == 'false'` -
  **a plain `Aphrodite/v*` tag push NEVER publishes headroom-core** (unlike
  `aphrodite`/`aphrodite-hermes`, whose publish steps carry
  `|| startsWith(github.ref, 'refs/tags/Aphrodite/')` - see the trigger table
  in the owning SKILL.md, Gate R7).

Trigger:

```bash
gh workflow run Publish -f publish_crates=true
```

Dependency order enforced by `needs:`: Test → Publish-Headroom-Core →
Publish-Aphrodite → Publish-Hermes. If headroom-core is not published first,
`Publish-Aphrodite` fails (its `path + version` dep strips the `path` key on
publish, so the matching version must already exist on crates.io).

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
commit in the submodule, then float the parent gitlink before dispatching.

## Post-event consumer verification (publisher's claim is not proof)

- Index serves the exact version: `curl -fsS
https://index.crates.io/ap/hr/aphrodite-headroom-core | grep '"vers"'`.
- A fresh resolve succeeds without the path-only fallback:
  `cargo update -p aphrodite-headroom-core --precise <version>` then
  `cargo tree -i aphrodite-headroom-core` in a clean checkout.
- Binary consumers are unaffected (headroom is a build-time dependency baked
  into the `aphrodite` binary - no runtime download path references it).

## Rename rule

The package name stays `aphrodite-headroom-core` - never rename it. A rename
would orphan the parent pin, break the sibling `package =` aliases, and burn
the crates.io namespace history.
